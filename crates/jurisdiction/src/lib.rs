#![deny(unsafe_code)]

//! Deterministic, evidence-bound jurisdiction resolution.
//!
//! This crate is intentionally a pure domain boundary. It does not geocode,
//! call a provider, inspect a wallet, consult a clock, or make a legal
//! determination. Adapters authenticate and normalize evidence before calling
//! [`ResolutionPolicy::resolve`].

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub type Digest32 = [u8; 32];
pub const MAX_POLICY_SOURCES: usize = 32;
pub const MAX_CANDIDATES: usize = 32;
pub const MAX_CODE_BYTES: usize = 64;
pub const JURISDICTION_EVIDENCE_SCHEMA_VERSION: u16 = 1;
pub const MAX_EVIDENCE_LIFETIME_SECONDS: i64 = 86_400;
const EVIDENCE_DOMAIN_SEPARATOR: &[u8] = b"solana-tax-protocol/jurisdiction-evidence/v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub enum EvidenceSource {
    MerchantRegistration,
    CustomerResidence,
    BillingAddress,
    ShippingDestination,
    PlaceOfPerformance,
    WalletResidence,
    IpGeolocation,
    #[serde(rename = "custom")]
    Custom(String),
}

impl EvidenceSource {
    pub fn canonical_name(&self) -> String {
        match self {
            Self::MerchantRegistration => "merchant_registration".to_string(),
            Self::CustomerResidence => "customer_residence".to_string(),
            Self::BillingAddress => "billing_address".to_string(),
            Self::ShippingDestination => "shipping_destination".to_string(),
            Self::PlaceOfPerformance => "place_of_performance".to_string(),
            Self::WalletResidence => "wallet_residence".to_string(),
            Self::IpGeolocation => "ip_geolocation".to_string(),
            Self::Custom(value) => value.clone(),
        }
    }

    fn tag(&self) -> u8 {
        match self {
            Self::MerchantRegistration => 0,
            Self::CustomerResidence => 1,
            Self::BillingAddress => 2,
            Self::ShippingDestination => 3,
            Self::PlaceOfPerformance => 4,
            Self::WalletResidence => 5,
            Self::IpGeolocation => 6,
            Self::Custom(_) => 7,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ConflictMode {
    HighestPrecedence,
    RequireAgreement,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResolutionPolicy {
    pub id: String,
    pub version: u32,
    /// First entry is the highest precedence. The list must be unique.
    pub ordered_sources: Vec<EvidenceSource>,
    pub conflict_mode: ConflictMode,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JurisdictionCandidate {
    pub jurisdiction_code: String,
    pub source: EvidenceSource,
    pub evidence_digest: Digest32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnsignedJurisdictionEvidence {
    pub schema_version: u16,
    pub merchant_id: Digest32,
    pub facts_digest: Digest32,
    pub jurisdiction_code: String,
    pub source: EvidenceSource,
    pub evidence_digest: Digest32,
    pub issued_at_unix: i64,
    pub expires_at_unix: i64,
    pub provider_key_id: Digest32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JurisdictionEvidenceEnvelope {
    pub unsigned: UnsignedJurisdictionEvidence,
    pub provider_public_key: [u8; 32],
    pub payload_digest: Digest32,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DecisionTraceEntry {
    pub source: EvidenceSource,
    pub jurisdiction_codes: Vec<String>,
    pub candidate_count: u32,
    pub selected: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JurisdictionResolution {
    pub policy_id: String,
    pub policy_version: u32,
    pub policy_digest: Digest32,
    pub evidence_digest: Digest32,
    pub jurisdiction_code: String,
    pub source: EvidenceSource,
    pub candidate_count: u32,
    pub trace: Vec<DecisionTraceEntry>,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum JurisdictionError {
    #[error("invalid resolution policy: {0}")]
    InvalidPolicy(&'static str),
    #[error("invalid jurisdiction candidate: {0}")]
    InvalidCandidate(&'static str),
    #[error("evidence source is not configured in the resolution policy: {0}")]
    UnknownSource(String),
    #[error("candidate evidence digest must be unique")]
    DuplicateEvidence,
    #[error("no jurisdiction candidates were supplied")]
    NoCandidates,
    #[error("jurisdiction candidates conflict for {source_name}: {jurisdictions:?}")]
    ConflictingJurisdictions {
        source_name: String,
        jurisdictions: Vec<String>,
    },
    #[error("resolution result does not match a fresh deterministic replay")]
    ReplayMismatch,
    #[error("unsupported jurisdiction evidence schema version")]
    UnsupportedEvidenceSchema,
    #[error("jurisdiction evidence contains an invalid identity, code, or digest")]
    InvalidEvidence,
    #[error("jurisdiction evidence validity window is invalid or too long")]
    InvalidEvidenceValidity,
    #[error("jurisdiction evidence provider key is invalid")]
    InvalidEvidenceKey,
    #[error("jurisdiction evidence signature is invalid")]
    InvalidEvidenceSignature,
    #[error("jurisdiction evidence payload digest does not match")]
    EvidencePayloadDigestMismatch,
    #[error("jurisdiction evidence does not bind to the quote")]
    EvidenceBindingMismatch,
    #[error("jurisdiction evidence is outside its validity window")]
    EvidenceOutsideValidityWindow,
}

impl UnsignedJurisdictionEvidence {
    pub fn validate(&self) -> Result<(), JurisdictionError> {
        if self.schema_version != JURISDICTION_EVIDENCE_SCHEMA_VERSION {
            return Err(JurisdictionError::UnsupportedEvidenceSchema);
        }
        normalize_code(&self.jurisdiction_code)?;
        if self.merchant_id == [0; 32]
            || self.facts_digest == [0; 32]
            || self.evidence_digest == [0; 32]
            || self.provider_key_id == [0; 32]
        {
            return Err(JurisdictionError::InvalidEvidence);
        }
        if self.issued_at_unix < 0
            || self.expires_at_unix <= self.issued_at_unix
            || self
                .expires_at_unix
                .checked_sub(self.issued_at_unix)
                .is_none_or(|lifetime| lifetime > MAX_EVIDENCE_LIFETIME_SECONDS)
        {
            return Err(JurisdictionError::InvalidEvidenceValidity);
        }
        Ok(())
    }

    pub fn canonical_payload(&self) -> Result<Vec<u8>, JurisdictionError> {
        self.validate()?;
        let mut encoder = PayloadEncoder::new();
        encoder.bytes(EVIDENCE_DOMAIN_SEPARATOR);
        encoder.u16(self.schema_version);
        encoder.bytes(&self.merchant_id);
        encoder.bytes(&self.facts_digest);
        encoder.text(&normalize_code(&self.jurisdiction_code)?)?;
        encode_source_payload(&mut encoder, &self.source)?;
        encoder.bytes(&self.evidence_digest);
        encoder.i64(self.issued_at_unix);
        encoder.i64(self.expires_at_unix);
        encoder.bytes(&self.provider_key_id);
        Ok(encoder.finish())
    }
}

impl JurisdictionEvidenceEnvelope {
    pub fn sign(
        unsigned: UnsignedJurisdictionEvidence,
        signing_key: &SigningKey,
    ) -> Result<Self, JurisdictionError> {
        unsigned.validate()?;
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
    ) -> Result<JurisdictionCandidate, JurisdictionError> {
        self.unsigned.validate()?;
        if self.provider_public_key != expected_key.to_bytes()
            || self.unsigned.merchant_id != merchant_id
            || self.unsigned.facts_digest != facts_digest
        {
            return Err(JurisdictionError::EvidenceBindingMismatch);
        }
        if now_unix < self.unsigned.issued_at_unix || now_unix >= self.unsigned.expires_at_unix {
            return Err(JurisdictionError::EvidenceOutsideValidityWindow);
        }
        let payload = self.unsigned.canonical_payload()?;
        let expected_digest: Digest32 = Sha256::digest(&payload).into();
        if self.payload_digest != expected_digest {
            return Err(JurisdictionError::EvidencePayloadDigestMismatch);
        }
        if self.signature.len() != 64 {
            return Err(JurisdictionError::InvalidEvidenceSignature);
        }
        let signature = Signature::from_slice(&self.signature)
            .map_err(|_| JurisdictionError::InvalidEvidenceSignature)?;
        expected_key
            .verify(&payload, &signature)
            .map_err(|_| JurisdictionError::InvalidEvidenceSignature)?;
        Ok(JurisdictionCandidate {
            jurisdiction_code: normalize_code(&self.unsigned.jurisdiction_code)?,
            source: self.unsigned.source.clone(),
            evidence_digest: self.unsigned.evidence_digest,
        })
    }
}

impl ResolutionPolicy {
    pub fn canonical_digest(&self) -> Result<Digest32, JurisdictionError> {
        self.validate()?;
        let mut encoder = CanonicalEncoder::new();
        encoder.text(&self.id)?;
        encoder.u32(self.version);
        encoder.u8(match self.conflict_mode {
            ConflictMode::HighestPrecedence => 0,
            ConflictMode::RequireAgreement => 1,
        });
        encoder.count(self.ordered_sources.len())?;
        for source in &self.ordered_sources {
            encode_source(&mut encoder, source)?;
        }
        Ok(encoder.digest())
    }

    pub fn resolve(
        &self,
        candidates: &[JurisdictionCandidate],
    ) -> Result<JurisdictionResolution, JurisdictionError> {
        self.validate()?;
        if candidates.is_empty() {
            return Err(JurisdictionError::NoCandidates);
        }
        if candidates.len() > MAX_CANDIDATES {
            return Err(JurisdictionError::InvalidCandidate(
                "candidate count exceeds the bounded resolution limit",
            ));
        }

        let policy_digest = self.canonical_digest()?;
        let mut normalized = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let code = normalize_code(&candidate.jurisdiction_code)?;
            if candidate.evidence_digest == [0; 32] {
                return Err(JurisdictionError::InvalidCandidate(
                    "evidence digest must be non-zero",
                ));
            }
            let rank = self
                .ordered_sources
                .iter()
                .position(|source| source == &candidate.source)
                .ok_or_else(|| {
                    JurisdictionError::UnknownSource(candidate.source.canonical_name())
                })?;
            normalized.push(NormalizedCandidate {
                code,
                source: candidate.source.clone(),
                evidence_digest: candidate.evidence_digest,
                rank,
            });
        }

        normalized.sort_by(|left, right| {
            left.rank
                .cmp(&right.rank)
                .then_with(|| left.code.cmp(&right.code))
                .then_with(|| left.evidence_digest.cmp(&right.evidence_digest))
        });
        for (index, candidate) in normalized.iter().enumerate() {
            if normalized[..index]
                .iter()
                .any(|previous| previous.evidence_digest == candidate.evidence_digest)
            {
                return Err(JurisdictionError::DuplicateEvidence);
            }
        }

        let (selected_source, selected_code) = match self.conflict_mode {
            ConflictMode::HighestPrecedence => {
                let highest_rank = normalized[0].rank;
                let top = normalized
                    .iter()
                    .filter(|candidate| candidate.rank == highest_rank)
                    .collect::<Vec<_>>();
                let jurisdictions = distinct_codes(&top);
                if jurisdictions.len() != 1 {
                    return Err(JurisdictionError::ConflictingJurisdictions {
                        source_name: top[0].source.canonical_name(),
                        jurisdictions,
                    });
                }
                (top[0].source.clone(), top[0].code.clone())
            }
            ConflictMode::RequireAgreement => {
                let jurisdictions = distinct_codes(&normalized.iter().collect::<Vec<_>>());
                if jurisdictions.len() != 1 {
                    return Err(JurisdictionError::ConflictingJurisdictions {
                        source_name: "all_sources".to_string(),
                        jurisdictions,
                    });
                }
                (normalized[0].source.clone(), normalized[0].code.clone())
            }
        };

        let evidence_digest = evidence_digest(policy_digest, &normalized)?;
        let trace = self
            .ordered_sources
            .iter()
            .map(|source| {
                let source_candidates = normalized
                    .iter()
                    .filter(|candidate| &candidate.source == source)
                    .collect::<Vec<_>>();
                let mut jurisdictions = distinct_codes(&source_candidates);
                jurisdictions.sort();
                DecisionTraceEntry {
                    source: source.clone(),
                    jurisdiction_codes: jurisdictions,
                    candidate_count: source_candidates.len() as u32,
                    selected: source == &selected_source,
                }
            })
            .collect();

        Ok(JurisdictionResolution {
            policy_id: self.id.clone(),
            policy_version: self.version,
            policy_digest,
            evidence_digest,
            jurisdiction_code: selected_code,
            source: selected_source,
            candidate_count: normalized.len() as u32,
            trace,
        })
    }

    fn validate(&self) -> Result<(), JurisdictionError> {
        if self.id.trim().is_empty() || self.id.len() > MAX_CODE_BYTES || self.version == 0 {
            return Err(JurisdictionError::InvalidPolicy(
                "policy id and positive version are required",
            ));
        }
        if self.ordered_sources.is_empty() || self.ordered_sources.len() > MAX_POLICY_SOURCES {
            return Err(JurisdictionError::InvalidPolicy(
                "policy source count is outside the supported bound",
            ));
        }
        for source in &self.ordered_sources {
            if source.canonical_name().trim().is_empty()
                || source.canonical_name().len() > MAX_CODE_BYTES
            {
                return Err(JurisdictionError::InvalidPolicy(
                    "policy source names must be non-empty and bounded",
                ));
            }
        }
        for (index, source) in self.ordered_sources.iter().enumerate() {
            if self.ordered_sources[..index]
                .iter()
                .any(|previous| previous == source)
            {
                return Err(JurisdictionError::InvalidPolicy(
                    "policy sources must be unique",
                ));
            }
        }
        Ok(())
    }
}

impl JurisdictionResolution {
    pub fn verify_replay(
        &self,
        policy: &ResolutionPolicy,
        candidates: &[JurisdictionCandidate],
    ) -> Result<(), JurisdictionError> {
        if policy.resolve(candidates)? != *self {
            return Err(JurisdictionError::ReplayMismatch);
        }
        Ok(())
    }
}

pub fn resolve(
    policy: &ResolutionPolicy,
    candidates: &[JurisdictionCandidate],
) -> Result<JurisdictionResolution, JurisdictionError> {
    policy.resolve(candidates)
}

/// Computes a stable digest for the verified candidate set before a policy is
/// selected. The quote idempotency boundary uses this to prevent a different
/// evidence set from replaying under the same key.
pub fn candidate_set_digest(
    candidates: &[JurisdictionCandidate],
) -> Result<Digest32, JurisdictionError> {
    if candidates.is_empty() {
        return Err(JurisdictionError::NoCandidates);
    }
    if candidates.len() > MAX_CANDIDATES {
        return Err(JurisdictionError::InvalidCandidate(
            "candidate count exceeds the bounded resolution limit",
        ));
    }
    let mut normalized = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if candidate.evidence_digest == [0; 32] {
            return Err(JurisdictionError::InvalidCandidate(
                "evidence digest must be non-zero",
            ));
        }
        normalized.push((
            normalize_code(&candidate.jurisdiction_code)?,
            candidate.source.clone(),
            candidate.evidence_digest,
        ));
    }
    normalized.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.canonical_name().cmp(&right.1.canonical_name()))
            .then_with(|| left.2.cmp(&right.2))
    });
    for (index, candidate) in normalized.iter().enumerate() {
        if normalized[..index]
            .iter()
            .any(|previous| previous.2 == candidate.2)
        {
            return Err(JurisdictionError::DuplicateEvidence);
        }
    }
    let mut encoder = CanonicalEncoder::new();
    encoder.text("solana-tax-protocol/jurisdiction-candidates/v1")?;
    encoder.count(normalized.len())?;
    for (code, source, evidence_digest) in normalized {
        encoder.text(&code)?;
        encode_source(&mut encoder, &source)?;
        encoder.bytes(&evidence_digest);
    }
    Ok(encoder.digest())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NormalizedCandidate {
    code: String,
    source: EvidenceSource,
    evidence_digest: Digest32,
    rank: usize,
}

fn normalize_code(value: &str) -> Result<String, JurisdictionError> {
    if value.trim() != value || value.trim().is_empty() || value.len() > MAX_CODE_BYTES {
        return Err(JurisdictionError::InvalidCandidate(
            "jurisdiction code must be non-empty, trimmed, and bounded",
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(JurisdictionError::InvalidCandidate(
            "jurisdiction code must not contain control characters",
        ));
    }
    Ok(value.to_ascii_uppercase())
}

fn distinct_codes(candidates: &[&NormalizedCandidate]) -> Vec<String> {
    let mut codes = candidates
        .iter()
        .map(|candidate| candidate.code.clone())
        .collect::<Vec<_>>();
    codes.sort();
    codes.dedup();
    codes
}

fn evidence_digest(
    policy_digest: Digest32,
    candidates: &[NormalizedCandidate],
) -> Result<Digest32, JurisdictionError> {
    let mut encoder = CanonicalEncoder::new();
    encoder.bytes(&policy_digest);
    encoder.count(candidates.len())?;
    for candidate in candidates {
        encoder.text(&candidate.code)?;
        encode_source(&mut encoder, &candidate.source)?;
        encoder.bytes(&candidate.evidence_digest);
    }
    Ok(encoder.digest())
}

fn encode_source(
    encoder: &mut CanonicalEncoder,
    source: &EvidenceSource,
) -> Result<(), JurisdictionError> {
    encoder.u8(source.tag());
    encoder.text(&source.canonical_name())
}

fn encode_source_payload(
    encoder: &mut PayloadEncoder,
    source: &EvidenceSource,
) -> Result<(), JurisdictionError> {
    encoder.u8(source.tag());
    encoder.text(&source.canonical_name())
}

struct CanonicalEncoder {
    bytes: Vec<u8>,
}

struct PayloadEncoder {
    bytes: Vec<u8>,
}

impl PayloadEncoder {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn bytes(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    fn text(&mut self, value: &str) -> Result<(), JurisdictionError> {
        let length = u16::try_from(value.len()).map_err(|_| JurisdictionError::InvalidEvidence)?;
        self.u16(length);
        self.bytes(value.as_bytes());
        Ok(())
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

impl CanonicalEncoder {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn bytes(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    fn text(&mut self, value: &str) -> Result<(), JurisdictionError> {
        self.count(value.len())?;
        self.bytes(value.as_bytes());
        Ok(())
    }

    fn count(&mut self, value: usize) -> Result<(), JurisdictionError> {
        self.u32(
            u32::try_from(value)
                .map_err(|_| JurisdictionError::InvalidPolicy("canonical input is too large"))?,
        );
        Ok(())
    }

    fn digest(self) -> Digest32 {
        Sha256::digest(self.bytes).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn digest(value: u8) -> Digest32 {
        [value; 32]
    }

    fn policy(mode: ConflictMode) -> ResolutionPolicy {
        ResolutionPolicy {
            id: "digital-commerce".to_string(),
            version: 1,
            ordered_sources: vec![
                EvidenceSource::ShippingDestination,
                EvidenceSource::CustomerResidence,
                EvidenceSource::MerchantRegistration,
            ],
            conflict_mode: mode,
        }
    }

    fn candidate(code: &str, source: EvidenceSource, evidence: u8) -> JurisdictionCandidate {
        JurisdictionCandidate {
            jurisdiction_code: code.to_string(),
            source,
            evidence_digest: digest(evidence),
        }
    }

    fn signed_evidence() -> JurisdictionEvidenceEnvelope {
        let signing_key = SigningKey::from_bytes(&[9; 32]);
        JurisdictionEvidenceEnvelope::sign(
            UnsignedJurisdictionEvidence {
                schema_version: JURISDICTION_EVIDENCE_SCHEMA_VERSION,
                merchant_id: [1; 32],
                facts_digest: [2; 32],
                jurisdiction_code: "us-ca".to_string(),
                source: EvidenceSource::CustomerResidence,
                evidence_digest: [3; 32],
                issued_at_unix: 1_800_000_000,
                expires_at_unix: 1_800_000_300,
                provider_key_id: [4; 32],
            },
            &signing_key,
        )
        .unwrap()
    }

    #[test]
    fn provider_signed_evidence_requires_key_merchant_facts_and_time_binding() {
        let envelope = signed_evidence();
        let signing_key = SigningKey::from_bytes(&[9; 32]);
        let candidate = envelope
            .verify_with_key_at(
                &signing_key.verifying_key(),
                [1; 32],
                [2; 32],
                1_800_000_100,
            )
            .unwrap();
        assert_eq!(candidate.jurisdiction_code, "US-CA");

        assert_eq!(
            envelope.verify_with_key_at(
                &SigningKey::from_bytes(&[8; 32]).verifying_key(),
                [1; 32],
                [2; 32],
                1_800_000_100,
            ),
            Err(JurisdictionError::EvidenceBindingMismatch)
        );
        assert_eq!(
            envelope.verify_with_key_at(
                &signing_key.verifying_key(),
                [1; 32],
                [5; 32],
                1_800_000_100
            ),
            Err(JurisdictionError::EvidenceBindingMismatch)
        );
        assert_eq!(
            envelope.verify_with_key_at(
                &signing_key.verifying_key(),
                [1; 32],
                [2; 32],
                1_800_000_300
            ),
            Err(JurisdictionError::EvidenceOutsideValidityWindow)
        );
    }

    #[test]
    fn provider_signed_evidence_rejects_payload_tampering() {
        let mut envelope = signed_evidence();
        envelope.unsigned.evidence_digest = [5; 32];
        let signing_key = SigningKey::from_bytes(&[9; 32]);
        assert_eq!(
            envelope.verify_with_key_at(
                &signing_key.verifying_key(),
                [1; 32],
                [2; 32],
                1_800_000_100
            ),
            Err(JurisdictionError::EvidencePayloadDigestMismatch)
        );
    }

    #[test]
    fn highest_precedence_is_independent_of_candidate_order() {
        let policy = policy(ConflictMode::HighestPrecedence);
        let first = vec![
            candidate("US-CA", EvidenceSource::CustomerResidence, 1),
            candidate("DE-BE", EvidenceSource::ShippingDestination, 2),
        ];
        let second = vec![first[1].clone(), first[0].clone()];
        let left = policy.resolve(&first).unwrap();
        let right = policy.resolve(&second).unwrap();
        assert_eq!(left, right);
        assert_eq!(left.jurisdiction_code, "DE-BE");
        assert_eq!(left.source, EvidenceSource::ShippingDestination);
        assert_eq!(left.candidate_count, 2);
    }

    #[test]
    fn same_precedence_conflict_fails_closed() {
        let policy = policy(ConflictMode::HighestPrecedence);
        let error = policy
            .resolve(&[
                candidate("US-CA", EvidenceSource::ShippingDestination, 1),
                candidate("US-NY", EvidenceSource::ShippingDestination, 2),
            ])
            .unwrap_err();
        assert!(matches!(
            error,
            JurisdictionError::ConflictingJurisdictions { .. }
        ));
    }

    #[test]
    fn require_agreement_rejects_lower_precedence_conflict() {
        let policy = policy(ConflictMode::RequireAgreement);
        let error = policy
            .resolve(&[
                candidate("US-CA", EvidenceSource::ShippingDestination, 1),
                candidate("US-NY", EvidenceSource::CustomerResidence, 2),
            ])
            .unwrap_err();
        assert!(matches!(
            error,
            JurisdictionError::ConflictingJurisdictions { .. }
        ));
    }

    #[test]
    fn unknown_source_and_duplicate_evidence_are_rejected() {
        let policy = policy(ConflictMode::HighestPrecedence);
        let unknown = policy
            .resolve(&[candidate("US-CA", EvidenceSource::BillingAddress, 1)])
            .unwrap_err();
        assert!(matches!(unknown, JurisdictionError::UnknownSource(_)));

        let duplicate = policy
            .resolve(&[
                candidate("US-CA", EvidenceSource::ShippingDestination, 1),
                candidate("US-CA", EvidenceSource::CustomerResidence, 1),
            ])
            .unwrap_err();
        assert_eq!(duplicate, JurisdictionError::DuplicateEvidence);
    }

    #[test]
    fn replay_detects_policy_or_evidence_tampering() {
        let policy = policy(ConflictMode::HighestPrecedence);
        let candidates = vec![candidate("us-ca", EvidenceSource::ShippingDestination, 1)];
        let result = policy.resolve(&candidates).unwrap();
        result.verify_replay(&policy, &candidates).unwrap();

        let mut tampered = result.clone();
        tampered.evidence_digest = digest(99);
        assert_eq!(
            tampered.verify_replay(&policy, &candidates),
            Err(JurisdictionError::ReplayMismatch)
        );
    }

    #[test]
    fn golden_vector_contract_is_decoded_and_resolved() {
        #[derive(Deserialize)]
        struct Fixture {
            schema: String,
            policy: ResolutionPolicy,
            candidates: Vec<JurisdictionCandidate>,
            expected: Expected,
        }
        #[derive(Deserialize)]
        struct Expected {
            jurisdiction_code: String,
            source: EvidenceSource,
        }

        let fixture: Fixture = serde_json::from_str(include_str!(
            "../../../test-vectors/jurisdiction-resolution-v1.json"
        ))
        .unwrap();
        assert_eq!(
            fixture.schema,
            "solana-tax-protocol/jurisdiction-resolution/v1"
        );
        let result = fixture.policy.resolve(&fixture.candidates).unwrap();
        assert_eq!(result.jurisdiction_code, fixture.expected.jurisdiction_code);
        assert_eq!(result.source, fixture.expected.source);
    }

    proptest! {
        #[test]
        fn candidate_permutation_does_not_change_resolution(
            code in prop::sample::select(vec!["US-CA".to_string(), "US-NY".to_string()]),
            evidence in 1u8..=200u8,
        ) {
            let policy = policy(ConflictMode::HighestPrecedence);
            let candidates = vec![
                candidate(&code, EvidenceSource::CustomerResidence, evidence),
                candidate(&code, EvidenceSource::ShippingDestination, evidence.saturating_add(1)),
            ];
            let mut reversed = candidates.clone();
            reversed.reverse();
            prop_assert_eq!(policy.resolve(&candidates), policy.resolve(&reversed));
        }
    }

    #[test]
    fn invalid_policy_entries_fail_before_resolution() {
        let mut policy = policy(ConflictMode::HighestPrecedence);
        policy
            .ordered_sources
            .push(EvidenceSource::ShippingDestination);
        assert_eq!(
            policy.canonical_digest(),
            Err(JurisdictionError::InvalidPolicy(
                "policy sources must be unique"
            ))
        );
    }

    #[test]
    fn ordering_of_trace_is_policy_order_not_input_order() {
        let policy = policy(ConflictMode::HighestPrecedence);
        let result = policy
            .resolve(&[
                candidate("US-CA", EvidenceSource::MerchantRegistration, 1),
                candidate("US-CA", EvidenceSource::ShippingDestination, 2),
            ])
            .unwrap();
        assert_eq!(result.trace[0].source, EvidenceSource::ShippingDestination);
        assert!(result.trace[0].selected);
        assert!(!result.trace[2].selected);
    }

    #[test]
    fn source_order_is_canonicalized_by_policy_position() {
        let left = policy(ConflictMode::HighestPrecedence).canonical_digest();
        let mut reordered = policy(ConflictMode::HighestPrecedence);
        reordered.ordered_sources.swap(0, 1);
        let right = reordered.canonical_digest();
        assert_ne!(left, right);
    }
}
