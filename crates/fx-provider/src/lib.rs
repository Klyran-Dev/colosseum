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

use arc_swap::ArcSwap;
use ed25519_dalek::{Signature, VerifyingKey};
use fx_evidence::{
    Digest32, FX_EVIDENCE_SCHEMA_VERSION, FxEvidenceEnvelope, FxEvidenceError, MAX_CODE_BYTES,
    MAX_CURRENCY_CODE_BYTES, UnsignedFxEvidence,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tax_engine::{MAX_ATOMIC_AMOUNT, RoundingMode};
use thiserror::Error;
use tokio::{sync::Semaphore, task::JoinSet, time::timeout};

pub const MAX_ALLOWED_PAIRS: usize = 64;
pub const MAX_ALLOWED_PROVIDERS: usize = 16;
pub const DEFAULT_PROVIDER_CONCURRENCY: usize = 8;
pub const MAX_PROVIDER_CONCURRENCY: usize = MAX_ALLOWED_PROVIDERS;
pub const DEFAULT_PROVIDER_TIMEOUT_MILLIS: u64 = 750;
pub const MAX_PROVIDER_TIMEOUT_MILLIS: u64 = 5_000;
pub const DEFAULT_PROVIDER_AGGREGATE_TIMEOUT_MILLIS: u64 = 1_000;
pub const MAX_PROVIDER_AGGREGATE_TIMEOUT_MILLIS: u64 = 10_000;
pub const DEFAULT_SIGNING_TIMEOUT_MILLIS: u64 = 500;
pub const MAX_SIGNING_TIMEOUT_MILLIS: u64 = 5_000;
pub const MAX_POLICY_REGISTRY_POLICIES: usize = 32;
pub const MAX_POLICY_REGISTRY_DOCUMENT_BYTES: usize = 64 * 1024;
pub const MAX_PROVIDER_OBSERVATION_AGE_SECONDS: i64 = 86_400;
pub const MAX_PROVIDER_FUTURE_SKEW_SECONDS: i64 = 300;
pub const MAX_PROVIDER_POLICY_LIFETIME_SECONDS: i64 = 86_400;
pub const MAX_DEVIATION_BASIS_POINTS: u32 = 1_000_000;
const ISSUED_EVIDENCE_DOMAIN_SEPARATOR: &[u8] =
    b"solana-tax-protocol/fx-provider-issued-evidence/v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct FxProviderPolicy {
    pub policy_digest: Digest32,
    pub allowed_pairs: Vec<CurrencyPair>,
    pub allowed_providers: Vec<String>,
    pub minimum_provider_agreement: u8,
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
        if self.allowed_providers.is_empty() || self.allowed_providers.len() > MAX_ALLOWED_PROVIDERS
        {
            return Err(FxProviderError::InvalidConfiguration(
                "allowed providers are empty or unbounded",
            ));
        }
        if self.minimum_provider_agreement == 0
            || usize::from(self.minimum_provider_agreement) > self.allowed_providers.len()
        {
            return Err(FxProviderError::InvalidConfiguration(
                "minimum provider agreement is invalid",
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
        for (index, provider) in self.allowed_providers.iter().enumerate() {
            if provider.trim().is_empty() || provider.len() > MAX_CODE_BYTES {
                return Err(FxProviderError::InvalidConfiguration(
                    "allowed provider identity is empty or unbounded",
                ));
            }
            if self.allowed_providers[..index]
                .iter()
                .any(|previous| previous == provider)
            {
                return Err(FxProviderError::InvalidConfiguration(
                    "allowed providers contain duplicates",
                ));
            }
        }
        Ok(())
    }

    pub fn select_candidate(
        &self,
        candidates: &[FxQuoteCandidate],
        now_unix: i64,
        reference_rate: Option<ExactRate>,
    ) -> Result<FxQuoteCandidate, FxProviderError> {
        self.validate()?;
        if candidates.len() < usize::from(self.minimum_provider_agreement) {
            return Err(FxProviderError::ProviderAgreementInsufficient);
        }
        if candidates.len() > MAX_ALLOWED_PROVIDERS {
            return Err(FxProviderError::InvalidCandidate(
                "provider candidate set is unbounded",
            ));
        }

        let mut validated = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            self.validate_candidate(candidate, now_unix, reference_rate)?;
            if validated
                .iter()
                .any(|previous: &FxQuoteCandidate| previous.provider == candidate.provider)
            {
                return Err(FxProviderError::DuplicateProvider);
            }
            validated.push(candidate.clone());
        }
        if validated.len() < usize::from(self.minimum_provider_agreement) {
            return Err(FxProviderError::ProviderAgreementInsufficient);
        }

        let first = validated
            .first()
            .ok_or(FxProviderError::ProviderAgreementInsufficient)?;
        if validated
            .iter()
            .skip(1)
            .any(|candidate| !same_economic_terms(first, candidate))
        {
            return Err(FxProviderError::ProviderAgreementDisagreement);
        }

        for provider in &self.allowed_providers {
            if let Some(candidate) = validated
                .iter()
                .find(|candidate| candidate.provider == *provider)
            {
                return Ok((*candidate).clone());
            }
        }
        Err(FxProviderError::ProviderAgreementInsufficient)
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
        if !self
            .allowed_providers
            .iter()
            .any(|provider| provider == &candidate.provider)
        {
            return Err(FxProviderError::UnsupportedProvider);
        }
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

#[derive(Clone, Debug)]
pub struct FxProviderPolicyRegistry {
    policies: Arc<ArcSwap<HashMap<Digest32, FxProviderPolicy>>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyRegistryDocument {
    policies: Vec<FxProviderPolicy>,
}

impl FxProviderPolicyRegistry {
    pub fn from_json(document: &str) -> Result<Self, FxProviderError> {
        let policies = parse_policy_registry(document)?;
        Ok(Self {
            policies: Arc::new(ArcSwap::from_pointee(policies)),
        })
    }

    pub fn reload_json(&self, document: &str) -> Result<usize, FxProviderError> {
        let policies = parse_policy_registry(document)?;
        let policy_count = policies.len();
        self.policies.store(Arc::new(policies));
        Ok(policy_count)
    }

    pub fn policy_count(&self) -> usize {
        self.policies.load().len()
    }

    pub fn policy(&self, policy_digest: Digest32) -> Option<FxProviderPolicy> {
        self.policies.load().get(&policy_digest).cloned()
    }

    pub fn issuer<S: FxEvidenceSigner>(
        &self,
        policy_digest: Digest32,
        signer: S,
    ) -> Result<FxEvidenceIssuer<S>, FxProviderError> {
        if policy_digest == [0; 32] {
            return Err(FxProviderError::PolicyNotFound);
        }
        let policy = self
            .policy(policy_digest)
            .ok_or(FxProviderError::PolicyNotFound)?;
        Ok(FxEvidenceIssuer::new(policy, signer))
    }

    pub fn issuer_async<S: AsyncFxEvidenceSigner>(
        &self,
        policy_digest: Digest32,
        signer: S,
    ) -> Result<FxEvidenceIssuer<S>, FxProviderError> {
        if policy_digest == [0; 32] {
            return Err(FxProviderError::PolicyNotFound);
        }
        let policy = self
            .policy(policy_digest)
            .ok_or(FxProviderError::PolicyNotFound)?;
        Ok(FxEvidenceIssuer::new(policy, signer))
    }
}

fn parse_policy_registry(
    document: &str,
) -> Result<HashMap<Digest32, FxProviderPolicy>, FxProviderError> {
    if document.len() > MAX_POLICY_REGISTRY_DOCUMENT_BYTES {
        return Err(FxProviderError::InvalidPolicyRegistry(
            "policy registry exceeds the bounded document size".to_string(),
        ));
    }
    let document: PolicyRegistryDocument = serde_json::from_str(document)
        .map_err(|error| FxProviderError::InvalidPolicyRegistry(format!("JSON: {error}")))?;
    if document.policies.is_empty() || document.policies.len() > MAX_POLICY_REGISTRY_POLICIES {
        return Err(FxProviderError::InvalidPolicyRegistry(
            "policy registry is empty or unbounded".to_string(),
        ));
    }
    let mut policies = HashMap::with_capacity(document.policies.len());
    for policy in document.policies {
        policy
            .validate()
            .map_err(|error| FxProviderError::InvalidPolicyRegistry(error.to_string()))?;
        if policies.insert(policy.policy_digest, policy).is_some() {
            return Err(FxProviderError::InvalidPolicyRegistry(
                "policy registry contains duplicate policy digests".to_string(),
            ));
        }
    }
    Ok(policies)
}

pub trait FxEvidenceSigner {
    fn key_id(&self) -> Digest32;
    fn public_key(&self) -> [u8; 32];
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, FxProviderError>;
}

#[async_trait::async_trait]
pub trait AsyncFxEvidenceSigner: Send + Sync {
    fn key_id(&self) -> Digest32;
    fn public_key(&self) -> [u8; 32];
    async fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, FxProviderError>;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FxProviderAcquisitionConfig {
    pub max_concurrency: usize,
    pub per_provider_timeout: Duration,
    pub aggregate_timeout: Duration,
}

impl Default for FxProviderAcquisitionConfig {
    fn default() -> Self {
        Self {
            max_concurrency: DEFAULT_PROVIDER_CONCURRENCY,
            per_provider_timeout: Duration::from_millis(DEFAULT_PROVIDER_TIMEOUT_MILLIS),
            aggregate_timeout: Duration::from_millis(DEFAULT_PROVIDER_AGGREGATE_TIMEOUT_MILLIS),
        }
    }
}

impl FxProviderAcquisitionConfig {
    fn validate(self) -> Result<Self, FxProviderError> {
        if self.max_concurrency == 0 || self.max_concurrency > MAX_PROVIDER_CONCURRENCY {
            return Err(FxProviderError::InvalidConfiguration(
                "provider concurrency is empty or unbounded",
            ));
        }
        if self.per_provider_timeout.is_zero()
            || self.per_provider_timeout > Duration::from_millis(MAX_PROVIDER_TIMEOUT_MILLIS)
        {
            return Err(FxProviderError::InvalidConfiguration(
                "provider timeout is empty or unbounded",
            ));
        }
        if self.aggregate_timeout.is_zero()
            || self.aggregate_timeout > Duration::from_millis(MAX_PROVIDER_AGGREGATE_TIMEOUT_MILLIS)
        {
            return Err(FxProviderError::InvalidConfiguration(
                "aggregate provider timeout is empty or unbounded",
            ));
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FxSigningConfig {
    pub timeout: Duration,
}

impl Default for FxSigningConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_millis(DEFAULT_SIGNING_TIMEOUT_MILLIS),
        }
    }
}

impl FxSigningConfig {
    fn validate(self) -> Result<Self, FxProviderError> {
        if self.timeout.is_zero()
            || self.timeout > Duration::from_millis(MAX_SIGNING_TIMEOUT_MILLIS)
        {
            return Err(FxProviderError::InvalidConfiguration(
                "FX signing timeout is empty or unbounded",
            ));
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FxProviderFailureKind {
    Timeout,
    SourceRejected,
    IdentityMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FxProviderAttemptFailure {
    pub provider: String,
    pub kind: FxProviderFailureKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FxProviderAcquisition {
    pub candidates: Vec<FxQuoteCandidate>,
    pub failures: Vec<FxProviderAttemptFailure>,
}

#[async_trait::async_trait]
pub trait AsyncFxQuoteSource: Send + Sync {
    fn provider_id(&self) -> &str;

    async fn quote(&self, request: &FxQuoteRequest) -> Result<FxQuoteCandidate, FxProviderError>;
}

pub async fn acquire_candidates(
    policy: &FxProviderPolicy,
    sources: &[Arc<dyn AsyncFxQuoteSource>],
    request: &FxQuoteRequest,
    config: FxProviderAcquisitionConfig,
) -> Result<FxProviderAcquisition, FxProviderError> {
    policy.validate()?;
    config.validate()?;
    validate_acquisition_request(policy, request, sources)?;

    let mut acquisition = timeout(
        config.aggregate_timeout,
        acquire_candidates_inner(sources, request, config),
    )
    .await
    .map_err(|_| FxProviderError::AcquisitionDeadlineExceeded)??;
    normalize_acquisition_order(policy, &mut acquisition);
    Ok(acquisition)
}

fn normalize_acquisition_order(policy: &FxProviderPolicy, acquisition: &mut FxProviderAcquisition) {
    acquisition
        .candidates
        .sort_by_key(|candidate| provider_priority(policy, &candidate.provider));
    acquisition
        .failures
        .sort_by_key(|failure| provider_priority(policy, &failure.provider));
}

fn provider_priority(policy: &FxProviderPolicy, provider: &str) -> usize {
    match policy
        .allowed_providers
        .iter()
        .position(|allowed| allowed == provider)
    {
        Some(index) => index,
        None => policy.allowed_providers.len(),
    }
}

async fn acquire_candidates_inner(
    sources: &[Arc<dyn AsyncFxQuoteSource>],
    request: &FxQuoteRequest,
    config: FxProviderAcquisitionConfig,
) -> Result<FxProviderAcquisition, FxProviderError> {
    let semaphore = Arc::new(Semaphore::new(config.max_concurrency));
    let mut tasks = JoinSet::new();

    for source in sources {
        let source = Arc::clone(source);
        let request = request.clone();
        let semaphore = Arc::clone(&semaphore);
        let provider = source.provider_id().to_string();
        tasks.spawn(async move {
            let permit = match semaphore.acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => return Err((provider, FxProviderFailureKind::SourceRejected)),
            };
            let result = timeout(config.per_provider_timeout, source.quote(&request)).await;
            drop(permit);
            match result {
                Ok(Ok(candidate)) if candidate.provider == provider => Ok((provider, candidate)),
                Ok(Ok(_)) => Err((provider, FxProviderFailureKind::IdentityMismatch)),
                Ok(Err(_)) => Err((provider, FxProviderFailureKind::SourceRejected)),
                Err(_) => Err((provider, FxProviderFailureKind::Timeout)),
            }
        });
    }

    let mut acquisition = FxProviderAcquisition {
        candidates: Vec::with_capacity(sources.len()),
        failures: Vec::with_capacity(sources.len()),
    };
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok((_provider, candidate))) => acquisition.candidates.push(candidate),
            Ok(Err((provider, kind))) => acquisition
                .failures
                .push(FxProviderAttemptFailure { provider, kind }),
            Err(_) => {
                tasks.abort_all();
                return Err(FxProviderError::ProviderTaskFailed);
            }
        }
    }
    Ok(acquisition)
}

fn validate_acquisition_request(
    policy: &FxProviderPolicy,
    request: &FxQuoteRequest,
    sources: &[Arc<dyn AsyncFxQuoteSource>],
) -> Result<(), FxProviderError> {
    if request.merchant_id == [0; 32] || request.facts_digest == [0; 32] {
        return Err(FxProviderError::InvalidCandidate(
            "merchant and facts digests must be non-zero",
        ));
    }
    validate_currency(&request.source_currency)?;
    validate_currency(&request.target_currency)?;
    if !policy
        .allowed_pairs
        .iter()
        .any(|pair| pair.matches(&request.source_currency, &request.target_currency))
    {
        return Err(FxProviderError::UnsupportedCurrencyPair);
    }
    if sources.is_empty() || sources.len() > MAX_ALLOWED_PROVIDERS {
        return Err(FxProviderError::InvalidConfiguration(
            "provider source set is empty or unbounded",
        ));
    }
    if sources.len() < usize::from(policy.minimum_provider_agreement) {
        return Err(FxProviderError::ProviderAgreementInsufficient);
    }
    for (index, source) in sources.iter().enumerate() {
        let provider = source.provider_id();
        if provider.trim().is_empty() || provider.len() > MAX_CODE_BYTES {
            return Err(FxProviderError::InvalidConfiguration(
                "provider source identity is empty or unbounded",
            ));
        }
        if !policy
            .allowed_providers
            .iter()
            .any(|allowed| allowed == provider)
        {
            return Err(FxProviderError::UnsupportedProvider);
        }
        if sources[..index]
            .iter()
            .any(|previous| previous.provider_id() == provider)
        {
            return Err(FxProviderError::DuplicateProvider);
        }
    }
    Ok(())
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

impl<S> FxEvidenceIssuer<S> {
    pub fn new(policy: FxProviderPolicy, signer: S) -> Self {
        Self { policy, signer }
    }
}

impl<S> FxEvidenceIssuer<S>
where
    S: FxEvidenceSigner,
{
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
        self.issue_validated(candidate, merchant_id, facts_digest, now_unix)
    }

    pub fn issue_candidates(
        &self,
        candidates: &[FxQuoteCandidate],
        merchant_id: Digest32,
        facts_digest: Digest32,
        now_unix: i64,
        reference_rate: Option<ExactRate>,
    ) -> Result<IssuedFxEvidence, FxProviderError> {
        if merchant_id == [0; 32] || facts_digest == [0; 32] {
            return Err(FxProviderError::InvalidCandidate(
                "merchant and facts digests must be non-zero",
            ));
        }
        let candidate = self
            .policy
            .select_candidate(candidates, now_unix, reference_rate)?;
        self.issue_validated(candidate, merchant_id, facts_digest, now_unix)
    }

    fn issue_validated(
        &self,
        candidate: FxQuoteCandidate,
        merchant_id: Digest32,
        facts_digest: Digest32,
        now_unix: i64,
    ) -> Result<IssuedFxEvidence, FxProviderError> {
        let prepared = prepare_evidence(
            &self.policy,
            self.signer.key_id(),
            self.signer.public_key(),
            candidate,
            merchant_id,
            facts_digest,
        )?;
        let signature = self.signer.sign(&prepared.payload)?;
        finalize_evidence(&self.policy, prepared, signature, now_unix)
    }
}

impl<S> FxEvidenceIssuer<S>
where
    S: AsyncFxEvidenceSigner,
{
    pub async fn issue_async(
        &self,
        candidate: FxQuoteCandidate,
        merchant_id: Digest32,
        facts_digest: Digest32,
        now_unix: i64,
        reference_rate: Option<ExactRate>,
        signing_config: FxSigningConfig,
    ) -> Result<IssuedFxEvidence, FxProviderError> {
        self.policy.validate()?;
        signing_config.validate()?;
        if merchant_id == [0; 32] || facts_digest == [0; 32] {
            return Err(FxProviderError::InvalidCandidate(
                "merchant and facts digests must be non-zero",
            ));
        }
        self.policy
            .validate_candidate(&candidate, now_unix, reference_rate)?;
        self.issue_validated_async(
            candidate,
            merchant_id,
            facts_digest,
            now_unix,
            signing_config,
        )
        .await
    }

    pub async fn issue_candidates_async(
        &self,
        candidates: &[FxQuoteCandidate],
        merchant_id: Digest32,
        facts_digest: Digest32,
        now_unix: i64,
        reference_rate: Option<ExactRate>,
        signing_config: FxSigningConfig,
    ) -> Result<IssuedFxEvidence, FxProviderError> {
        signing_config.validate()?;
        if merchant_id == [0; 32] || facts_digest == [0; 32] {
            return Err(FxProviderError::InvalidCandidate(
                "merchant and facts digests must be non-zero",
            ));
        }
        let candidate = self
            .policy
            .select_candidate(candidates, now_unix, reference_rate)?;
        self.issue_validated_async(
            candidate,
            merchant_id,
            facts_digest,
            now_unix,
            signing_config,
        )
        .await
    }

    async fn issue_validated_async(
        &self,
        candidate: FxQuoteCandidate,
        merchant_id: Digest32,
        facts_digest: Digest32,
        now_unix: i64,
        signing_config: FxSigningConfig,
    ) -> Result<IssuedFxEvidence, FxProviderError> {
        let prepared = prepare_evidence(
            &self.policy,
            self.signer.key_id(),
            self.signer.public_key(),
            candidate,
            merchant_id,
            facts_digest,
        )?;
        let signature = timeout(signing_config.timeout, self.signer.sign(&prepared.payload))
            .await
            .map_err(|_| FxProviderError::SigningDeadlineExceeded)??;
        finalize_evidence(&self.policy, prepared, signature, now_unix)
    }
}

struct PreparedFxEvidence {
    unsigned: UnsignedFxEvidence,
    payload: Vec<u8>,
    public_key: [u8; 32],
    source_evidence_digest: Digest32,
    legal_authority_digest: Digest32,
}

fn prepare_evidence(
    policy: &FxProviderPolicy,
    key_id: Digest32,
    public_key: [u8; 32],
    candidate: FxQuoteCandidate,
    merchant_id: Digest32,
    facts_digest: Digest32,
) -> Result<PreparedFxEvidence, FxProviderError> {
    let legal_authority_digest = match candidate.legal_acceptance {
        LegalAcceptance::Approved { authority_digest } if authority_digest != [0; 32] => {
            authority_digest
        }
        _ if policy.require_legal_acceptance => {
            return Err(FxProviderError::LegalAcceptanceRequired);
        }
        _ => [0; 32],
    };
    if key_id == [0; 32] || public_key == [0; 32] {
        return Err(FxProviderError::SignerIdentityInvalid);
    }
    let source_evidence_digest = candidate.evidence_digest;
    let evidence_digest = issued_evidence_digest(
        source_evidence_digest,
        policy.policy_digest,
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
    Ok(PreparedFxEvidence {
        unsigned,
        payload,
        public_key,
        source_evidence_digest,
        legal_authority_digest,
    })
}

fn finalize_evidence(
    policy: &FxProviderPolicy,
    prepared: PreparedFxEvidence,
    signature: Vec<u8>,
    now_unix: i64,
) -> Result<IssuedFxEvidence, FxProviderError> {
    if Signature::from_slice(&signature).is_err() {
        return Err(FxProviderError::SignerReturnedInvalidSignature);
    }
    let verifying_key = VerifyingKey::from_bytes(&prepared.public_key)
        .map_err(|_| FxProviderError::SignerIdentityInvalid)?;
    let envelope = FxEvidenceEnvelope {
        unsigned: prepared.unsigned,
        provider_public_key: prepared.public_key,
        payload_digest: Sha256::digest(&prepared.payload).into(),
        signature,
    };
    envelope
        .verify_with_key_at_and_future_skew(
            &verifying_key,
            envelope.unsigned.merchant_id,
            envelope.unsigned.facts_digest,
            now_unix,
            policy.max_future_skew_seconds,
        )
        .map_err(FxProviderError::Evidence)?;
    Ok(IssuedFxEvidence {
        envelope,
        source_evidence_digest: prepared.source_evidence_digest,
        policy_digest: policy.policy_digest,
        legal_authority_digest: prepared.legal_authority_digest,
    })
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum FxProviderError {
    #[error("invalid FX provider configuration: {0}")]
    InvalidConfiguration(&'static str),
    #[error("invalid FX policy registry: {0}")]
    InvalidPolicyRegistry(String),
    #[error("FX provider policy not found")]
    PolicyNotFound,
    #[error("invalid FX quote candidate: {0}")]
    InvalidCandidate(&'static str),
    #[error("unsupported FX currency pair")]
    UnsupportedCurrencyPair,
    #[error("FX provider is not approved by policy")]
    UnsupportedProvider,
    #[error("provider candidates contain duplicate identities")]
    DuplicateProvider,
    #[error("provider agreement is insufficient")]
    ProviderAgreementInsufficient,
    #[error("provider candidates disagree on economic terms")]
    ProviderAgreementDisagreement,
    #[error("FX provider acquisition exceeded its aggregate deadline")]
    AcquisitionDeadlineExceeded,
    #[error("FX provider acquisition task failed")]
    ProviderTaskFailed,
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
    #[error("FX signing exceeded its deadline")]
    SigningDeadlineExceeded,
    #[error("FX evidence envelope is invalid: {0}")]
    Evidence(#[from] FxEvidenceError),
}

fn same_economic_terms(first: &FxQuoteCandidate, second: &FxQuoteCandidate) -> bool {
    first.source_currency == second.source_currency
        && first.target_currency == second.target_currency
        && first.numerator == second.numerator
        && first.denominator == second.denominator
        && first.rounding_mode == second.rounding_mode
        && first.quantum_atomic == second.quantum_atomic
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
    fn rejects_a_quote_from_a_provider_outside_the_policy_allowlist() {
        let mut unapproved = candidate();
        unapproved.provider = "unapproved-provider".to_string();
        let error = FxEvidenceIssuer::new(policy(), TestSigner::new([21; 32], [22; 32]))
            .issue(
                unapproved,
                [7; 32],
                [8; 32],
                1_800_000_100,
                Some(ExactRate {
                    numerator: 9,
                    denominator: 8,
                }),
            )
            .expect_err("unapproved provider must fail closed");
        assert_eq!(error, FxProviderError::UnsupportedProvider);
    }

    #[test]
    fn selects_agreeing_provider_candidates_by_policy_priority_not_input_order() {
        let mut agreement_policy = policy();
        agreement_policy.allowed_providers =
            vec!["provider-a".to_string(), "provider-b".to_string()];
        agreement_policy.minimum_provider_agreement = 2;
        agreement_policy.max_deviation_basis_points = None;
        let preferred = candidate();
        let mut secondary = preferred.clone();
        secondary.provider = "provider-b".to_string();
        secondary.quote_id = "quote-2".to_string();
        secondary.evidence_digest = [11; 32];

        let selected = agreement_policy
            .select_candidate(
                &[secondary.clone(), preferred.clone()],
                1_800_000_100,
                Some(ExactRate {
                    numerator: 9,
                    denominator: 8,
                }),
            )
            .expect("agreeing candidates");
        assert_eq!(selected, preferred);

        let issued = FxEvidenceIssuer::new(agreement_policy, TestSigner::new([23; 32], [24; 32]))
            .issue_candidates(
                &[secondary, preferred],
                [7; 32],
                [8; 32],
                1_800_000_100,
                Some(ExactRate {
                    numerator: 9,
                    denominator: 8,
                }),
            )
            .expect("agreement-bound issuance");
        assert_eq!(issued.envelope.unsigned.provider, "provider-a");
    }

    #[test]
    fn rejects_insufficient_duplicate_and_disagreeing_provider_candidates() {
        let mut agreement_policy = policy();
        agreement_policy.allowed_providers =
            vec!["provider-a".to_string(), "provider-b".to_string()];
        agreement_policy.minimum_provider_agreement = 2;
        agreement_policy.max_deviation_basis_points = None;
        let first = candidate();
        let mut duplicate = first.clone();
        duplicate.quote_id = "quote-duplicate".to_string();
        let mut disagreeing = first.clone();
        disagreeing.provider = "provider-b".to_string();
        disagreeing.numerator = 10;
        disagreeing.quote_id = "quote-disagreeing".to_string();

        let reference = None;
        assert_eq!(
            agreement_policy
                .select_candidate(std::slice::from_ref(&first), 1_800_000_100, reference)
                .expect_err("one provider is insufficient"),
            FxProviderError::ProviderAgreementInsufficient
        );
        assert_eq!(
            agreement_policy
                .select_candidate(&[first.clone(), duplicate], 1_800_000_100, reference)
                .expect_err("duplicate provider must fail"),
            FxProviderError::DuplicateProvider
        );
        assert_eq!(
            agreement_policy
                .select_candidate(&[first, disagreeing], 1_800_000_100, reference)
                .expect_err("rate disagreement must fail"),
            FxProviderError::ProviderAgreementDisagreement
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

    #[test]
    fn policy_registry_replaces_atomically_and_retains_last_good_snapshot() {
        let first = policy();
        let second = FxProviderPolicy {
            policy_digest: [11; 32],
            ..first.clone()
        };
        let registry = FxProviderPolicyRegistry::from_json(&policy_document(&first))
            .expect("initial policy registry");
        assert_eq!(registry.policy_count(), 1);
        assert_eq!(registry.policy(first.policy_digest), Some(first.clone()));

        registry
            .reload_json(&policy_document(&second))
            .expect("valid replacement");
        assert_eq!(registry.policy_count(), 1);
        assert_eq!(registry.policy(second.policy_digest), Some(second));
        assert_eq!(registry.policy(first.policy_digest), None);

        assert!(registry.reload_json("{not-json").is_err());
        assert!(registry.reload_json("{\"policies\":[]}").is_err());
        assert_eq!(registry.policy_count(), 1);
        assert!(registry.policy([11; 32]).is_some());
    }

    #[test]
    fn registry_bound_issuer_requires_exact_digest_and_is_snapshot_isolated() {
        let first = policy();
        let second = FxProviderPolicy {
            policy_digest: [12; 32],
            ..first.clone()
        };
        let registry = FxProviderPolicyRegistry::from_json(&policy_document(&first))
            .expect("initial policy registry");

        let issuer = registry
            .issuer(first.policy_digest, TestSigner::new([13; 32], [14; 32]))
            .expect("active policy issuer");
        assert_eq!(issuer.policy, first);

        assert!(matches!(
            registry.issuer([0; 32], TestSigner::new([15; 32], [16; 32])),
            Err(FxProviderError::PolicyNotFound)
        ));
        assert!(matches!(
            registry.issuer([99; 32], TestSigner::new([17; 32], [18; 32])),
            Err(FxProviderError::PolicyNotFound)
        ));

        registry
            .reload_json(&policy_document(&second))
            .expect("policy replacement");
        assert_eq!(issuer.policy, first);
        let replacement_issuer = registry
            .issuer(second.policy_digest, TestSigner::new([19; 32], [20; 32]))
            .expect("replacement policy issuer");
        assert_eq!(replacement_issuer.policy, second);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn registry_bound_async_issuer_requires_exact_digest_and_is_snapshot_isolated() {
        let first = policy();
        let second = FxProviderPolicy {
            policy_digest: [25; 32],
            ..first.clone()
        };
        let registry = FxProviderPolicyRegistry::from_json(&policy_document(&first))
            .expect("initial policy registry");

        let issuer = registry
            .issuer_async(first.policy_digest, TestSigner::new([26; 32], [27; 32]))
            .expect("active async policy issuer");
        let issued = issuer
            .issue_async(
                candidate(),
                [7; 32],
                [8; 32],
                1_800_000_100,
                Some(ExactRate {
                    numerator: 9,
                    denominator: 8,
                }),
                FxSigningConfig::default(),
            )
            .await
            .expect("async evidence");
        assert_eq!(issued.policy_digest, first.policy_digest);

        assert!(matches!(
            registry.issuer_async([0; 32], TestSigner::new([28; 32], [29; 32])),
            Err(FxProviderError::PolicyNotFound)
        ));
        registry
            .reload_json(&policy_document(&second))
            .expect("policy replacement");
        assert_eq!(issuer.policy, first);
    }

    #[test]
    fn policy_registry_rejects_duplicate_invalid_oversized_and_unknown_documents() {
        let first = policy();
        let duplicate = format!(
            "{{\"policies\":[{},{}]}}",
            serde_json::to_string(&first).expect("policy JSON"),
            serde_json::to_string(&first).expect("policy JSON")
        );
        assert!(matches!(
            FxProviderPolicyRegistry::from_json(&duplicate),
            Err(FxProviderError::InvalidPolicyRegistry(_))
        ));

        let mut invalid = first.clone();
        invalid.allowed_pairs.clear();
        assert!(matches!(
            FxProviderPolicyRegistry::from_json(&policy_document(&invalid)),
            Err(FxProviderError::InvalidPolicyRegistry(_))
        ));

        let mut invalid_providers = first.clone();
        invalid_providers.allowed_providers.clear();
        assert!(matches!(
            FxProviderPolicyRegistry::from_json(&policy_document(&invalid_providers)),
            Err(FxProviderError::InvalidPolicyRegistry(_))
        ));

        let mut invalid_agreement = first.clone();
        invalid_agreement.minimum_provider_agreement = 0;
        assert!(matches!(
            FxProviderPolicyRegistry::from_json(&policy_document(&invalid_agreement)),
            Err(FxProviderError::InvalidPolicyRegistry(_))
        ));

        invalid_agreement.minimum_provider_agreement = 2;
        assert!(matches!(
            FxProviderPolicyRegistry::from_json(&policy_document(&invalid_agreement)),
            Err(FxProviderError::InvalidPolicyRegistry(_))
        ));

        assert!(matches!(
            FxProviderPolicyRegistry::from_json("{\"policies\":[]}"),
            Err(FxProviderError::InvalidPolicyRegistry(_))
        ));

        let unknown = format!(
            "{{\"policies\":[{}],\"unexpected\":true}}",
            serde_json::to_string(&first).expect("policy JSON")
        );
        assert!(matches!(
            FxProviderPolicyRegistry::from_json(&unknown),
            Err(FxProviderError::InvalidPolicyRegistry(_))
        ));

        let oversized = format!(
            "{{\"policies\":[]}}{}",
            "x".repeat(MAX_POLICY_REGISTRY_DOCUMENT_BYTES)
        );
        assert!(matches!(
            FxProviderPolicyRegistry::from_json(&oversized),
            Err(FxProviderError::InvalidPolicyRegistry(_))
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn acquires_candidates_with_bounded_concurrency_and_partial_failure_evidence() {
        let mut acquisition_policy = policy();
        acquisition_policy.allowed_providers = vec![
            "provider-a".to_string(),
            "provider-b".to_string(),
            "provider-c".to_string(),
            "provider-d".to_string(),
        ];
        acquisition_policy.minimum_provider_agreement = 2;
        acquisition_policy.max_deviation_basis_points = None;
        let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let maximum = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let sources = [
            source("provider-a", 20, false, None, &active, &maximum),
            source("provider-b", 20, false, None, &active, &maximum),
            source("provider-c", 20, true, None, &active, &maximum),
            source("provider-d", 20, false, None, &active, &maximum),
        ];
        let sources: Vec<Arc<dyn AsyncFxQuoteSource>> = sources
            .into_iter()
            .map(|source| Arc::new(source) as Arc<dyn AsyncFxQuoteSource>)
            .collect();

        let acquisition = acquire_candidates(
            &acquisition_policy,
            &sources,
            &request(),
            FxProviderAcquisitionConfig {
                max_concurrency: 2,
                per_provider_timeout: Duration::from_millis(200),
                aggregate_timeout: Duration::from_millis(500),
            },
        )
        .await
        .expect("bounded acquisition");

        assert_eq!(acquisition.candidates.len(), 3);
        assert_eq!(acquisition.failures.len(), 1);
        assert_eq!(
            acquisition.failures[0],
            FxProviderAttemptFailure {
                provider: "provider-c".to_string(),
                kind: FxProviderFailureKind::SourceRejected,
            }
        );
        assert_eq!(active.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert!(maximum.load(std::sync::atomic::Ordering::SeqCst) <= 2);

        let selected = acquisition_policy
            .select_candidate(&acquisition.candidates, 1_800_000_100, None)
            .expect("partial failure still leaves sufficient agreement");
        assert_eq!(selected.provider, "provider-a");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn orders_acquisition_outcomes_by_policy_priority_not_completion() {
        let mut acquisition_policy = policy();
        acquisition_policy.allowed_providers = vec![
            "provider-a".to_string(),
            "provider-b".to_string(),
            "provider-c".to_string(),
            "provider-d".to_string(),
        ];
        acquisition_policy.minimum_provider_agreement = 1;
        acquisition_policy.max_deviation_basis_points = None;
        let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let maximum = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let sources: Vec<Arc<dyn AsyncFxQuoteSource>> = vec![
            Arc::new(source("provider-a", 40, false, None, &active, &maximum)),
            Arc::new(source("provider-b", 10, true, None, &active, &maximum)),
            Arc::new(source("provider-c", 30, false, None, &active, &maximum)),
            Arc::new(source("provider-d", 0, true, None, &active, &maximum)),
        ];

        let acquisition = acquire_candidates(
            &acquisition_policy,
            &sources,
            &request(),
            FxProviderAcquisitionConfig {
                max_concurrency: 4,
                per_provider_timeout: Duration::from_millis(200),
                aggregate_timeout: Duration::from_millis(500),
            },
        )
        .await
        .expect("bounded acquisition");

        assert_eq!(
            acquisition
                .candidates
                .iter()
                .map(|candidate| candidate.provider.as_str())
                .collect::<Vec<_>>(),
            vec!["provider-a", "provider-c"]
        );
        assert_eq!(
            acquisition
                .failures
                .iter()
                .map(|failure| failure.provider.as_str())
                .collect::<Vec<_>>(),
            vec!["provider-b", "provider-d"]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn classifies_timeout_and_source_identity_substitution() {
        let mut acquisition_policy = policy();
        acquisition_policy.allowed_providers =
            vec!["provider-a".to_string(), "provider-b".to_string()];
        acquisition_policy.max_deviation_basis_points = None;
        let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let maximum = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let sources: Vec<Arc<dyn AsyncFxQuoteSource>> = vec![
            Arc::new(source("provider-a", 50, false, None, &active, &maximum)),
            Arc::new(source(
                "provider-b",
                1,
                false,
                Some("provider-a"),
                &active,
                &maximum,
            )),
        ];

        let acquisition = acquire_candidates(
            &acquisition_policy,
            &sources,
            &request(),
            FxProviderAcquisitionConfig {
                max_concurrency: 2,
                per_provider_timeout: Duration::from_millis(10),
                aggregate_timeout: Duration::from_millis(100),
            },
        )
        .await
        .expect("individual failures are reported in the acquisition");

        assert!(acquisition.candidates.is_empty());
        assert!(acquisition.failures.contains(&FxProviderAttemptFailure {
            provider: "provider-a".to_string(),
            kind: FxProviderFailureKind::Timeout,
        }));
        assert!(acquisition.failures.contains(&FxProviderAttemptFailure {
            provider: "provider-b".to_string(),
            kind: FxProviderFailureKind::IdentityMismatch,
        }));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn aggregate_deadline_cancels_outstanding_provider_work() {
        let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let maximum = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let sources: Vec<Arc<dyn AsyncFxQuoteSource>> = vec![Arc::new(source(
            "provider-a",
            200,
            false,
            None,
            &active,
            &maximum,
        ))];
        let error = acquire_candidates(
            &policy(),
            &sources,
            &request(),
            FxProviderAcquisitionConfig {
                max_concurrency: 1,
                per_provider_timeout: Duration::from_secs(5),
                aggregate_timeout: Duration::from_millis(20),
            },
        )
        .await
        .expect_err("aggregate deadline");
        assert_eq!(error, FxProviderError::AcquisitionDeadlineExceeded);
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert_eq!(active.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_duplicate_sources_before_any_provider_call() {
        let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let maximum = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let sources: Vec<Arc<dyn AsyncFxQuoteSource>> = vec![
            Arc::new(source("provider-a", 0, false, None, &active, &maximum)),
            Arc::new(source("provider-a", 0, false, None, &active, &maximum)),
        ];
        assert_eq!(
            acquire_candidates(
                &FxProviderPolicy {
                    allowed_providers: vec!["provider-a".to_string()],
                    ..policy()
                },
                &sources,
                &request(),
                FxProviderAcquisitionConfig::default(),
            )
            .await
            .expect_err("duplicate source identity"),
            FxProviderError::DuplicateProvider
        );
        assert_eq!(active.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn issues_exact_evidence_through_the_async_signer_boundary() {
        let issuer = FxEvidenceIssuer::new(policy(), TestSigner::new([31; 32], [32; 32]));
        let issued = issuer
            .issue_async(
                candidate(),
                [7; 32],
                [8; 32],
                1_800_000_100,
                Some(ExactRate {
                    numerator: 9,
                    denominator: 8,
                }),
                FxSigningConfig::default(),
            )
            .await
            .expect("async signing");
        assert_eq!(issued.envelope.unsigned.merchant_id, [7; 32]);
        assert_eq!(issued.envelope.unsigned.provider_key_id, [32; 32]);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_async_signing_timeout_without_issuing() {
        let signer = SlowAsyncSigner {
            inner: TestSigner::new([33; 32], [34; 32]),
            delay: Duration::from_millis(50),
        };
        let issuer = FxEvidenceIssuer::new(policy(), signer);
        assert_eq!(
            issuer
                .issue_async(
                    candidate(),
                    [7; 32],
                    [8; 32],
                    1_800_000_100,
                    Some(ExactRate {
                        numerator: 9,
                        denominator: 8,
                    }),
                    FxSigningConfig {
                        timeout: Duration::from_millis(10),
                    },
                )
                .await
                .expect_err("signing timeout"),
            FxProviderError::SigningDeadlineExceeded
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_malformed_async_signature_and_invalid_identity() {
        let malformed = FxEvidenceIssuer::new(
            policy(),
            MalformedAsyncSigner {
                inner: TestSigner::new([35; 32], [36; 32]),
            },
        );
        assert_eq!(
            malformed
                .issue_async(
                    candidate(),
                    [7; 32],
                    [8; 32],
                    1_800_000_100,
                    Some(ExactRate {
                        numerator: 9,
                        denominator: 8,
                    }),
                    FxSigningConfig::default(),
                )
                .await
                .expect_err("malformed signature"),
            FxProviderError::SignerReturnedInvalidSignature
        );

        let invalid_identity = FxEvidenceIssuer::new(policy(), InvalidIdentityAsyncSigner);
        assert_eq!(
            invalid_identity
                .issue_async(
                    candidate(),
                    [7; 32],
                    [8; 32],
                    1_800_000_100,
                    Some(ExactRate {
                        numerator: 9,
                        denominator: 8,
                    }),
                    FxSigningConfig::default(),
                )
                .await
                .expect_err("invalid signer identity"),
            FxProviderError::SignerIdentityInvalid
        );

        let mismatch = FxEvidenceIssuer::new(
            policy(),
            MismatchedAsyncSigner {
                inner: TestSigner::new([37; 32], [38; 32]),
                public_key: FxEvidenceSigner::public_key(&TestSigner::new([39; 32], [40; 32])),
            },
        );
        assert_eq!(
            mismatch
                .issue_async(
                    candidate(),
                    [7; 32],
                    [8; 32],
                    1_800_000_100,
                    Some(ExactRate {
                        numerator: 9,
                        denominator: 8,
                    }),
                    FxSigningConfig::default(),
                )
                .await
                .expect_err("public-key mismatch"),
            FxProviderError::Evidence(FxEvidenceError::InvalidSignature)
        );

        let failed = FxEvidenceIssuer::new(policy(), TestSigner::failing([41; 32], [42; 32]));
        assert_eq!(
            failed
                .issue_async(
                    candidate(),
                    [7; 32],
                    [8; 32],
                    1_800_000_100,
                    Some(ExactRate {
                        numerator: 9,
                        denominator: 8,
                    }),
                    FxSigningConfig::default(),
                )
                .await
                .expect_err("signer failure"),
            FxProviderError::SignerFailed
        );
    }

    fn request() -> FxQuoteRequest {
        FxQuoteRequest {
            merchant_id: [7; 32],
            facts_digest: [8; 32],
            source_currency: "USD".to_string(),
            target_currency: "EUR".to_string(),
        }
    }

    struct TestAsyncSource {
        provider: String,
        returned_provider: String,
        delay_millis: u64,
        reject: bool,
        candidate: FxQuoteCandidate,
        active: Arc<std::sync::atomic::AtomicUsize>,
        maximum: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl AsyncFxQuoteSource for TestAsyncSource {
        fn provider_id(&self) -> &str {
            &self.provider
        }

        async fn quote(
            &self,
            _request: &FxQuoteRequest,
        ) -> Result<FxQuoteCandidate, FxProviderError> {
            let _guard = ActiveGuard::new(Arc::clone(&self.active), Arc::clone(&self.maximum));
            tokio::time::sleep(Duration::from_millis(self.delay_millis)).await;
            if self.reject {
                return Err(FxProviderError::SignerFailed);
            }
            let mut candidate = self.candidate.clone();
            candidate.provider = self.returned_provider.clone();
            Ok(candidate)
        }
    }

    struct ActiveGuard {
        active: Arc<std::sync::atomic::AtomicUsize>,
    }

    impl ActiveGuard {
        fn new(
            active: Arc<std::sync::atomic::AtomicUsize>,
            maximum: Arc<std::sync::atomic::AtomicUsize>,
        ) -> Self {
            let current = active.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let mut observed = maximum.load(std::sync::atomic::Ordering::SeqCst);
            while current > observed {
                match maximum.compare_exchange(
                    observed,
                    current,
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::SeqCst,
                ) {
                    Ok(_) => break,
                    Err(next) => observed = next,
                }
            }
            Self { active }
        }
    }

    impl Drop for ActiveGuard {
        fn drop(&mut self) {
            self.active
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    fn source(
        provider: &str,
        delay_millis: u64,
        reject: bool,
        returned_provider: Option<&str>,
        active: &Arc<std::sync::atomic::AtomicUsize>,
        maximum: &Arc<std::sync::atomic::AtomicUsize>,
    ) -> TestAsyncSource {
        TestAsyncSource {
            provider: provider.to_string(),
            returned_provider: returned_provider.unwrap_or(provider).to_string(),
            delay_millis,
            reject,
            candidate: candidate(),
            active: Arc::clone(active),
            maximum: Arc::clone(maximum),
        }
    }

    struct SlowAsyncSigner {
        inner: TestSigner,
        delay: Duration,
    }

    #[async_trait::async_trait]
    impl AsyncFxEvidenceSigner for SlowAsyncSigner {
        fn key_id(&self) -> Digest32 {
            FxEvidenceSigner::key_id(&self.inner)
        }

        fn public_key(&self) -> [u8; 32] {
            FxEvidenceSigner::public_key(&self.inner)
        }

        async fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, FxProviderError> {
            tokio::time::sleep(self.delay).await;
            FxEvidenceSigner::sign(&self.inner, payload)
        }
    }

    struct MalformedAsyncSigner {
        inner: TestSigner,
    }

    #[async_trait::async_trait]
    impl AsyncFxEvidenceSigner for MalformedAsyncSigner {
        fn key_id(&self) -> Digest32 {
            FxEvidenceSigner::key_id(&self.inner)
        }

        fn public_key(&self) -> [u8; 32] {
            FxEvidenceSigner::public_key(&self.inner)
        }

        async fn sign(&self, _payload: &[u8]) -> Result<Vec<u8>, FxProviderError> {
            Ok(vec![1; 3])
        }
    }

    struct InvalidIdentityAsyncSigner;

    #[async_trait::async_trait]
    impl AsyncFxEvidenceSigner for InvalidIdentityAsyncSigner {
        fn key_id(&self) -> Digest32 {
            [0; 32]
        }

        fn public_key(&self) -> [u8; 32] {
            [0; 32]
        }

        async fn sign(&self, _payload: &[u8]) -> Result<Vec<u8>, FxProviderError> {
            Ok(Vec::new())
        }
    }

    struct MismatchedAsyncSigner {
        inner: TestSigner,
        public_key: [u8; 32],
    }

    #[async_trait::async_trait]
    impl AsyncFxEvidenceSigner for MismatchedAsyncSigner {
        fn key_id(&self) -> Digest32 {
            FxEvidenceSigner::key_id(&self.inner)
        }

        fn public_key(&self) -> [u8; 32] {
            self.public_key
        }

        async fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, FxProviderError> {
            FxEvidenceSigner::sign(&self.inner, payload)
        }
    }

    fn policy_document(policy: &FxProviderPolicy) -> String {
        format!(
            "{{\"policies\":[{}]}}",
            serde_json::to_string(policy).expect("policy JSON")
        )
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

    #[async_trait::async_trait]
    impl AsyncFxEvidenceSigner for TestSigner {
        fn key_id(&self) -> Digest32 {
            self.key_id
        }

        fn public_key(&self) -> [u8; 32] {
            self.signing_key.verifying_key().to_bytes()
        }

        async fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, FxProviderError> {
            <Self as FxEvidenceSigner>::sign(self, payload)
        }
    }

    fn policy() -> FxProviderPolicy {
        FxProviderPolicy {
            policy_digest: [6; 32],
            allowed_pairs: vec![CurrencyPair::new("USD", "EUR").expect("pair")],
            allowed_providers: vec!["provider-a".to_string()],
            minimum_provider_agreement: 1,
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
