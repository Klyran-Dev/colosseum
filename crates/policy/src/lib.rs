#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

//! Canonical calculation-attestation contract.
//!
//! This crate owns the bytes signed by an issuer and the verification rules
//! shared by the policy boundary and the future Solana instruction verifier.
//! It deliberately accepts time as an argument so verification is deterministic
//! in tests and never reads a process clock from domain code.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tax_applicability::{ApplicabilityDecision, ApplicabilityFacts, Treatment};
use thiserror::Error;

pub const LEGACY_ATTESTATION_SCHEMA_VERSION: u16 = 1;
pub const ATTESTATION_SCHEMA_VERSION: u16 = 2;
pub const LEGACY_DOCUMENT_CALCULATION_ATTESTATION_SCHEMA_VERSION: u16 = 1;
pub const DOCUMENT_CALCULATION_ATTESTATION_SCHEMA_VERSION: u16 = 2;
pub const APPLICABILITY_ATTESTATION_SCHEMA_VERSION: u16 = 1;
pub const TAX_DOCUMENT_SCHEMA_V1: &str = "solana-tax-protocol/tax-document/v1";
pub const MAX_ATTESTATION_LIFETIME_SECONDS: i64 = 86_400;
pub const MAX_TEXT_BYTES: usize = 128;
pub type Digest32 = [u8; 32];

const DOMAIN_SEPARATOR: &[u8] = b"solana-tax-protocol/calculation-attestation/v1";
const DOMAIN_SEPARATOR_V2: &[u8] = b"solana-tax-protocol/calculation-attestation/v2";
const DOCUMENT_DOMAIN_SEPARATOR: &[u8] = b"solana-tax-protocol/document-calculation-attestation/v1";
const DOCUMENT_DOMAIN_SEPARATOR_V2: &[u8] =
    b"solana-tax-protocol/document-calculation-attestation/v2";
const APPLICABILITY_DOMAIN_SEPARATOR: &[u8] = b"solana-tax-protocol/applicability-attestation/v1";
const MERCHANT_ID_DOMAIN: &[u8] = b"solana-tax-protocol/merchant-identity/v1";
const TREATMENT_OUTCOME_DOMAIN: &[u8] = b"solana-tax-protocol/treatment-outcome/v1";

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

/// Computes the canonical digest committed by calculation-attestation v2.
/// Signed decimal amounts are represented as two's-complement little-endian
/// i128 values so negative intermediate evidence cannot be confused with a
/// large unsigned amount across language boundaries.
pub fn treatment_outcome_digest(
    treatment: Treatment,
    arithmetic_tax_atomic: i128,
    collected_tax_atomic: i128,
    self_assessed_tax_atomic: i128,
    withheld_tax_atomic: i128,
    taxable_base_atomic: i128,
) -> Digest32 {
    let mut payload = Vec::with_capacity(TREATMENT_OUTCOME_DOMAIN.len() + 1 + (5 * 16));
    payload.extend_from_slice(TREATMENT_OUTCOME_DOMAIN);
    payload.push(treatment_tag(treatment));
    for amount in [
        arithmetic_tax_atomic,
        collected_tax_atomic,
        self_assessed_tax_atomic,
        withheld_tax_atomic,
        taxable_base_atomic,
    ] {
        payload.extend_from_slice(&amount.to_le_bytes());
    }
    hash_bytes(&payload)
}

/// Returns the treatment binding expected by an attestation schema. Legacy
/// v1 evidence can remain readable for ordinary taxable calculations, but it
/// never authorizes a non-taxable treatment because those channels did not
/// exist in its signed payload.
pub fn treatment_outcome_binding(
    schema_version: u16,
    treatment: Treatment,
    arithmetic_tax_atomic: i128,
    collected_tax_atomic: i128,
    self_assessed_tax_atomic: i128,
    withheld_tax_atomic: i128,
    taxable_base_atomic: i128,
) -> Result<Option<Digest32>, AttestationError> {
    match schema_version {
        1 => {
            if treatment != Treatment::Taxable {
                return Err(AttestationError::LegacyTreatmentOutcomeUnsupported);
            }
            Ok(None)
        }
        2 => Ok(Some(treatment_outcome_digest(
            treatment,
            arithmetic_tax_atomic,
            collected_tax_atomic,
            self_assessed_tax_atomic,
            withheld_tax_atomic,
            taxable_base_atomic,
        ))),
        _ => Err(AttestationError::UnsupportedSchemaVersion),
    }
}

fn treatment_tag(treatment: Treatment) -> u8 {
    match treatment {
        Treatment::Taxable => 0,
        Treatment::ZeroRated => 1,
        Treatment::Exempt => 2,
        Treatment::ReverseCharge => 3,
        Treatment::Withholding => 4,
        Treatment::OutOfScope => 5,
        Treatment::Unresolved => 6,
    }
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

/// Strict browser/API wire encoding for protocol counters. Unlike the
/// compatibility module above, this boundary rejects JSON numbers so a
/// JavaScript safe-integer conversion can never become part of an authority
/// or request digest.
pub mod serde_strict_u64_string {
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
        let value = String::deserialize(deserializer)?;
        let parsed = value.parse::<u64>().map_err(D::Error::custom)?;
        if value != parsed.to_string() {
            return Err(D::Error::custom("u64 string is not canonical"));
        }
        Ok(parsed)
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
    /// Zero for legacy v1 evidence. Required and signed for v2 evidence.
    #[serde(default)]
    pub treatment_outcome_digest: Digest32,
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
    pub treatment_outcome_digest: Option<Digest32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnsignedDocumentAttestation {
    pub schema_version: u16,
    pub document_schema: String,
    pub merchant_id: Digest32,
    pub document_id: String,
    pub engine_version: String,
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
    /// Zero for legacy v1 evidence. Required and signed for v2 evidence.
    #[serde(default)]
    pub treatment_outcome_digest: Digest32,
    pub issued_at_unix: i64,
    pub expires_at_unix: i64,
    #[serde(with = "serde_u64_string")]
    pub sequence: u64,
    pub issuer_key_id: Digest32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DocumentCalculationAttestation {
    pub unsigned: UnsignedDocumentAttestation,
    pub issuer_public_key: [u8; 32],
    pub payload_digest: Digest32,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentAttestationBinding<'a> {
    pub document_schema: &'a str,
    pub merchant_id: Digest32,
    pub document_id: &'a str,
    pub engine_version: &'a str,
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
    pub treatment_outcome_digest: Option<Digest32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnsignedApplicabilityAttestation {
    pub schema_version: u16,
    pub merchant_id: Digest32,
    pub jurisdiction_resolution_digest: Digest32,
    pub facts: ApplicabilityFacts,
    pub decision: ApplicabilityDecision,
    pub rule_set_id: String,
    pub rule_version: u32,
    pub rule_digest: Digest32,
    pub issued_at_unix: i64,
    pub expires_at_unix: i64,
    #[serde(with = "serde_strict_u64_string")]
    pub sequence: u64,
    pub issuer_key_id: Digest32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicabilityAttestation {
    pub unsigned: UnsignedApplicabilityAttestation,
    pub issuer_public_key: [u8; 32],
    pub payload_digest: Digest32,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicabilityAttestationBinding<'a> {
    pub merchant_id: Digest32,
    pub jurisdiction_resolution_digest: Digest32,
    pub facts_digest: Digest32,
    pub decision_digest: Digest32,
    pub evidence_digest: Digest32,
    pub policy_id: &'a str,
    pub policy_version: u32,
    pub policy_digest: Digest32,
    pub treatment: Treatment,
    pub product_category: &'a str,
    pub supply_type: &'a str,
    pub rule_set_id: &'a str,
    pub rule_version: u32,
    pub rule_digest: Digest32,
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
    #[error("applicability facts are invalid")]
    InvalidApplicabilityFacts,
    #[error("applicability decision is invalid")]
    InvalidApplicabilityDecision,
    #[error("applicability facts do not match the signed decision")]
    ApplicabilityFactsBindingMismatch,
    #[error("applicability attestation outlives its source evidence")]
    ApplicabilityEvidenceWindowMismatch,
    #[error("legacy v1 attestation cannot authorize a non-taxable treatment")]
    LegacyTreatmentOutcomeUnsupported,
}

impl UnsignedAttestation {
    pub fn validate(&self) -> Result<(), AttestationError> {
        if self.schema_version != LEGACY_ATTESTATION_SCHEMA_VERSION
            && self.schema_version != ATTESTATION_SCHEMA_VERSION
        {
            return Err(AttestationError::UnsupportedSchemaVersion);
        }
        if (self.schema_version == LEGACY_ATTESTATION_SCHEMA_VERSION
            && self.treatment_outcome_digest != [0; 32])
            || (self.schema_version == ATTESTATION_SCHEMA_VERSION
                && self.treatment_outcome_digest == [0; 32])
        {
            return Err(AttestationError::BindingMismatch);
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
        if self.schema_version == LEGACY_ATTESTATION_SCHEMA_VERSION {
            payload.extend_from_slice(DOMAIN_SEPARATOR);
        } else {
            payload.extend_from_slice(DOMAIN_SEPARATOR_V2);
        }
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
        if self.schema_version == ATTESTATION_SCHEMA_VERSION {
            payload.extend_from_slice(&self.treatment_outcome_digest);
        }
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
            || (actual.schema_version == ATTESTATION_SCHEMA_VERSION
                && binding.treatment_outcome_digest != Some(actual.treatment_outcome_digest))
            || (actual.schema_version == LEGACY_ATTESTATION_SCHEMA_VERSION
                && binding.treatment_outcome_digest.is_some())
        {
            return Err(AttestationError::BindingMismatch);
        }
        Ok(())
    }
}

impl UnsignedApplicabilityAttestation {
    pub fn validate(&self) -> Result<(), AttestationError> {
        if self.schema_version != APPLICABILITY_ATTESTATION_SCHEMA_VERSION {
            return Err(AttestationError::UnsupportedSchemaVersion);
        }
        if self.rule_set_id.trim().is_empty() || self.rule_set_id.len() > MAX_TEXT_BYTES {
            return Err(AttestationError::InvalidTextField);
        }
        if self.rule_version == 0 {
            return Err(AttestationError::InvalidRuleVersion);
        }
        if self.merchant_id == [0; 32]
            || self.jurisdiction_resolution_digest == [0; 32]
            || self.rule_digest == [0; 32]
            || self.issuer_key_id == [0; 32]
        {
            return Err(AttestationError::EmptyIdentityOrDigest);
        }
        self.facts
            .validate()
            .map_err(|_| AttestationError::InvalidApplicabilityFacts)?;
        self.decision
            .validate()
            .map_err(|_| AttestationError::InvalidApplicabilityDecision)?;
        if self
            .facts
            .canonical_digest()
            .map_err(|_| AttestationError::InvalidApplicabilityFacts)?
            != self.decision.facts_digest
        {
            return Err(AttestationError::ApplicabilityFactsBindingMismatch);
        }
        if self.expires_at_unix > self.facts.valid_until_unix {
            return Err(AttestationError::ApplicabilityEvidenceWindowMismatch);
        }
        if self.issued_at_unix <= 0
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
        let facts_digest = self
            .facts
            .canonical_digest()
            .map_err(|_| AttestationError::InvalidApplicabilityFacts)?;
        let decision_digest = self
            .decision
            .canonical_digest()
            .map_err(|_| AttestationError::InvalidApplicabilityDecision)?;
        let mut payload = Vec::with_capacity(512);
        payload.extend_from_slice(APPLICABILITY_DOMAIN_SEPARATOR);
        payload.extend_from_slice(&self.schema_version.to_le_bytes());
        payload.extend_from_slice(&self.merchant_id);
        payload.extend_from_slice(&self.jurisdiction_resolution_digest);
        payload.extend_from_slice(&facts_digest);
        payload.extend_from_slice(&decision_digest);
        append_text(&mut payload, &self.rule_set_id)?;
        payload.extend_from_slice(&self.rule_version.to_le_bytes());
        payload.extend_from_slice(&self.rule_digest);
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

impl ApplicabilityAttestation {
    pub fn sign(
        unsigned: UnsignedApplicabilityAttestation,
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
        if self.payload_digest != hash_bytes(&payload) {
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

    pub fn verify_binding(
        &self,
        binding: &ApplicabilityAttestationBinding<'_>,
    ) -> Result<(), AttestationError> {
        self.verify()?;
        let actual = &self.unsigned;
        let facts_digest = actual
            .facts
            .canonical_digest()
            .map_err(|_| AttestationError::InvalidApplicabilityFacts)?;
        if actual.merchant_id != binding.merchant_id
            || actual.jurisdiction_resolution_digest != binding.jurisdiction_resolution_digest
            || facts_digest != binding.facts_digest
            || actual
                .decision
                .canonical_digest()
                .map_err(|_| AttestationError::InvalidApplicabilityDecision)?
                != binding.decision_digest
            || actual.facts.evidence_digest != binding.evidence_digest
            || actual.decision.policy_id != binding.policy_id
            || actual.decision.policy_version != binding.policy_version
            || actual.decision.policy_digest != binding.policy_digest
            || actual.decision.treatment != binding.treatment
            || actual.facts.product_category != binding.product_category
            || actual.facts.supply_type != binding.supply_type
            || actual.rule_set_id != binding.rule_set_id
            || actual.rule_version != binding.rule_version
            || actual.rule_digest != binding.rule_digest
        {
            return Err(AttestationError::BindingMismatch);
        }
        Ok(())
    }
}

impl UnsignedDocumentAttestation {
    pub fn validate(&self) -> Result<(), AttestationError> {
        if (self.schema_version != LEGACY_DOCUMENT_CALCULATION_ATTESTATION_SCHEMA_VERSION
            && self.schema_version != DOCUMENT_CALCULATION_ATTESTATION_SCHEMA_VERSION)
            || self.document_schema != TAX_DOCUMENT_SCHEMA_V1
        {
            return Err(AttestationError::UnsupportedSchemaVersion);
        }
        if (self.schema_version == LEGACY_DOCUMENT_CALCULATION_ATTESTATION_SCHEMA_VERSION
            && self.treatment_outcome_digest != [0; 32])
            || (self.schema_version == DOCUMENT_CALCULATION_ATTESTATION_SCHEMA_VERSION
                && self.treatment_outcome_digest == [0; 32])
        {
            return Err(AttestationError::BindingMismatch);
        }
        if self.document_id.trim().is_empty()
            || self.engine_version.trim().is_empty()
            || self.rule_set_id.trim().is_empty()
            || self.jurisdiction.trim().is_empty()
            || self.currency.trim().is_empty()
            || self.document_id.len() > MAX_TEXT_BYTES
            || self.engine_version.len() > MAX_TEXT_BYTES
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
        let mut payload = Vec::with_capacity(320);
        if self.schema_version == LEGACY_DOCUMENT_CALCULATION_ATTESTATION_SCHEMA_VERSION {
            payload.extend_from_slice(DOCUMENT_DOMAIN_SEPARATOR);
        } else {
            payload.extend_from_slice(DOCUMENT_DOMAIN_SEPARATOR_V2);
        }
        payload.extend_from_slice(&self.schema_version.to_le_bytes());
        append_text(&mut payload, &self.document_schema)?;
        payload.extend_from_slice(&self.merchant_id);
        append_text(&mut payload, &self.document_id)?;
        append_text(&mut payload, &self.engine_version)?;
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
        if self.schema_version == DOCUMENT_CALCULATION_ATTESTATION_SCHEMA_VERSION {
            payload.extend_from_slice(&self.treatment_outcome_digest);
        }
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

impl DocumentCalculationAttestation {
    pub fn sign(
        unsigned: UnsignedDocumentAttestation,
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
        if self.payload_digest != hash_bytes(&payload) {
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

    pub fn verify_binding(
        &self,
        binding: &DocumentAttestationBinding<'_>,
    ) -> Result<(), AttestationError> {
        self.verify()?;
        let actual = &self.unsigned;
        if actual.document_schema != binding.document_schema
            || actual.merchant_id != binding.merchant_id
            || actual.document_id != binding.document_id
            || actual.engine_version != binding.engine_version
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
            || (actual.schema_version == DOCUMENT_CALCULATION_ATTESTATION_SCHEMA_VERSION
                && binding.treatment_outcome_digest != Some(actual.treatment_outcome_digest))
            || (actual.schema_version == LEGACY_DOCUMENT_CALCULATION_ATTESTATION_SCHEMA_VERSION
                && binding.treatment_outcome_digest.is_some())
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
            treatment_outcome_digest: treatment_outcome_digest(
                Treatment::Taxable,
                155,
                155,
                0,
                0,
                1_155,
            ),
            issued_at_unix: 1_800_000_000,
            expires_at_unix: 1_800_000_300,
            sequence: 9,
            issuer_key_id: [5; 32],
        }
    }

    fn unsigned_document() -> UnsignedDocumentAttestation {
        UnsignedDocumentAttestation {
            schema_version: DOCUMENT_CALCULATION_ATTESTATION_SCHEMA_VERSION,
            document_schema: TAX_DOCUMENT_SCHEMA_V1.to_string(),
            merchant_id: [1; 32],
            document_id: "invoice-1001".to_string(),
            engine_version: "tax-engine-v1".to_string(),
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
            treatment_outcome_digest: treatment_outcome_digest(
                Treatment::Taxable,
                155,
                155,
                0,
                0,
                1_155,
            ),
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
    fn treatment_outcome_digest_matches_the_shared_golden_vector() {
        let vector: serde_json::Value = serde_json::from_str(include_str!(
            "../../../test-vectors/treatment-outcome-digest-v1.json"
        ))
        .expect("treatment outcome vector must deserialize");
        let digest = treatment_outcome_digest(Treatment::ReverseCharge, 155, 0, 155, 0, 1_155);
        let expected = vector["digest_hex"]
            .as_str()
            .expect("digest hex is required");
        let actual = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn v2_attestation_binds_every_treatment_channel() {
        let mut attestation = CalculationAttestation::sign(unsigned(), &signing_key()).unwrap();
        let binding = AttestationBinding {
            merchant_id: [1; 32],
            rule_set_id: "us-ca-sales-tax",
            rule_version: 7,
            jurisdiction: "US-CA",
            currency: "USD",
            asset_mint: [2; 32],
            facts_digest: [3; 32],
            rule_digest: [4; 32],
            gross_atomic: 1_155,
            tax_atomic: 155,
            net_atomic: 1_000,
            treatment_outcome_digest: Some(attestation.unsigned.treatment_outcome_digest),
        };
        attestation.verify_binding(&binding).unwrap();
        attestation.unsigned.treatment_outcome_digest[0] ^= 1;
        assert_eq!(
            attestation.verify(),
            Err(AttestationError::PayloadDigestMismatch)
        );
    }

    #[test]
    fn shared_calculation_attestation_vector_matches_canonical_bytes() {
        let attestation: CalculationAttestation = serde_json::from_str(include_str!(
            "../../../test-vectors/calculation-attestation-v1.json"
        ))
        .expect("shared calculation-attestation vector must deserialize");
        attestation
            .verify_at(1_800_000_100)
            .expect("shared calculation-attestation vector must verify");
        assert_eq!(
            attestation
                .unsigned
                .payload_digest()
                .expect("payload digest"),
            attestation.payload_digest
        );
        attestation
            .verify_binding(&AttestationBinding {
                merchant_id: [1; 32],
                rule_set_id: "us-ca-sales-tax",
                rule_version: 7,
                jurisdiction: "US-CA",
                currency: "USD",
                asset_mint: [2; 32],
                facts_digest: [3; 32],
                rule_digest: [4; 32],
                gross_atomic: 1_155,
                tax_atomic: 155,
                net_atomic: 1_000,
                treatment_outcome_digest: None,
            })
            .expect("legacy taxable evidence remains readable");
    }

    #[test]
    fn legacy_v1_binding_rejects_non_taxable_treatment() {
        assert_eq!(
            treatment_outcome_binding(1, Treatment::ReverseCharge, 155, 0, 155, 0, 1_155),
            Err(AttestationError::LegacyTreatmentOutcomeUnsupported)
        );
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
            treatment_outcome_digest: Some(treatment_outcome_digest(
                Treatment::Taxable,
                155,
                155,
                0,
                0,
                1_155,
            )),
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
    fn signs_and_verifies_a_document_attestation_with_a_distinct_domain() {
        let attestation =
            DocumentCalculationAttestation::sign(unsigned_document(), &signing_key()).unwrap();
        assert_eq!(
            attestation.unsigned.payload_digest().unwrap(),
            attestation.payload_digest
        );
        attestation.verify_at(1_800_000_100).unwrap();
        let binding = DocumentAttestationBinding {
            document_schema: TAX_DOCUMENT_SCHEMA_V1,
            merchant_id: [1; 32],
            document_id: "invoice-1001",
            engine_version: "tax-engine-v1",
            rule_set_id: "us-ca-sales-tax",
            rule_version: 7,
            jurisdiction: "US-CA",
            currency: "USD",
            asset_mint: [2; 32],
            facts_digest: [3; 32],
            rule_digest: [4; 32],
            gross_atomic: 1_155,
            tax_atomic: 155,
            net_atomic: 1_000,
            treatment_outcome_digest: Some(treatment_outcome_digest(
                Treatment::Taxable,
                155,
                155,
                0,
                0,
                1_155,
            )),
        };
        attestation.verify_binding(&binding).unwrap();
    }

    #[test]
    fn document_attestation_tampering_and_scope_substitution_fail_closed() {
        let mut attestation =
            DocumentCalculationAttestation::sign(unsigned_document(), &signing_key()).unwrap();
        attestation.unsigned.document_id = "invoice-other".to_string();
        assert_eq!(
            attestation.verify(),
            Err(AttestationError::PayloadDigestMismatch)
        );

        let aggregate = CalculationAttestation::sign(unsigned(), &signing_key()).unwrap();
        let document_payload = aggregate.unsigned.canonical_payload().unwrap();
        assert_ne!(
            document_payload,
            DocumentCalculationAttestation::sign(unsigned_document(), &signing_key())
                .unwrap()
                .unsigned
                .canonical_payload()
                .unwrap()
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

    fn unsigned_applicability() -> UnsignedApplicabilityAttestation {
        let facts = tax_applicability::ApplicabilityFacts {
            schema: tax_applicability::APPLICABILITY_SCHEMA_V1.to_string(),
            product_category: "digital-service".to_string(),
            supply_type: "cross-border".to_string(),
            customer_class: "business".to_string(),
            merchant_registration: "registered".to_string(),
            customer_registration: "unregistered".to_string(),
            transaction_type: "sale".to_string(),
            evidence_digest: [6; 32],
            valid_until_unix: 1_800_000_600,
        };
        let facts_digest = facts.canonical_digest().unwrap();
        let decision = tax_applicability::ApplicabilityDecision {
            schema: tax_applicability::APPLICABILITY_SCHEMA_V1.to_string(),
            policy_id: "policy-us-ca-digital".to_string(),
            policy_version: 3,
            policy_digest: [7; 32],
            facts_digest,
            treatment: tax_applicability::Treatment::Taxable,
            selected_rule_id: Some("taxable-digital-service".to_string()),
            matched_rule_ids: vec!["taxable-digital-service".to_string()],
            evaluated_at_unix: 1_800_000_100,
        };
        UnsignedApplicabilityAttestation {
            schema_version: APPLICABILITY_ATTESTATION_SCHEMA_VERSION,
            merchant_id: [1; 32],
            jurisdiction_resolution_digest: [2; 32],
            facts,
            decision,
            rule_set_id: "us-ca-sales-tax".to_string(),
            rule_version: 7,
            rule_digest: [3; 32],
            issued_at_unix: 1_800_000_100,
            expires_at_unix: 1_800_000_500,
            sequence: 9,
            issuer_key_id: [4; 32],
        }
    }

    #[test]
    fn signs_and_verifies_an_applicability_attestation() {
        let attestation =
            ApplicabilityAttestation::sign(unsigned_applicability(), &signing_key()).unwrap();
        attestation.verify_at(1_800_000_200).unwrap();
        let binding = ApplicabilityAttestationBinding {
            merchant_id: [1; 32],
            jurisdiction_resolution_digest: [2; 32],
            facts_digest: attestation.unsigned.decision.facts_digest,
            decision_digest: attestation.unsigned.decision.canonical_digest().unwrap(),
            evidence_digest: [6; 32],
            policy_id: "policy-us-ca-digital",
            policy_version: 3,
            policy_digest: [7; 32],
            treatment: tax_applicability::Treatment::Taxable,
            product_category: "digital-service",
            supply_type: "cross-border",
            rule_set_id: "us-ca-sales-tax",
            rule_version: 7,
            rule_digest: [3; 32],
        };
        attestation.verify_binding(&binding).unwrap();
        let mut mismatched_decision = binding.clone();
        mismatched_decision.decision_digest = [8; 32];
        assert_eq!(
            attestation.verify_binding(&mismatched_decision),
            Err(AttestationError::BindingMismatch)
        );
    }

    #[test]
    fn applicability_attestation_rejects_facts_substitution_and_evidence_overrun() {
        let mut substituted = unsigned_applicability();
        substituted.facts.product_category = "physical-good".to_string();
        assert_eq!(
            ApplicabilityAttestation::sign(substituted, &signing_key()),
            Err(AttestationError::ApplicabilityFactsBindingMismatch)
        );

        let mut expired_source = unsigned_applicability();
        expired_source.expires_at_unix = 1_800_000_601;
        assert_eq!(
            ApplicabilityAttestation::sign(expired_source, &signing_key()),
            Err(AttestationError::ApplicabilityEvidenceWindowMismatch)
        );
    }

    #[test]
    fn applicability_attestation_has_a_distinct_canonical_domain() {
        let applicability =
            ApplicabilityAttestation::sign(unsigned_applicability(), &signing_key()).unwrap();
        let calculation = CalculationAttestation::sign(unsigned(), &signing_key()).unwrap();
        assert_ne!(
            applicability.unsigned.canonical_payload().unwrap(),
            calculation.unsigned.canonical_payload().unwrap()
        );
    }

    #[test]
    fn applicability_attestation_rejects_numeric_sequence_json() {
        let attestation =
            ApplicabilityAttestation::sign(unsigned_applicability(), &signing_key()).unwrap();
        let mut wire = serde_json::to_value(attestation).unwrap();
        wire["unsigned"]["sequence"] = serde_json::json!(9);
        assert!(serde_json::from_value::<ApplicabilityAttestation>(wire).is_err());
    }
}
