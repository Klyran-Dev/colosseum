#![deny(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

//! Vendor-neutral FX provider policy and server-side evidence issuance.
//!
//! Provider transport, legal ownership, and production key custody are
//! application concerns. This crate keeps policy evaluation exact and bounded
//! before it creates an envelope consumed by the deterministic tax engine.

use ed25519_dalek::{Signature, VerifyingKey};
use fx_evidence::{
    Digest32, FX_EVIDENCE_SCHEMA_VERSION, FxEvidenceEnvelope, FxEvidenceError, MAX_CODE_BYTES,
    MAX_CURRENCY_CODE_BYTES, UnsignedFxEvidence,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tax_engine::{MAX_ATOMIC_AMOUNT, RoundingMode};
use thiserror::Error;

pub const MAX_ALLOWED_PAIRS: usize = 64;
pub const MAX_PROVIDER_OBSERVATION_AGE_SECONDS: i64 = 86_400;
pub const MAX_PROVIDER_FUTURE_SKEW_SECONDS: i64 = 300;
pub const MAX_PROVIDER_POLICY_LIFETIME_SECONDS: i64 = 86_400;
pub const MAX_DEVIATION_BASIS_POINTS: u32 = 1_000_000;
const ISSUED_EVIDENCE_DOMAIN_SEPARATOR: &[u8] =
    b"solana-tax-protocol/fx-provider-issued-evidence/v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CurrencyPair {
    pub source_currency: String,
    pub target_currency: String,
}

impl CurrencyPair {
    pub fn new(source_currency: &str, target_currency: &str) -> Result<Self, FxProviderError> {
        validate_currency(source_currency)?;
        validate_currency(target_currency)?;
        if source_currency == target_currency {
            return Err(FxProviderError::InvalidConfiguration(
                "currency pair must contain distinct currencies",
            ));
        }
        Ok(Self {
            source_currency: source_currency.to_string(),
            target_currency: target_currency.to_string(),
        })
    }

    fn matches(&self, source_currency: &str, target_currency: &str) -> bool {
        self.source_currency == source_currency && self.target_currency == target_currency
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LegalAcceptance {
    Approved { authority_digest: Digest32 },
    NotApproved,
    Rejected,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FxQuoteCandidate {
    pub source_currency: String,
    pub target_currency: String,
    pub numerator: i128,
    pub denominator: i128,
    pub rounding_mode: RoundingMode,
    pub quantum_atomic: i128,
    pub provider: String,
    pub quote_id: String,
    pub observed_at_unix: i64,
    pub expires_at_unix: i64,
    pub evidence_digest: Digest32,
    pub legal_acceptance: LegalAcceptance,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExactRate {
    pub numerator: i128,
    pub denominator: i128,
}

impl ExactRate {
    fn validate(self) -> Result<Self, FxProviderError> {
        if self.numerator <= 0 || self.denominator <= 0 {
            return Err(FxProviderError::InvalidReferenceRate);
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FxProviderPolicy {
    pub policy_digest: Digest32,
    pub allowed_pairs: Vec<CurrencyPair>,
    pub max_observation_age_seconds: i64,
    pub max_future_skew_seconds: i64,
    pub max_quote_lifetime_seconds: i64,
    pub require_legal_acceptance: bool,
    pub max_deviation_basis_points: Option<u32>,
}

impl FxProviderPolicy {
    pub fn validate(&self) -> Result<(), FxProviderError> {
        if self.policy_digest == [0; 32] {
            return Err(FxProviderError::InvalidConfiguration(
                "policy digest must be non-zero",
            ));
        }
        if self.allowed_pairs.is_empty() || self.allowed_pairs.len() > MAX_ALLOWED_PAIRS {
            return Err(FxProviderError::InvalidConfiguration(
                "allowed currency pairs are empty or unbounded",
            ));
        }
        if self.max_observation_age_seconds <= 0
            || self.max_observation_age_seconds > MAX_PROVIDER_OBSERVATION_AGE_SECONDS
            || self.max_future_skew_seconds < 0
            || self.max_future_skew_seconds > MAX_PROVIDER_FUTURE_SKEW_SECONDS
            || self.max_quote_lifetime_seconds <= 0
            || self.max_quote_lifetime_seconds > MAX_PROVIDER_POLICY_LIFETIME_SECONDS
        {
            return Err(FxProviderError::InvalidConfiguration(
                "FX policy time bounds are invalid",
            ));
        }
        if self
            .max_deviation_basis_points
            .is_some_and(|basis_points| basis_points > MAX_DEVIATION_BASIS_POINTS)
        {
            return Err(FxProviderError::InvalidConfiguration(
                "FX deviation bound is invalid",
            ));
        }
        for (index, pair) in self.allowed_pairs.iter().enumerate() {
            CurrencyPair::new(&pair.source_currency, &pair.target_currency)?;
            if self.allowed_pairs[..index]
                .iter()
                .any(|previous| previous == pair)
            {
                return Err(FxProviderError::InvalidConfiguration(
                    "allowed currency pairs contain duplicates",
                ));
            }
        }
        Ok(())
    }

    fn validate_candidate(
        &self,
        candidate: &FxQuoteCandidate,
        now_unix: i64,
        reference_rate: Option<ExactRate>,
    ) -> Result<(), FxProviderError> {
        validate_currency(&candidate.source_currency)?;
        validate_currency(&candidate.target_currency)?;
        if !self
            .allowed_pairs
            .iter()
            .any(|pair| pair.matches(&candidate.source_currency, &candidate.target_currency))
        {
            return Err(FxProviderError::UnsupportedCurrencyPair);
        }
        if candidate.numerator <= 0 || candidate.denominator <= 0 {
            return Err(FxProviderError::InvalidCandidate("rate must be positive"));
        }
        if candidate.quantum_atomic <= 0 || candidate.quantum_atomic > MAX_ATOMIC_AMOUNT {
            return Err(FxProviderError::InvalidCandidate(
                "rounding quantum is invalid",
            ));
        }
        validate_bounded_text(&candidate.provider, MAX_CODE_BYTES, "provider")?;
        validate_bounded_text(&candidate.quote_id, MAX_CODE_BYTES, "quote identity")?;
        if candidate.evidence_digest == [0; 32] {
            return Err(FxProviderError::InvalidCandidate(
                "provider evidence digest must be non-zero",
            ));
        }
        if self.require_legal_acceptance
            && !matches!(
                candidate.legal_acceptance,
                LegalAcceptance::Approved {
                    authority_digest: digest
                } if digest != [0; 32]
            )
        {
            return Err(FxProviderError::LegalAcceptanceRequired);
        }
        if candidate.observed_at_unix < 0 || candidate.expires_at_unix <= candidate.observed_at_unix
        {
            return Err(FxProviderError::InvalidCandidate(
                "observation window is invalid",
            ));
        }
        if candidate.observed_at_unix > now_unix {
            let bounded_skew = candidate
                .observed_at_unix
                .checked_sub(now_unix)
                .ok_or(FxProviderError::ObservationInFuture)?;
            if bounded_skew > self.max_future_skew_seconds {
                return Err(FxProviderError::ObservationInFuture);
            }
        }
        if now_unix
            .checked_sub(candidate.observed_at_unix)
            .is_none_or(|age| age > self.max_observation_age_seconds)
        {
            return Err(FxProviderError::ObservationTooOld);
        }
        let lifetime = candidate
            .expires_at_unix
            .checked_sub(candidate.observed_at_unix)
            .ok_or(FxProviderError::LifetimeExceeded)?;
        if lifetime > self.max_quote_lifetime_seconds {
            return Err(FxProviderError::LifetimeExceeded);
        }
        if candidate.expires_at_unix <= now_unix {
            return Err(FxProviderError::QuoteExpired);
        }
        if let Some(max_deviation_basis_points) = self.max_deviation_basis_points {
            let reference_rate = reference_rate
                .ok_or(FxProviderError::ReferenceRateRequired)?
                .validate()?;
            validate_deviation(
                ExactRate {
                    numerator: candidate.numerator,
                    denominator: candidate.denominator,
                },
                reference_rate,
                max_deviation_basis_points,
            )?;
        }
        Ok(())
    }
}

pub trait FxEvidenceSigner {
    fn key_id(&self) -> Digest32;
    fn public_key(&self) -> [u8; 32];
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, FxProviderError>;
}

pub trait FxQuoteSource {
    fn quote(&self, request: &FxQuoteRequest) -> Result<FxQuoteCandidate, FxProviderError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FxQuoteRequest {
    pub merchant_id: Digest32,
    pub facts_digest: Digest32,
    pub source_currency: String,
    pub target_currency: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssuedFxEvidence {
    pub envelope: FxEvidenceEnvelope,
    pub source_evidence_digest: Digest32,
    pub policy_digest: Digest32,
    pub legal_authority_digest: Digest32,
}

pub struct FxEvidenceIssuer<S> {
    policy: FxProviderPolicy,
    signer: S,
}

impl<S> FxEvidenceIssuer<S>
where
    S: FxEvidenceSigner,
{
    pub fn new(policy: FxProviderPolicy, signer: S) -> Self {
        Self { policy, signer }
    }

    pub fn issue(
        &self,
        candidate: FxQuoteCandidate,
        merchant_id: Digest32,
        facts_digest: Digest32,
        now_unix: i64,
        reference_rate: Option<ExactRate>,
    ) -> Result<IssuedFxEvidence, FxProviderError> {
        self.policy.validate()?;
        if merchant_id == [0; 32] || facts_digest == [0; 32] {
            return Err(FxProviderError::InvalidCandidate(
                "merchant and facts digests must be non-zero",
            ));
        }
        self.policy
            .validate_candidate(&candidate, now_unix, reference_rate)?;
        let legal_authority_digest = match candidate.legal_acceptance {
            LegalAcceptance::Approved { authority_digest } if authority_digest != [0; 32] => {
                authority_digest
            }
            _ if self.policy.require_legal_acceptance => {
                return Err(FxProviderError::LegalAcceptanceRequired);
            }
            _ => [0; 32],
        };
        let key_id = self.signer.key_id();
        let public_key = self.signer.public_key();
        if key_id == [0; 32] || public_key == [0; 32] {
            return Err(FxProviderError::SignerIdentityInvalid);
        }
        let source_evidence_digest = candidate.evidence_digest;
        let evidence_digest = issued_evidence_digest(
            source_evidence_digest,
            self.policy.policy_digest,
            legal_authority_digest,
        );
        let unsigned = UnsignedFxEvidence {
            schema_version: FX_EVIDENCE_SCHEMA_VERSION,
            merchant_id,
            facts_digest,
            source_currency: candidate.source_currency,
            target_currency: candidate.target_currency,
            numerator: candidate.numerator,
            denominator: candidate.denominator,
            rounding_mode: candidate.rounding_mode,
            quantum_atomic: candidate.quantum_atomic,
            provider: candidate.provider,
            quote_id: candidate.quote_id,
            observed_at_unix: candidate.observed_at_unix,
            expires_at_unix: candidate.expires_at_unix,
            evidence_digest,
            provider_key_id: key_id,
        };
        let payload = unsigned.canonical_payload()?;
        let signature = self.signer.sign(&payload)?;
        if Signature::from_slice(&signature).is_err() {
            return Err(FxProviderError::SignerReturnedInvalidSignature);
        }
        let verifying_key = VerifyingKey::from_bytes(&public_key)
            .map_err(|_| FxProviderError::SignerIdentityInvalid)?;
        let envelope = FxEvidenceEnvelope {
            unsigned,
            provider_public_key: public_key,
            payload_digest: Sha256::digest(&payload).into(),
            signature,
        };
        envelope
            .verify_with_key_at_and_future_skew(
                &verifying_key,
                merchant_id,
                facts_digest,
                now_unix,
                self.policy.max_future_skew_seconds,
            )
            .map_err(FxProviderError::Evidence)?;
        Ok(IssuedFxEvidence {
            envelope,
            source_evidence_digest,
            policy_digest: self.policy.policy_digest,
            legal_authority_digest,
        })
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum FxProviderError {
    #[error("invalid FX provider configuration: {0}")]
    InvalidConfiguration(&'static str),
    #[error("invalid FX quote candidate: {0}")]
    InvalidCandidate(&'static str),
    #[error("unsupported FX currency pair")]
    UnsupportedCurrencyPair,
    #[error("legal FX acceptance is required")]
    LegalAcceptanceRequired,
    #[error("FX observation is too old")]
    ObservationTooOld,
    #[error("FX observation is in the future")]
    ObservationInFuture,
    #[error("FX quote lifetime exceeds policy")]
    LifetimeExceeded,
    #[error("FX quote is expired")]
    QuoteExpired,
    #[error("a reference FX rate is required")]
    ReferenceRateRequired,
    #[error("reference FX rate is invalid")]
    InvalidReferenceRate,
    #[error("FX deviation exceeds policy")]
    DeviationExceeded,
    #[error("FX deviation arithmetic overflowed")]
    DeviationOverflow,
    #[error("FX signer failed")]
    SignerFailed,
    #[error("FX signer identity is invalid")]
    SignerIdentityInvalid,
    #[error("FX signer returned an invalid signature")]
    SignerReturnedInvalidSignature,
    #[error("FX evidence envelope is invalid: {0}")]
    Evidence(#[from] FxEvidenceError),
}

fn validate_deviation(
    candidate: ExactRate,
    reference: ExactRate,
    max_deviation_basis_points: u32,
) -> Result<(), FxProviderError> {
    let candidate_cross = candidate
        .numerator
        .checked_mul(reference.denominator)
        .ok_or(FxProviderError::DeviationOverflow)?;
    let reference_cross = reference
        .numerator
        .checked_mul(candidate.denominator)
        .ok_or(FxProviderError::DeviationOverflow)?;
    let difference = if candidate_cross >= reference_cross {
        (candidate_cross - reference_cross).unsigned_abs()
    } else {
        (reference_cross - candidate_cross).unsigned_abs()
    };
    let scaled_difference = difference
        .checked_mul(10_000)
        .ok_or(FxProviderError::DeviationOverflow)?;
    let reference_base = reference_cross.unsigned_abs();
    let permitted = reference_base
        .checked_mul(u128::from(max_deviation_basis_points))
        .ok_or(FxProviderError::DeviationOverflow)?;
    if scaled_difference > permitted {
        return Err(FxProviderError::DeviationExceeded);
    }
    Ok(())
}

fn issued_evidence_digest(
    source_evidence_digest: Digest32,
    policy_digest: Digest32,
    legal_authority_digest: Digest32,
) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(ISSUED_EVIDENCE_DOMAIN_SEPARATOR);
    hasher.update(source_evidence_digest);
    hasher.update(policy_digest);
    hasher.update(legal_authority_digest);
    hasher.finalize().into()
}

fn validate_currency(value: &str) -> Result<(), FxProviderError> {
    validate_bounded_text(value, MAX_CURRENCY_CODE_BYTES, "currency")
}

fn validate_bounded_text(
    value: &str,
    max_bytes: usize,
    label: &'static str,
) -> Result<(), FxProviderError> {
    if value.trim().is_empty() || value.len() > max_bytes {
        return Err(FxProviderError::InvalidCandidate(label));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issues_exact_server_bound_evidence_after_policy_acceptance() {
        let signer = TestSigner::new([3; 32], [4; 32]);
        let policy = policy();
        let candidate = candidate();
        let issued = FxEvidenceIssuer::new(policy.clone(), signer)
            .issue(
                candidate,
                [7; 32],
                [8; 32],
                1_800_000_100,
                Some(ExactRate {
                    numerator: 9,
                    denominator: 8,
                }),
            )
            .expect("policy-approved evidence");

        assert_eq!(issued.policy_digest, policy.policy_digest);
        assert_eq!(issued.legal_authority_digest, [10; 32]);
        assert_ne!(
            issued.source_evidence_digest,
            issued.envelope.unsigned.evidence_digest
        );
        assert_eq!(issued.envelope.unsigned.merchant_id, [7; 32]);
        assert_eq!(issued.envelope.unsigned.facts_digest, [8; 32]);
    }

    #[test]
    fn rejects_pair_legal_freshness_future_and_lifetime_failures() {
        let signer = TestSigner::new([3; 32], [4; 32]);
        let issuer = FxEvidenceIssuer::new(policy(), signer);

        let mut unsupported = candidate();
        unsupported.target_currency = "JPY".to_string();
        assert_eq!(
            issuer.issue(unsupported, [7; 32], [8; 32], 1_800_000_100, None),
            Err(FxProviderError::UnsupportedCurrencyPair)
        );

        let mut unapproved = candidate();
        unapproved.legal_acceptance = LegalAcceptance::Rejected;
        assert_eq!(
            issuer.issue(unapproved, [7; 32], [8; 32], 1_800_000_100, None),
            Err(FxProviderError::LegalAcceptanceRequired)
        );

        let mut stale = candidate();
        stale.observed_at_unix = 1_799_999_000;
        assert_eq!(
            issuer.issue(stale, [7; 32], [8; 32], 1_800_000_100, None),
            Err(FxProviderError::ObservationTooOld)
        );

        let mut future = candidate();
        future.observed_at_unix = 1_800_000_500;
        future.expires_at_unix = 1_800_000_800;
        assert_eq!(
            issuer.issue(future, [7; 32], [8; 32], 1_800_000_100, None),
            Err(FxProviderError::ObservationInFuture)
        );

        let mut long_lived = candidate();
        long_lived.expires_at_unix = 1_800_100_000;
        assert_eq!(
            issuer.issue(long_lived, [7; 32], [8; 32], 1_800_000_100, None),
            Err(FxProviderError::LifetimeExceeded)
        );

        let mut bounded_skew = candidate();
        bounded_skew.observed_at_unix = 1_800_000_120;
        assert!(
            issuer
                .issue(
                    bounded_skew,
                    [7; 32],
                    [8; 32],
                    1_800_000_100,
                    Some(ExactRate {
                        numerator: 9,
                        denominator: 8,
                    })
                )
                .is_ok()
        );
    }

    #[test]
    fn enforces_exact_deviation_boundary_and_reference_presence() {
        let signer = TestSigner::new([3; 32], [4; 32]);
        let issuer = FxEvidenceIssuer::new(policy(), signer);
        let accepted = issuer
            .issue(
                candidate(),
                [7; 32],
                [8; 32],
                1_800_000_100,
                Some(ExactRate {
                    numerator: 9,
                    denominator: 8,
                }),
            )
            .expect("exact boundary is accepted");
        assert_eq!(accepted.envelope.unsigned.numerator, 9);

        let mut outside = candidate();
        outside.numerator = 10;
        assert_eq!(
            issuer.issue(
                outside,
                [7; 32],
                [8; 32],
                1_800_000_100,
                Some(ExactRate {
                    numerator: 9,
                    denominator: 8
                })
            ),
            Err(FxProviderError::DeviationExceeded)
        );

        let mut lower = candidate();
        lower.numerator = 8;
        assert_eq!(
            issuer.issue(
                lower,
                [7; 32],
                [8; 32],
                1_800_000_100,
                Some(ExactRate {
                    numerator: 9,
                    denominator: 8,
                })
            ),
            Err(FxProviderError::DeviationExceeded)
        );

        assert_eq!(
            issuer.issue(candidate(), [7; 32], [8; 32], 1_800_000_100, None),
            Err(FxProviderError::ReferenceRateRequired)
        );
    }

    #[test]
    fn rejects_overflow_and_signer_failures_without_issuing() {
        let overflow_policy = policy();
        let signer = TestSigner::new([3; 32], [4; 32]);
        let issuer = FxEvidenceIssuer::new(overflow_policy, signer);
        let mut huge = candidate();
        huge.numerator = i128::MAX;
        assert_eq!(
            issuer.issue(
                huge,
                [7; 32],
                [8; 32],
                1_800_000_100,
                Some(ExactRate {
                    numerator: i128::MAX,
                    denominator: 2
                })
            ),
            Err(FxProviderError::DeviationOverflow)
        );

        let failing = TestSigner::failing([3; 32], [4; 32]);
        let issuer = FxEvidenceIssuer::new(policy(), failing);
        assert_eq!(
            issuer.issue(
                candidate(),
                [7; 32],
                [8; 32],
                1_800_000_100,
                Some(ExactRate {
                    numerator: 9,
                    denominator: 8,
                })
            ),
            Err(FxProviderError::SignerFailed)
        );
    }

    #[derive(Clone)]
    struct TestSigner {
        signing_key: ed25519_dalek::SigningKey,
        key_id: Digest32,
        fail: bool,
    }

    impl TestSigner {
        fn new(seed: [u8; 32], key_id: Digest32) -> Self {
            Self {
                signing_key: ed25519_dalek::SigningKey::from_bytes(&seed),
                key_id,
                fail: false,
            }
        }

        fn failing(seed: [u8; 32], key_id: Digest32) -> Self {
            Self {
                signing_key: ed25519_dalek::SigningKey::from_bytes(&seed),
                key_id,
                fail: true,
            }
        }
    }

    impl FxEvidenceSigner for TestSigner {
        fn key_id(&self) -> Digest32 {
            self.key_id
        }
        fn public_key(&self) -> [u8; 32] {
            self.signing_key.verifying_key().to_bytes()
        }
        fn sign(&self, _payload: &[u8]) -> Result<Vec<u8>, FxProviderError> {
            if self.fail {
                return Err(FxProviderError::SignerFailed);
            }
            Ok(ed25519_dalek::Signer::sign(&self.signing_key, _payload)
                .to_bytes()
                .to_vec())
        }
    }

    fn policy() -> FxProviderPolicy {
        FxProviderPolicy {
            policy_digest: [6; 32],
            allowed_pairs: vec![CurrencyPair::new("USD", "EUR").expect("pair")],
            max_observation_age_seconds: 600,
            max_future_skew_seconds: 60,
            max_quote_lifetime_seconds: 900,
            require_legal_acceptance: true,
            max_deviation_basis_points: Some(1_000),
        }
    }

    fn candidate() -> FxQuoteCandidate {
        FxQuoteCandidate {
            source_currency: "USD".to_string(),
            target_currency: "EUR".to_string(),
            numerator: 9,
            denominator: 8,
            rounding_mode: tax_engine::RoundingMode::HalfEven,
            quantum_atomic: 1,
            provider: "provider-a".to_string(),
            quote_id: "quote-1".to_string(),
            observed_at_unix: 1_800_000_000,
            expires_at_unix: 1_800_000_300,
            evidence_digest: [9; 32],
            legal_acceptance: LegalAcceptance::Approved {
                authority_digest: [10; 32],
            },
        }
    }
}
