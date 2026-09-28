//! Canonical calculation-attestation contract.
//!
//! This crate owns the bytes signed by an issuer and the verification rules
//! shared by the policy boundary and the future Solana instruction verifier.
//! It deliberately accepts time as an argument so verification is deterministic
//! in tests and never reads a process clock from domain code.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const ATTESTATION_SCHEMA_VERSION: u16 = 1;
pub const MAX_ATTESTATION_LIFETIME_SECONDS: i64 = 86_400;
pub const MAX_TEXT_BYTES: usize = 128;
pub type Digest32 = [u8; 32];

const DOMAIN_SEPARATOR: &[u8] = b"solana-tax-protocol/calculation-attestation/v1";
const MERCHANT_ID_DOMAIN: &[u8] = b"solana-tax-protocol/merchant-identity/v1";

/// Derives the canonical digest used in attestation bindings for an
/// authenticated merchant UUID. The raw UUID is intentionally not exposed as
/// the policy identity; issuers and verifiers must use this domain-separated
/// representation.
pub fn merchant_identity_digest(merchant_uuid_bytes: &[u8; 16]) -> Digest32 {
    let mut input = Vec::with_capacity(MERCHANT_ID_DOMAIN.len() + merchant_uuid_bytes.len());
    input.extend_from_slice(MERCHANT_ID_DOMAIN);
    input.extend_from_slice(merchant_uuid_bytes);
    hash_bytes(&input)
}

pub mod serde_u64_string {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum WireValue {
            String(String),
            Number(u64),
        }

        match WireValue::deserialize(deserializer)? {
            WireValue::Number(value) => Ok(value),
            WireValue::String(value) => {
                let parsed = value.parse::<u64>().map_err(D::Error::custom)?;
                if value != parsed.to_string() {
                    return Err(D::Error::custom("u64 string is not canonical"));
                }
                Ok(parsed)
            }
        }
    }
}

pub mod serde_optional_u64_string {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serializer.serialize_some(&value.to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum WireValue {
            String(String),
            Number(u64),
        }

        Option::<WireValue>::deserialize(deserializer)?
            .map(|value| match value {
                WireValue::Number(value) => Ok(value),
                WireValue::String(value) => {
                    let parsed = value.parse::<u64>().map_err(D::Error::custom)?;
                    if value != parsed.to_string() {
                        return Err(D::Error::custom("u64 string is not canonical"));
                    }
                    Ok(parsed)
                }
            })
            .transpose()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnsignedAttestation {
    pub schema_version: u16,
    pub merchant_id: Digest32,
    pub rule_set_id: String,
    pub rule_version: u32,
    pub jurisdiction: String,
    pub currency: String,
    pub asset_mint: Digest32,
    pub facts_digest: Digest32,
    pub rule_digest: Digest32,
    #[serde(with = "serde_u64_string")]
    pub gross_atomic: u64,
    #[serde(with = "serde_u64_string")]
    pub tax_atomic: u64,
    #[serde(with = "serde_u64_string")]
    pub net_atomic: u64,
    pub issued_at_unix: i64,
    pub expires_at_unix: i64,
    #[serde(with = "serde_u64_string")]
    pub sequence: u64,
    pub issuer_key_id: Digest32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CalculationAttestation {
    pub unsigned: UnsignedAttestation,
    pub issuer_public_key: [u8; 32],
    pub payload_digest: Digest32,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationBinding<'a> {
    pub merchant_id: Digest32,
    pub rule_set_id: &'a str,
    pub rule_version: u32,
    pub jurisdiction: &'a str,
    pub currency: &'a str,
    pub asset_mint: Digest32,
    pub facts_digest: Digest32,
    pub rule_digest: Digest32,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum AttestationError {
    #[error("unsupported attestation schema version")]
    UnsupportedSchemaVersion,
    #[error("attestation text field is empty or exceeds the bound")]
    InvalidTextField,
    #[error("currency must be a three-letter uppercase code")]
    InvalidCurrency,
    #[error("attestation contains an all-zero identity, mint, or digest")]
    EmptyIdentityOrDigest,
    #[error("attestation rule version must be positive")]
    InvalidRuleVersion,
    #[error("tax plus net does not equal gross")]
    AmountConservationViolation,
    #[error("attestation validity window is invalid or too long")]
    InvalidValidityWindow,
    #[error("attestation sequence must be positive")]
    InvalidSequence,
    #[error("canonical payload encoding failed")]
    EncodingFailure,
    #[error("issuer public key is invalid")]
    InvalidIssuerKey,
    #[error("attestation signature is invalid")]
    InvalidSignature,
    #[error("attestation payload digest does not match")]
    PayloadDigestMismatch,
    #[error("attestation is outside its validity window")]
    OutsideValidityWindow,
    #[error("attestation does not bind to the requested calculation")]
    BindingMismatch,
}

impl UnsignedAttestation {
    pub fn validate(&self) -> Result<(), AttestationError> {
        if self.schema_version != ATTESTATION_SCHEMA_VERSION {
            return Err(AttestationError::UnsupportedSchemaVersion);
        }
        if self.rule_set_id.trim().is_empty()
            || self.jurisdiction.trim().is_empty()
            || self.currency.trim().is_empty()
            || self.rule_set_id.len() > MAX_TEXT_BYTES
            || self.jurisdiction.len() > MAX_TEXT_BYTES
            || self.currency.len() > MAX_TEXT_BYTES
        {
            return Err(AttestationError::InvalidTextField);
        }
        if self.currency.len() != 3 || !self.currency.bytes().all(|byte| byte.is_ascii_uppercase())
        {
            return Err(AttestationError::InvalidCurrency);
        }
        if self.rule_version == 0 {
            return Err(AttestationError::InvalidRuleVersion);
        }
        if self.merchant_id == [0; 32]
            || self.asset_mint == [0; 32]
            || self.facts_digest == [0; 32]
            || self.rule_digest == [0; 32]
            || self.issuer_key_id == [0; 32]
        {
            return Err(AttestationError::EmptyIdentityOrDigest);
        }
        if self.tax_atomic.checked_add(self.net_atomic) != Some(self.gross_atomic) {
            return Err(AttestationError::AmountConservationViolation);
        }
        if self.issued_at_unix < 0
            || self.expires_at_unix <= self.issued_at_unix
            || self
                .expires_at_unix
                .checked_sub(self.issued_at_unix)
                .is_none_or(|lifetime| lifetime > MAX_ATTESTATION_LIFETIME_SECONDS)
        {
            return Err(AttestationError::InvalidValidityWindow);
        }
        if self.sequence == 0 {
            return Err(AttestationError::InvalidSequence);
        }
        Ok(())
    }

    pub fn canonical_payload(&self) -> Result<Vec<u8>, AttestationError> {
        self.validate()?;
        let mut payload = Vec::with_capacity(256);
        payload.extend_from_slice(DOMAIN_SEPARATOR);
        payload.extend_from_slice(&self.schema_version.to_le_bytes());
        payload.extend_from_slice(&self.merchant_id);
        append_text(&mut payload, &self.rule_set_id)?;
        payload.extend_from_slice(&self.rule_version.to_le_bytes());
        append_text(&mut payload, &self.jurisdiction)?;
        append_text(&mut payload, &self.currency)?;
        payload.extend_from_slice(&self.asset_mint);
        payload.extend_from_slice(&self.facts_digest);
        payload.extend_from_slice(&self.rule_digest);
        payload.extend_from_slice(&self.gross_atomic.to_le_bytes());
        payload.extend_from_slice(&self.tax_atomic.to_le_bytes());
        payload.extend_from_slice(&self.net_atomic.to_le_bytes());
        payload.extend_from_slice(&self.issued_at_unix.to_le_bytes());
        payload.extend_from_slice(&self.expires_at_unix.to_le_bytes());
        payload.extend_from_slice(&self.sequence.to_le_bytes());
        payload.extend_from_slice(&self.issuer_key_id);
        Ok(payload)
    }

    pub fn payload_digest(&self) -> Result<Digest32, AttestationError> {
        Ok(hash_bytes(&self.canonical_payload()?))
    }
}

impl CalculationAttestation {
    pub fn sign(
        unsigned: UnsignedAttestation,
        signing_key: &SigningKey,
    ) -> Result<Self, AttestationError> {
        unsigned.validate()?;
        let payload = unsigned.canonical_payload()?;
        let payload_digest = hash_bytes(&payload);
        let signature = signing_key.sign(&payload).to_bytes().to_vec();
        Ok(Self {
            unsigned,
            issuer_public_key: signing_key.verifying_key().to_bytes(),
            payload_digest,
            signature,
        })
    }

    pub fn verify(&self) -> Result<(), AttestationError> {
        self.unsigned.validate()?;
        if self.signature.len() != 64 {
            return Err(AttestationError::InvalidSignature);
        }
        let payload = self.unsigned.canonical_payload()?;
        let expected_digest = hash_bytes(&payload);
        if self.payload_digest != expected_digest {
            return Err(AttestationError::PayloadDigestMismatch);
        }
        let public_key = VerifyingKey::from_bytes(&self.issuer_public_key)
            .map_err(|_| AttestationError::InvalidIssuerKey)?;
        let signature = Signature::from_slice(&self.signature)
            .map_err(|_| AttestationError::InvalidSignature)?;
        public_key
            .verify(&payload, &signature)
            .map_err(|_| AttestationError::InvalidSignature)
    }

    pub fn verify_at(&self, now_unix: i64) -> Result<(), AttestationError> {
        self.verify()?;
        if now_unix < self.unsigned.issued_at_unix || now_unix >= self.unsigned.expires_at_unix {
            return Err(AttestationError::OutsideValidityWindow);
        }
        Ok(())
    }

    pub fn verify_binding(&self, binding: &AttestationBinding<'_>) -> Result<(), AttestationError> {
        self.verify()?;
        let actual = &self.unsigned;
        if actual.merchant_id != binding.merchant_id
            || actual.rule_set_id != binding.rule_set_id
            || actual.rule_version != binding.rule_version
            || actual.jurisdiction != binding.jurisdiction
            || actual.currency != binding.currency
            || actual.asset_mint != binding.asset_mint
            || actual.facts_digest != binding.facts_digest
            || actual.rule_digest != binding.rule_digest
            || actual.gross_atomic != binding.gross_atomic
            || actual.tax_atomic != binding.tax_atomic
            || actual.net_atomic != binding.net_atomic
        {
            return Err(AttestationError::BindingMismatch);
        }
        Ok(())
    }
}

fn append_text(payload: &mut Vec<u8>, value: &str) -> Result<(), AttestationError> {
    if value.is_empty() || value.len() > MAX_TEXT_BYTES {
        return Err(AttestationError::EncodingFailure);
    }
    let length = u16::try_from(value.len()).map_err(|_| AttestationError::EncodingFailure)?;
    payload.extend_from_slice(&length.to_le_bytes());
    payload.extend_from_slice(value.as_bytes());
    Ok(())
}

fn hash_bytes(bytes: &[u8]) -> Digest32 {
    Sha256::digest(bytes).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unsigned() -> UnsignedAttestation {
        UnsignedAttestation {
            schema_version: ATTESTATION_SCHEMA_VERSION,
            merchant_id: [1; 32],
            rule_set_id: "us-ca-sales-tax".to_string(),
            rule_version: 7,
            jurisdiction: "US-CA".to_string(),
            currency: "USD".to_string(),
            asset_mint: [2; 32],
            facts_digest: [3; 32],
            rule_digest: [4; 32],
            gross_atomic: 1_155,
            tax_atomic: 155,
            net_atomic: 1_000,
            issued_at_unix: 1_800_000_000,
            expires_at_unix: 1_800_000_300,
            sequence: 9,
            issuer_key_id: [5; 32],
        }
    }

    fn signing_key() -> SigningKey {
        SigningKey::from_bytes(&[7; 32])
    }

    #[test]
    fn signs_and_verifies_canonical_attestation() {
        let attestation = CalculationAttestation::sign(unsigned(), &signing_key()).unwrap();
        assert_eq!(
            attestation.unsigned.payload_digest().unwrap(),
            attestation.payload_digest
        );
        attestation.verify_at(1_800_000_100).unwrap();
    }

    #[test]
    fn rejects_payload_tampering() {
        let mut attestation = CalculationAttestation::sign(unsigned(), &signing_key()).unwrap();
        attestation.unsigned.tax_atomic = 156;
        assert_eq!(
            attestation.verify(),
            Err(AttestationError::AmountConservationViolation)
        );
    }

    #[test]
    fn rejects_binding_mismatch() {
        let attestation = CalculationAttestation::sign(unsigned(), &signing_key()).unwrap();
        let binding = AttestationBinding {
            merchant_id: [1; 32],
            rule_set_id: "us-ca-sales-tax",
            rule_version: 8,
            jurisdiction: "US-CA",
            currency: "USD",
            asset_mint: [2; 32],
            facts_digest: [3; 32],
            rule_digest: [4; 32],
            gross_atomic: 1_155,
            tax_atomic: 155,
            net_atomic: 1_000,
        };
        assert_eq!(
            attestation.verify_binding(&binding),
            Err(AttestationError::BindingMismatch)
        );
    }

    #[test]
    fn rejects_expired_attestation() {
        let attestation = CalculationAttestation::sign(unsigned(), &signing_key()).unwrap();
        assert_eq!(
            attestation.verify_at(1_800_000_300),
            Err(AttestationError::OutsideValidityWindow)
        );
    }

    #[test]
    fn merchant_identity_digest_is_domain_separated_and_stable() {
        let first = merchant_identity_digest(&[1; 16]);
        assert_eq!(first, merchant_identity_digest(&[1; 16]));
        assert_ne!(first, merchant_identity_digest(&[2; 16]));
        let plain_digest: Digest32 = Sha256::digest([1; 16]).into();
        assert_ne!(first, plain_digest);
    }

    #[test]
    fn u64_wire_values_serialize_as_canonical_strings_and_read_legacy_numbers() {
        #[derive(Debug, Deserialize, Serialize)]
        struct WireValue {
            #[serde(with = "serde_u64_string")]
            value: u64,
        }

        let encoded = serde_json::to_string(&WireValue { value: u64::MAX }).unwrap();
        assert_eq!(encoded, r#"{"value":"18446744073709551615"}"#);
        assert_eq!(
            serde_json::from_str::<WireValue>(r#"{"value":9007199254740992}"#)
                .unwrap()
                .value,
            9_007_199_254_740_992
        );
        assert_eq!(
            serde_json::from_str::<WireValue>(r#"{"value":"18446744073709551615"}"#)
                .unwrap()
                .value,
            u64::MAX
        );
        assert!(serde_json::from_str::<WireValue>(r#"{"value":"01"}"#).is_err());
        assert!(serde_json::from_str::<WireValue>(r#"{"value":"18446744073709551616"}"#).is_err());
    }

    #[test]
    fn optional_u64_wire_values_preserve_null_and_canonical_strings() {
        #[derive(Debug, Deserialize, Serialize)]
        struct WireValue {
            #[serde(with = "serde_optional_u64_string")]
            value: Option<u64>,
        }

        assert_eq!(
            serde_json::to_string(&WireValue { value: None }).unwrap(),
            r#"{"value":null}"#
        );
        assert_eq!(
            serde_json::to_string(&WireValue {
                value: Some(u64::MAX),
            })
            .unwrap(),
            r#"{"value":"18446744073709551615"}"#
        );
        assert_eq!(
            serde_json::from_str::<WireValue>(r#"{"value":"9"}"#)
                .unwrap()
                .value,
            Some(9)
        );
        assert!(serde_json::from_str::<WireValue>(r#"{"value":"09"}"#).is_err());
    }
}
