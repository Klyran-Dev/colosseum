#![deny(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

//! Server-owned, signed foreign-exchange evidence.
//!
//! This bounded context verifies immutable provider envelopes without reading
//! a clock or calling a network. Application adapters provide the merchant and
//! facts digests, current time, and atomically rotated provider-key snapshot.

use arc_swap::ArcSwap;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Arc};
use tax_engine::{FxConversionEvidence, MAX_ATOMIC_AMOUNT, RoundingMode};
use thiserror::Error;

pub type Digest32 = [u8; 32];
pub const FX_EVIDENCE_SCHEMA_VERSION: u16 = 1;
pub const MAX_PROVIDER_KEYS: usize = 32;
pub const MAX_PROVIDER_REGISTRY_DOCUMENT_BYTES: usize = 64 * 1024;
pub const MAX_CODE_BYTES: usize = 64;
pub const MAX_QUOTE_ID_BYTES: usize = 128;
pub const MAX_CURRENCY_CODE_BYTES: usize = 16;
pub const MAX_EVIDENCE_LIFETIME_SECONDS: i64 = 86_400;
const EVIDENCE_DOMAIN_SEPARATOR: &[u8] = b"solana-tax-protocol/fx-evidence/v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnsignedFxEvidence {
    pub schema_version: u16,
    pub merchant_id: Digest32,
    pub facts_digest: Digest32,
    pub source_currency: String,
    pub target_currency: String,
    #[serde(with = "tax_engine::serde_i128_string")]
    pub numerator: i128,
    #[serde(with = "tax_engine::serde_i128_string")]
    pub denominator: i128,
    pub rounding_mode: RoundingMode,
    #[serde(with = "tax_engine::serde_i128_string")]
    pub quantum_atomic: i128,
    pub provider: String,
    pub quote_id: String,
    pub observed_at_unix: i64,
    pub expires_at_unix: i64,
    pub evidence_digest: Digest32,
    pub provider_key_id: Digest32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FxEvidenceEnvelope {
    pub unsigned: UnsignedFxEvidence,
    pub provider_public_key: [u8; 32],
    pub payload_digest: Digest32,
    pub signature: Vec<u8>,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum FxEvidenceError {
    #[error("invalid FX evidence: {0}")]
    InvalidEvidence(&'static str),
    #[error("unsupported FX evidence schema version")]
    UnsupportedSchema,
    #[error("FX evidence validity window is invalid or too long")]
    InvalidValidityWindow,
    #[error("FX provider key is invalid")]
    InvalidProviderKey,
    #[error("FX evidence does not bind to the request")]
    EvidenceBindingMismatch,
    #[error("FX evidence payload digest does not match")]
    PayloadDigestMismatch,
    #[error("FX evidence signature is invalid")]
    InvalidSignature,
    #[error("FX evidence is outside its validity window")]
    EvidenceOutsideValidityWindow,
    #[error("FX provider key is unknown")]
    UnknownProviderKey,
    #[error("FX provider key registry is empty")]
    EmptyRegistry,
    #[error("FX provider key registry is invalid: {0}")]
    InvalidRegistry(String),
}

impl UnsignedFxEvidence {
    pub fn validate(&self) -> Result<(), FxEvidenceError> {
        if self.schema_version != FX_EVIDENCE_SCHEMA_VERSION {
            return Err(FxEvidenceError::UnsupportedSchema);
        }
        if self.merchant_id == [0; 32]
            || self.facts_digest == [0; 32]
            || self.evidence_digest == [0; 32]
            || self.provider_key_id == [0; 32]
        {
            return Err(FxEvidenceError::InvalidEvidence(
                "identity and evidence digests must be non-zero",
            ));
        }
        validate_currency(&self.source_currency)?;
        validate_currency(&self.target_currency)?;
        if self.source_currency == self.target_currency {
            return Err(FxEvidenceError::InvalidEvidence(
                "source and target currencies must differ",
            ));
        }
        if self.numerator <= 0 || self.denominator <= 0 {
            return Err(FxEvidenceError::InvalidEvidence("rate must be positive"));
        }
        if self.quantum_atomic <= 0 || self.quantum_atomic > MAX_ATOMIC_AMOUNT {
            return Err(FxEvidenceError::InvalidEvidence(
                "rounding quantum must be positive and bounded",
            ));
        }
        validate_bounded_text(&self.provider, MAX_CODE_BYTES, "provider")?;
        validate_bounded_text(&self.quote_id, MAX_QUOTE_ID_BYTES, "quote identity")?;
        if self.observed_at_unix < 0
            || self.expires_at_unix <= self.observed_at_unix
            || self
                .expires_at_unix
                .checked_sub(self.observed_at_unix)
                .is_none_or(|lifetime| lifetime > MAX_EVIDENCE_LIFETIME_SECONDS)
        {
            return Err(FxEvidenceError::InvalidValidityWindow);
        }
        Ok(())
    }

    pub fn canonical_payload(&self) -> Result<Vec<u8>, FxEvidenceError> {
        self.validate()?;
        let mut encoder = PayloadEncoder::new();
        encoder.bytes(EVIDENCE_DOMAIN_SEPARATOR);
        encoder.u16(self.schema_version);
        encoder.bytes(&self.merchant_id);
        encoder.bytes(&self.facts_digest);
        encoder.text(&self.source_currency)?;
        encoder.text(&self.target_currency)?;
        encoder.i128(self.numerator);
        encoder.i128(self.denominator);
        encoder.u8(rounding_mode_tag(self.rounding_mode));
        encoder.i128(self.quantum_atomic);
        encoder.text(&self.provider)?;
        encoder.text(&self.quote_id)?;
        encoder.i64(self.observed_at_unix);
        encoder.i64(self.expires_at_unix);
        encoder.bytes(&self.evidence_digest);
        encoder.bytes(&self.provider_key_id);
        Ok(encoder.finish())
    }
}

impl FxEvidenceEnvelope {
    pub fn sign(
        unsigned: UnsignedFxEvidence,
        signing_key: &SigningKey,
    ) -> Result<Self, FxEvidenceError> {
        let payload = unsigned.canonical_payload()?;
        let payload_digest: Digest32 = Sha256::digest(&payload).into();
        Ok(Self {
            unsigned,
            provider_public_key: signing_key.verifying_key().to_bytes(),
            payload_digest,
            signature: signing_key.sign(&payload).to_bytes().to_vec(),
        })
    }

    pub fn verify_with_key_at(
        &self,
        expected_key: &VerifyingKey,
        merchant_id: Digest32,
        facts_digest: Digest32,
        now_unix: i64,
    ) -> Result<FxConversionEvidence, FxEvidenceError> {
        self.verify_with_key_at_and_future_skew(
            expected_key,
            merchant_id,
            facts_digest,
            now_unix,
            0,
        )
    }

    pub fn verify_with_key_at_and_future_skew(
        &self,
        expected_key: &VerifyingKey,
        merchant_id: Digest32,
        facts_digest: Digest32,
        now_unix: i64,
        max_future_skew_seconds: i64,
    ) -> Result<FxConversionEvidence, FxEvidenceError> {
        self.unsigned.validate()?;
        if max_future_skew_seconds < 0 {
            return Err(FxEvidenceError::InvalidValidityWindow);
        }
        if self.provider_public_key != expected_key.to_bytes()
            || self.unsigned.merchant_id != merchant_id
            || self.unsigned.facts_digest != facts_digest
        {
            return Err(FxEvidenceError::EvidenceBindingMismatch);
        }
        let future_skew = self
            .unsigned
            .observed_at_unix
            .checked_sub(now_unix)
            .unwrap_or(i64::MAX);
        if future_skew > max_future_skew_seconds || now_unix >= self.unsigned.expires_at_unix {
            return Err(FxEvidenceError::EvidenceOutsideValidityWindow);
        }
        let payload = self.unsigned.canonical_payload()?;
        let expected_digest: Digest32 = Sha256::digest(&payload).into();
        if self.payload_digest != expected_digest {
            return Err(FxEvidenceError::PayloadDigestMismatch);
        }
        let signature = Signature::from_slice(&self.signature)
            .map_err(|_| FxEvidenceError::InvalidSignature)?;
        expected_key
            .verify(&payload, &signature)
            .map_err(|_| FxEvidenceError::InvalidSignature)?;
        Ok(FxConversionEvidence {
            source_currency: self.unsigned.source_currency.clone(),
            target_currency: self.unsigned.target_currency.clone(),
            numerator: self.unsigned.numerator,
            denominator: self.unsigned.denominator,
            rounding_mode: self.unsigned.rounding_mode,
            quantum_atomic: self.unsigned.quantum_atomic,
            provider: self.unsigned.provider.clone(),
            quote_id: self.unsigned.quote_id.clone(),
            observed_at_unix: self.unsigned.observed_at_unix,
            evidence_digest: self.unsigned.evidence_digest,
        })
    }
}

#[derive(Clone, Debug)]
pub struct FxProviderKeyRegistry {
    keys: Arc<ArcSwap<HashMap<Digest32, VerifyingKey>>>,
}

#[derive(Debug, Deserialize)]
struct RegistryDocument {
    keys: Vec<RegistryKey>,
}

#[derive(Debug, Deserialize)]
struct RegistryKey {
    key_id: String,
    public_key: String,
}

impl FxProviderKeyRegistry {
    pub fn from_json(document: &str) -> Result<Self, FxEvidenceError> {
        let keys = parse_registry(document)?;
        Ok(Self {
            keys: Arc::new(ArcSwap::from_pointee(keys)),
        })
    }

    pub fn reload_json(&self, document: &str) -> Result<usize, FxEvidenceError> {
        let keys = parse_registry(document)?;
        let key_count = keys.len();
        self.keys.store(Arc::new(keys));
        Ok(key_count)
    }

    pub fn key_count(&self) -> usize {
        self.keys.load().len()
    }

    pub fn verify(
        &self,
        envelope: &FxEvidenceEnvelope,
        merchant_id: Digest32,
        facts_digest: Digest32,
        now_unix: i64,
    ) -> Result<FxConversionEvidence, FxEvidenceError> {
        let keys = self.keys.load();
        let key = keys
            .get(&envelope.unsigned.provider_key_id)
            .ok_or(FxEvidenceError::UnknownProviderKey)?;
        envelope.verify_with_key_at(key, merchant_id, facts_digest, now_unix)
    }
}

fn parse_registry(document: &str) -> Result<HashMap<Digest32, VerifyingKey>, FxEvidenceError> {
    if document.len() > MAX_PROVIDER_REGISTRY_DOCUMENT_BYTES {
        return Err(FxEvidenceError::InvalidRegistry(
            "registry exceeds the bounded document size".to_string(),
        ));
    }
    let document: RegistryDocument = serde_json::from_str(document)
        .map_err(|error| FxEvidenceError::InvalidRegistry(format!("JSON: {error}")))?;
    if document.keys.is_empty() || document.keys.len() > MAX_PROVIDER_KEYS {
        return Err(FxEvidenceError::EmptyRegistry);
    }
    let mut keys = HashMap::with_capacity(document.keys.len());
    for entry in document.keys {
        let key_id = decode_hex_digest(&entry.key_id)?;
        let public_key_bytes = URL_SAFE_NO_PAD
            .decode(entry.public_key.as_bytes())
            .map_err(|_| {
                FxEvidenceError::InvalidRegistry("public_key must be base64url".to_string())
            })?;
        let public_key: [u8; 32] = public_key_bytes.try_into().map_err(|_| {
            FxEvidenceError::InvalidRegistry("public_key must contain 32 bytes".to_string())
        })?;
        let verifying_key = VerifyingKey::from_bytes(&public_key)
            .map_err(|_| FxEvidenceError::InvalidProviderKey)?;
        if keys.insert(key_id, verifying_key).is_some() {
            return Err(FxEvidenceError::InvalidRegistry(
                "duplicate provider key_id".to_string(),
            ));
        }
    }
    Ok(keys)
}

fn decode_hex_digest(value: &str) -> Result<Digest32, FxEvidenceError> {
    if value.len() != 64 {
        return Err(FxEvidenceError::InvalidRegistry(
            "key_id must contain 64 hexadecimal characters".to_string(),
        ));
    }
    let mut digest = [0_u8; 32];
    for (index, output) in digest.iter_mut().enumerate() {
        let high = hex_nibble(value.as_bytes()[index * 2]).ok_or_else(|| {
            FxEvidenceError::InvalidRegistry("key_id must be hexadecimal".to_string())
        })?;
        let low = hex_nibble(value.as_bytes()[index * 2 + 1]).ok_or_else(|| {
            FxEvidenceError::InvalidRegistry("key_id must be hexadecimal".to_string())
        })?;
        *output = (high << 4) | low;
    }
    if digest == [0; 32] {
        return Err(FxEvidenceError::InvalidRegistry(
            "key_id must be non-zero".to_string(),
        ));
    }
    Ok(digest)
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn validate_currency(value: &str) -> Result<(), FxEvidenceError> {
    validate_bounded_text(value, MAX_CURRENCY_CODE_BYTES, "currency")
}

fn validate_bounded_text(
    value: &str,
    max_bytes: usize,
    label: &'static str,
) -> Result<(), FxEvidenceError> {
    if value.trim().is_empty() || value.len() > max_bytes {
        return Err(FxEvidenceError::InvalidEvidence(label));
    }
    Ok(())
}

fn rounding_mode_tag(mode: RoundingMode) -> u8 {
    match mode {
        RoundingMode::Floor => 0,
        RoundingMode::Ceil => 1,
        RoundingMode::HalfUp => 2,
        RoundingMode::HalfEven => 3,
        RoundingMode::AwayFromZero => 4,
    }
}

struct PayloadEncoder {
    bytes: Vec<u8>,
}

impl PayloadEncoder {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn bytes(&mut self, value: &[u8]) {
        self.bytes
            .extend_from_slice(&(value.len() as u32).to_be_bytes());
        self.bytes.extend_from_slice(value);
    }

    fn text(&mut self, value: &str) -> Result<(), FxEvidenceError> {
        if value.len() > u32::MAX as usize {
            return Err(FxEvidenceError::InvalidEvidence("text is too large"));
        }
        self.bytes(value.as_bytes());
        Ok(())
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn i128(&mut self, value: i128) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use tax_engine::{FxConversionEvidence, RoundingMode};

    fn sample_unsigned(provider_key_id: Digest32) -> UnsignedFxEvidence {
        UnsignedFxEvidence {
            schema_version: FX_EVIDENCE_SCHEMA_VERSION,
            merchant_id: [7; 32],
            facts_digest: [8; 32],
            source_currency: "USD".to_string(),
            target_currency: "EUR".to_string(),
            numerator: 9,
            denominator: 8,
            rounding_mode: RoundingMode::HalfEven,
            quantum_atomic: 1,
            provider: "provider-a".to_string(),
            quote_id: "quote-1".to_string(),
            observed_at_unix: 1_800_000_000,
            expires_at_unix: 1_800_000_300,
            evidence_digest: [9; 32],
            provider_key_id,
        }
    }

    #[test]
    fn signed_envelope_verifies_and_maps_exact_evidence() {
        let signing_key = SigningKey::from_bytes(&[3; 32]);
        let envelope = FxEvidenceEnvelope::sign(sample_unsigned([4; 32]), &signing_key)
            .expect("signed fixture");
        let mapped = envelope
            .verify_with_key_at(
                &signing_key.verifying_key(),
                [7; 32],
                [8; 32],
                1_800_000_100,
            )
            .expect("verified fixture");
        assert_eq!(
            mapped,
            FxConversionEvidence {
                source_currency: "USD".to_string(),
                target_currency: "EUR".to_string(),
                numerator: 9,
                denominator: 8,
                rounding_mode: RoundingMode::HalfEven,
                quantum_atomic: 1,
                provider: "provider-a".to_string(),
                quote_id: "quote-1".to_string(),
                observed_at_unix: 1_800_000_000,
                evidence_digest: [9; 32],
            }
        );
    }

    #[test]
    fn tampering_and_binding_substitution_fail_closed() {
        let signing_key = SigningKey::from_bytes(&[3; 32]);
        let mut envelope = FxEvidenceEnvelope::sign(sample_unsigned([4; 32]), &signing_key)
            .expect("signed fixture");
        envelope.unsigned.numerator = 10;
        assert!(matches!(
            envelope.verify_with_key_at(
                &signing_key.verifying_key(),
                [7; 32],
                [8; 32],
                1_800_000_100
            ),
            Err(FxEvidenceError::PayloadDigestMismatch) | Err(FxEvidenceError::InvalidSignature)
        ));

        let envelope = FxEvidenceEnvelope::sign(sample_unsigned([4; 32]), &signing_key)
            .expect("signed fixture");
        assert_eq!(
            envelope.verify_with_key_at(
                &signing_key.verifying_key(),
                [6; 32],
                [8; 32],
                1_800_000_100
            ),
            Err(FxEvidenceError::EvidenceBindingMismatch)
        );
        assert_eq!(
            envelope.verify_with_key_at(
                &signing_key.verifying_key(),
                [7; 32],
                [6; 32],
                1_800_000_100
            ),
            Err(FxEvidenceError::EvidenceBindingMismatch)
        );
    }

    #[test]
    fn validity_window_rejects_expiry_and_future_observation() {
        let signing_key = SigningKey::from_bytes(&[3; 32]);
        let envelope = FxEvidenceEnvelope::sign(sample_unsigned([4; 32]), &signing_key)
            .expect("signed fixture");
        assert_eq!(
            envelope.verify_with_key_at(
                &signing_key.verifying_key(),
                [7; 32],
                [8; 32],
                1_800_000_300
            ),
            Err(FxEvidenceError::EvidenceOutsideValidityWindow)
        );
        assert_eq!(
            envelope.verify_with_key_at(
                &signing_key.verifying_key(),
                [7; 32],
                [8; 32],
                1_799_999_999
            ),
            Err(FxEvidenceError::EvidenceOutsideValidityWindow)
        );
    }

    #[test]
    fn registry_rotation_is_atomic_and_retains_snapshot_on_invalid_reload() {
        let first_key = SigningKey::from_bytes(&[3; 32]);
        let second_key = SigningKey::from_bytes(&[5; 32]);
        let first = registry_document([4; 32], &first_key);
        let second = registry_document([6; 32], &second_key);
        let registry = FxProviderKeyRegistry::from_json(&first).expect("initial registry");
        let first_envelope =
            FxEvidenceEnvelope::sign(sample_unsigned([4; 32]), &first_key).expect("first evidence");
        assert!(
            registry
                .verify(&first_envelope, [7; 32], [8; 32], 1_800_000_100)
                .is_ok()
        );

        registry.reload_json(&second).expect("valid rotation");
        assert_eq!(registry.key_count(), 1);
        assert_eq!(
            registry.verify(&first_envelope, [7; 32], [8; 32], 1_800_000_100),
            Err(FxEvidenceError::UnknownProviderKey)
        );
        assert!(registry.reload_json("{not-json").is_err());
        assert_eq!(registry.key_count(), 1);
    }

    #[test]
    fn malformed_bounds_and_unknown_keys_fail_closed() {
        let signing_key = SigningKey::from_bytes(&[3; 32]);
        let mut invalid = sample_unsigned([4; 32]);
        invalid.denominator = 0;
        assert_eq!(
            FxEvidenceEnvelope::sign(invalid, &signing_key),
            Err(FxEvidenceError::InvalidEvidence("rate must be positive"))
        );

        let registry = FxProviderKeyRegistry::from_json(&registry_document([4; 32], &signing_key))
            .expect("registry");
        let unknown = FxEvidenceEnvelope::sign(sample_unsigned([5; 32]), &signing_key)
            .expect("unknown-key evidence");
        assert_eq!(
            registry.verify(&unknown, [7; 32], [8; 32], 1_800_000_100),
            Err(FxEvidenceError::UnknownProviderKey)
        );
    }

    fn registry_document(key_id: Digest32, signing_key: &SigningKey) -> String {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        format!(
            "{{\"keys\":[{{\"key_id\":\"{}\",\"public_key\":\"{}\"}}]}}",
            key_id
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            URL_SAFE_NO_PAD.encode(signing_key.verifying_key().to_bytes())
        )
    }
}
