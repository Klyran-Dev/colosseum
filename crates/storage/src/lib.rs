#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

//! PostgreSQL persistence adapter for published rule releases and calculation evidence.
//!
//! This crate is an infrastructure adapter. It deliberately does not make
//! PostgreSQL authoritative for funds and it does not put database concerns in
//! the pure tax-engine bounded context.

use aes_gcm_siv::{
    Aes256GcmSiv, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use deadpool_postgres::{
    Config as PoolConfig, ManagerConfig, Pool, RecyclingMethod, Runtime,
    tokio_postgres::{IsolationLevel, NoTls, types::Json},
};
use document_payment::{DocumentPaymentAuthorization, PaymentScope};
use indexer::{
    ApplicabilitySettlementExecutedEvent, BalanceExpectation, BalanceSnapshot, Checkpoint,
    FinalizedApplicabilityBinding, FinalizedPage, FinalizedRefundClaimCancellationEvidence,
    FinalizedRefundClaimEvidence, IndexedAdjustmentRefundExecution, IndexedReceipt,
    IndexedSettlementAdjustment, IndexedSplitSettlement, MAX_REPLAY_ATTEMPTS, MAX_SCAN_RANGE_SLOTS,
    ProtocolPauseChangedEvent, RawChainEvidence, RawChainTransaction, Reconciliation, ReplayGap,
    ReplayGapReason, ReplayWork, SETTLEMENT_ROUTE_VERIFICATION_SCHEMA, STABLECOIN_ASSET_KIND,
    ScanRange, VaultBalanceExpectation, decode_refund_claim_cancelled_event,
    decode_refund_claim_created_event, merge_adjustment_refund_execution, merge_raw_evidence,
    merge_receipt, merge_settlement_adjustment, rebuild_receipt_from_raw_evidence,
    rebuild_split_settlement_from_raw_evidence, reconcile,
};
use jurisdiction::{JurisdictionCandidate, ResolutionPolicy, candidate_set_digest};
use policy::{
    ApplicabilityAttestation, CalculationAttestation, DocumentAttestationBinding,
    DocumentCalculationAttestation, merchant_identity_digest,
};
use refund_workflow::{
    RefundArtifact, RefundCancellationArtifact, RefundCancellationServerInputs,
    RefundClaimServerInputs, RefundExecuteServerInputs, cancellation_request_digest,
    claim_request_digest, execute_request_digest, refund_instruction_digest,
    validate_cancellation_artifact, validate_claim_artifact, validate_execute_artifact,
};
use revenue_streaming::{
    AssetId, Digest as RevenueStreamDigest, FinalizedAccrual, FinalizedCorrection,
    FinalizedRelease, RecipientId, Signature as RevenueStreamSignature, StreamPlan, StreamPlanId,
    StreamPlanSpec,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use settlement_adjustments::{
    Adjustment, AdjustmentDelta, AdjustmentKind, AdjustmentProposal, AdjustmentStatus,
    RefundBalanceProof, authority_challenge_digest, authority_challenge_message,
    validate_transition,
};
use settlement_coordinator::{SettlementStatus, can_transition};
use sha2::{Digest, Sha256};
use solana_pubkey::Pubkey;
use std::{
    collections::{HashMap, VecDeque},
    str::FromStr,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tax_engine::{
    CalculationResult, Digest32, RuleSet, RuleSetDocument, TAX_DOCUMENT_SCHEMA_V1,
    TaxDocumentFacts, TaxDocumentResult, TaxError, TaxFacts,
};
use thiserror::Error;
use uuid::Uuid;

const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const REFUND_EXECUTE_INTENT_SELECT_SQL: &str =
    "SELECT id, merchant_id, idempotency_key, request_digest,
            claim_intent_id, funding_configuration_id,
            funding_configuration_digest, claim, adjustment_id,
            original_receipt, merchant, mint, token_program,
            approver, refund_funder, refund_funder_subject,
            instruction_digest, artifact_digest, status,
            signed_transaction_digest, signature, submitted_slot::text,
            commitment, last_error_code, unsigned_artifact
     FROM refund_execute_intents
     {where}";

pub const CALCULATION_STATUS: &str = "calculated_unattested";
pub const DEFAULT_POOL_SIZE: usize = 16;
pub const DEFAULT_POOL_WAIT_TIMEOUT: Duration = Duration::from_millis(250);
pub const DEFAULT_STATEMENT_TIMEOUT: Duration = Duration::from_millis(900);
pub const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_millis(250);
pub const DEFAULT_OUTBOX_BATCH_SIZE: usize = 32;
pub const MAX_OUTBOX_BATCH_SIZE: usize = 100;
pub const DEFAULT_OUTBOX_LEASE: Duration = Duration::from_secs(30);
pub const MAX_OUTBOX_ATTEMPTS: i32 = 10;
pub const OUTBOX_PAYLOAD_SCHEMA_VERSION: u32 = 1;
pub const REFUND_FUNDING_VERIFIER_VERSION: &str = "refund-funding-topology-v1";
pub const REFUND_FUNDING_VERIFICATION_LEASE: Duration = Duration::from_secs(60);
pub const REFUND_PROOF_CANDIDATE_LEASE: Duration = Duration::from_secs(60);
pub const MAX_REFUND_PROOF_ATTEMPTS: i16 = 3;
pub const MAX_REFUND_PROOF_ERROR_BYTES: usize = 1_024;
pub const APPLICABILITY_BINDING_VERIFICATION_LEASE: Duration = Duration::from_secs(60);
pub const MAX_APPLICABILITY_BINDING_ATTEMPTS: i16 = 8;
pub const MAX_APPLICABILITY_BINDING_ERROR_BYTES: usize = 1_024;
pub const REVENUE_STREAM_ACCRUAL_RECONCILIATION_LEASE: Duration = Duration::from_secs(60);
pub const MAX_REVENUE_STREAM_ACCRUAL_ATTEMPTS: i16 = 8;
pub const MAX_REVENUE_STREAM_RECONCILIATION_CONTEXTS: usize = 64;
pub const MAX_REVENUE_STREAM_RECONCILIATION_ERROR_BYTES: usize = 1_024;
const REFUND_FUNDING_VERIFIER_ACTOR: &str = "system:refund-funding-verifier";
pub const DEFAULT_REPLAY_BATCH_SIZE: usize = 16;
pub const MAX_REPLAY_BATCH_SIZE: usize = 64;
pub const DEFAULT_REPLAY_LEASE: Duration = Duration::from_secs(30);
pub const MAX_REPLAY_OPERATOR_NOTE_BYTES: usize = 1024;
pub const MAX_REPLAY_OPERATOR_SUBJECT_BYTES: usize = 128;
pub const MAX_REPLAY_LIST_LIMIT: usize = 100;
pub const MAX_ROUTE_VERIFICATION_RUN_LIST_LIMIT: usize = 50;
pub const MAX_EVIDENCE_ACCESS_AUDIT_LIST_LIMIT: usize = 100;
pub const MAX_PROJECTION_REBUILD_BATCH_SIZE: usize = 1_000;
pub const MAX_VAULT_RECONCILIATION_TARGETS: usize = 64;
pub const VAULT_RECONCILIATION_STALE_AFTER_SECONDS: i64 = 90;
pub const DEFAULT_COMPILED_RULE_CACHE_CAPACITY: usize = 256;
pub const DEFAULT_QUOTE_TTL: Duration = Duration::from_secs(15 * 60);
pub const ATTESTED_CALCULATION_STATUS: &str = "attested";
pub const MAX_QUOTE_RETRY_ATTEMPTS: usize = 3;
const QUOTE_RETRY_BACKOFF: Duration = Duration::from_millis(2);
pub const MAX_REFUND_INTENT_RETRY_ATTEMPTS: usize = 3;
const REFUND_INTENT_RETRY_BACKOFF: Duration = Duration::from_millis(2);
const MAX_EVIDENCE_LOOKUP_ATTEMPTS: usize = 3;
const EVIDENCE_LOOKUP_BACKOFF: Duration = Duration::from_millis(2);
pub const MAX_RULE_GOVERNANCE_EVIDENCE_BYTES: usize = 16 * 1024;
pub const MAX_RULE_DRAFT_PROVENANCE_BYTES: usize = 16 * 1024;
pub const MAX_JURISDICTION_POLICY_PROVENANCE_BYTES: usize = 16 * 1024;
pub const MAX_APPLICABILITY_CONTEXT_PROVENANCE_BYTES: usize = 16 * 1024;
pub const MAX_FX_EVIDENCE_DOCUMENT_BYTES: usize = 64 * 1024;
pub const MAX_VAULT_RELEASE_EVIDENCE_BYTES: usize = 16 * 1024;
pub const MAX_REVENUE_STREAM_EVIDENCE_BYTES: usize = 16 * 1024;
pub const MAX_VAULT_RELEASE_WINDOW: Duration = Duration::from_secs(90 * 24 * 60 * 60);
pub const MAX_PERSISTED_FACTS_BYTES: usize = 64 * 1024;
pub const MAX_EVIDENCE_KEY_RING_SIZE: usize = 8;
const EVIDENCE_AAD_VERSION: &[u8] = b"stp.tax-facts.v1";

const REPLAY_GAP_VIEW_SELECT: &str =
    "SELECT id, stream_key, start_slot::text, end_slot::text,
            observed_finalized_slot::text, reason, attempts::text, status,
            last_error_code,
            to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
            to_char(dead_lettered_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
            to_char(operator_acknowledged_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
            operator_acknowledged_by, operator_acknowledgement
     FROM indexer_replay_gaps";

const ROUTE_VERIFICATION_RUN_VIEW_SELECT: &str =
    "SELECT id, run_kind, status, batch_size, examined_count,
            activated_count, already_active_count, failed_count, failure_code,
            operator_approval_id,
            to_char(started_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
            to_char(completed_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')
     FROM settlement_route_verification_runs";

const SETTLEMENT_INTENT_TRANSITION_SQL: &str = "UPDATE settlement_intents
     SET status = $3,
         signed_transaction_digest = COALESCE($4, signed_transaction_digest),
         signature = COALESCE($5, signature),
         slot = COALESCE($6::text::numeric, slot),
         commitment = COALESCE($7, commitment),
         last_error_code = $8,
         updated_at = now()
     WHERE id = $1 AND merchant_id = $2 AND status = $9";

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("invalid storage configuration: {0}")]
    InvalidConfiguration(String),
    #[error("database pool error: {0}")]
    Pool(String),
    #[error("database error: {0}")]
    Database(#[from] deadpool_postgres::tokio_postgres::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("tax calculation error: {0}")]
    Tax(#[from] TaxError),
    #[error("FX evidence is missing or invalid: {0}")]
    FxEvidenceIntegrity(String),
    #[error("published rule release was not found")]
    RuleNotFound,
    #[error("stored rule release digest does not match its canonical rule document")]
    RuleDigestMismatch,
    #[error("stored rule release identity does not match the requested selector")]
    RuleIdentityMismatch,
    #[error("published jurisdiction policy was not found for this merchant and rule release")]
    JurisdictionPolicyNotFound,
    #[error("stored jurisdiction policy digest does not match its canonical document")]
    JurisdictionPolicyDigestMismatch,
    #[error("resolved jurisdiction does not match the requested quote selector")]
    JurisdictionMismatch,
    #[error("idempotency key was reused with a different request")]
    IdempotencyConflict,
    #[error("idempotent request is already being completed")]
    IdempotencyInProgress,
    #[error("document payment authorization was not found")]
    DocumentPaymentAuthorizationNotFound,
    #[error("document payment authorization conflicts with the settlement intent")]
    DocumentPaymentAuthorizationConflict,
    #[error("document payment authorization failed integrity validation: {0}")]
    DocumentPaymentAuthorizationIntegrity(String),
    #[error("stored idempotent response is invalid")]
    InvalidStoredResponse,
    #[error("stored tax calculation replay evidence is unavailable")]
    ReplayEvidenceUnavailable,
    #[error("stored tax calculation replay evidence failed integrity validation: {0}")]
    ReplayEvidenceIntegrity(String),
    #[error("stored document calculation replay evidence is unavailable")]
    DocumentReplayEvidenceUnavailable,
    #[error("stored document calculation replay evidence failed integrity validation: {0}")]
    DocumentReplayEvidenceIntegrity(String),
    #[error("document quote was not found for this merchant")]
    DocumentQuoteNotFound,
    #[error("document quote request is invalid: {0}")]
    InvalidDocumentQuote(String),
    #[error("document calculation cannot be used as aggregate authority")]
    DocumentCalculationScopeMismatch,
    #[error("aggregate calculation cannot be used as document authority")]
    AggregateCalculationScopeMismatch,
    #[error("settlement quote was not found for this merchant")]
    QuoteNotFound,
    #[error("settlement quote has expired")]
    QuoteExpired,
    #[error("settlement quote is not in a settleable state")]
    QuoteNotSettleable,
    #[error("public checkout session was not found")]
    CheckoutSessionNotFound,
    #[error("public checkout session has expired")]
    CheckoutSessionExpired,
    #[error("public checkout session token is invalid")]
    CheckoutSessionTokenInvalid,
    #[error("public checkout session is already bound to another settlement intent")]
    CheckoutSessionIntentConflict,
    #[error("public checkout session failed integrity validation: {0}")]
    CheckoutSessionIntegrity(String),
    #[error("calculation attestation evidence was not found")]
    AttestationNotFound,
    #[error("calculation attestation evidence conflicts with the stored record")]
    AttestationConflict,
    #[error("calculation attestation evidence is invalid: {0}")]
    InvalidAttestation(String),
    #[error("calculation attestation evidence is too large")]
    AttestationEvidenceTooLarge,
    #[error("tax applicability context was not found for this merchant")]
    ApplicabilityContextNotFound,
    #[error("tax applicability context conflicts with the stored record")]
    ApplicabilityContextConflict,
    #[error("tax applicability context is invalid: {0}")]
    InvalidApplicabilityContext(String),
    #[error("tax applicability context lifecycle transition is invalid")]
    ApplicabilityContextTransitionConflict,
    #[error("applicability binding verification conflicts with existing evidence")]
    ApplicabilityBindingVerificationConflict,
    #[error("applicability binding verification evidence is invalid: {0}")]
    InvalidApplicabilityBindingVerification(String),
    #[error("settlement route was not found for this merchant")]
    SettlementRouteNotFound,
    #[error("settlement route is not active or is outside its effective window")]
    SettlementRouteUnavailable,
    #[error("settlement route verification evidence is invalid: {0}")]
    SettlementRouteVerificationInvalid(String),
    #[error("stored settlement route failed integrity validation: {0}")]
    SettlementRouteIntegrity(String),
    #[error("settlement route cannot be activated in its current lifecycle state")]
    SettlementRouteActivationConflict,
    #[error("settlement route verification run lifecycle conflict")]
    SettlementRouteVerificationRunConflict,
    #[error("revenue stream destination route was not found for this merchant")]
    RevenueStreamDestinationRouteNotFound,
    #[error("revenue stream destination route is not active or is outside its effective window")]
    RevenueStreamDestinationRouteUnavailable,
    #[error("revenue stream destination route verification evidence is invalid: {0}")]
    RevenueStreamDestinationRouteVerificationInvalid(String),
    #[error("revenue stream destination route cannot be activated in its current lifecycle state")]
    RevenueStreamDestinationRouteActivationConflict,
    #[error("refund funding configuration was not found")]
    RefundFundingConfigurationNotFound,
    #[error("refund funding configuration idempotency key was reused for different input")]
    RefundFundingConfigurationIdempotencyConflict,
    #[error("refund funding configuration is invalid: {0}")]
    InvalidRefundFundingConfiguration(String),
    #[error("refund funding configuration transition is invalid")]
    InvalidRefundFundingConfigurationTransition,
    #[error("refund funding configuration governance conflict")]
    RefundFundingConfigurationGovernanceConflict,
    #[error("refund funding verification evidence is invalid: {0}")]
    RefundFundingConfigurationVerificationInvalid(String),
    #[error("refund funding configuration cannot be activated in its current lifecycle state")]
    RefundFundingConfigurationActivationConflict,
    #[error("refund claim intent was not found")]
    RefundClaimIntentNotFound,
    #[error("refund claim intent idempotency key was reused for different input")]
    RefundClaimIntentIdempotencyConflict,
    #[error("refund claim intent is invalid: {0}")]
    InvalidRefundClaimIntent(String),
    #[error("refund claim nonce could not be allocated")]
    RefundClaimNonceAllocationConflict,
    #[error("refund execute intent was not found")]
    RefundExecuteIntentNotFound,
    #[error("refund execute intent idempotency key was reused for different input")]
    RefundExecuteIntentIdempotencyConflict,
    #[error("refund execute intent is invalid: {0}")]
    InvalidRefundExecuteIntent(String),
    #[error("refund cancellation intent was not found")]
    RefundCancellationIntentNotFound,
    #[error("refund cancellation intent idempotency key was reused for different input")]
    RefundCancellationIntentIdempotencyConflict,
    #[error("refund cancellation intent is invalid: {0}")]
    InvalidRefundCancellationIntent(String),
    #[error("settlement split plan was not found for this merchant")]
    SettlementSplitPlanNotFound,
    #[error("settlement split plan is not active or is outside its effective window")]
    SettlementSplitPlanUnavailable,
    #[error("stored settlement split plan failed integrity validation: {0}")]
    SettlementSplitPlanIntegrity(String),
    #[error("financial amount exceeds PostgreSQL bigint range")]
    AmountOutOfRange,
    #[error("indexer stream key must contain 1 to 200 bytes")]
    InvalidIndexerStreamKey,
    #[error("persisted indexer state is invalid: {0}")]
    InvalidIndexerState(String),
    #[error("persisted indexer checkpoint does not match the requested page")]
    IndexerCheckpointConflict,
    #[error("replay gap reason is invalid: {0}")]
    InvalidReplayReason(String),
    #[error("replay gap state is invalid: {0}")]
    InvalidReplayState(String),
    #[error("replay gap was not found")]
    ReplayGapNotFound,
    #[error("replay gap is not dead-lettered")]
    ReplayGapNotDeadLettered,
    #[error("replay gap requires operator acknowledgement before requeue")]
    ReplayGapNotAcknowledged,
    #[error("replay gap is already acknowledged by another action")]
    ReplayGapAlreadyAcknowledged,
    #[error("invalid replay-gap operator input: {0}")]
    InvalidReplayOperatorInput(String),
    #[error("indexer domain validation failed: {0}")]
    Indexer(#[from] indexer::IndexerError),
    #[error("settlement digest must contain exactly 32 bytes")]
    InvalidSettlementDigest,
    #[error("settlement status is invalid: {0}")]
    InvalidSettlementStatus(String),
    #[error("settlement lifecycle transition is not allowed")]
    InvalidSettlementTransition,
    #[error("finalized settlement receipt binding was not found")]
    SettlementReceiptBindingNotFound,
    #[error("finalized settlement receipt binding is ambiguous or incomplete: {0}")]
    InvalidSettlementReceiptBinding(String),
    #[error("adjustment authority challenge is invalid: {0}")]
    InvalidAdjustmentAuthorityChallenge(String),
    #[error("adjustment authority challenge was not found or is no longer usable")]
    AdjustmentAuthorityChallengeNotFound,
    #[error("adjustment authority binding was not found")]
    AdjustmentAuthorityBindingNotFound,
    #[error("adjustment authority binding conflicts with an active proof")]
    AdjustmentAuthorityConflict,
    #[error("adjustment evidence intent was not found")]
    AdjustmentEvidenceIntentNotFound,
    #[error("adjustment evidence intent conflicts with immutable input")]
    AdjustmentEvidenceIntentConflict,
    #[error("adjustment evidence intent is invalid: {0}")]
    InvalidAdjustmentEvidenceIntent(String),
    #[error("adjustment chain intent was not found")]
    AdjustmentChainIntentNotFound,
    #[error("adjustment chain intent conflicts with immutable evidence")]
    AdjustmentChainIntentConflict,
    #[error("adjustment chain intent lifecycle transition is not allowed")]
    InvalidAdjustmentChainTransition,
    #[error("settlement evidence exceeds the bounded size")]
    SettlementEvidenceTooLarge,
    #[error("invalid tax-rule governance transition")]
    InvalidRuleGovernanceTransition,
    #[error("tax-rule governance actor is not authorized for the current lifecycle state")]
    RuleGovernanceConflict,
    #[error("tax-rule governance rule was not found")]
    RuleGovernanceNotFound,
    #[error("tax-rule simulation rule was not found")]
    RuleSimulationNotFound,
    #[error("tax-rule simulation facts jurisdiction does not match the persisted rule")]
    RuleSimulationJurisdictionMismatch,
    #[error("tax-rule simulation is not available for a retired rule")]
    RuleSimulationUnavailable,
    #[error("persisted tax-rule simulation evidence failed integrity validation: {0}")]
    RuleSimulationIntegrity(String),
    #[error("tax-rule governance evidence must be a JSON object")]
    InvalidRuleGovernanceEvidence,
    #[error("tax-rule governance evidence exceeds the bounded size")]
    RuleGovernanceEvidenceTooLarge,
    #[error("invalid tax-rule release provenance: {0}")]
    InvalidRuleReleaseProvenance(String),
    #[error("invalid tax-rule draft: {0}")]
    InvalidRuleDraft(String),
    #[error("a rule draft with this tenant, external ID, and version already exists")]
    RuleDraftConflict,
    #[error("jurisdiction policy governance transition is invalid")]
    InvalidJurisdictionPolicyTransition,
    #[error("jurisdiction policy governance actor conflict")]
    JurisdictionPolicyGovernanceConflict,
    #[error("jurisdiction policy governance record was not found")]
    JurisdictionPolicyGovernanceNotFound,
    #[error("jurisdiction policy governance evidence must be a JSON object")]
    InvalidJurisdictionPolicyEvidence,
    #[error("jurisdiction policy governance evidence exceeds the bounded size")]
    JurisdictionPolicyEvidenceTooLarge,
    #[error("jurisdiction policy draft is invalid: {0}")]
    InvalidJurisdictionPolicyDraft(String),
    #[error("a jurisdiction policy already exists for this tenant and rule release")]
    JurisdictionPolicyDraftConflict,
    #[error("vault release claim was not found for this merchant")]
    VaultReleaseClaimNotFound,
    #[error("vault reconciliation target was not found for this merchant")]
    VaultReconciliationTargetNotFound,
    #[error("vault release claim transition is invalid")]
    InvalidVaultReleaseClaimTransition,
    #[error("vault release claim actor conflict")]
    VaultReleaseClaimActorConflict,
    #[error("vault release claim evidence must be a JSON object")]
    InvalidVaultReleaseEvidence,
    #[error("vault release claim evidence exceeds the bounded size")]
    VaultReleaseEvidenceTooLarge,
    #[error("vault release claim input is invalid: {0}")]
    InvalidVaultReleaseClaim(String),
    #[error("vault release claim idempotency key was reused with a different request")]
    VaultReleaseClaimIdempotencyConflict,
    #[error("revenue stream plan was not found for this merchant")]
    RevenueStreamPlanNotFound,
    #[error("revenue stream plan idempotency key was reused with a different request")]
    RevenueStreamPlanIdempotencyConflict,
    #[error("revenue stream plan transition is invalid")]
    InvalidRevenueStreamPlanTransition,
    #[error("revenue stream plan actor conflict")]
    RevenueStreamPlanActorConflict,
    #[error("revenue stream input is invalid: {0}")]
    InvalidRevenueStream(String),
    #[error("revenue stream evidence conflicts with an existing record")]
    RevenueStreamEvidenceConflict,
    #[error(
        "revenue stream accrual reconciliation job lease is missing or owned by another worker"
    )]
    RevenueStreamAccrualJobLeaseLost,
    #[error("revenue stream accrual reconciliation job is invalid: {0}")]
    InvalidRevenueStreamAccrualJob(String),
    #[error("revenue stream claim was not found for this merchant")]
    RevenueStreamClaimNotFound,
    #[error("revenue stream claim has insufficient available balance")]
    RevenueStreamInsufficientAvailable,
    #[error("normalized tax facts encryption is not configured")]
    EvidenceCipherUnavailable,
    #[error("normalized tax facts encryption failed: {0}")]
    EvidenceEncryption(String),
    #[error("normalized tax facts ciphertext failed integrity validation: {0}")]
    EvidenceIntegrity(String),
    #[error("normalized tax facts exceed the bounded persistence limit")]
    EvidenceTooLarge,
    #[error("settlement adjustment was not found for this merchant")]
    SettlementAdjustmentNotFound,
    #[error("settlement adjustment request is invalid: {0}")]
    InvalidSettlementAdjustment(String),
    #[error("settlement adjustment transition is invalid")]
    InvalidSettlementAdjustmentTransition,
    #[error("settlement adjustment actor conflict")]
    SettlementAdjustmentActorConflict,
    #[error("settlement adjustment idempotency key was reused with a different request")]
    SettlementAdjustmentIdempotencyConflict,
    #[error("refund balance proof was not found for the indexed execution")]
    RefundBalanceProofExecutionNotFound,
    #[error("refund balance proof is invalid: {0}")]
    InvalidRefundBalanceProof(String),
    #[error("refund balance proof conflicts with existing immutable evidence")]
    RefundBalanceProofConflict,
    #[error("refund balance proof candidate lease is missing or owned by another worker")]
    RefundBalanceProofLeaseLost,
}

#[derive(Clone)]
pub struct EvidenceKey {
    key_id: String,
    key: [u8; 32],
}

impl std::fmt::Debug for EvidenceKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EvidenceKey")
            .field("key_id", &self.key_id)
            .field("key", &"[redacted]")
            .finish()
    }
}

pub trait EvidenceKeyProvider: Send + Sync {
    fn current_key(&self) -> Result<EvidenceKey, StorageError>;
    fn resolve_key(&self, key_id: &str) -> Result<EvidenceKey, StorageError>;
}

#[derive(Clone)]
pub struct LocalEvidenceKeyProvider {
    current_key_id: String,
    keys: Arc<HashMap<String, [u8; 32]>>,
}

impl std::fmt::Debug for LocalEvidenceKeyProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalEvidenceKeyProvider")
            .field("current_key_id", &self.current_key_id)
            .field("key_count", &self.keys.len())
            .finish()
    }
}

#[derive(Debug, Deserialize)]
struct EvidenceKeyRingDocument {
    current_key_id: String,
    keys: Vec<EvidenceKeyDocument>,
}

#[derive(Debug, Deserialize)]
struct EvidenceKeyDocument {
    key_id: String,
    key_b64: String,
}

impl LocalEvidenceKeyProvider {
    pub fn from_base64(key_id: impl Into<String>, encoded_key: &str) -> Result<Self, StorageError> {
        let key_id = key_id.into();
        let key = decode_evidence_key(&key_id, encoded_key)?;
        let mut keys = HashMap::with_capacity(1);
        keys.insert(key_id.clone(), key);
        Ok(Self {
            current_key_id: key_id,
            keys: Arc::new(keys),
        })
    }

    pub fn from_json(document: &str) -> Result<Self, StorageError> {
        let document: EvidenceKeyRingDocument =
            serde_json::from_str(document).map_err(|error| {
                StorageError::InvalidConfiguration(format!("invalid evidence key ring: {error}"))
            })?;
        if document.keys.is_empty() || document.keys.len() > MAX_EVIDENCE_KEY_RING_SIZE {
            return Err(StorageError::InvalidConfiguration(format!(
                "evidence key ring must contain 1 to {} keys",
                MAX_EVIDENCE_KEY_RING_SIZE
            )));
        }
        let mut keys = HashMap::with_capacity(document.keys.len());
        for entry in document.keys {
            let key = decode_evidence_key(&entry.key_id, &entry.key_b64)?;
            if keys.insert(entry.key_id.clone(), key).is_some() {
                return Err(StorageError::InvalidConfiguration(
                    "evidence key ring contains duplicate key IDs".into(),
                ));
            }
        }
        if !keys.contains_key(&document.current_key_id) {
            return Err(StorageError::InvalidConfiguration(
                "evidence key ring current key ID is not present".into(),
            ));
        }
        Ok(Self {
            current_key_id: document.current_key_id,
            keys: Arc::new(keys),
        })
    }
}

impl EvidenceKeyProvider for LocalEvidenceKeyProvider {
    fn current_key(&self) -> Result<EvidenceKey, StorageError> {
        let key = self
            .keys
            .get(&self.current_key_id)
            .ok_or(StorageError::EvidenceCipherUnavailable)?;
        Ok(EvidenceKey {
            key_id: self.current_key_id.clone(),
            key: *key,
        })
    }

    fn resolve_key(&self, key_id: &str) -> Result<EvidenceKey, StorageError> {
        let key = self.keys.get(key_id).ok_or_else(|| {
            StorageError::EvidenceIntegrity(
                "ciphertext references an unavailable evidence key".into(),
            )
        })?;
        Ok(EvidenceKey {
            key_id: key_id.to_string(),
            key: *key,
        })
    }
}

fn decode_evidence_key(key_id: &str, encoded_key: &str) -> Result<[u8; 32], StorageError> {
    if key_id.trim().is_empty() || key_id.len() > 128 {
        return Err(StorageError::InvalidConfiguration(
            "evidence key ID must contain 1 to 128 bytes".into(),
        ));
    }
    let key_bytes = BASE64_STANDARD.decode(encoded_key).map_err(|error| {
        StorageError::InvalidConfiguration(format!("invalid evidence key: {error}"))
    })?;
    key_bytes.try_into().map_err(|bytes: Vec<u8>| {
        StorageError::InvalidConfiguration(format!(
            "evidence key must contain exactly 32 bytes, got {}",
            bytes.len()
        ))
    })
}

#[derive(Clone)]
pub struct EvidenceCipher {
    provider: Arc<dyn EvidenceKeyProvider>,
    active_key_id: String,
}

impl std::fmt::Debug for EvidenceCipher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EvidenceCipher")
            .field("active_key_id", &self.active_key_id)
            .field("provider", &"[redacted]")
            .finish()
    }
}

#[derive(Clone, Debug)]
struct EncryptedEvidence {
    ciphertext: Vec<u8>,
    nonce: [u8; 12],
    key_id: String,
}

impl EvidenceCipher {
    pub fn from_base64(key_id: impl Into<String>, encoded_key: &str) -> Result<Self, StorageError> {
        Self::from_provider(Arc::new(LocalEvidenceKeyProvider::from_base64(
            key_id,
            encoded_key,
        )?))
    }

    pub fn from_key_ring_json(document: &str) -> Result<Self, StorageError> {
        Self::from_provider(Arc::new(LocalEvidenceKeyProvider::from_json(document)?))
    }

    pub fn from_provider(provider: Arc<dyn EvidenceKeyProvider>) -> Result<Self, StorageError> {
        let active_key_id = provider.current_key()?.key_id;
        Ok(Self {
            provider,
            active_key_id,
        })
    }

    pub fn key_id(&self) -> &str {
        &self.active_key_id
    }

    fn encrypt_facts(
        &self,
        merchant_id: Uuid,
        facts_digest: &[u8; 32],
        facts: &TaxFacts,
    ) -> Result<EncryptedEvidence, StorageError> {
        let plaintext = serde_json::to_vec(facts)?;
        if plaintext.len() > MAX_PERSISTED_FACTS_BYTES {
            return Err(StorageError::EvidenceTooLarge);
        }
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&Uuid::new_v4().as_bytes()[..12]);
        let key = self.provider.current_key()?;
        let cipher = Aes256GcmSiv::new_from_slice(&key.key)
            .map_err(|error| StorageError::EvidenceEncryption(error.to_string()))?;
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: &plaintext,
                    aad: &evidence_aad(merchant_id, facts_digest),
                },
            )
            .map_err(|error| StorageError::EvidenceEncryption(error.to_string()))?;
        Ok(EncryptedEvidence {
            ciphertext,
            nonce,
            key_id: key.key_id,
        })
    }

    fn encrypt_document_facts(
        &self,
        merchant_id: Uuid,
        facts_digest: &[u8; 32],
        facts: &TaxDocumentFacts,
    ) -> Result<EncryptedEvidence, StorageError> {
        let plaintext = serde_json::to_vec(facts)?;
        if plaintext.len() > MAX_PERSISTED_FACTS_BYTES {
            return Err(StorageError::EvidenceTooLarge);
        }
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&Uuid::new_v4().as_bytes()[..12]);
        let key = self.provider.current_key()?;
        let cipher = Aes256GcmSiv::new_from_slice(&key.key)
            .map_err(|error| StorageError::EvidenceEncryption(error.to_string()))?;
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: &plaintext,
                    aad: &evidence_aad(merchant_id, facts_digest),
                },
            )
            .map_err(|error| StorageError::EvidenceEncryption(error.to_string()))?;
        Ok(EncryptedEvidence {
            ciphertext,
            nonce,
            key_id: key.key_id,
        })
    }

    fn decrypt_facts(
        &self,
        merchant_id: Uuid,
        facts_digest: &[u8; 32],
        key_id: &str,
        nonce: &[u8],
        ciphertext: &[u8],
    ) -> Result<Value, StorageError> {
        let nonce: [u8; 12] = nonce.try_into().map_err(|_| {
            StorageError::EvidenceIntegrity("evidence nonce must contain 12 bytes".into())
        })?;
        let key = self.provider.resolve_key(key_id)?;
        let cipher = Aes256GcmSiv::new_from_slice(&key.key)
            .map_err(|error| StorageError::EvidenceEncryption(error.to_string()))?;
        let plaintext = cipher
            .decrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: ciphertext,
                    aad: &evidence_aad(merchant_id, facts_digest),
                },
            )
            .map_err(|_| StorageError::EvidenceIntegrity("AEAD authentication failed".into()))?;
        if plaintext.len() > MAX_PERSISTED_FACTS_BYTES {
            return Err(StorageError::EvidenceTooLarge);
        }
        serde_json::from_slice(&plaintext)
            .map_err(|error| StorageError::EvidenceIntegrity(error.to_string()))
    }
}

fn evidence_aad(merchant_id: Uuid, facts_digest: &[u8; 32]) -> Vec<u8> {
    let mut aad = Vec::with_capacity(EVIDENCE_AAD_VERSION.len() + 16 + 32 + 2);
    aad.extend_from_slice(EVIDENCE_AAD_VERSION);
    aad.push(0);
    aad.extend_from_slice(merchant_id.as_bytes());
    aad.push(0);
    aad.extend_from_slice(facts_digest);
    aad
}

pub fn validate_create_revenue_stream_plan_command(
    command: &CreateRevenueStreamPlanCommand,
) -> Result<(), StorageError> {
    if command.merchant_id.get_version_num() != 7
        || command.destination_route_id.get_version_num() != 7
        || command.actor_subject.trim().is_empty()
        || command.actor_subject.len() > 256
        || command
            .actor_subject
            .bytes()
            .any(|byte| byte.is_ascii_control())
        || command.idempotency_key.trim().is_empty()
        || command.idempotency_key.len() > 200
        || command
            .idempotency_key
            .bytes()
            .any(|byte| byte.is_ascii_control())
        || command.spec.merchant_id != command.merchant_id
        || command
            .spec
            .operator_subject
            .as_deref()
            .is_none_or(|subject| {
                subject.trim().is_empty()
                    || subject.len() > 256
                    || subject.bytes().any(|byte| byte.is_ascii_control())
                    || subject == command.actor_subject
            })
    {
        return Err(StorageError::InvalidRevenueStream(
            "stream plan identity, actor, or idempotency input is invalid".into(),
        ));
    }
    StreamPlan::new(command.spec.clone()).map_err(|error| {
        StorageError::InvalidRevenueStream(format!("stream plan spec is invalid: {error}"))
    })?;
    Ok(())
}

pub fn revenue_stream_plan_request_digest(
    command: &CreateRevenueStreamPlanCommand,
) -> Result<[u8; 32], StorageError> {
    validate_create_revenue_stream_plan_command(command)?;
    let canonical = serde_json::to_vec(&serde_json::json!({
        "schema": "revenue-stream-plan/v1",
        "merchant_id": command.merchant_id,
        "destination_route_id": command.destination_route_id,
        "idempotency_key": command.idempotency_key,
        "spec": command.spec,
    }))?;
    Ok(Sha256::digest(canonical).into())
}

#[derive(Clone, Debug)]
pub struct PgStore {
    pool: Pool,
    engine_version: String,
    compiled_rule_cache: CompiledRuleCache,
    evidence_cipher: Option<EvidenceCipher>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DatabasePoolStats {
    pub max_size: usize,
    pub size: usize,
    pub available: usize,
    pub waiting: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuleCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub entries: usize,
    pub capacity: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectionRebuildReport {
    pub scanned_rows: u64,
    pub inserted_rows: u64,
    pub duplicate_rows: u64,
    pub projection_digest: [u8; 32],
}

#[derive(Clone, Debug)]
struct CompiledRuleCache {
    state: Arc<Mutex<CompiledRuleCacheState>>,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    evictions: Arc<AtomicU64>,
    capacity: usize,
}

#[derive(Debug)]
struct CompiledRuleCacheState {
    entries: HashMap<[u8; 32], Arc<tax_engine::CompiledRuleSet>>,
    lru: VecDeque<[u8; 32]>,
}

impl CompiledRuleCache {
    fn new(capacity: usize) -> Result<Self, StorageError> {
        if capacity == 0 {
            return Err(StorageError::InvalidConfiguration(
                "compiled rule cache capacity must be positive".to_string(),
            ));
        }
        Ok(Self {
            state: Arc::new(Mutex::new(CompiledRuleCacheState {
                entries: HashMap::with_capacity(capacity),
                lru: VecDeque::with_capacity(capacity),
            })),
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            evictions: Arc::new(AtomicU64::new(0)),
            capacity,
        })
    }

    fn lock(&self) -> MutexGuard<'_, CompiledRuleCacheState> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn get_or_compile(
        &self,
        digest: [u8; 32],
        rule_set: &RuleSet,
    ) -> Result<Arc<tax_engine::CompiledRuleSet>, StorageError> {
        {
            let mut state = self.lock();
            if let Some(compiled) = state.entries.get(&digest).cloned() {
                touch_lru(&mut state.lru, digest);
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Ok(compiled);
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        let compiled = Arc::new(rule_set.compile()?);
        if compiled.rule_digest() != digest {
            return Err(StorageError::RuleDigestMismatch);
        }
        let mut state = self.lock();
        if let Some(existing) = state.entries.get(&digest).cloned() {
            touch_lru(&mut state.lru, digest);
            return Ok(existing);
        }
        if state.entries.len() >= self.capacity
            && let Some(evicted) = state.lru.pop_front()
        {
            state.entries.remove(&evicted);
            self.evictions.fetch_add(1, Ordering::Relaxed);
        }
        state.entries.insert(digest, Arc::clone(&compiled));
        state.lru.push_back(digest);
        Ok(compiled)
    }

    fn stats(&self) -> RuleCacheStats {
        RuleCacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            entries: self.lock().entries.len(),
            capacity: self.capacity,
        }
    }
}

fn touch_lru(lru: &mut VecDeque<[u8; 32]>, digest: [u8; 32]) {
    if let Some(position) = lru.iter().position(|candidate| *candidate == digest) {
        lru.remove(position);
    }
    lru.push_back(digest);
}

fn is_retryable_quote_error(error: &StorageError) -> bool {
    match error {
        StorageError::Database(database_error) => database_error
            .code()
            .is_some_and(|code| is_retryable_quote_sql_state(code.code())),
        _ => false,
    }
}

fn is_retryable_quote_sql_state(code: &str) -> bool {
    matches!(code, "40001" | "40P01")
}

fn is_retryable_refund_intent_sql_state(code: &str) -> bool {
    matches!(code, "40001" | "40P01")
}

fn is_retryable_refund_intent_error(error: &StorageError) -> bool {
    match error {
        StorageError::Database(database_error) => database_error
            .code()
            .is_some_and(|code| is_retryable_refund_intent_sql_state(code.code())),
        _ => false,
    }
}

fn is_retryable_evidence_lookup_error(error: &StorageError) -> bool {
    match error {
        StorageError::Pool(_) => true,
        StorageError::Database(database_error) => {
            database_error.is_closed()
                || database_error
                    .code()
                    .is_some_and(|code| is_retryable_quote_sql_state(code.code()))
        }
        _ => false,
    }
}

#[derive(Clone, Debug)]
pub struct QuoteCommand {
    pub merchant_id: Uuid,
    pub idempotency_key: String,
    pub rule_set_id: String,
    pub rule_version: u32,
    pub jurisdiction: String,
    pub regime: String,
    pub facts: TaxFacts,
    pub applicability_context_id: Option<Uuid>,
    pub jurisdiction_candidates: Vec<JurisdictionCandidate>,
    pub fx_evidence: Option<PersistedFxEvidence>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedFxEvidence {
    pub pre_resolution_facts_digest: Digest32,
    pub envelope: fx_evidence::FxEvidenceEnvelope,
}

#[derive(Clone, Debug)]
pub struct DocumentQuoteCommand {
    pub merchant_id: Uuid,
    pub idempotency_key: String,
    pub schema: String,
    pub rule_set_id: String,
    pub rule_version: u32,
    pub jurisdiction: String,
    pub regime: String,
    pub facts: TaxDocumentFacts,
    pub applicability_context_id: Option<Uuid>,
    pub jurisdiction_candidates: Vec<JurisdictionCandidate>,
    pub fx_evidence: Option<PersistedFxEvidence>,
}

#[derive(Clone, Debug)]
pub struct CreateRuleDraftCommand {
    pub merchant_id: Uuid,
    pub actor_subject: String,
    pub idempotency_key: String,
    pub external_id: String,
    pub version: u32,
    pub jurisdiction: String,
    pub regime: String,
    pub effective_from_unix: i64,
    pub effective_until_unix: Option<i64>,
    pub rule_set: RuleSet,
    pub provenance: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreatedRuleDraft {
    pub rule_set_id: Uuid,
    pub external_id: String,
    pub version: i32,
    pub jurisdiction: String,
    pub regime: String,
    pub status: String,
    pub rule_digest: Digest32,
    pub effective_from: String,
    pub effective_until: Option<String>,
    pub governance_revision: i64,
}

#[derive(Clone, Debug)]
pub enum RuleDraftOutcome {
    Created(CreatedRuleDraft),
    Replay(CreatedRuleDraft),
}

#[derive(Clone, Debug)]
pub struct CreateJurisdictionPolicyCommand {
    pub merchant_id: Uuid,
    pub actor_subject: String,
    pub idempotency_key: String,
    pub rule_set_id: Uuid,
    pub policy: ResolutionPolicy,
    pub effective_from_unix: i64,
    pub effective_until_unix: Option<i64>,
    pub provenance: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreatedJurisdictionPolicy {
    pub policy_id: Uuid,
    pub rule_set_id: Uuid,
    pub policy_digest: Digest32,
    pub status: String,
    pub effective_from: String,
    pub effective_until: Option<String>,
    pub governance_revision: i64,
}

#[derive(Clone, Debug)]
pub enum JurisdictionPolicyDraftOutcome {
    Created(CreatedJurisdictionPolicy),
    Replay(CreatedJurisdictionPolicy),
}

#[derive(Clone, Debug)]
pub struct JurisdictionPolicyGovernanceTransition {
    pub merchant_id: Uuid,
    pub policy_id: Uuid,
    pub actor_subject: String,
    pub to_status: String,
    pub evidence: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JurisdictionPolicyGovernanceResult {
    pub policy_id: Uuid,
    pub from_status: String,
    pub to_status: String,
    pub author_subject: Option<String>,
    pub reviewer_subject: Option<String>,
    pub approver_subject: Option<String>,
    pub governance_revision: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JurisdictionPolicyGovernanceEvent {
    pub id: Uuid,
    pub actor_subject: String,
    pub action: String,
    pub from_status: String,
    pub to_status: String,
    pub evidence: Value,
    pub occurred_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JurisdictionPolicyGovernanceState {
    pub merchant_id: Uuid,
    pub policy_id: Uuid,
    pub rule_set_id: Uuid,
    pub policy_digest: Digest32,
    pub policy: ResolutionPolicy,
    pub provenance: Value,
    pub status: String,
    pub author_subject: Option<String>,
    pub reviewer_subject: Option<String>,
    pub approver_subject: Option<String>,
    pub governance_revision: i64,
    pub effective_from: String,
    pub effective_until: Option<String>,
    pub events: Vec<JurisdictionPolicyGovernanceEvent>,
}

#[derive(Clone, Debug)]
pub struct CreateVaultReleaseClaimCommand {
    pub merchant_id: Uuid,
    pub actor_subject: String,
    pub idempotency_key: String,
    pub route_id: Uuid,
    pub destination: String,
    pub nonce: u64,
    pub amount_atomic: u64,
    pub reference_digest: Digest32,
    pub execute_after_unix: i64,
    pub expires_at_unix: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VaultReleaseClaim {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub idempotency_key: String,
    pub request_digest: Digest32,
    pub mint: String,
    pub token_program: String,
    pub tax_vault: String,
    pub destination: String,
    pub claim_pda: Option<String>,
    #[serde(with = "policy::serde_u64_string")]
    pub nonce: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub amount_atomic: u64,
    pub reference_digest: Digest32,
    pub status: String,
    pub proposer_subject: String,
    pub approver_subject: Option<String>,
    pub execute_after_unix: i64,
    pub expires_at_unix: i64,
    pub transaction_signature: Option<String>,
    #[serde(with = "policy::serde_optional_u64_string")]
    pub slot: Option<u64>,
    pub last_error_code: Option<String>,
}

#[derive(Clone, Debug)]
pub enum VaultReleaseClaimOutcome {
    Created(VaultReleaseClaim),
    Replay(VaultReleaseClaim),
}

#[derive(Clone, Debug)]
pub struct VaultReleaseClaimTransition {
    pub merchant_id: Uuid,
    pub claim_id: Uuid,
    pub actor_subject: String,
    pub to_status: String,
    pub evidence: Value,
    pub claim_pda: Option<String>,
    pub transaction_signature: Option<String>,
    pub slot: Option<u64>,
    pub last_error_code: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultReleaseClaimTransitionResult {
    pub claim_id: Uuid,
    pub from_status: String,
    pub to_status: String,
    pub approver_subject: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CreateRevenueStreamPlanCommand {
    pub merchant_id: Uuid,
    pub destination_route_id: Uuid,
    pub actor_subject: String,
    pub idempotency_key: String,
    pub spec: StreamPlanSpec,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RevenueStreamPlanRecord {
    pub id: StreamPlanId,
    pub merchant_id: Uuid,
    pub idempotency_key: String,
    pub request_digest: [u8; 32],
    pub destination_route_id: Option<Uuid>,
    pub spec: StreamPlanSpec,
    pub status: String,
    pub proposer_subject: String,
    pub approver_subject: Option<String>,
    #[serde(with = "policy::serde_u64_string")]
    pub accrued_total_atomic: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub released_total_atomic: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub next_nonce: u64,
}

#[derive(Clone, Debug)]
pub struct RevenueStreamReconciliationContextRecord {
    pub merchant_id: Uuid,
    pub stream_plan: RevenueStreamPlanRecord,
    pub settlement_split_plan: SettlementSplitPlan,
}

#[derive(Clone, Debug)]
pub struct RevenueStreamAccrualReconciliationJob {
    pub stream_key: String,
    pub source_signature: String,
    pub evidence_digest: Digest32,
    pub observed_finalized_slot: u64,
    pub claim_token: Uuid,
    pub split_settlement: IndexedSplitSettlement,
    pub receipt: IndexedReceipt,
}

#[derive(Clone, Debug)]
pub enum RevenueStreamPlanOutcome {
    Created(RevenueStreamPlanRecord),
    Replay(RevenueStreamPlanRecord),
}

#[derive(Clone, Debug)]
pub struct RevenueStreamPlanTransition {
    pub merchant_id: Uuid,
    pub plan_id: StreamPlanId,
    pub actor_subject: String,
    pub to_status: String,
    pub observed_slot: Option<u64>,
    pub evidence: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevenueStreamPlanTransitionResult {
    pub plan_id: StreamPlanId,
    pub from_status: String,
    pub to_status: String,
    pub approver_subject: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RecordRevenueStreamAccrualCommand {
    pub merchant_id: Uuid,
    pub actor_subject: String,
    pub plan_id: StreamPlanId,
    pub accrual: FinalizedAccrual,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RevenueStreamAccrualOutcome {
    Recorded,
    Duplicate,
}

#[derive(Clone, Debug)]
pub struct RecordRevenueStreamCorrectionCommand {
    pub merchant_id: Uuid,
    pub actor_subject: String,
    pub plan_id: StreamPlanId,
    pub correction: FinalizedCorrection,
}

#[derive(Clone, Debug)]
pub struct PrepareRevenueStreamClaimCommand {
    pub merchant_id: Uuid,
    pub actor_subject: String,
    pub plan_id: StreamPlanId,
    pub idempotency_key: String,
    pub claim_id: Uuid,
    pub amount_atomic: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RevenueStreamClaimRecord {
    pub id: Uuid,
    pub plan_id: StreamPlanId,
    pub merchant_id: Uuid,
    pub idempotency_key: String,
    #[serde(with = "policy::serde_u64_string")]
    pub amount_atomic: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub nonce: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub accrual_total_atomic: u64,
    pub plan_digest: [u8; 32],
    pub asset: [u8; 32],
    pub recipient: [u8; 32],
    pub claim_digest: [u8; 32],
    pub status: String,
    pub release_signature: Option<Vec<u8>>,
    pub finalized_slot: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct FinalizeRevenueStreamClaimCommand {
    pub merchant_id: Uuid,
    pub actor_subject: String,
    pub release: FinalizedRelease,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RevenueStreamClaimOutcome {
    Recorded,
    Duplicate,
}

#[derive(Clone, Debug)]
pub struct CreateSettlementAdjustmentCommand {
    pub merchant_id: Uuid,
    pub actor_subject: String,
    pub idempotency_key: String,
    pub proposal: AdjustmentProposal,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SettlementAdjustmentRecord {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub idempotency_key: String,
    pub request_digest: Digest32,
    pub adjustment_digest: Digest32,
    pub original_settlement_id: Uuid,
    pub asset_id: [u8; 32],
    pub kind: AdjustmentKind,
    pub delta: AdjustmentDelta,
    pub reason_code: String,
    pub evidence_digest: Digest32,
    pub source_calculation_id: Uuid,
    pub replacement_calculation_id: Option<Uuid>,
    pub status: String,
    pub proposer_subject: String,
    pub approver_subject: Option<String>,
}

#[derive(Clone, Debug)]
pub enum SettlementAdjustmentOutcome {
    Created(SettlementAdjustmentRecord),
    Replay(SettlementAdjustmentRecord),
}

#[derive(Clone, Debug)]
pub struct SettlementAdjustmentTransition {
    pub merchant_id: Uuid,
    pub adjustment_id: Uuid,
    pub actor_subject: String,
    pub to_status: String,
    pub evidence: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettlementAdjustmentTransitionResult {
    pub adjustment_id: Uuid,
    pub from_status: String,
    pub to_status: String,
    pub approver_subject: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StoredQuote {
    pub calculation_id: Uuid,
    pub status: String,
    #[serde(default)]
    pub expires_at_unix: i64,
    #[serde(default)]
    pub applicability_context_id: Option<Uuid>,
    #[serde(default)]
    pub applicability_attestation_digest: Option<Digest32>,
    pub result: CalculationResult,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StoredDocumentQuote {
    pub calculation_id: Uuid,
    pub status: String,
    pub expires_at_unix: i64,
    pub schema: String,
    #[serde(default)]
    pub applicability_context_id: Option<Uuid>,
    #[serde(default)]
    pub applicability_attestation_digest: Option<Digest32>,
    pub result: TaxDocumentResult,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StoredCalculationAttestation {
    pub calculation_id: Uuid,
    pub payload_digest: Digest32,
    pub issuer_public_key: [u8; 32],
    pub expires_at_unix: i64,
    pub attestation: CalculationAttestation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StoredDocumentCalculationAttestation {
    pub calculation_id: Uuid,
    pub payload_digest: Digest32,
    pub issuer_public_key: [u8; 32],
    pub expires_at_unix: i64,
    pub attestation: DocumentCalculationAttestation,
}

#[derive(Clone, Debug)]
pub struct CreateApplicabilityContextCommand {
    pub merchant_id: Uuid,
    pub idempotency_key: String,
    pub attestation: ApplicabilityAttestation,
    pub effective_from_unix: i64,
    pub effective_until_unix: Option<i64>,
    pub provenance: Value,
    pub actor_subject: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StoredApplicabilityContext {
    pub context_id: Uuid,
    pub merchant_id: Uuid,
    pub status: String,
    pub policy_id: String,
    pub policy_version: u32,
    pub policy_digest: Digest32,
    pub facts_digest: Digest32,
    pub decision_digest: Digest32,
    pub jurisdiction_resolution_digest: Digest32,
    pub evidence_digest: Digest32,
    pub attestation_digest: Digest32,
    pub effective_from_unix: i64,
    pub effective_until_unix: Option<i64>,
    pub evidence_expires_at_unix: i64,
    pub provenance: Value,
    pub attestation: ApplicabilityAttestation,
}

#[derive(Clone, Debug)]
pub enum ApplicabilityContextOutcome {
    Created(StoredApplicabilityContext),
    Replay(StoredApplicabilityContext),
}

#[derive(Clone, Debug)]
pub struct ApplicabilityContextTransition {
    pub context_id: Uuid,
    pub merchant_id: Uuid,
    pub expected_status: String,
    pub next_status: String,
    pub actor_subject: String,
    pub evidence: Value,
}

#[derive(Clone, Debug)]
pub struct RuleGovernanceTransition {
    pub merchant_id: Uuid,
    pub rule_set_id: Uuid,
    pub actor_subject: String,
    pub to_status: String,
    pub evidence: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleGovernanceResult {
    pub rule_set_id: Uuid,
    pub from_status: String,
    pub to_status: String,
    pub author_subject: Option<String>,
    pub reviewer_subject: Option<String>,
    pub approver_subject: Option<String>,
    pub governance_revision: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RuleGovernanceEvent {
    pub id: Uuid,
    pub actor_subject: String,
    pub action: String,
    pub from_status: String,
    pub to_status: String,
    pub evidence: Value,
    pub occurred_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RuleGovernanceState {
    pub merchant_id: Uuid,
    pub rule_set_id: Uuid,
    pub external_id: String,
    pub version: i32,
    pub jurisdiction: String,
    pub regime: String,
    pub status: String,
    pub rule_digest_hex: String,
    pub author_subject: Option<String>,
    pub reviewer_subject: Option<String>,
    pub approver_subject: Option<String>,
    pub governance_revision: i64,
    pub effective_from: String,
    pub effective_until: Option<String>,
    pub events: Vec<RuleGovernanceEvent>,
}

type GovernanceSubjects = (Option<String>, Option<String>, Option<String>);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QuoteReplayVerification {
    pub calculation_id: Uuid,
    pub status: String,
    pub engine_version: String,
    pub facts_digest: Digest32,
    pub rule_digest: Digest32,
}

#[derive(Clone, Debug)]
pub struct EvidenceAccessAuditCommand {
    pub merchant_id: Uuid,
    pub calculation_id: Uuid,
    pub actor_subject: String,
    pub request_id: Uuid,
    pub purpose: String,
    pub outcome: String,
    pub facts_digest: Option<Digest32>,
}

/// Redacted, tenant-scoped projection for the evidence-access explorer.
/// Decrypted facts and request bodies never cross this read boundary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EvidenceAccessAuditEventView {
    pub id: Uuid,
    pub calculation_id: Uuid,
    pub actor_subject: String,
    pub request_id: Uuid,
    pub purpose: String,
    pub outcome: String,
    pub facts_digest: Option<Digest32>,
    pub occurred_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SettlementRoute {
    pub id: Uuid,
    pub route_key: String,
    pub version: i32,
    pub merchant_account: String,
    pub mint: String,
    pub mint_decimals: u8,
    pub tax_vault: String,
    pub net_destination: String,
    pub token_program: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CheckoutSession {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub calculation_id: Uuid,
    pub route_id: Uuid,
    pub status: String,
    pub expires_at_unix: i64,
    pub settlement_intent_id: Option<Uuid>,
}

#[derive(Clone, Debug)]
pub struct CreateCheckoutSessionCommand {
    pub merchant_id: Uuid,
    pub calculation_id: Uuid,
    pub route_id: Uuid,
    pub token_digest: [u8; 32],
    pub expires_at_unix: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RevenueStreamDestinationRoute {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub route_key: String,
    pub version: i32,
    pub recipient_owner: String,
    pub mint: String,
    pub destination_token_account: String,
    pub token_program: String,
    pub verification_digest: RevenueStreamDigest,
    pub verification_slot: u64,
    pub verification_commitment: String,
    pub verification_evidence: Value,
}

/// A bounded, business-facing selector for a finalized payout destination.
/// It intentionally omits mutable or unnecessary account topology from the
/// selection contract; the server resolves the complete route by `id`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RevenueStreamDestinationReference {
    pub id: Uuid,
    pub route_key: String,
    pub version: i32,
    pub recipient_owner: String,
    pub mint: String,
    pub token_program: String,
    #[serde(with = "policy::serde_u64_string")]
    pub verification_slot: u64,
}

/// A published, effective, canonical-validated tax policy available to a
/// revenue-plan proposer. The policy document remains server-owned.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RevenueStreamPolicyReference {
    pub id: Uuid,
    pub rule_set_id: Uuid,
    pub rule_external_id: String,
    pub rule_version: i32,
    pub jurisdiction: String,
    pub regime: String,
    #[serde(skip_serializing)]
    pub policy_digest: Digest32,
    pub effective_from: String,
    pub effective_until: Option<String>,
}

/// A bounded selector for an active settlement split. Recipient account
/// topology is intentionally kept out of the revenue-plan catalog response;
/// the API re-loads and validates the complete split by `id`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RevenueStreamSplitPlanReference {
    pub id: Uuid,
    pub plan_key: String,
    pub version: i32,
    pub mint: String,
    #[serde(skip_serializing)]
    pub plan_digest: Digest32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RefundFundingConfiguration {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub idempotency_key: String,
    pub request_digest: Digest32,
    pub configuration_digest: Digest32,
    pub configuration_key: String,
    pub version: i32,
    pub status: String,
    pub merchant_account: String,
    pub mint: String,
    pub tax_vault: String,
    pub vault_authority: String,
    pub refund_funder: String,
    pub refund_funder_subject: Option<String>,
    pub net_source: String,
    pub token_program: String,
    pub proposer_subject: String,
    pub approver_subject: Option<String>,
    pub verification_digest: Option<Digest32>,
    pub verification_slot: Option<String>,
    pub verification_commitment: Option<String>,
    pub verification_evidence: Option<Value>,
    pub verifier_version: Option<String>,
    pub verified_at: Option<String>,
    pub effective_from: String,
    pub effective_until: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CreateRefundFundingConfigurationCommand {
    pub merchant_id: Uuid,
    pub actor_subject: String,
    pub idempotency_key: String,
    pub configuration_key: String,
    pub version: i32,
    pub merchant_account: String,
    pub mint: String,
    pub tax_vault: String,
    pub vault_authority: String,
    pub refund_funder: String,
    pub refund_funder_subject: String,
    pub net_source: String,
    pub token_program: String,
    pub effective_from_unix: i64,
    pub effective_until_unix: Option<i64>,
}

#[derive(Clone, Debug)]
pub enum RefundFundingConfigurationOutcome {
    Created(RefundFundingConfiguration),
    Replay(RefundFundingConfiguration),
}

#[derive(Clone, Debug)]
pub struct RefundFundingConfigurationTransition {
    pub merchant_id: Uuid,
    pub configuration_id: Uuid,
    pub actor_subject: String,
    pub to_status: String,
    pub evidence: Value,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RefundFundingConfigurationTransitionResult {
    pub configuration_id: Uuid,
    pub from_status: String,
    pub to_status: String,
    pub approver_subject: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RefundFundingVerificationCandidate {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub version: i32,
    pub configuration_digest: Digest32,
    pub merchant_account: String,
    pub mint: String,
    pub tax_vault: String,
    pub vault_authority: String,
    pub refund_funder: String,
    pub refund_funder_subject: Option<String>,
    pub net_source: String,
    pub token_program: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RefundFundingVerificationMetricsSnapshot {
    pub pending: u64,
    pub leases_active: u64,
    pub leases_expired: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RefundClaimIntent {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub idempotency_key: String,
    pub request_digest: Digest32,
    pub adjustment_id: Uuid,
    pub funding_configuration_id: Uuid,
    pub receipt_signature: String,
    pub receipt_slot: u64,
    pub mint: [u8; 32],
    pub token_program: String,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub claim_nonce: u64,
    pub claim: [u8; 32],
    pub instruction_digest: Digest32,
    pub artifact_digest: Digest32,
    pub status: String,
    pub signed_transaction_digest: Option<Digest32>,
    pub signature: Option<String>,
    pub submitted_slot: Option<u64>,
    pub commitment: Option<String>,
    pub last_error_code: Option<String>,
    pub unsigned_artifact: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RefundClaimReconciliationCandidate {
    pub intent: RefundClaimIntent,
    pub artifact: RefundArtifact,
    pub claim_token: Uuid,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RefundExecuteIntent {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub idempotency_key: String,
    pub request_digest: Digest32,
    pub claim_intent_id: Uuid,
    pub funding_configuration_id: Uuid,
    pub funding_configuration_digest: Digest32,
    pub claim: [u8; 32],
    pub adjustment_id: Uuid,
    pub original_receipt: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub token_program: String,
    pub approver: String,
    pub refund_funder: String,
    pub refund_funder_subject: String,
    pub instruction_digest: Digest32,
    pub artifact_digest: Digest32,
    pub status: String,
    pub signed_transaction_digest: Option<Digest32>,
    pub signature: Option<String>,
    pub submitted_slot: Option<u64>,
    pub commitment: Option<String>,
    pub last_error_code: Option<String>,
    pub unsigned_artifact: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RefundCancellationIntent {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub idempotency_key: String,
    pub request_digest: Digest32,
    pub claim_intent_id: Uuid,
    pub adjustment_id: Uuid,
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub adjustment: [u8; 32],
    pub claim_nonce: u64,
    pub claim: [u8; 32],
    pub governance_authority: String,
    pub governance_subject: String,
    pub instruction_digest: Digest32,
    pub artifact_digest: Digest32,
    pub status: String,
    pub signed_transaction_digest: Option<Digest32>,
    pub signature: Option<String>,
    pub submitted_slot: Option<u64>,
    pub commitment: Option<String>,
    pub last_error_code: Option<String>,
    pub unsigned_artifact: Value,
}

#[derive(Clone, Debug)]
pub struct RefundClaimCancellationReconciliationCandidate {
    pub intent: RefundCancellationIntent,
    pub claim_token: Uuid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalizedSettlementAdjustmentBinding {
    pub adjustment: [u8; 32],
    pub original_receipt: [u8; 32],
    pub nonce: u64,
    pub signature: String,
    pub slot: u64,
    pub adjustment_digest: Digest32,
    pub evidence_digest: Digest32,
}

#[derive(Clone, Debug)]
pub struct CreateRefundClaimIntentCommand {
    pub merchant_id: Uuid,
    pub idempotency_key: String,
    pub request_digest: Digest32,
    pub inputs: RefundClaimServerInputs,
    pub artifact: RefundArtifact,
}

#[derive(Clone, Debug)]
pub struct RefundClaimIntentTransition {
    pub intent_id: Uuid,
    pub merchant_id: Uuid,
    pub expected_status: String,
    pub next_status: String,
    pub signed_transaction_digest: Option<Digest32>,
    pub signature: Option<String>,
    pub slot: Option<u64>,
    pub last_error_code: Option<String>,
    pub evidence: Value,
    /// When present, the transition must own the automatic reconciliation
    /// lease. API/operator transitions leave this unset and clear any stale
    /// worker lease as part of their compare-and-set.
    pub reconciliation_claim_token: Option<Uuid>,
}

#[derive(Clone, Debug)]
pub struct RefundClaimEvidenceExpectation {
    pub claim: [u8; 32],
    pub adjustment: [u8; 32],
    pub original_receipt: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub nonce: u64,
    pub gross_magnitude: u64,
    pub tax_magnitude: u64,
    pub net_magnitude: u64,
    pub refund_funder: [u8; 32],
    pub refund_destination: [u8; 32],
    pub proposer: [u8; 32],
    pub reference_digest: [u8; 32],
    pub execute_after_unix: i64,
    pub expires_at_unix: i64,
}

#[derive(Clone, Debug)]
pub struct RefundClaimCancellationEvidenceExpectation {
    pub claim: [u8; 32],
    pub adjustment: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub nonce: u64,
    pub governance_authority: [u8; 32],
    pub reference_digest: [u8; 32],
}

/// Derives the finalized claim-event expectation exclusively from the
/// immutable intent and artifact stored by the API. This is shared by the
/// authenticated operator endpoint and the automatic indexer worker so they
/// cannot evolve different authority or account-binding rules.
pub fn refund_claim_evidence_expectation(
    intent: &RefundClaimIntent,
    artifact: &RefundArtifact,
) -> Result<RefundClaimEvidenceExpectation, StorageError> {
    const CLAIM_INSTRUCTION_BYTES: usize = 8 + 8 + 32 + 8 + 8;
    let recomputed_instruction_digest = refund_instruction_digest(&artifact.instruction)
        .map_err(|error| StorageError::InvalidRefundClaimIntent(error.to_string()))?;
    if artifact.instruction_digest != intent.instruction_digest
        || recomputed_instruction_digest != intent.instruction_digest
        || artifact.artifact_digest != intent.artifact_digest
    {
        return Err(StorageError::InvalidRefundClaimIntent(
            "refund claim artifact digest is not bound to its durable intent".into(),
        ));
    }
    let instruction_data = BASE64_STANDARD
        .decode(&artifact.instruction.instruction_data_base64)
        .map_err(|_| {
            StorageError::InvalidRefundClaimIntent(
                "refund claim instruction data is not canonical base64".into(),
            )
        })?;
    if instruction_data.len() != CLAIM_INSTRUCTION_BYTES
        || instruction_data[8..16] != artifact.instruction.claim_nonce.to_le_bytes()
        || artifact.instruction.claim != Pubkey::new_from_array(intent.claim).to_string()
        || artifact.instruction.claim_nonce != intent.claim_nonce
    {
        return Err(StorageError::InvalidRefundClaimIntent(
            "refund claim instruction data does not match its durable identity".into(),
        ));
    }
    let reference_digest: [u8; 32] = instruction_data[16..48].try_into().map_err(|_| {
        StorageError::InvalidRefundClaimIntent("refund reference digest is malformed".into())
    })?;
    let execute_after_unix =
        i64::from_le_bytes(instruction_data[48..56].try_into().map_err(|_| {
            StorageError::InvalidRefundClaimIntent("refund execute window is malformed".into())
        })?);
    let expires_at_unix =
        i64::from_le_bytes(instruction_data[56..64].try_into().map_err(|_| {
            StorageError::InvalidRefundClaimIntent("refund expiry window is malformed".into())
        })?);
    let account_key = |index: usize, field: &str| -> Result<[u8; 32], StorageError> {
        let value = artifact
            .instruction
            .accounts
            .get(index)
            .map(|account| account.pubkey.as_str())
            .ok_or_else(|| {
                StorageError::InvalidRefundClaimIntent(format!(
                    "refund claim {field} account is missing"
                ))
            })?;
        let key = Pubkey::from_str(value).map_err(|_| {
            StorageError::InvalidRefundClaimIntent(format!(
                "refund claim {field} account is invalid"
            ))
        })?;
        if key == Pubkey::default() || key.to_string() != value {
            return Err(StorageError::InvalidRefundClaimIntent(format!(
                "refund claim {field} account is not canonical"
            )));
        }
        Ok(key.to_bytes())
    };
    Ok(RefundClaimEvidenceExpectation {
        claim: intent.claim,
        adjustment: account_key(7, "adjustment")?,
        original_receipt: account_key(8, "original receipt")?,
        merchant: account_key(2, "merchant")?,
        mint: account_key(3, "mint")?,
        nonce: intent.claim_nonce,
        gross_magnitude: intent.gross_atomic,
        tax_magnitude: intent.tax_atomic,
        net_magnitude: intent.net_atomic,
        refund_funder: account_key(1, "refund funder")?,
        refund_destination: account_key(12, "refund destination")?,
        proposer: account_key(0, "proposer")?,
        reference_digest,
        execute_after_unix,
        expires_at_unix,
    })
}

#[derive(Clone, Debug)]
pub struct CreateRefundExecuteIntentCommand {
    pub merchant_id: Uuid,
    pub idempotency_key: String,
    pub request_digest: Digest32,
    pub claim_intent_id: Uuid,
    pub inputs: RefundExecuteServerInputs,
    pub artifact: RefundArtifact,
}

#[derive(Clone, Debug)]
pub struct CreateRefundCancellationIntentCommand {
    pub merchant_id: Uuid,
    pub idempotency_key: String,
    pub request_digest: Digest32,
    pub inputs: RefundCancellationServerInputs,
    pub artifact: RefundCancellationArtifact,
}

#[derive(Clone, Debug)]
pub struct RefundExecuteIntentTransition {
    pub intent_id: Uuid,
    pub merchant_id: Uuid,
    pub expected_status: String,
    pub next_status: String,
    pub signed_transaction_digest: Option<Digest32>,
    pub signature: Option<String>,
    pub slot: Option<u64>,
    pub last_error_code: Option<String>,
    pub evidence: Value,
}

#[derive(Clone, Debug)]
pub struct RefundCancellationIntentTransition {
    pub intent_id: Uuid,
    pub merchant_id: Uuid,
    pub expected_status: String,
    pub next_status: String,
    pub signed_transaction_digest: Option<Digest32>,
    pub signature: Option<String>,
    pub slot: Option<u64>,
    pub last_error_code: Option<String>,
    pub evidence: Value,
}

#[derive(Clone, Debug)]
pub struct RefundCancellationFinalization {
    pub cancellation_intent_id: Uuid,
    pub claim_intent_id: Uuid,
    pub merchant_id: Uuid,
    pub expected_cancellation_status: String,
    pub signature: String,
    pub slot: u64,
    pub evidence: Value,
    pub reconciliation_claim_token: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SettlementSplitPlan {
    pub id: Uuid,
    pub plan_key: String,
    pub version: i32,
    pub merchant_account: String,
    pub mint: String,
    pub plan_digest: Digest32,
    pub recipients: Vec<split_plan::SplitRecipient>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SettlementRouteVerificationCandidate {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub route_key: String,
    pub version: i32,
    pub merchant_account: String,
    pub mint: String,
    pub tax_vault: String,
    pub net_destination: String,
    pub token_program: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SettlementRouteVerificationRunSummary {
    pub examined: usize,
    pub activated: usize,
    pub already_active: usize,
    pub failed: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SettlementRouteVerificationRunView {
    pub id: Uuid,
    pub run_kind: String,
    pub status: String,
    pub batch_size: u32,
    pub examined_count: u32,
    pub activated_count: u32,
    pub already_active_count: u32,
    pub failed_count: u32,
    pub failure_code: Option<String>,
    pub operator_approval_id: Option<Uuid>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RevenueStreamDestinationRouteVerificationCandidate {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub route_key: String,
    pub version: i32,
    pub recipient_owner: String,
    pub mint: String,
    pub destination_token_account: String,
    pub token_program: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VaultReconciliationTarget {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub route_id: Uuid,
    pub stream_key: String,
    pub mint: String,
    pub token_program: String,
    pub tax_vault: String,
    #[serde(with = "policy::serde_u64_string")]
    pub opening_balance_atomic: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub opening_balance_slot: u64,
    pub opening_reference: String,
    #[serde(with = "policy::serde_u64_string")]
    pub receipt_tax_atomic: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub release_atomic: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RefundBalanceProofTopology {
    pub claim: [u8; 32],
    pub adjustment: [u8; 32],
    pub original_receipt: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub token_program: [u8; 32],
    pub tax_vault: [u8; 32],
    pub refund_source: [u8; 32],
    pub payer_destination: [u8; 32],
    pub vault_authority: [u8; 32],
    pub refund_funder: [u8; 32],
    pub payer: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundBalanceProofCandidate {
    pub execution: IndexedAdjustmentRefundExecution,
    pub lease_token: Uuid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicabilityBindingVerificationCandidate {
    pub stream_key: String,
    pub signature: String,
    pub event_index: u16,
    pub evidence: RawChainEvidence,
    pub claim_token: Uuid,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApplicabilityBindingVerificationView {
    pub settlement_intent_id: Uuid,
    pub status: String,
    pub signature: Option<String>,
    pub event_index: Option<u16>,
    pub event_slot: Option<String>,
    pub provider_slot: Option<String>,
    pub receipt: Option<String>,
    pub binding: Option<String>,
    pub merchant: Option<String>,
    pub mint: Option<String>,
    pub context_id_hex: Option<String>,
    pub attestation_digest_hex: Option<String>,
    pub event_evidence_digest_hex: Option<String>,
    pub verification_commitment: Option<String>,
    pub attempts: Option<u16>,
    pub last_error_code: Option<String>,
    pub verified_at: Option<String>,
    pub claimed_at: Option<String>,
    pub dead_lettered_at: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VaultReconciliationMetricsSnapshot {
    pub mismatches: u64,
    pub errors: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VaultReconciliationRunView {
    pub id: Uuid,
    pub status: String,
    pub opening_balance_atomic: String,
    pub receipt_tax_atomic: String,
    pub release_atomic: String,
    pub expected_balance_atomic: Option<String>,
    pub observed_balance_atomic: Option<String>,
    pub observed_slot: Option<String>,
    pub error_code: Option<String>,
    pub created_at: String,
    pub stale: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VaultReconciliationTargetView {
    pub target: VaultReconciliationTarget,
    pub latest_run: Option<VaultReconciliationRunView>,
}

#[derive(Clone, Debug)]
pub struct VaultReconciliationRun {
    pub target_id: Uuid,
    pub route_id: Uuid,
    pub merchant_id: Uuid,
    pub stream_key: String,
    pub mint: String,
    pub token_program: String,
    pub tax_vault: String,
    pub expectation: VaultBalanceExpectation,
    pub expected_balance_atomic: Option<u64>,
    pub observed_balance_atomic: Option<u64>,
    pub observed_slot: Option<u64>,
    pub status: &'static str,
    pub error_code: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub enum QuoteOutcome {
    Created(StoredQuote),
    Replay(StoredQuote),
}

#[derive(Clone, Debug)]
pub enum DocumentQuoteOutcome {
    Created(StoredDocumentQuote),
    Replay(StoredDocumentQuote),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DocumentQuoteReplayVerification {
    pub calculation_id: Uuid,
    pub status: String,
    pub schema: String,
    pub engine_version: String,
    pub facts_digest: Digest32,
    pub rule_digest: Digest32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OutboxEvent {
    pub id: Uuid,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub payload: Value,
    pub attempt_count: i32,
    pub claim_token: Uuid,
}

#[derive(Clone, Debug)]
pub struct ReplayGapRecord {
    pub id: Uuid,
    pub work: ReplayWork,
    pub claim_token: Uuid,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReplayGapView {
    pub id: Uuid,
    pub stream_key: String,
    pub start_slot: u64,
    pub end_slot: u64,
    pub observed_finalized_slot: u64,
    pub reason: String,
    pub attempts: u32,
    pub status: String,
    pub last_error_code: Option<String>,
    pub created_at: String,
    pub dead_lettered_at: Option<String>,
    pub operator_acknowledged_at: Option<String>,
    pub operator_acknowledged_by: Option<String>,
    pub operator_acknowledgement: Option<String>,
}

/// Fixed-cardinality operational snapshot for one indexer stream.
///
/// This is telemetry only. It is intentionally separate from finalized chain
/// evidence and contains no user-controlled metric labels.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReplayGapMetricsSnapshot {
    pub queued: u64,
    pub leased: u64,
    pub dead_lettered: u64,
    pub leased_slots: u64,
}

/// Fixed-cardinality operational snapshot for server-owned refund proofs.
///
/// This is a backlog signal only. The indexed execution and immutable proof
/// rows remain the authoritative evidence records.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RefundProofMetricsSnapshot {
    pub pending: u64,
}

/// Fixed-cardinality operational snapshot for automatic refund-claim
/// finality reconciliation. This is a backlog signal only; finalized chain
/// event evidence remains authoritative.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RefundClaimReconciliationMetricsSnapshot {
    pub pending: u64,
}

/// Fixed-cardinality operational snapshot for automatic refund-claim
/// cancellation finality reconciliation. Finalized chain evidence remains
/// authoritative; this is only a backlog signal.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RefundClaimCancellationReconciliationMetricsSnapshot {
    pub pending: u64,
}

/// Fixed-cardinality signal for targeted-recovery attempts interrupted before
/// their terminal audit row was written.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TargetedRecoveryMetricsSnapshot {
    pub stale_requested: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolPauseMetricsSnapshot {
    pub known: bool,
    pub paused: bool,
    pub slot: u64,
}

#[derive(Clone, Debug)]
pub struct TargetedRecoveryAuditCommand {
    pub request_id: Uuid,
    pub stream_key: String,
    pub target_address: String,
    pub target_signature: String,
    pub operator_subject: String,
    pub note: String,
    pub outcome: String,
    pub pages_scanned: Option<u32>,
    pub slot: Option<u64>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettlementIntent {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub idempotency_key: String,
    pub request_digest: [u8; 32],
    pub instruction_digest: [u8; 32],
    pub status: String,
    pub signed_transaction_digest: Option<[u8; 32]>,
    pub signature: Option<String>,
    pub slot: Option<u64>,
    pub commitment: Option<String>,
    pub last_error_code: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DocumentPaymentAuthorizationRecord {
    pub id: Uuid,
    pub intent_id: Uuid,
    pub merchant_id: Uuid,
    pub calculation_id: Uuid,
    pub route_id: Uuid,
    pub mint: String,
    pub source_attestation_digest: Digest32,
    pub payment_scope_digest: Digest32,
    pub scope: PaymentScope,
    #[serde(with = "policy::serde_u64_string")]
    pub gross_atomic: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub tax_atomic: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub net_atomic: u64,
}

#[derive(Clone, Debug)]
pub struct CreateDocumentPaymentIntentCommand {
    pub merchant_id: Uuid,
    pub idempotency_key: String,
    pub request_digest: Digest32,
    pub instruction_digest: Digest32,
    pub calculation_id: Uuid,
    pub route_id: Uuid,
    pub mint: String,
    pub authorization: DocumentPaymentAuthorization,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentPaymentIntent {
    pub intent: SettlementIntent,
    pub authorization: DocumentPaymentAuthorizationRecord,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentPaymentIntentOutcome {
    Created(DocumentPaymentIntent),
    Replay(DocumentPaymentIntent),
}

/// Server-owned identity and financial evidence for an adjustment's original
/// settlement. This is assembled only from a tenant-scoped finalized intent
/// and the canonical finalized receipt projection; it is never accepted from
/// an API caller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettlementReceiptBinding {
    pub settlement_id: Uuid,
    pub signature: String,
    pub slot: u64,
    pub receipt: [u8; 32],
    pub payer: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub nonce: u64,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub policy_digest: [u8; 32],
    pub evidence_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdjustmentAuthorityChallenge {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub subject: String,
    pub role: String,
    pub message: Vec<u8>,
    pub message_digest: [u8; 32],
    pub expires_at_unix: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdjustmentAuthorityBinding {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub subject: String,
    pub role: String,
    pub public_key: [u8; 32],
    pub challenge_id: Uuid,
    pub proof_signature: [u8; 64],
}

#[derive(Clone, Debug)]
pub struct CreateAdjustmentAuthorityChallenge {
    pub merchant_id: Uuid,
    pub subject: String,
    pub role: String,
    pub now_unix: i64,
    pub ttl_seconds: i64,
}

#[derive(Clone, Debug)]
pub struct BindAdjustmentAuthority {
    pub merchant_id: Uuid,
    pub subject: String,
    pub role: String,
    pub challenge_id: Uuid,
    pub public_key: [u8; 32],
    pub proof_signature: [u8; 64],
    pub now_unix: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettlementAdjustmentEvidenceIntent {
    pub id: Uuid,
    pub adjustment_id: Uuid,
    pub merchant_id: Uuid,
    pub nonce: u64,
    pub proposer: [u8; 32],
    pub approver: [u8; 32],
    pub original_receipt: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub token_program: String,
    pub registry_address: [u8; 32],
    pub registry_slot: u64,
    pub registry_commitment: String,
    pub registry_account_digest: [u8; 32],
    pub request_digest: [u8; 32],
    pub instruction_digest: Option<[u8; 32]>,
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct CreateSettlementAdjustmentEvidenceIntent {
    pub adjustment_id: Uuid,
    pub merchant_id: Uuid,
    pub proposer: [u8; 32],
    pub approver: [u8; 32],
    pub original_receipt: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub token_program: String,
    pub registry_address: [u8; 32],
    pub registry_slot: u64,
    pub registry_commitment: String,
    pub registry_account_digest: [u8; 32],
    pub request_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettlementAdjustmentChainIntent {
    pub id: Uuid,
    pub evidence_intent_id: Uuid,
    pub adjustment_id: Uuid,
    pub merchant_id: Uuid,
    pub instruction_digest: [u8; 32],
    pub status: String,
    pub signed_transaction_digest: Option<[u8; 32]>,
    pub signature: Option<String>,
    pub slot: Option<u64>,
    pub commitment: Option<String>,
    pub last_error_code: Option<String>,
    pub finalized_event_index: Option<u16>,
    pub finalized_event_evidence_digest: Option<[u8; 32]>,
}

#[derive(Clone, Debug)]
pub struct CreateSettlementAdjustmentChainIntent {
    pub evidence_intent_id: Uuid,
    pub adjustment_id: Uuid,
    pub merchant_id: Uuid,
    pub instruction_digest: [u8; 32],
}

#[derive(Clone, Debug)]
pub struct SettlementAdjustmentChainTransition {
    pub intent_id: Uuid,
    pub merchant_id: Uuid,
    pub expected_status: String,
    pub next_status: String,
    pub signed_transaction_digest: Option<[u8; 32]>,
    pub signature: Option<String>,
    pub slot: Option<u64>,
    pub last_error_code: Option<String>,
    pub finalized_event_index: Option<u16>,
    pub finalized_event_evidence_digest: Option<[u8; 32]>,
    pub evidence: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SettlementIntentOutcome {
    Created(SettlementIntent),
    Replay(SettlementIntent),
}

#[derive(Clone, Debug)]
pub struct SettlementTransition {
    pub intent_id: Uuid,
    pub merchant_id: Uuid,
    pub expected_status: String,
    pub next_status: String,
    pub signed_transaction_digest: Option<[u8; 32]>,
    pub signature: Option<String>,
    pub slot: Option<u64>,
    pub last_error_code: Option<String>,
    pub evidence: Value,
}

impl PgStore {
    pub fn from_url(
        database_url: &str,
        pool_size: usize,
        pool_wait_timeout: Duration,
        engine_version: impl Into<String>,
    ) -> Result<Self, StorageError> {
        Self::from_url_with_evidence_cipher(
            database_url,
            pool_size,
            pool_wait_timeout,
            engine_version,
            None,
        )
    }

    pub fn from_url_with_evidence_cipher(
        database_url: &str,
        pool_size: usize,
        pool_wait_timeout: Duration,
        engine_version: impl Into<String>,
        evidence_cipher: Option<EvidenceCipher>,
    ) -> Result<Self, StorageError> {
        if database_url.trim().is_empty() {
            return Err(StorageError::InvalidConfiguration(
                "database URL must not be empty".to_string(),
            ));
        }
        if pool_size == 0 {
            return Err(StorageError::InvalidConfiguration(
                "database pool size must be positive".to_string(),
            ));
        }

        let mut config = PoolConfig::new();
        config.url = Some(database_url.to_string());
        config.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Verified,
        });
        let pool = config
            .builder(NoTls)
            .map_err(|error| StorageError::InvalidConfiguration(error.to_string()))?
            .runtime(Runtime::Tokio1)
            .max_size(pool_size)
            .wait_timeout(Some(pool_wait_timeout))
            .create_timeout(Some(pool_wait_timeout))
            .recycle_timeout(Some(pool_wait_timeout))
            .queue_mode(deadpool_postgres::QueueMode::Fifo)
            .build()
            .map_err(|error| StorageError::InvalidConfiguration(error.to_string()))?;
        Ok(Self {
            pool,
            engine_version: engine_version.into(),
            compiled_rule_cache: CompiledRuleCache::new(DEFAULT_COMPILED_RULE_CACHE_CAPACITY)?,
            evidence_cipher,
        })
    }

    pub fn rule_cache_stats(&self) -> RuleCacheStats {
        self.compiled_rule_cache.stats()
    }

    /// Returns an eventually consistent, connection-free pool snapshot for
    /// bounded operational telemetry. It must never be used as settlement or
    /// authorization truth.
    pub fn database_pool_stats(&self) -> DatabasePoolStats {
        let status = self.pool.status();
        DatabasePoolStats {
            max_size: status.max_size,
            size: status.size,
            available: status.available,
            waiting: status.waiting,
        }
    }

    pub async fn ping(&self) -> Result<(), StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        client.query_one("SELECT 1", &[]).await?;
        Ok(())
    }

    pub async fn claim_outbox(
        &self,
        batch_size: usize,
        lease: Duration,
    ) -> Result<Vec<OutboxEvent>, StorageError> {
        validate_outbox_options(batch_size, lease)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let claim_token = Uuid::new_v4();
        let lease_millis = i64::try_from(lease.as_millis()).map_err(|_| {
            StorageError::InvalidConfiguration("outbox lease duration is too large".to_string())
        })?;
        let batch_size = i64::try_from(batch_size).map_err(|_| {
            StorageError::InvalidConfiguration(
                "outbox batch size exceeds database range".to_string(),
            )
        })?;
        let rows = transaction
            .query(
                "WITH candidates AS (
                    SELECT id
                    FROM outbox_events
                    WHERE published_at IS NULL
                      AND dead_lettered_at IS NULL
                      AND attempt_count < $1
                      AND (claimed_at IS NULL
                           OR claimed_at < now() - ($2::bigint * interval '1 millisecond'))
                    ORDER BY occurred_at, id
                    FOR UPDATE SKIP LOCKED
                    LIMIT $3
                 )
                 UPDATE outbox_events AS events
                 SET claim_token = $4,
                     claimed_at = now(),
                     attempt_count = events.attempt_count + 1
                 FROM candidates
                 WHERE events.id = candidates.id
                 RETURNING events.id, events.aggregate_type, events.aggregate_id,
                           events.event_type, events.payload, events.attempt_count",
                &[
                    &MAX_OUTBOX_ATTEMPTS,
                    &lease_millis,
                    &batch_size,
                    &claim_token,
                ],
            )
            .await?;
        let events = rows
            .into_iter()
            .map(|row| OutboxEvent {
                id: row.get(0),
                aggregate_type: row.get(1),
                aggregate_id: row.get(2),
                event_type: row.get(3),
                payload: row.get(4),
                attempt_count: row.get(5),
                claim_token,
            })
            .collect();
        transaction.commit().await?;
        Ok(events)
    }

    pub async fn mark_outbox_published(
        &self,
        event_id: Uuid,
        claim_token: Uuid,
    ) -> Result<bool, StorageError> {
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let updated = transaction
            .execute(
                "UPDATE outbox_events
                 SET published_at = now(), claimed_at = NULL, claim_token = NULL
                 WHERE id = $1 AND claim_token = $2 AND published_at IS NULL",
                &[&event_id, &claim_token],
            )
            .await?;
        transaction.commit().await?;
        Ok(updated == 1)
    }

    pub async fn record_outbox_failure(
        &self,
        event_id: Uuid,
        claim_token: Uuid,
        error_code: &str,
    ) -> Result<bool, StorageError> {
        if error_code.trim().is_empty() || error_code.len() > 128 {
            return Err(StorageError::InvalidConfiguration(
                "outbox error code must contain 1 to 128 bytes".to_string(),
            ));
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let updated = transaction
            .execute(
                "UPDATE outbox_events
                 SET claimed_at = NULL,
                     claim_token = NULL,
                     last_error_code = $3,
                     dead_lettered_at = CASE
                         WHEN attempt_count >= $4 THEN now()
                         ELSE dead_lettered_at
                     END
                 WHERE id = $1 AND claim_token = $2 AND published_at IS NULL",
                &[&event_id, &claim_token, &error_code, &MAX_OUTBOX_ATTEMPTS],
            )
            .await?;
        transaction.commit().await?;
        Ok(updated == 1)
    }

    pub async fn enqueue_replay_gap(
        &self,
        stream_key: &str,
        gap: ReplayGap,
    ) -> Result<Uuid, StorageError> {
        validate_indexer_stream_key(stream_key)?;
        validate_replay_gap(&gap)?;
        let reason = replay_reason_to_db(gap.reason);
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "INSERT INTO indexer_replay_gaps
                    (stream_key, start_slot, end_slot, observed_finalized_slot,
                     reason, attempts)
                 VALUES ($1, $2::text::numeric, $3::text::numeric,
                         $4::text::numeric, $5, $6)
                 ON CONFLICT (stream_key, start_slot, end_slot, reason)
                     WHERE status IN ('queued', 'leased') DO NOTHING
                 RETURNING id",
                &[
                    &stream_key,
                    &gap.range.start_slot.to_string(),
                    &gap.range.end_slot.to_string(),
                    &gap.observed_finalized_slot.to_string(),
                    &reason,
                    &(i16::try_from(gap.attempts).map_err(|_| {
                        StorageError::InvalidReplayState("attempt count exceeds smallint".into())
                    })?),
                ],
            )
            .await?;
        if let Some(row) = row {
            return Ok(row.get(0));
        }
        client
            .query_one(
                "SELECT id
                 FROM indexer_replay_gaps
                 WHERE stream_key = $1 AND start_slot = $2::text::numeric
                   AND end_slot = $3::text::numeric AND reason = $4
                   AND status IN ('queued', 'leased')",
                &[
                    &stream_key,
                    &gap.range.start_slot.to_string(),
                    &gap.range.end_slot.to_string(),
                    &reason,
                ],
            )
            .await
            .map(|row| row.get(0))
            .map_err(StorageError::from)
    }

    pub async fn claim_replay_gaps(
        &self,
        stream_key: &str,
        batch_size: usize,
        lease: Duration,
    ) -> Result<Vec<ReplayGapRecord>, StorageError> {
        validate_indexer_stream_key(stream_key)?;
        validate_replay_options(batch_size, lease)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let claim_token = Uuid::new_v4();
        let lease_millis = i64::try_from(lease.as_millis()).map_err(|_| {
            StorageError::InvalidConfiguration("replay lease duration is too large".to_string())
        })?;
        let batch_size = i64::try_from(batch_size).map_err(|_| {
            StorageError::InvalidConfiguration(
                "replay batch size exceeds database range".to_string(),
            )
        })?;
        let max_attempts = i16::try_from(MAX_REPLAY_ATTEMPTS)
            .map_err(|_| StorageError::InvalidReplayState("retry limit exceeds smallint".into()))?;
        let rows = transaction
            .query(
                "WITH candidates AS (
                    SELECT id
                    FROM indexer_replay_gaps
                    WHERE stream_key = $1
                      AND ((status = 'queued' AND attempts < $2)
                           OR (status = 'leased' AND attempts <= $2
                               AND claimed_at < now() - ($3::bigint * interval '1 millisecond')))
                    ORDER BY created_at, id
                    FOR UPDATE SKIP LOCKED
                    LIMIT $4
                 )
                 UPDATE indexer_replay_gaps AS gaps
                 SET status = 'leased',
                     claim_token = $5,
                     claimed_at = now(),
                     attempts = LEAST(gaps.attempts + 1, $2),
                     updated_at = now()
                 FROM candidates
                 WHERE gaps.id = candidates.id
                 RETURNING gaps.id, gaps.start_slot::text, gaps.end_slot::text,
                           gaps.observed_finalized_slot::text, gaps.reason,
                           gaps.attempts::text",
                &[
                    &stream_key,
                    &max_attempts,
                    &lease_millis,
                    &batch_size,
                    &claim_token,
                ],
            )
            .await?;
        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            records.push(parse_replay_gap_row(row, claim_token)?);
        }
        transaction.commit().await?;
        Ok(records)
    }

    pub async fn acknowledge_replay_gap(
        &self,
        gap_id: Uuid,
        claim_token: Uuid,
    ) -> Result<bool, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let updated = client
            .execute(
                "UPDATE indexer_replay_gaps
                 SET status = 'completed', claim_token = NULL, claimed_at = NULL,
                     completed_at = now(), updated_at = now()
                 WHERE id = $1 AND claim_token = $2 AND status = 'leased'",
                &[&gap_id, &claim_token],
            )
            .await?;
        Ok(updated == 1)
    }

    /// Enqueue only the bounded applicability event families. The raw event
    /// is written in the same transaction, so a committed job can never point
    /// at an event that is not available to the verifier.
    async fn enqueue_applicability_binding_verification_jobs(
        transaction: &deadpool_postgres::tokio_postgres::Transaction<'_>,
        stream_key: &str,
        page: &FinalizedPage,
    ) -> Result<(), StorageError> {
        for evidence in &page.raw_evidence {
            if evidence.event_discriminator != indexer::APPLICABILITY_SETTLEMENT_EVENT_DISCRIMINATOR
                && evidence.event_discriminator
                    != indexer::APPLICABILITY_DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR
            {
                continue;
            }
            transaction
                .execute(
                    "INSERT INTO applicability_binding_verification_jobs
                        (stream_key, signature, event_index)
                     VALUES ($1, $2, $3::smallint)
                     ON CONFLICT (stream_key, signature, event_index) DO NOTHING",
                    &[
                        &stream_key,
                        &evidence.signature,
                        &(evidence.event_index as i16),
                    ],
                )
                .await?;
        }
        Ok(())
    }

    /// Persist a validated replay page without moving the forward checkpoint.
    ///
    /// Replay repairs are projection work: they may cover a historical range
    /// that is already behind the canonical cursor. The transaction therefore
    /// applies the same idempotent receipt/evidence merge rules as forward
    /// indexing but deliberately has no checkpoint write.
    pub async fn persist_replayed_page(
        &self,
        stream_key: &str,
        page: &FinalizedPage,
    ) -> Result<(), StorageError> {
        validate_indexer_stream_key(stream_key)?;
        page.range.validate()?;
        page.validate(page.range)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;

        for receipt in &page.receipts {
            let existing = transaction
                .query_opt(
                    "SELECT signature, slot::text, gross_atomic::text, tax_atomic::text,
                            net_atomic::text, evidence_digest, receipt, payer,
                            merchant, mint, nonce::text, policy_digest
                     FROM indexed_receipts
                     WHERE stream_key = $1 AND signature = $2
                     FOR UPDATE",
                    &[&stream_key, &receipt.signature],
                )
                .await?;
            if let Some(row) = existing {
                let existing_receipt = parse_receipt_row(row)?;
                merge_receipt(Some(existing_receipt), receipt.clone())?;
                continue;
            }
            transaction
                .execute(
                    "INSERT INTO indexed_receipts
                        (stream_key, signature, slot, gross_atomic, tax_atomic, net_atomic,
                         evidence_digest, receipt, payer, merchant, mint, nonce, policy_digest)
                     VALUES ($1, $2, $3::text::numeric, $4::text::numeric,
                             $5::text::numeric, $6::text::numeric, $7, $8, $9, $10, $11,
                             $12::text::numeric, $13)",
                    &[
                        &stream_key,
                        &receipt.signature,
                        &receipt.slot.to_string(),
                        &receipt.gross_atomic.to_string(),
                        &receipt.tax_atomic.to_string(),
                        &receipt.net_atomic.to_string(),
                        &&receipt.evidence_digest[..],
                        &&receipt.receipt[..],
                        &&receipt.payer[..],
                        &&receipt.merchant[..],
                        &&receipt.mint[..],
                        &receipt.nonce.to_string(),
                        &&receipt.policy_digest[..],
                    ],
                )
                .await?;
        }
        for split in &page.split_settlements {
            let existing = transaction
                .query_opt(
                    "SELECT signature, slot::text, receipt, payer, merchant, mint,
                            nonce::text, gross_atomic::text, tax_atomic::text,
                            net_atomic::text, split_plan_version::text,
                            split_plan_digest, policy_digest, evidence_digest
                     FROM indexed_split_settlements
                     WHERE stream_key = $1 AND signature = $2
                     FOR UPDATE",
                    &[&stream_key, &split.signature],
                )
                .await?;
            if let Some(row) = existing {
                let existing_split = parse_split_settlement_row(row)?;
                if existing_split != *split {
                    return Err(StorageError::Indexer(
                        indexer::IndexerError::ConflictingEvidence,
                    ));
                }
                continue;
            }
            transaction
                .execute(
                    "INSERT INTO indexed_split_settlements
                        (stream_key, signature, slot, receipt, payer, merchant, mint,
                         nonce, gross_atomic, tax_atomic, net_atomic, split_plan_version,
                         split_plan_digest, policy_digest, evidence_digest)
                     VALUES ($1, $2, $3::text::numeric, $4, $5, $6, $7,
                             $8::text::numeric, $9::text::numeric, $10::text::numeric,
                             $11::text::numeric, $12::text::numeric, $13, $14, $15)",
                    &[
                        &stream_key,
                        &split.signature,
                        &split.slot.to_string(),
                        &&split.receipt[..],
                        &&split.payer[..],
                        &&split.merchant[..],
                        &&split.mint[..],
                        &split.nonce.to_string(),
                        &split.gross_atomic.to_string(),
                        &split.tax_atomic.to_string(),
                        &split.net_atomic.to_string(),
                        &split.split_plan_version.to_string(),
                        &&split.split_plan_digest[..],
                        &&split.policy_digest[..],
                        &&split.evidence_digest[..],
                    ],
                )
                .await?;
        }
        persist_revenue_stream_accrual_jobs(&transaction, stream_key, page).await?;
        persist_indexed_adjustment_refund_executions(
            &transaction,
            stream_key,
            &page.refund_executions,
        )
        .await?;
        persist_indexed_settlement_adjustments(&transaction, stream_key, &page.adjustment_records)
            .await?;
        for evidence in &page.raw_evidence {
            merge_raw_evidence(None, evidence.clone())?;
            let existing = transaction
                .query_opt(
                    "SELECT signature, slot::text, event_index::text,
                            event_discriminator, event_data, evidence_digest
                     FROM raw_chain_evidence
                     WHERE stream_key = $1 AND signature = $2 AND event_index = $3::text::smallint
                     FOR UPDATE",
                    &[
                        &stream_key,
                        &evidence.signature,
                        &evidence.event_index.to_string(),
                    ],
                )
                .await?;
            if let Some(row) = existing {
                let existing_evidence = parse_raw_evidence_row(row)?;
                merge_raw_evidence(Some(existing_evidence), evidence.clone())?;
                continue;
            }
            transaction
                .execute(
                    "INSERT INTO raw_chain_evidence
                        (stream_key, signature, slot, event_index, event_discriminator,
                         event_data, evidence_digest)
                     VALUES ($1, $2, $3::text::numeric, $4::text::smallint, $5, $6, $7)",
                    &[
                        &stream_key,
                        &evidence.signature,
                        &evidence.slot.to_string(),
                        &evidence.event_index.to_string(),
                        &&evidence.event_discriminator[..],
                        &evidence.event_data,
                        &&evidence.evidence_digest[..],
                    ],
                )
                .await?;
        }
        for transaction_evidence in &page.raw_transactions {
            transaction_evidence.validate(page.range, page.observed_finalized_slot)?;
            let existing = transaction
                .query_opt(
                    "SELECT signature, slot::text, transaction_index::text,
                            payload, payload_digest
                     FROM raw_chain_transactions
                     WHERE stream_key = $1 AND signature = $2
                       AND transaction_index = $3::text::numeric
                     FOR UPDATE",
                    &[
                        &stream_key,
                        &transaction_evidence.signature,
                        &transaction_evidence.transaction_index.to_string(),
                    ],
                )
                .await?;
            if let Some(row) = existing {
                let existing_transaction = parse_raw_transaction_row(row)?;
                if existing_transaction != *transaction_evidence {
                    return Err(StorageError::Indexer(
                        indexer::IndexerError::ConflictingEvidence,
                    ));
                }
                continue;
            }
            transaction
                .execute(
                    "INSERT INTO raw_chain_transactions
                        (stream_key, signature, slot, transaction_index, payload, payload_digest)
                     VALUES ($1, $2, $3::text::numeric, $4::text::numeric, $5, $6)",
                    &[
                        &stream_key,
                        &transaction_evidence.signature,
                        &transaction_evidence.slot.to_string(),
                        &transaction_evidence.transaction_index.to_string(),
                        &transaction_evidence.payload,
                        &&transaction_evidence.payload_digest[..],
                    ],
                )
                .await?;
        }
        Self::enqueue_applicability_binding_verification_jobs(&transaction, stream_key, page)
            .await?;
        persist_protocol_pause_events(
            &transaction,
            stream_key,
            &page.protocol_pause_changes,
            page.range,
            page.observed_finalized_slot,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn complete_applicability_binding_job_in_transaction(
        transaction: &deadpool_postgres::tokio_postgres::Transaction<'_>,
        stream_key: &str,
        signature: &str,
        event_index: u16,
        claim_token: Uuid,
    ) -> Result<(), StorageError> {
        let updated = transaction
            .execute(
                "UPDATE applicability_binding_verification_jobs
                 SET completed_at = now(), claim_token = NULL, claimed_at = NULL,
                     last_error_code = NULL, updated_at = now()
                 WHERE stream_key = $1 AND signature = $2
                   AND event_index = $3::smallint AND claim_token = $4
                   AND completed_at IS NULL AND dead_lettered_at IS NULL",
                &[&stream_key, &signature, &(event_index as i16), &claim_token],
            )
            .await?;
        if updated != 1 {
            return Err(StorageError::InvalidApplicabilityBindingVerification(
                "applicability verification lease was lost before completion".to_string(),
            ));
        }
        Ok(())
    }

    /// Claims one raw applicability event without holding a database lock
    /// across the finalized account RPC. The returned lease token fences both
    /// successful completion and bounded failure updates.
    pub async fn claim_applicability_binding_verification_candidate(
        &self,
        stream_key: &str,
    ) -> Result<Option<ApplicabilityBindingVerificationCandidate>, StorageError> {
        validate_indexer_stream_key(stream_key)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT raw.signature, raw.slot::text, raw.event_index::text,
                        raw.event_discriminator, raw.event_data, raw.evidence_digest
                 FROM applicability_binding_verification_jobs AS jobs
                 JOIN raw_chain_evidence AS raw
                   ON raw.stream_key = jobs.stream_key
                  AND raw.signature = jobs.signature
                  AND raw.event_index = jobs.event_index
                 WHERE jobs.stream_key = $1
                   AND jobs.completed_at IS NULL
                   AND jobs.dead_lettered_at IS NULL
                   AND jobs.attempts < $3::smallint
                   AND (jobs.claim_token IS NULL
                        OR jobs.claimed_at < now() - ($2::bigint * interval '1 second'))
                   AND NOT EXISTS (
                       SELECT 1
                       FROM applicability_binding_verifications AS verified
                       WHERE verified.stream_key = jobs.stream_key
                         AND verified.signature = jobs.signature
                         AND verified.event_index = jobs.event_index
                   )
                 ORDER BY jobs.created_at, jobs.id
                 FOR UPDATE OF jobs SKIP LOCKED
                 LIMIT 1",
                &[
                    &stream_key,
                    &(APPLICABILITY_BINDING_VERIFICATION_LEASE.as_secs() as i64),
                    &MAX_APPLICABILITY_BINDING_ATTEMPTS,
                ],
            )
            .await?;
        let Some(row) = row else {
            transaction.commit().await?;
            return Ok(None);
        };
        let evidence = parse_raw_evidence_row(row)?;
        if evidence.event_discriminator != indexer::APPLICABILITY_SETTLEMENT_EVENT_DISCRIMINATOR
            && evidence.event_discriminator
                != indexer::APPLICABILITY_DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR
        {
            return Err(StorageError::InvalidApplicabilityBindingVerification(
                "verification job references a non-applicability event".to_string(),
            ));
        }
        evidence.validate(
            ScanRange {
                start_slot: evidence.slot,
                end_slot: evidence.slot,
            },
            evidence.slot,
        )?;
        let claim_token = Uuid::now_v7();
        let updated = transaction
            .execute(
                "UPDATE applicability_binding_verification_jobs
                 SET claim_token = $4, claimed_at = now(), attempts = attempts + 1,
                     updated_at = now()
                 WHERE stream_key = $1 AND signature = $2
                   AND event_index = $3::smallint
                   AND completed_at IS NULL AND dead_lettered_at IS NULL
                   AND attempts < $5::smallint
                   AND (claim_token IS NULL
                        OR claimed_at < now() - ($6::bigint * interval '1 second'))",
                &[
                    &stream_key,
                    &evidence.signature,
                    &(evidence.event_index as i16),
                    &claim_token,
                    &MAX_APPLICABILITY_BINDING_ATTEMPTS,
                    &(APPLICABILITY_BINDING_VERIFICATION_LEASE.as_secs() as i64),
                ],
            )
            .await?;
        if updated != 1 {
            transaction.rollback().await?;
            return Ok(None);
        }
        transaction.commit().await?;
        Ok(Some(ApplicabilityBindingVerificationCandidate {
            stream_key: stream_key.to_string(),
            signature: evidence.signature.clone(),
            event_index: evidence.event_index,
            evidence,
            claim_token,
        }))
    }

    /// Commits the typed finalized projection and clears the lease in one
    /// serializable transaction. A stale worker cannot change job state.
    pub async fn complete_applicability_binding_verification_candidate(
        &self,
        candidate: &ApplicabilityBindingVerificationCandidate,
        verification: &FinalizedApplicabilityBinding,
    ) -> Result<(), StorageError> {
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        Self::persist_finalized_applicability_binding_in_transaction(
            &transaction,
            &candidate.stream_key,
            &candidate.signature,
            candidate.event_index,
            verification,
            Some(candidate.claim_token),
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// Releases a failed lease for bounded retry and records explicit
    /// dead-letter evidence after the attempt budget is exhausted.
    pub async fn fail_applicability_binding_verification_candidate(
        &self,
        candidate: &ApplicabilityBindingVerificationCandidate,
        error_code: &str,
    ) -> Result<(), StorageError> {
        if error_code.is_empty() || error_code.len() > MAX_APPLICABILITY_BINDING_ERROR_BYTES {
            return Err(StorageError::InvalidApplicabilityBindingVerification(
                "verification error code is outside the bounded domain".to_string(),
            ));
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let updated = transaction
            .execute(
                "UPDATE applicability_binding_verification_jobs
                 SET claim_token = NULL, claimed_at = NULL, last_error_code = $5,
                     dead_lettered_at = CASE
                         WHEN attempts >= $6::smallint THEN now()
                         ELSE NULL
                     END,
                     updated_at = now()
                 WHERE stream_key = $1 AND signature = $2
                   AND event_index = $3::smallint AND claim_token = $4
                   AND completed_at IS NULL AND dead_lettered_at IS NULL",
                &[
                    &candidate.stream_key,
                    &candidate.signature,
                    &(candidate.event_index as i16),
                    &candidate.claim_token,
                    &error_code,
                    &MAX_APPLICABILITY_BINDING_ATTEMPTS,
                ],
            )
            .await?;
        if updated != 1 {
            transaction.rollback().await?;
            return Err(StorageError::InvalidApplicabilityBindingVerification(
                "applicability verification lease was lost before failure update".to_string(),
            ));
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Persist one finalized applicability-account verification only after its
    /// matching raw event is present. The serializable transaction makes the
    /// verification projection idempotent while rejecting any substitution of
    /// account, event, digest, or provider-slot evidence.
    pub async fn persist_finalized_applicability_binding(
        &self,
        stream_key: &str,
        signature: &str,
        event_index: u16,
        verification: &FinalizedApplicabilityBinding,
    ) -> Result<(), StorageError> {
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        Self::persist_finalized_applicability_binding_in_transaction(
            &transaction,
            stream_key,
            signature,
            event_index,
            verification,
            None,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn persist_finalized_applicability_binding_in_transaction(
        transaction: &deadpool_postgres::tokio_postgres::Transaction<'_>,
        stream_key: &str,
        signature: &str,
        event_index: u16,
        verification: &FinalizedApplicabilityBinding,
        claim_token: Option<Uuid>,
    ) -> Result<(), StorageError> {
        validate_indexer_stream_key(stream_key)?;
        if signature.trim().is_empty() || signature.len() > 128 || event_index > i16::MAX as u16 {
            return Err(StorageError::InvalidApplicabilityBindingVerification(
                "signature or event index is outside the bounded domain".to_string(),
            ));
        }
        let receipt = Pubkey::new_from_array(verification.receipt);
        let (expected_binding, expected_bump) = Pubkey::find_program_address(
            &[b"applicability-binding", receipt.as_ref()],
            &settlement_client::settlement_program_id(),
        );
        if expected_binding.to_bytes() != verification.binding || expected_bump != verification.bump
        {
            return Err(StorageError::InvalidApplicabilityBindingVerification(
                "verification binding PDA or bump does not match the receipt".to_string(),
            ));
        }
        if let Some(token) = claim_token
            && token.get_version_num() != 7
        {
            return Err(StorageError::InvalidApplicabilityBindingVerification(
                "applicability verification claim token must be UUIDv7".to_string(),
            ));
        }

        let evidence_row = transaction
            .query_opt(
                "SELECT signature, slot::text, event_index::text,
                        event_discriminator, event_data, evidence_digest
                 FROM raw_chain_evidence
                 WHERE stream_key = $1 AND signature = $2
                   AND event_index = $3::text::smallint
                 FOR UPDATE",
                &[&stream_key, &signature, &(event_index as i16)],
            )
            .await?
            .ok_or_else(|| {
                StorageError::InvalidApplicabilityBindingVerification(
                    "matching raw applicability event is not persisted".to_string(),
                )
            })?;
        let evidence = parse_raw_evidence_row(evidence_row)?;
        evidence.validate(
            ScanRange {
                start_slot: evidence.slot,
                end_slot: evidence.slot,
            },
            verification.slot,
        )?;
        let event = ApplicabilitySettlementExecutedEvent::decode(&evidence.event_data).map_err(
            |error| StorageError::InvalidApplicabilityBindingVerification(error.to_string()),
        )?;
        event.validate_binding(verification)?;

        let existing = transaction
            .query_opt(
                "SELECT event_slot::text, provider_slot::text, receipt, binding,
                        merchant, mint, context_id, attestation_digest,
                        event_evidence_digest, verification_commitment
                 FROM applicability_binding_verifications
                 WHERE stream_key = $1 AND signature = $2
                   AND event_index = $3::text::smallint
                 FOR UPDATE",
                &[&stream_key, &signature, &(event_index as i16)],
            )
            .await?;
        if let Some(row) = existing {
            let existing_event_slot = parse_u64(row.get(0), "applicability event slot")?;
            let existing_provider_slot = parse_u64(row.get(1), "applicability provider slot")?;
            let existing_receipt = parse_digest(row.get(2))?;
            let existing_binding = parse_digest(row.get(3))?;
            let existing_merchant = parse_digest(row.get(4))?;
            let existing_mint = parse_digest(row.get(5))?;
            let existing_context_id: [u8; 16] =
                row.get::<_, Vec<u8>>(6).try_into().map_err(|_| {
                    StorageError::InvalidApplicabilityBindingVerification(
                        "stored applicability context ID is not 16 bytes".to_string(),
                    )
                })?;
            let existing_attestation_digest = parse_digest(row.get(7))?;
            let existing_event_evidence_digest = parse_digest(row.get(8))?;
            let existing_commitment: String = row.get(9);
            if existing_event_slot != evidence.slot
                || existing_provider_slot != verification.slot
                || existing_receipt != verification.receipt
                || existing_binding != verification.binding
                || existing_merchant != verification.merchant
                || existing_mint != verification.mint
                || existing_context_id != verification.context_id
                || existing_attestation_digest != verification.attestation_digest
                || existing_event_evidence_digest != evidence.evidence_digest
                || existing_commitment != "finalized"
            {
                return Err(StorageError::ApplicabilityBindingVerificationConflict);
            }
            if let Some(claim_token) = claim_token {
                Self::complete_applicability_binding_job_in_transaction(
                    transaction,
                    stream_key,
                    signature,
                    event_index,
                    claim_token,
                )
                .await?;
            }
            return Ok(());
        }

        transaction
            .execute(
                "INSERT INTO applicability_binding_verifications
                    (stream_key, signature, event_index, event_slot, provider_slot,
                     receipt, binding, merchant, mint, context_id, attestation_digest,
                     event_evidence_digest, verification_commitment)
                 VALUES ($1, $2, $3::smallint, $4::text::numeric,
                         $5::text::numeric, $6, $7, $8, $9, $10, $11, $12, 'finalized')",
                &[
                    &stream_key,
                    &signature,
                    &(event_index as i16),
                    &evidence.slot.to_string(),
                    &verification.slot.to_string(),
                    &&verification.receipt[..],
                    &&verification.binding[..],
                    &&verification.merchant[..],
                    &&verification.mint[..],
                    &&verification.context_id[..],
                    &&verification.attestation_digest[..],
                    &&evidence.evidence_digest[..],
                ],
            )
            .await
            .map_err(|error| {
                if error.code()
                    == Some(&deadpool_postgres::tokio_postgres::error::SqlState::UNIQUE_VIOLATION)
                {
                    StorageError::ApplicabilityBindingVerificationConflict
                } else {
                    StorageError::Database(error)
                }
            })?;
        if let Some(claim_token) = claim_token {
            Self::complete_applicability_binding_job_in_transaction(
                transaction,
                stream_key,
                signature,
                event_index,
                claim_token,
            )
            .await?;
        }
        Ok(())
    }

    /// Rebuild missing normalized receipts from immutable finalized event
    /// evidence in bounded, keyset-paginated transactions.
    ///
    /// This operation never mutates `chain_checkpoints`. It is intentionally
    /// separate from forward indexing and replay because a projection repair
    /// must not imply that a slot was scanned or finalized by this worker.
    pub async fn rebuild_indexed_receipts_from_raw_evidence(
        &self,
        stream_key: &str,
        start_slot: u64,
        end_slot: u64,
        batch_size: usize,
    ) -> Result<ProjectionRebuildReport, StorageError> {
        validate_indexer_stream_key(stream_key)?;
        if start_slot > end_slot {
            return Err(StorageError::InvalidIndexerState(
                "projection rebuild slot range is reversed".to_string(),
            ));
        }
        if batch_size == 0 || batch_size > MAX_PROJECTION_REBUILD_BATCH_SIZE {
            return Err(StorageError::InvalidIndexerState(format!(
                "projection rebuild batch size must be between 1 and {MAX_PROJECTION_REBUILD_BATCH_SIZE}"
            )));
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let batch_limit = i64::try_from(batch_size).map_err(|_| {
            StorageError::InvalidIndexerState("projection rebuild batch is too large".to_string())
        })?;
        let mut cursor_slot = start_slot;
        let mut cursor_signature = String::new();
        let mut cursor_event_index: i16 = -1;
        let mut scanned_rows = 0_u64;
        let mut inserted_rows = 0_u64;
        let mut duplicate_rows = 0_u64;

        loop {
            let rows = client
                .query(
                    "SELECT signature, slot::text, event_index::text,
                            event_discriminator, event_data, evidence_digest
                     FROM raw_chain_evidence
                     WHERE stream_key = $1
                       AND slot >= $2::text::numeric
                       AND slot <= $3::text::numeric
                       AND (slot, signature, event_index) >
                           ($4::text::numeric, $5, $6::smallint)
                     ORDER BY slot, signature, event_index
                     LIMIT $7",
                    &[
                        &stream_key,
                        &start_slot.to_string(),
                        &end_slot.to_string(),
                        &cursor_slot.to_string(),
                        &cursor_signature,
                        &cursor_event_index,
                        &batch_limit,
                    ],
                )
                .await?;
            if rows.is_empty() {
                break;
            }

            let transaction = client
                .build_transaction()
                .isolation_level(IsolationLevel::Serializable)
                .start()
                .await?;
            transaction
                .batch_execute(&format!(
                    "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                    DEFAULT_LOCK_TIMEOUT.as_millis(),
                    DEFAULT_STATEMENT_TIMEOUT.as_millis()
                ))
                .await?;
            for row in &rows {
                let evidence = parse_raw_evidence_row(row.clone())?;
                let receipt = rebuild_receipt_from_raw_evidence(&evidence)?;
                let existing = transaction
                    .query_opt(
                        "SELECT signature, slot::text, gross_atomic::text,
                                tax_atomic::text, net_atomic::text, evidence_digest,
                                receipt, payer, merchant, mint, nonce::text, policy_digest
                         FROM indexed_receipts
                         WHERE stream_key = $1 AND signature = $2
                         FOR UPDATE",
                        &[&stream_key, &receipt.signature],
                    )
                    .await?;
                if let Some(existing) = existing {
                    merge_receipt(Some(parse_receipt_row(existing)?), receipt)?;
                    duplicate_rows = duplicate_rows.saturating_add(1);
                } else {
                    transaction
                        .execute(
                            "INSERT INTO indexed_receipts
                                (stream_key, signature, slot, gross_atomic, tax_atomic,
                                 net_atomic, evidence_digest, receipt, payer, merchant, mint,
                                 nonce, policy_digest)
                             VALUES ($1, $2, $3::text::numeric, $4::text::numeric,
                                     $5::text::numeric, $6::text::numeric, $7, $8, $9,
                                     $10, $11, $12::text::numeric, $13)",
                            &[
                                &stream_key,
                                &receipt.signature,
                                &receipt.slot.to_string(),
                                &receipt.gross_atomic.to_string(),
                                &receipt.tax_atomic.to_string(),
                                &receipt.net_atomic.to_string(),
                                &&receipt.evidence_digest[..],
                                &&receipt.receipt[..],
                                &&receipt.payer[..],
                                &&receipt.merchant[..],
                                &&receipt.mint[..],
                                &receipt.nonce.to_string(),
                                &&receipt.policy_digest[..],
                            ],
                        )
                        .await?;
                    inserted_rows = inserted_rows.saturating_add(1);
                }
                if evidence.event_discriminator == indexer::SPLIT_SETTLEMENT_EVENT_DISCRIMINATOR {
                    let split = rebuild_split_settlement_from_raw_evidence(&evidence)?;
                    let existing = transaction
                        .query_opt(
                            "SELECT signature, slot::text, receipt, payer, merchant, mint,
                                    nonce::text, gross_atomic::text, tax_atomic::text,
                                    net_atomic::text, split_plan_version::text,
                                    split_plan_digest, policy_digest, evidence_digest
                             FROM indexed_split_settlements
                             WHERE stream_key = $1 AND signature = $2
                             FOR UPDATE",
                            &[&stream_key, &split.signature],
                        )
                        .await?;
                    if let Some(existing) = existing {
                        if parse_split_settlement_row(existing)? != split {
                            return Err(StorageError::Indexer(
                                indexer::IndexerError::ConflictingEvidence,
                            ));
                        }
                        duplicate_rows = duplicate_rows.saturating_add(1);
                    } else {
                        transaction
                            .execute(
                                "INSERT INTO indexed_split_settlements
                                    (stream_key, signature, slot, receipt, payer, merchant, mint,
                                     nonce, gross_atomic, tax_atomic, net_atomic, split_plan_version,
                                     split_plan_digest, policy_digest, evidence_digest)
                                 VALUES ($1, $2, $3::text::numeric, $4, $5, $6, $7,
                                         $8::text::numeric, $9::text::numeric, $10::text::numeric,
                                         $11::text::numeric, $12::text::numeric, $13, $14, $15)",
                                &[
                                    &stream_key,
                                    &split.signature,
                                    &split.slot.to_string(),
                                    &&split.receipt[..],
                                    &&split.payer[..],
                                    &&split.merchant[..],
                                    &&split.mint[..],
                                    &split.nonce.to_string(),
                                    &split.gross_atomic.to_string(),
                                    &split.tax_atomic.to_string(),
                                    &split.net_atomic.to_string(),
                                    &split.split_plan_version.to_string(),
                                    &&split.split_plan_digest[..],
                                    &&split.policy_digest[..],
                                    &&split.evidence_digest[..],
                                ],
                            )
                            .await?;
                        inserted_rows = inserted_rows.saturating_add(1);
                    }
                }
            }
            transaction.commit().await?;
            scanned_rows = scanned_rows.saturating_add(rows.len() as u64);
            let last = parse_raw_evidence_row(
                rows.last()
                    .ok_or_else(|| {
                        StorageError::InvalidIndexerState(
                            "projection rebuild returned an empty batch".to_string(),
                        )
                    })?
                    .clone(),
            )?;
            cursor_slot = last.slot;
            cursor_signature = last.signature;
            cursor_event_index = i16::try_from(last.event_index).map_err(|_| {
                StorageError::InvalidIndexerState("event index exceeds PostgreSQL smallint".into())
            })?;
        }

        let rows = client
            .query(
                "SELECT signature, slot::text, gross_atomic::text,
                        tax_atomic::text, net_atomic::text, evidence_digest,
                        receipt, payer, merchant, mint, nonce::text, policy_digest
                 FROM indexed_receipts
                 WHERE stream_key = $1
                   AND slot >= $2::text::numeric
                   AND slot <= $3::text::numeric
                 ORDER BY slot, signature",
                &[&stream_key, &start_slot.to_string(), &end_slot.to_string()],
            )
            .await?;
        let mut digest = Sha256::new();
        digest.update(b"solana-tax-protocol/indexed-receipts-projection/v2");
        for row in rows {
            let receipt = parse_receipt_row(row)?;
            write_projection_text(&mut digest, &receipt.signature);
            digest.update(receipt.slot.to_le_bytes());
            digest.update(receipt.receipt);
            digest.update(receipt.payer);
            digest.update(receipt.merchant);
            digest.update(receipt.mint);
            digest.update(receipt.nonce.to_le_bytes());
            digest.update(receipt.gross_atomic.to_le_bytes());
            digest.update(receipt.tax_atomic.to_le_bytes());
            digest.update(receipt.net_atomic.to_le_bytes());
            digest.update(receipt.policy_digest);
            digest.update(receipt.evidence_digest);
        }
        Ok(ProjectionRebuildReport {
            scanned_rows,
            inserted_rows,
            duplicate_rows,
            projection_digest: digest.finalize().into(),
        })
    }

    /// Advance a leased replay row to the next unprocessed slot. A remaining
    /// suffix is re-queued with its failed-attempt counter reset because the
    /// completed chunk was successful; the row is completed when no suffix
    /// remains. The claim token fences stale workers.
    pub async fn advance_replay_gap(
        &self,
        gap_id: Uuid,
        claim_token: Uuid,
        next_start_slot: u64,
    ) -> Result<bool, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let updated = client
            .execute(
                "UPDATE indexer_replay_gaps
                 SET start_slot = CASE
                         WHEN $3::text::numeric <= end_slot THEN $3::text::numeric
                         ELSE end_slot
                     END,
                     status = CASE
                         WHEN $3::text::numeric <= end_slot THEN 'queued'
                         ELSE 'completed'
                     END,
                     attempts = CASE
                         WHEN $3::text::numeric <= end_slot THEN 0
                         ELSE attempts
                     END,
                     claim_token = NULL,
                     claimed_at = NULL,
                     completed_at = CASE
                         WHEN $3::text::numeric <= end_slot THEN completed_at
                         ELSE now()
                     END,
                     updated_at = now()
                 WHERE id = $1
                   AND claim_token = $2
                   AND status = 'leased'
                   AND $3::text::numeric > start_slot
                   AND $3::text::numeric <= end_slot + 1",
                &[&gap_id, &claim_token, &next_start_slot.to_string()],
            )
            .await?;
        Ok(updated == 1)
    }

    pub async fn fail_replay_gap(
        &self,
        gap_id: Uuid,
        claim_token: Uuid,
        error_code: &str,
    ) -> Result<bool, StorageError> {
        validate_replay_error_code(error_code)?;
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let max_attempts = i16::try_from(MAX_REPLAY_ATTEMPTS)
            .map_err(|_| StorageError::InvalidReplayState("retry limit exceeds smallint".into()))?;
        let updated = client
            .execute(
                "UPDATE indexer_replay_gaps
                 SET status = CASE WHEN attempts >= $4 THEN 'dead_lettered' ELSE 'queued' END,
                     claim_token = NULL, claimed_at = NULL, last_error_code = $3,
                     dead_lettered_at = CASE WHEN attempts >= $4 THEN now() ELSE dead_lettered_at END,
                     updated_at = now()
                 WHERE id = $1 AND claim_token = $2 AND status = 'leased'",
                &[&gap_id, &claim_token, &error_code, &max_attempts],
            )
            .await?;
        Ok(updated == 1)
    }

    pub async fn requeue_dead_lettered_gap(&self, gap_id: Uuid) -> Result<bool, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let updated = client
            .execute(
                "UPDATE indexer_replay_gaps
                 SET status = 'queued', attempts = 0, claim_token = NULL,
                     claimed_at = NULL, last_error_code = NULL,
                     dead_lettered_at = NULL, updated_at = now()
                 WHERE id = $1 AND status = 'dead_lettered'",
                &[&gap_id],
            )
            .await?;
        Ok(updated == 1)
    }

    /// Loads bounded replay backlog gauges for one configured stream. The
    /// stream key is a service configuration value, never a Prometheus label.
    pub async fn load_replay_gap_metrics(
        &self,
        stream_key: &str,
    ) -> Result<ReplayGapMetricsSnapshot, StorageError> {
        validate_indexer_stream_key(stream_key)?;
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_one(
                "SELECT
                    COUNT(*) FILTER (WHERE status = 'queued')::text,
                    COUNT(*) FILTER (WHERE status = 'leased')::text,
                    COUNT(*) FILTER (WHERE status = 'dead_lettered')::text,
                    COALESCE(
                        SUM(CASE WHEN status = 'leased'
                                 THEN end_slot - start_slot + 1
                                 ELSE 0 END),
                        0
                    )::text
                 FROM indexer_replay_gaps
                 WHERE stream_key = $1",
                &[&stream_key],
            )
            .await?;
        Ok(ReplayGapMetricsSnapshot {
            queued: parse_u64(row.get(0), "queued replay gaps")?,
            leased: parse_u64(row.get(1), "leased replay gaps")?,
            dead_lettered: parse_u64(row.get(2), "dead-lettered replay gaps")?,
            leased_slots: parse_u64(row.get(3), "leased replay slots")?,
        })
    }

    /// Loads the bounded count of indexed refund executions that do not yet
    /// have an immutable finalized balance proof for one configured stream.
    /// The stream key is service configuration, never a caller-controlled
    /// metric label.
    pub async fn load_refund_proof_metrics(
        &self,
        stream_key: &str,
    ) -> Result<RefundProofMetricsSnapshot, StorageError> {
        validate_indexer_stream_key(stream_key)?;
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_one(
                "SELECT COUNT(*)::text
                 FROM indexed_adjustment_refund_executions execution
                 LEFT JOIN refund_balance_proofs proof
                   ON proof.stream_key = execution.stream_key
                  AND proof.signature = execution.signature
                  AND proof.event_index = execution.event_index
                 WHERE execution.stream_key = $1
                   AND proof.id IS NULL",
                &[&stream_key],
            )
            .await?;
        Ok(RefundProofMetricsSnapshot {
            pending: parse_u64(row.get(0), "pending refund proofs")?,
        })
    }

    /// Loads the bounded count of submitted refund claims whose finalized
    /// transaction evidence still needs reconciliation. The query contains
    /// no tenant or signature label and is safe to expose as a fixed metric.
    pub async fn load_refund_claim_reconciliation_metrics(
        &self,
    ) -> Result<RefundClaimReconciliationMetricsSnapshot, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_one(
                "SELECT COUNT(*)::text
                 FROM refund_claim_intents
                 WHERE signature IS NOT NULL
                   AND status IN ('submitting', 'submitted', 'processed', 'confirmed', 'unknown')",
                &[],
            )
            .await?;
        Ok(RefundClaimReconciliationMetricsSnapshot {
            pending: parse_u64(row.get(0), "pending refund claim reconciliations")?,
        })
    }

    pub async fn load_refund_claim_cancellation_reconciliation_metrics(
        &self,
    ) -> Result<RefundClaimCancellationReconciliationMetricsSnapshot, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_one(
                "SELECT COUNT(*)::text
                 FROM refund_claim_cancellation_intents
                 WHERE signature IS NOT NULL
                   AND status IN ('submitting', 'submitted', 'processed', 'confirmed', 'unknown')",
                &[],
            )
            .await?;
        Ok(RefundClaimCancellationReconciliationMetricsSnapshot {
            pending: parse_u64(
                row.get(0),
                "pending refund claim cancellation reconciliations",
            )?,
        })
    }

    /// Loads the bounded count of targeted-recovery requests older than the
    /// reconciliation grace period without a terminal audit row.
    pub async fn load_targeted_recovery_metrics(
        &self,
    ) -> Result<TargetedRecoveryMetricsSnapshot, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_one(
                "SELECT COUNT(*)::text
                 FROM indexer_targeted_recovery_actions requested
                 WHERE requested.outcome = 'requested'
                   AND requested.created_at < now() - interval '5 minutes'
                   AND NOT EXISTS (
                       SELECT 1
                       FROM indexer_targeted_recovery_actions terminal
                       WHERE terminal.request_id = requested.request_id
                         AND terminal.outcome IN ('recovered', 'failed')
                   )",
                &[],
            )
            .await?;
        Ok(TargetedRecoveryMetricsSnapshot {
            stale_requested: parse_u64(row.get(0), "stale targeted recovery requests")?,
        })
    }

    /// Loads the bounded server-owned vault targets and their exact projection
    /// aggregates. Active route identity is rechecked here so a stale target
    /// cannot silently reconcile a retired or out-of-window route.
    pub async fn load_active_vault_reconciliation_targets(
        &self,
        limit: usize,
    ) -> Result<Vec<VaultReconciliationTarget>, StorageError> {
        if limit == 0 || limit > MAX_VAULT_RECONCILIATION_TARGETS {
            return Err(StorageError::InvalidConfiguration(
                "vault reconciliation target limit must be between 1 and 64".to_string(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT target.id, target.merchant_id, target.route_id,
                        target.stream_key, target.mint, target.token_program,
                        target.tax_vault, target.opening_balance_atomic::text,
                        target.opening_balance_slot::text, target.opening_reference,
                        COALESCE((
                            SELECT SUM(receipt.tax_atomic)
                            FROM indexed_receipts receipt
                            WHERE receipt.stream_key = target.stream_key
                        ), 0)::text,
                        COALESCE((
                            SELECT SUM(claim.amount_atomic)
                            FROM vault_release_claims claim
                            WHERE claim.status = 'executed'
                              AND claim.slot IS NOT NULL
                              AND claim.mint = target.mint
                              AND claim.token_program = target.token_program
                              AND claim.tax_vault = target.tax_vault
                        ), 0)::text
                 FROM vault_reconciliation_targets target
                 JOIN settlement_routes route
                   ON route.id = target.route_id
                  AND route.merchant_id = target.merchant_id
                 WHERE target.status = 'active'
                   AND route.status = 'active'
                   AND route.effective_from <= now()
                   AND (route.effective_until IS NULL OR now() < route.effective_until)
                 ORDER BY target.updated_at, target.id
                 LIMIT $1",
                &[&(limit as i64)],
            )
            .await?;
        rows.into_iter()
            .map(|row| {
                Ok(VaultReconciliationTarget {
                    id: row.get(0),
                    merchant_id: row.get(1),
                    route_id: row.get(2),
                    stream_key: row.get(3),
                    mint: row.get(4),
                    token_program: row.get(5),
                    tax_vault: row.get(6),
                    opening_balance_atomic: parse_u64(row.get(7), "opening vault balance")?,
                    opening_balance_slot: parse_u64(row.get(8), "opening vault slot")?,
                    opening_reference: row.get(9),
                    receipt_tax_atomic: parse_u64(row.get(10), "receipt tax aggregate")?,
                    release_atomic: parse_u64(row.get(11), "vault release aggregate")?,
                })
            })
            .collect()
    }

    /// Returns tenant-scoped target metadata and the latest append-only run.
    /// Every value that crosses the API boundary remains an exact decimal
    /// string; the internal target arithmetic stays typed as `u64`.
    pub async fn list_vault_reconciliation_target_views(
        &self,
        merchant_id: Uuid,
        limit: usize,
    ) -> Result<Vec<VaultReconciliationTargetView>, StorageError> {
        if merchant_id.is_nil() || limit == 0 || limit > MAX_VAULT_RECONCILIATION_TARGETS {
            return Err(StorageError::InvalidConfiguration(
                "vault reconciliation target query is outside its bounded domain".to_string(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT target.id, target.merchant_id, target.route_id,
                        target.stream_key, target.mint, target.token_program,
                        target.tax_vault, target.opening_balance_atomic::text,
                        target.opening_balance_slot::text, target.opening_reference,
                        COALESCE((SELECT SUM(receipt.tax_atomic)
                                  FROM indexed_receipts receipt
                                  WHERE receipt.stream_key = target.stream_key), 0)::text,
                        COALESCE((SELECT SUM(claim.amount_atomic)
                                  FROM vault_release_claims claim
                                  WHERE claim.status = 'executed'
                                    AND claim.slot IS NOT NULL
                                    AND claim.mint = target.mint
                                    AND claim.token_program = target.token_program
                                    AND claim.tax_vault = target.tax_vault), 0)::text,
                        latest.id, latest.status, latest.opening_balance_atomic::text,
                        latest.receipt_tax_atomic::text, latest.release_atomic::text,
                        latest.expected_balance_atomic::text,
                        latest.observed_balance_atomic::text,
                        latest.observed_slot::text, latest.error_code,
                        latest.created_at, latest.stale
                 FROM vault_reconciliation_targets target
                 LEFT JOIN LATERAL (
                    SELECT run.id, run.status, run.opening_balance_atomic,
                           run.receipt_tax_atomic, run.release_atomic,
                           run.expected_balance_atomic,
                           run.observed_balance_atomic, run.observed_slot,
                           run.error_code,
                           to_char(run.created_at AT TIME ZONE 'UTC',
                                   'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"') AS created_at,
                           run.created_at < now() -
                               ($3::bigint * interval '1 second') AS stale
                    FROM vault_reconciliation_runs run
                    WHERE run.target_id = target.id
                    ORDER BY run.created_at DESC, run.id DESC
                    LIMIT 1
                 ) latest ON true
                 WHERE target.merchant_id = $1
                   AND target.status = 'active'
                 ORDER BY target.updated_at, target.id
                 LIMIT $2",
                &[
                    &merchant_id,
                    &(limit as i64),
                    &VAULT_RECONCILIATION_STALE_AFTER_SECONDS,
                ],
            )
            .await?;
        rows.into_iter()
            .map(parse_vault_reconciliation_target_view_row)
            .collect()
    }

    /// Returns bounded run history only after proving the target belongs to
    /// the authenticated merchant. Unknown targets do not disclose whether an
    /// identifier exists under another tenant.
    pub async fn list_vault_reconciliation_runs(
        &self,
        merchant_id: Uuid,
        target_id: Uuid,
        limit: usize,
    ) -> Result<Vec<VaultReconciliationRunView>, StorageError> {
        if merchant_id.is_nil() || target_id.is_nil() || limit == 0 || limit > 64 {
            return Err(StorageError::InvalidConfiguration(
                "vault reconciliation run query is outside its bounded domain".to_string(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let target_exists = client
            .query_one(
                "SELECT EXISTS(
                    SELECT 1 FROM vault_reconciliation_targets
                    WHERE id = $1 AND merchant_id = $2
                )",
                &[&target_id, &merchant_id],
            )
            .await?
            .get::<_, bool>(0);
        if !target_exists {
            return Err(StorageError::VaultReconciliationTargetNotFound);
        }
        let rows = client
            .query(
                "SELECT id, status, opening_balance_atomic::text,
                        receipt_tax_atomic::text, release_atomic::text,
                        expected_balance_atomic::text,
                        observed_balance_atomic::text, observed_slot::text,
                        error_code,
                        to_char(created_at AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                        created_at < now() -
                            ($3::bigint * interval '1 second') AS stale
                 FROM vault_reconciliation_runs
                 WHERE target_id = $1
                 ORDER BY created_at DESC, id DESC
                 LIMIT $2",
                &[
                    &target_id,
                    &(limit as i64),
                    &VAULT_RECONCILIATION_STALE_AFTER_SECONDS,
                ],
            )
            .await?;
        rows.into_iter()
            .map(parse_vault_reconciliation_run_view_row)
            .collect()
    }

    /// Appends one finalized vault reconciliation observation. The caller must
    /// supply only the server-loaded target and the provider observation; it
    /// cannot choose the expected balance independently.
    pub async fn record_vault_reconciliation_run(
        &self,
        run: VaultReconciliationRun,
    ) -> Result<Uuid, StorageError> {
        if run.target_id.is_nil()
            || run.route_id.is_nil()
            || run.merchant_id.is_nil()
            || run.stream_key.trim().is_empty()
            || run.stream_key.len() > 200
            || run.mint.trim().is_empty()
            || run.token_program.trim().is_empty()
            || run.tax_vault.trim().is_empty()
            || !matches!(run.status, "matched" | "mismatch" | "error")
        {
            return Err(StorageError::InvalidConfiguration(
                "vault reconciliation run identity or status is invalid".to_string(),
            ));
        }
        if (run.status == "error") != run.error_code.is_some()
            || (run.status == "matched"
                && (run.expected_balance_atomic.is_none()
                    || run.observed_balance_atomic.is_none()
                    || run.observed_slot.is_none()))
        {
            return Err(StorageError::InvalidConfiguration(
                "vault reconciliation run evidence does not match its status".to_string(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_one(
                "INSERT INTO vault_reconciliation_runs
                    (target_id, route_id, merchant_id, stream_key, mint,
                     token_program, tax_vault, opening_balance_atomic,
                     receipt_tax_atomic, release_atomic, expected_balance_atomic,
                     observed_balance_atomic, observed_slot, status, error_code)
                 VALUES ($1, $2, $3, $4, $5, $6, $7,
                         $8::text::numeric, $9::text::numeric, $10::text::numeric,
                         $11::text::numeric, $12::text::numeric, $13::text::numeric,
                         $14, $15)
                 RETURNING id",
                &[
                    &run.target_id,
                    &run.route_id,
                    &run.merchant_id,
                    &run.stream_key,
                    &run.mint,
                    &run.token_program,
                    &run.tax_vault,
                    &run.expectation.opening_balance_atomic.to_string(),
                    &run.expectation.receipt_tax_atomic.to_string(),
                    &run.expectation.release_atomic.to_string(),
                    &run.expected_balance_atomic.map(|value| value.to_string()),
                    &run.observed_balance_atomic.map(|value| value.to_string()),
                    &run.observed_slot.map(|value| value.to_string()),
                    &run.status,
                    &run.error_code,
                ],
            )
            .await?;
        Ok(row.get(0))
    }

    /// Returns the latest result for each active target. A historical mismatch
    /// remains alertable until a later matched run proves it has cleared.
    pub async fn load_vault_reconciliation_metrics(
        &self,
    ) -> Result<VaultReconciliationMetricsSnapshot, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_one(
                "SELECT
                    COUNT(*) FILTER (WHERE latest.status = 'mismatch')::text,
                    COUNT(*) FILTER (WHERE latest.status = 'error')::text
                 FROM (
                    SELECT DISTINCT ON (target_id) target_id, status
                    FROM vault_reconciliation_runs
                    ORDER BY target_id, created_at DESC, id DESC
                 ) latest
                 JOIN vault_reconciliation_targets target
                   ON target.id = latest.target_id
                  AND target.status = 'active'",
                &[],
            )
            .await?;
        Ok(VaultReconciliationMetricsSnapshot {
            mismatches: parse_u64(row.get(0), "vault reconciliation mismatches")?,
            errors: parse_u64(row.get(1), "vault reconciliation errors")?,
        })
    }

    /// Loads the last finalized on-chain pause event for the stream. An empty
    /// result is explicitly unknown and must remain alertable until the
    /// indexer has observed at least one finalized governance event.
    pub async fn load_protocol_pause_metrics(
        &self,
        stream_key: &str,
    ) -> Result<ProtocolPauseMetricsSnapshot, StorageError> {
        validate_indexer_stream_key(stream_key)?;
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT paused, slot::text
                 FROM protocol_pause_events
                 WHERE stream_key = $1
                 ORDER BY slot DESC, event_index DESC
                 LIMIT 1",
                &[&stream_key],
            )
            .await?;
        match row {
            Some(row) => Ok(ProtocolPauseMetricsSnapshot {
                known: true,
                paused: row.get(0),
                slot: parse_u64(row.get(1), "protocol pause slot")?,
            }),
            None => Ok(ProtocolPauseMetricsSnapshot {
                known: false,
                paused: false,
                slot: 0,
            }),
        }
    }

    /// Appends one immutable targeted-recovery audit event. This control-plane
    /// record is deliberately separate from chain evidence and never changes
    /// a checkpoint or projection by itself.
    pub async fn record_targeted_recovery_audit(
        &self,
        command: TargetedRecoveryAuditCommand,
    ) -> Result<Uuid, StorageError> {
        validate_targeted_recovery_audit(&command)?;
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_one(
                "INSERT INTO indexer_targeted_recovery_actions
                    (request_id, stream_key, target_address, target_signature,
                     operator_subject, note, outcome, pages_scanned, slot, error_code)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::text::numeric, $10)
                 RETURNING id",
                &[
                    &command.request_id,
                    &command.stream_key,
                    &command.target_address,
                    &command.target_signature,
                    &command.operator_subject,
                    &command.note,
                    &command.outcome,
                    &command.pages_scanned.map(|value| value as i32),
                    &command.slot.map(|value| value.to_string()),
                    &command.error_code,
                ],
            )
            .await?;
        Ok(row.get(0))
    }

    /// Lists a bounded, system-level replay-gap view for an authenticated
    /// indexer operator. This is operational metadata and never represents
    /// finalized settlement truth.
    pub async fn list_replay_gaps(
        &self,
        stream_key: &str,
        status: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ReplayGapView>, StorageError> {
        validate_indexer_stream_key(stream_key)?;
        validate_replay_status_filter(status)?;
        if !(1..=MAX_REPLAY_LIST_LIMIT).contains(&limit) {
            return Err(StorageError::InvalidReplayOperatorInput(format!(
                "replay-gap list limit must be between 1 and {MAX_REPLAY_LIST_LIMIT}"
            )));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let limit = i64::try_from(limit).map_err(|_| {
            StorageError::InvalidReplayOperatorInput("replay-gap list limit is too large".into())
        })?;
        let rows = match status {
            Some(status) => {
                let query = format!(
                    "{REPLAY_GAP_VIEW_SELECT}
                     WHERE stream_key = $1 AND status = $2
                     ORDER BY created_at, id
                     LIMIT $3"
                );
                client
                    .query(&query, &[&stream_key, &status, &limit])
                    .await?
            }
            None => {
                let query = format!(
                    "{REPLAY_GAP_VIEW_SELECT}
                     WHERE stream_key = $1
                     ORDER BY created_at, id
                     LIMIT $2"
                );
                client.query(&query, &[&stream_key, &limit]).await?
            }
        };
        rows.into_iter().map(parse_replay_gap_view).collect()
    }

    /// Lists bounded system-level route-verification evidence for an
    /// authenticated indexer operator. This is operational metadata only;
    /// finalized Solana account evidence and the settlement program remain
    /// authoritative.
    pub async fn list_settlement_route_verification_runs(
        &self,
        limit: usize,
    ) -> Result<Vec<SettlementRouteVerificationRunView>, StorageError> {
        if !(1..=MAX_ROUTE_VERIFICATION_RUN_LIST_LIMIT).contains(&limit) {
            return Err(StorageError::InvalidReplayOperatorInput(format!(
                "route-verification run list limit must be between 1 and {MAX_ROUTE_VERIFICATION_RUN_LIST_LIMIT}"
            )));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let limit = i64::try_from(limit).map_err(|_| {
            StorageError::InvalidReplayOperatorInput(
                "route-verification run list limit is too large".into(),
            )
        })?;
        let query = format!(
            "{ROUTE_VERIFICATION_RUN_VIEW_SELECT}
             ORDER BY started_at DESC, id DESC
             LIMIT $1"
        );
        client
            .query(&query, &[&limit])
            .await?
            .into_iter()
            .map(parse_settlement_route_verification_run_view)
            .collect()
    }

    /// Acknowledge a dead-lettered replay gap as an operator review action.
    /// The row remains dead-lettered until a separate reviewed requeue.
    pub async fn acknowledge_replay_gap_for_operator(
        &self,
        gap_id: Uuid,
        operator_subject: &str,
        note: &str,
    ) -> Result<ReplayGapView, StorageError> {
        validate_replay_operator_input(operator_subject, note)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT status, operator_acknowledged_by, operator_acknowledgement
                 FROM indexer_replay_gaps WHERE id = $1 FOR UPDATE",
                &[&gap_id],
            )
            .await?
            .ok_or(StorageError::ReplayGapNotFound)?;
        let status: String = row.get(0);
        if status != "dead_lettered" {
            return Err(StorageError::ReplayGapNotDeadLettered);
        }
        let existing_subject: Option<String> = row.get(1);
        let existing_note: Option<String> = row.get(2);
        if let (Some(existing_subject), Some(existing_note)) = (existing_subject, existing_note) {
            transaction.commit().await?;
            if existing_subject == operator_subject && existing_note == note {
                return self
                    .load_replay_gap_view(gap_id)
                    .await?
                    .ok_or(StorageError::ReplayGapNotFound);
            }
            return Err(StorageError::ReplayGapAlreadyAcknowledged);
        }
        transaction
            .execute(
                "UPDATE indexer_replay_gaps
                 SET operator_acknowledged_at = now(),
                     operator_acknowledged_by = $2,
                     operator_acknowledgement = $3,
                     updated_at = now()
                 WHERE id = $1 AND status = 'dead_lettered'
                   AND operator_acknowledged_at IS NULL",
                &[&gap_id, &operator_subject, &note],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO replay_gap_operator_actions
                    (gap_id, action, operator_subject, note)
                 VALUES ($1, 'acknowledge', $2, $3)",
                &[&gap_id, &operator_subject, &note],
            )
            .await?;
        transaction.commit().await?;
        self.load_replay_gap_view(gap_id)
            .await?
            .ok_or(StorageError::ReplayGapNotFound)
    }

    /// Requeue a dead-lettered gap only after an operator acknowledgement.
    /// The current acknowledgement is cleared so every retry exhaustion needs
    /// a fresh review, while the append-only action ledger preserves history.
    pub async fn requeue_replay_gap_for_operator(
        &self,
        gap_id: Uuid,
        operator_subject: &str,
        note: &str,
    ) -> Result<ReplayGapView, StorageError> {
        validate_replay_operator_input(operator_subject, note)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT status, operator_acknowledged_at
                 FROM indexer_replay_gaps WHERE id = $1 FOR UPDATE",
                &[&gap_id],
            )
            .await?
            .ok_or(StorageError::ReplayGapNotFound)?;
        let status: String = row.get(0);
        if status != "dead_lettered" {
            return Err(StorageError::ReplayGapNotDeadLettered);
        }
        let acknowledged_at: Option<std::time::SystemTime> = row.get(1);
        if acknowledged_at.is_none() {
            return Err(StorageError::ReplayGapNotAcknowledged);
        }
        transaction
            .execute(
                "UPDATE indexer_replay_gaps
                 SET status = 'queued', attempts = 0, claim_token = NULL,
                     claimed_at = NULL, last_error_code = NULL,
                     dead_lettered_at = NULL,
                     operator_acknowledged_at = NULL,
                     operator_acknowledged_by = NULL,
                     operator_acknowledgement = NULL,
                     updated_at = now()
                 WHERE id = $1 AND status = 'dead_lettered'
                   AND operator_acknowledged_at IS NOT NULL",
                &[&gap_id],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO replay_gap_operator_actions
                    (gap_id, action, operator_subject, note)
                 VALUES ($1, 'requeue', $2, $3)",
                &[&gap_id, &operator_subject, &note],
            )
            .await?;
        transaction.commit().await?;
        self.load_replay_gap_view(gap_id)
            .await?
            .ok_or(StorageError::ReplayGapNotFound)
    }

    async fn load_replay_gap_view(
        &self,
        gap_id: Uuid,
    ) -> Result<Option<ReplayGapView>, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let query = format!("{REPLAY_GAP_VIEW_SELECT} WHERE id = $1");
        client
            .query_opt(&query, &[&gap_id])
            .await?
            .map(parse_replay_gap_view)
            .transpose()
    }

    pub async fn load_checkpoint(
        &self,
        stream_key: &str,
    ) -> Result<Option<Checkpoint>, StorageError> {
        validate_indexer_stream_key(stream_key)?;
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        client
            .query_opt(
                "SELECT next_slot::text, finalized_slot::text, generation::text
                 FROM chain_checkpoints
                 WHERE stream_key = $1",
                &[&stream_key],
            )
            .await?
            .map(parse_checkpoint_row)
            .transpose()
    }

    /// Persist one finalized, bounded scan page and its receipts atomically.
    ///
    /// The caller supplies the checkpoint it read before scanning. A concurrent
    /// worker cannot advance the same stream without making this transaction
    /// fail closed. Retrying an already committed page is idempotent when the
    /// persisted checkpoint already equals the calculated next checkpoint.
    pub async fn persist_finalized_page(
        &self,
        stream_key: &str,
        checkpoint: Checkpoint,
        page: &FinalizedPage,
    ) -> Result<Checkpoint, StorageError> {
        validate_indexer_stream_key(stream_key)?;
        let next_checkpoint = checkpoint.apply_page(
            page.range,
            page.observed_finalized_slot,
            page.receipts.clone(),
            page.complete,
        )?;
        for evidence in &page.raw_evidence {
            let receipt_match = page.receipts.iter().any(|receipt| {
                receipt.signature == evidence.signature
                    && receipt.evidence_digest == evidence.evidence_digest
            });
            let refund_match = page.refund_executions.iter().any(|execution| {
                execution.signature == evidence.signature
                    && execution.event_index == evidence.event_index
                    && execution.evidence_digest == evidence.evidence_digest
            });
            let adjustment_match = page.adjustment_records.iter().any(|record| {
                record.signature == evidence.signature
                    && record.event_index == evidence.event_index
                    && record.evidence_digest == evidence.evidence_digest
            });
            if !receipt_match && !refund_match && !adjustment_match {
                return Err(StorageError::Indexer(
                    indexer::IndexerError::InvalidEvidence,
                ));
            }
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;

        let current = transaction
            .query_opt(
                "SELECT next_slot::text, finalized_slot::text, generation::text
                 FROM chain_checkpoints
                 WHERE stream_key = $1
                 FOR UPDATE",
                &[&stream_key],
            )
            .await?
            .map(parse_checkpoint_row)
            .transpose()?;
        let current = match current {
            Some(value) => value,
            None => {
                transaction
                    .execute(
                        "INSERT INTO chain_checkpoints
                            (stream_key, next_slot, finalized_slot, generation)
                         VALUES ($1, $2::text::numeric, $3::text::numeric, $4::text::numeric)",
                        &[
                            &stream_key,
                            &checkpoint.next_slot.to_string(),
                            &checkpoint.finalized_slot.to_string(),
                            &checkpoint.generation.to_string(),
                        ],
                    )
                    .await?;
                checkpoint
            }
        };

        if current == next_checkpoint {
            transaction.commit().await?;
            return Ok(current);
        }
        if current != checkpoint {
            return Err(StorageError::IndexerCheckpointConflict);
        }

        for receipt in &page.receipts {
            let existing = transaction
                .query_opt(
                    "SELECT signature, slot::text, gross_atomic::text, tax_atomic::text,
                            net_atomic::text, evidence_digest, receipt, payer,
                            merchant, mint, nonce::text, policy_digest
                     FROM indexed_receipts
                     WHERE stream_key = $1 AND signature = $2
                     FOR UPDATE",
                    &[&stream_key, &receipt.signature],
                )
                .await?;
            if let Some(row) = existing {
                let existing_receipt = parse_receipt_row(row)?;
                merge_receipt(Some(existing_receipt), receipt.clone())?;
                continue;
            }
            transaction
                .execute(
                    "INSERT INTO indexed_receipts
                        (stream_key, signature, slot, gross_atomic, tax_atomic, net_atomic,
                         evidence_digest, receipt, payer, merchant, mint, nonce, policy_digest)
                     VALUES ($1, $2, $3::text::numeric, $4::text::numeric,
                             $5::text::numeric, $6::text::numeric, $7, $8, $9, $10, $11,
                             $12::text::numeric, $13)",
                    &[
                        &stream_key,
                        &receipt.signature,
                        &receipt.slot.to_string(),
                        &receipt.gross_atomic.to_string(),
                        &receipt.tax_atomic.to_string(),
                        &receipt.net_atomic.to_string(),
                        &&receipt.evidence_digest[..],
                        &&receipt.receipt[..],
                        &&receipt.payer[..],
                        &&receipt.merchant[..],
                        &&receipt.mint[..],
                        &receipt.nonce.to_string(),
                        &&receipt.policy_digest[..],
                    ],
                )
                .await?;
        }
        for split in &page.split_settlements {
            let existing = transaction
                .query_opt(
                    "SELECT signature, slot::text, receipt, payer, merchant, mint,
                            nonce::text, gross_atomic::text, tax_atomic::text,
                            net_atomic::text, split_plan_version::text,
                            split_plan_digest, policy_digest, evidence_digest
                     FROM indexed_split_settlements
                     WHERE stream_key = $1 AND signature = $2
                     FOR UPDATE",
                    &[&stream_key, &split.signature],
                )
                .await?;
            if let Some(row) = existing {
                let existing_split = parse_split_settlement_row(row)?;
                if existing_split != *split {
                    return Err(StorageError::Indexer(
                        indexer::IndexerError::ConflictingEvidence,
                    ));
                }
                continue;
            }
            transaction
                .execute(
                    "INSERT INTO indexed_split_settlements
                        (stream_key, signature, slot, receipt, payer, merchant, mint,
                         nonce, gross_atomic, tax_atomic, net_atomic, split_plan_version,
                         split_plan_digest, policy_digest, evidence_digest)
                     VALUES ($1, $2, $3::text::numeric, $4, $5, $6, $7,
                             $8::text::numeric, $9::text::numeric, $10::text::numeric,
                             $11::text::numeric, $12::text::numeric, $13, $14, $15)",
                    &[
                        &stream_key,
                        &split.signature,
                        &split.slot.to_string(),
                        &&split.receipt[..],
                        &&split.payer[..],
                        &&split.merchant[..],
                        &&split.mint[..],
                        &split.nonce.to_string(),
                        &split.gross_atomic.to_string(),
                        &split.tax_atomic.to_string(),
                        &split.net_atomic.to_string(),
                        &split.split_plan_version.to_string(),
                        &&split.split_plan_digest[..],
                        &&split.policy_digest[..],
                        &&split.evidence_digest[..],
                    ],
                )
                .await?;
        }
        persist_revenue_stream_accrual_jobs(&transaction, stream_key, page).await?;
        persist_indexed_adjustment_refund_executions(
            &transaction,
            stream_key,
            &page.refund_executions,
        )
        .await?;
        persist_indexed_settlement_adjustments(&transaction, stream_key, &page.adjustment_records)
            .await?;
        for evidence in &page.raw_evidence {
            merge_raw_evidence(None, evidence.clone())?;
            let existing = transaction
                .query_opt(
                    "SELECT signature, slot::text, event_index::text,
                            event_discriminator, event_data, evidence_digest
                     FROM raw_chain_evidence
                     WHERE stream_key = $1 AND signature = $2 AND event_index = $3::text::smallint
                     FOR UPDATE",
                    &[
                        &stream_key,
                        &evidence.signature,
                        &evidence.event_index.to_string(),
                    ],
                )
                .await?;
            if let Some(row) = existing {
                let existing_evidence = parse_raw_evidence_row(row)?;
                merge_raw_evidence(Some(existing_evidence), evidence.clone())?;
                continue;
            }
            transaction
                .execute(
                    "INSERT INTO raw_chain_evidence
                        (stream_key, signature, slot, event_index, event_discriminator,
                         event_data, evidence_digest)
                     VALUES ($1, $2, $3::text::numeric, $4::text::smallint, $5, $6, $7)",
                    &[
                        &stream_key,
                        &evidence.signature,
                        &evidence.slot.to_string(),
                        &evidence.event_index.to_string(),
                        &&evidence.event_discriminator[..],
                        &evidence.event_data,
                        &&evidence.evidence_digest[..],
                    ],
                )
                .await?;
        }
        for transaction_evidence in &page.raw_transactions {
            transaction_evidence.validate(page.range, page.observed_finalized_slot)?;
            let existing = transaction
                .query_opt(
                    "SELECT signature, slot::text, transaction_index::text,
                            payload, payload_digest
                     FROM raw_chain_transactions
                     WHERE stream_key = $1 AND signature = $2
                       AND transaction_index = $3::text::numeric
                     FOR UPDATE",
                    &[
                        &stream_key,
                        &transaction_evidence.signature,
                        &transaction_evidence.transaction_index.to_string(),
                    ],
                )
                .await?;
            if let Some(row) = existing {
                let existing_transaction = parse_raw_transaction_row(row)?;
                if existing_transaction != *transaction_evidence {
                    return Err(StorageError::Indexer(
                        indexer::IndexerError::ConflictingEvidence,
                    ));
                }
                continue;
            }
            transaction
                .execute(
                    "INSERT INTO raw_chain_transactions
                        (stream_key, signature, slot, transaction_index, payload, payload_digest)
                     VALUES ($1, $2, $3::text::numeric, $4::text::numeric, $5, $6)",
                    &[
                        &stream_key,
                        &transaction_evidence.signature,
                        &transaction_evidence.slot.to_string(),
                        &transaction_evidence.transaction_index.to_string(),
                        &transaction_evidence.payload,
                        &&transaction_evidence.payload_digest[..],
                    ],
                )
                .await?;
        }
        Self::enqueue_applicability_binding_verification_jobs(&transaction, stream_key, page)
            .await?;
        persist_protocol_pause_events(
            &transaction,
            stream_key,
            &page.protocol_pause_changes,
            page.range,
            page.observed_finalized_slot,
        )
        .await?;
        transaction
            .execute(
                "UPDATE chain_checkpoints
                 SET next_slot = $2::text::numeric,
                     finalized_slot = $3::text::numeric,
                     generation = $4::text::numeric,
                     updated_at = now()
                 WHERE stream_key = $1",
                &[
                    &stream_key,
                    &next_checkpoint.next_slot.to_string(),
                    &next_checkpoint.finalized_slot.to_string(),
                    &next_checkpoint.generation.to_string(),
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(next_checkpoint)
    }

    /// Persist one immutable transaction-bound refund proof only after the
    /// indexed event and its raw Anchor evidence are present in the same
    /// serializable evidence boundary. This method never changes adjustment
    /// lifecycle state and never treats the proof as payment authority.
    pub async fn persist_refund_balance_proof(
        &self,
        stream_key: &str,
        execution: &IndexedAdjustmentRefundExecution,
        lease_token: Uuid,
        topology: RefundBalanceProofTopology,
        proof: RefundBalanceProof,
    ) -> Result<Uuid, StorageError> {
        validate_indexer_stream_key(stream_key)?;
        if lease_token.get_version_num() != 7 {
            return Err(StorageError::RefundBalanceProofLeaseLost);
        }
        merge_adjustment_refund_execution(None, execution.clone())?;
        validate_refund_balance_proof_binding(execution, &topology, &proof)?;
        let proof_digest = proof
            .digest()
            .map_err(|error| StorageError::InvalidRefundBalanceProof(error.to_string()))?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;

        let execution_row = transaction
            .query_opt(
                "SELECT signature, slot::text, event_index::text, claim,
                        adjustment, original_receipt, merchant, mint,
                        nonce::text, gross_magnitude::text, tax_magnitude::text,
                        net_magnitude::text, refund_funder, refund_destination,
                        approver, reference_digest, evidence_digest,
                        proof_claim_token
                 FROM indexed_adjustment_refund_executions
                 WHERE stream_key = $1 AND signature = $2 AND event_index = $3::text::smallint
                 FOR UPDATE",
                &[
                    &stream_key,
                    &execution.signature,
                    &execution.event_index.to_string(),
                ],
            )
            .await?
            .ok_or(StorageError::RefundBalanceProofExecutionNotFound)?;
        let stored_lease_token: Option<Uuid> = execution_row.get(17);
        if parse_indexed_adjustment_refund_execution_row(execution_row)? != *execution {
            return Err(StorageError::RefundBalanceProofConflict);
        }
        if stored_lease_token != Some(lease_token) {
            return Err(StorageError::RefundBalanceProofLeaseLost);
        }
        let raw_row = transaction
            .query_opt(
                "SELECT event_discriminator, evidence_digest
                 FROM raw_chain_evidence
                 WHERE stream_key = $1 AND signature = $2 AND event_index = $3::text::smallint
                 FOR UPDATE",
                &[
                    &stream_key,
                    &execution.signature,
                    &execution.event_index.to_string(),
                ],
            )
            .await?
            .ok_or(StorageError::InvalidRefundBalanceProof(
                "raw refund event evidence is missing".to_string(),
            ))?;
        let event_discriminator: Vec<u8> = raw_row.get(0);
        let evidence_digest: Vec<u8> = raw_row.get(1);
        if event_discriminator.as_slice() != indexer::ADJUSTMENT_REFUND_EXECUTED_EVENT_DISCRIMINATOR
            || evidence_digest.as_slice() != execution.evidence_digest
        {
            return Err(StorageError::InvalidRefundBalanceProof(
                "raw refund evidence is not bound to the indexed execution".to_string(),
            ));
        }

        let existing = transaction
            .query_opt(
                "SELECT id, slot::text, claim, adjustment, original_receipt, merchant,
                        mint, token_program, tax_vault, refund_source,
                        payer_destination, vault_authority, refund_funder, payer,
                        nonce::text, gross_atomic::text, tax_atomic::text,
                        net_atomic::text, tax_vault_before_atomic::text,
                        tax_vault_after_atomic::text, refund_source_before_atomic::text,
                        refund_source_after_atomic::text,
                        payer_destination_before_atomic::text,
                        payer_destination_after_atomic::text, proof_digest
                 FROM refund_balance_proofs
                 WHERE stream_key = $1 AND signature = $2 AND event_index = $3::text::smallint
                 FOR UPDATE",
                &[
                    &stream_key,
                    &execution.signature,
                    &execution.event_index.to_string(),
                ],
            )
            .await?;
        if let Some(row) = existing {
            let existing_record = parse_refund_balance_proof_row(row)?;
            if existing_record.1 != topology
                || existing_record.2 != execution.nonce
                || existing_record.3 != proof
                || existing_record.4 != proof_digest
            {
                return Err(StorageError::RefundBalanceProofConflict);
            }
            finalize_refund_execute_intents_for_proof(
                &transaction,
                stream_key,
                execution,
                proof_digest,
            )
            .await?;
            let lease_cleared = transaction
                .execute(
                    "UPDATE indexed_adjustment_refund_executions
                     SET proof_claim_token = NULL, proof_claimed_at = NULL
                     WHERE stream_key = $1 AND signature = $2
                       AND event_index = $3::text::smallint
                       AND proof_claim_token = $4",
                    &[
                        &stream_key,
                        &execution.signature,
                        &execution.event_index.to_string(),
                        &lease_token,
                    ],
                )
                .await?;
            if lease_cleared != 1 {
                return Err(StorageError::RefundBalanceProofLeaseLost);
            }
            transaction.commit().await?;
            return Ok(existing_record.0);
        }

        let row = transaction
            .query_one(
                "INSERT INTO refund_balance_proofs
                    (stream_key, signature, event_index, slot, claim, adjustment,
                     original_receipt, merchant, mint, token_program, tax_vault,
                     refund_source, payer_destination, vault_authority,
                     refund_funder, payer, nonce, gross_atomic, tax_atomic,
                     net_atomic, tax_vault_before_atomic, tax_vault_after_atomic,
                     refund_source_before_atomic, refund_source_after_atomic,
                     payer_destination_before_atomic, payer_destination_after_atomic,
                     proof_digest)
                 VALUES ($1, $2, $3::text::smallint, $4::text::numeric, $5, $6,
                         $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                         $17::text::numeric, $18::text::numeric, $19::text::numeric,
                         $20::text::numeric, $21::text::numeric, $22::text::numeric,
                         $23::text::numeric, $24::text::numeric, $25::text::numeric,
                         $26::text::numeric, $27)
                 RETURNING id",
                &[
                    &stream_key,
                    &execution.signature,
                    &execution.event_index.to_string(),
                    &proof.finalized_slot.to_string(),
                    &&topology.claim[..],
                    &&topology.adjustment[..],
                    &&topology.original_receipt[..],
                    &&topology.merchant[..],
                    &&topology.mint[..],
                    &&topology.token_program[..],
                    &&topology.tax_vault[..],
                    &&topology.refund_source[..],
                    &&topology.payer_destination[..],
                    &&topology.vault_authority[..],
                    &&topology.refund_funder[..],
                    &&topology.payer[..],
                    &execution.nonce.to_string(),
                    &proof.gross_atomic.to_string(),
                    &proof.tax_atomic.to_string(),
                    &proof.net_atomic.to_string(),
                    &proof.tax_vault_before_atomic.to_string(),
                    &proof.tax_vault_after_atomic.to_string(),
                    &proof.refund_source_before_atomic.to_string(),
                    &proof.refund_source_after_atomic.to_string(),
                    &proof.payer_destination_before_atomic.to_string(),
                    &proof.payer_destination_after_atomic.to_string(),
                    &&proof_digest[..],
                ],
            )
            .await?;
        let id = row.get(0);
        finalize_refund_execute_intents_for_proof(
            &transaction,
            stream_key,
            execution,
            proof_digest,
        )
        .await?;
        let lease_cleared = transaction
            .execute(
                "UPDATE indexed_adjustment_refund_executions
                 SET proof_claim_token = NULL, proof_claimed_at = NULL
                 WHERE stream_key = $1 AND signature = $2
                   AND event_index = $3::text::smallint
                   AND proof_claim_token = $4",
                &[
                    &stream_key,
                    &execution.signature,
                    &execution.event_index.to_string(),
                    &lease_token,
                ],
            )
            .await?;
        if lease_cleared != 1 {
            return Err(StorageError::RefundBalanceProofLeaseLost);
        }
        transaction.commit().await?;
        Ok(id)
    }

    /// Claims one server-owned indexed refund execution that has not yet
    /// received immutable balance evidence. The UUIDv7 lease is committed
    /// before external RPC work; an expired lease is reclaimable after the
    /// bounded crash window.
    pub async fn load_pending_refund_balance_proof_execution(
        &self,
        stream_key: &str,
    ) -> Result<Option<RefundBalanceProofCandidate>, StorageError> {
        validate_indexer_stream_key(stream_key)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT execution.signature, execution.slot::text,
                        execution.event_index::text, execution.claim,
                        execution.adjustment, execution.original_receipt,
                        execution.merchant, execution.mint,
                        execution.nonce::text, execution.gross_magnitude::text,
                        execution.tax_magnitude::text, execution.net_magnitude::text,
                        execution.refund_funder, execution.refund_destination,
                        execution.approver, execution.reference_digest,
                        execution.evidence_digest
                 FROM indexed_adjustment_refund_executions execution
                 LEFT JOIN refund_balance_proofs proof
                   ON proof.stream_key = execution.stream_key
                  AND proof.signature = execution.signature
                  AND proof.event_index = execution.event_index
                 WHERE execution.stream_key = $1
                   AND proof.id IS NULL
                   AND execution.proof_claim_dead_lettered_at IS NULL
                   AND execution.proof_claim_attempts < $3::smallint
                   AND (execution.proof_claim_token IS NULL
                        OR execution.proof_claimed_at < now()
                            - ($2::bigint * interval '1 second'))
                 ORDER BY execution.proof_claimed_at NULLS FIRST,
                          execution.slot ASC, execution.signature ASC,
                          execution.event_index ASC
                 FOR UPDATE OF execution SKIP LOCKED
                 LIMIT 1",
                &[
                    &stream_key,
                    &(REFUND_PROOF_CANDIDATE_LEASE.as_secs() as i64),
                    &MAX_REFUND_PROOF_ATTEMPTS,
                ],
            )
            .await?;
        let Some(row) = row else {
            transaction.commit().await?;
            return Ok(None);
        };
        let execution = parse_indexed_adjustment_refund_execution_row(row)?;
        let lease_token = Uuid::now_v7();
        let claimed = transaction
            .execute(
                "UPDATE indexed_adjustment_refund_executions
                 SET proof_claim_token = $4,
                     proof_claimed_at = now(),
                     proof_claim_attempts = proof_claim_attempts + 1,
                     proof_claim_last_error = NULL,
                     proof_claim_dead_lettered_at = NULL
                 WHERE stream_key = $1 AND signature = $2
                   AND event_index = $3::text::smallint
                   AND proof_claim_dead_lettered_at IS NULL
                   AND proof_claim_attempts < $6::smallint
                   AND (proof_claim_token IS NULL
                        OR proof_claimed_at < now()
                            - ($5::bigint * interval '1 second'))",
                &[
                    &stream_key,
                    &execution.signature,
                    &execution.event_index.to_string(),
                    &lease_token,
                    &(REFUND_PROOF_CANDIDATE_LEASE.as_secs() as i64),
                    &MAX_REFUND_PROOF_ATTEMPTS,
                ],
            )
            .await?;
        if claimed != 1 {
            transaction.rollback().await?;
            return Ok(None);
        }
        transaction.commit().await?;
        Ok(Some(RefundBalanceProofCandidate {
            execution,
            lease_token,
        }))
    }

    /// Records a bounded proof-worker failure while fencing the exact lease
    /// owner. The third attempt becomes explicit dead-letter evidence and is
    /// excluded from automatic retries until an operator/reconciliation flow
    /// handles it.
    pub async fn record_refund_proof_candidate_failure(
        &self,
        stream_key: &str,
        execution: &IndexedAdjustmentRefundExecution,
        lease_token: Uuid,
        reason: &str,
    ) -> Result<bool, StorageError> {
        validate_indexer_stream_key(stream_key)?;
        if lease_token.get_version_num() != 7
            || reason.trim().is_empty()
            || reason.len() > MAX_REFUND_PROOF_ERROR_BYTES
            || reason.bytes().any(|byte| byte.is_ascii_control())
        {
            return Err(StorageError::InvalidRefundBalanceProof(
                "refund proof failure identity or reason is invalid".to_string(),
            ));
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let updated = transaction
            .execute(
                "UPDATE indexed_adjustment_refund_executions
                 SET proof_claim_token = NULL,
                     proof_claimed_at = NULL,
                     proof_claim_last_error = $4,
                     proof_claim_dead_lettered_at = CASE
                         WHEN proof_claim_attempts >= $5::smallint THEN now()
                         ELSE NULL
                     END
                 WHERE stream_key = $1 AND signature = $2
                   AND event_index = $3::text::smallint
                   AND proof_claim_token = $6
                   AND proof_claim_dead_lettered_at IS NULL",
                &[
                    &stream_key,
                    &execution.signature,
                    &execution.event_index.to_string(),
                    &reason,
                    &MAX_REFUND_PROOF_ATTEMPTS,
                    &lease_token,
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(updated == 1)
    }

    pub async fn record_reconciliation(
        &self,
        stream_key: &str,
        expected: BalanceExpectation,
        observed: BalanceSnapshot,
    ) -> Result<(Uuid, Reconciliation), StorageError> {
        validate_indexer_stream_key(stream_key)?;
        let result = reconcile(expected, observed);
        let status = match result {
            Reconciliation::Matched => "matched",
            Reconciliation::Mismatch { .. } => "mismatch",
        };
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_one(
                "INSERT INTO reconciliation_runs
                    (stream_key, expected_gross_atomic, expected_tax_atomic,
                     observed_gross_atomic, observed_tax_atomic, status)
                 VALUES ($1, $2::text::numeric, $3::text::numeric,
                         $4::text::numeric, $5::text::numeric, $6)
                 RETURNING id",
                &[
                    &stream_key,
                    &expected.gross_atomic.to_string(),
                    &expected.tax_atomic.to_string(),
                    &observed.gross_atomic.to_string(),
                    &observed.tax_atomic.to_string(),
                    &status,
                ],
            )
            .await?;
        Ok((row.get(0), result))
    }

    /// Creates a durable settlement intent or returns the exact same intent
    /// for a replayed merchant/idempotency/digest tuple.
    pub async fn create_settlement_intent(
        &self,
        merchant_id: Uuid,
        idempotency_key: &str,
        request_digest: [u8; 32],
        instruction_digest: [u8; 32],
    ) -> Result<SettlementIntentOutcome, StorageError> {
        if idempotency_key.trim().is_empty() || idempotency_key.len() > 200 {
            return Err(StorageError::InvalidConfiguration(
                "settlement idempotency key must contain 1 to 200 bytes".into(),
            ));
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let inserted = transaction
            .query_opt(
                "INSERT INTO settlement_intents
                    (merchant_id, idempotency_key, request_digest, instruction_digest)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
                 RETURNING id, merchant_id, idempotency_key, request_digest,
                           instruction_digest, status, signed_transaction_digest,
                           signature, slot::text, commitment, last_error_code",
                &[
                    &merchant_id,
                    &idempotency_key,
                    &&request_digest[..],
                    &&instruction_digest[..],
                ],
            )
            .await?;
        if let Some(row) = inserted {
            let intent = parse_settlement_intent_row(row)?;
            transaction.commit().await?;
            return Ok(SettlementIntentOutcome::Created(intent));
        }
        let row = transaction
            .query_one(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        instruction_digest, status, signed_transaction_digest,
                        signature, slot::text, commitment, last_error_code
                 FROM settlement_intents
                 WHERE merchant_id = $1 AND idempotency_key = $2
                 FOR UPDATE",
                &[&merchant_id, &idempotency_key],
            )
            .await?;
        let intent = parse_settlement_intent_row(row)?;
        if intent.request_digest != request_digest
            || intent.instruction_digest != instruction_digest
        {
            return Err(StorageError::IdempotencyConflict);
        }
        transaction.commit().await?;
        Ok(SettlementIntentOutcome::Replay(intent))
    }

    /// Creates the durable, tenant-scoped mapping between a document payment
    /// authorization and its settlement intent in one serializable
    /// transaction. A replay must return the original immutable mapping; a
    /// reused idempotency key can never silently change payment scope.
    pub async fn create_document_payment_intent(
        &self,
        command: CreateDocumentPaymentIntentCommand,
    ) -> Result<DocumentPaymentIntentOutcome, StorageError> {
        if command.idempotency_key.trim().is_empty() || command.idempotency_key.len() > 200 {
            return Err(StorageError::InvalidConfiguration(
                "document settlement idempotency key must contain 1 to 200 bytes".into(),
            ));
        }
        if command.mint.trim().is_empty() || command.mint.len() > 44 {
            return Err(StorageError::DocumentPaymentAuthorizationIntegrity(
                "settlement mint is invalid".into(),
            ));
        }
        let authorization = &command.authorization;
        if authorization.source_attestation_digest == [0; 32]
            || authorization.payment_scope_digest == [0; 32]
            || authorization.gross_atomic == 0
            || authorization
                .tax_atomic
                .checked_add(authorization.net_atomic)
                != Some(authorization.gross_atomic)
        {
            return Err(StorageError::DocumentPaymentAuthorizationIntegrity(
                "document payment amounts or digests are invalid".into(),
            ));
        }
        let scope_document = serde_json::to_value(&authorization.scope)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let inserted = transaction
            .query_opt(
                "INSERT INTO settlement_intents
                    (merchant_id, idempotency_key, request_digest, instruction_digest)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
                 RETURNING id, merchant_id, idempotency_key, request_digest,
                           instruction_digest, status, signed_transaction_digest,
                           signature, slot::text, commitment, last_error_code",
                &[
                    &command.merchant_id,
                    &command.idempotency_key,
                    &&command.request_digest[..],
                    &&command.instruction_digest[..],
                ],
            )
            .await?;
        let (intent, created) = if let Some(row) = inserted {
            let intent = parse_settlement_intent_row(row)?;
            transaction
                .execute(
                    "INSERT INTO document_payment_authorizations
                        (intent_id, merchant_id, calculation_id, route_id, mint,
                         source_attestation_digest, payment_scope_digest, scope_document,
                         gross_atomic, tax_atomic, net_atomic)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::text::numeric,
                             $10::text::numeric, $11::text::numeric)",
                    &[
                        &intent.id,
                        &command.merchant_id,
                        &command.calculation_id,
                        &command.route_id,
                        &command.mint,
                        &&authorization.source_attestation_digest[..],
                        &&authorization.payment_scope_digest[..],
                        &scope_document,
                        &authorization.gross_atomic.to_string(),
                        &authorization.tax_atomic.to_string(),
                        &authorization.net_atomic.to_string(),
                    ],
                )
                .await?;
            (intent, true)
        } else {
            let row = transaction
                .query_one(
                    "SELECT id, merchant_id, idempotency_key, request_digest,
                            instruction_digest, status, signed_transaction_digest,
                            signature, slot::text, commitment, last_error_code
                     FROM settlement_intents
                     WHERE merchant_id = $1 AND idempotency_key = $2
                     FOR UPDATE",
                    &[&command.merchant_id, &command.idempotency_key],
                )
                .await?;
            (parse_settlement_intent_row(row)?, false)
        };
        if intent.request_digest != command.request_digest
            || intent.instruction_digest != command.instruction_digest
        {
            transaction.rollback().await?;
            return Err(StorageError::IdempotencyConflict);
        }
        let authorization_row = transaction
            .query_opt(
                "SELECT id, intent_id, merchant_id, calculation_id, route_id, mint,
                        source_attestation_digest, payment_scope_digest, scope_document,
                        gross_atomic::text, tax_atomic::text, net_atomic::text
                 FROM document_payment_authorizations
                 WHERE intent_id = $1 AND merchant_id = $2",
                &[&intent.id, &command.merchant_id],
            )
            .await?;
        if !created && authorization_row.is_none() {
            transaction.rollback().await?;
            return Err(StorageError::DocumentPaymentAuthorizationConflict);
        }
        let authorization = authorization_row
            .ok_or(StorageError::DocumentPaymentAuthorizationNotFound)
            .and_then(parse_document_payment_authorization_row)?;
        transaction.commit().await?;
        let outcome = DocumentPaymentIntent {
            intent,
            authorization,
        };
        Ok(if created {
            DocumentPaymentIntentOutcome::Created(outcome)
        } else {
            DocumentPaymentIntentOutcome::Replay(outcome)
        })
    }

    /// Advances an intent only when the caller still owns the expected state.
    /// The status update and append-only event are one transaction.
    pub async fn transition_settlement_intent(
        &self,
        command: SettlementTransition,
    ) -> Result<bool, StorageError> {
        let SettlementTransition {
            intent_id,
            merchant_id,
            expected_status,
            next_status,
            signed_transaction_digest,
            signature,
            slot,
            last_error_code,
            evidence,
        } = command;
        let expected = parse_settlement_status(&expected_status)?;
        let next = parse_settlement_status(&next_status)?;
        if !can_transition(expected, next) {
            return Err(StorageError::InvalidSettlementTransition);
        }
        if evidence.to_string().len() > 64 * 1024 {
            return Err(StorageError::SettlementEvidenceTooLarge);
        }
        if signature
            .as_deref()
            .is_some_and(|value| value.is_empty() || value.len() > 128)
        {
            return Err(StorageError::InvalidConfiguration(
                "settlement signature must contain 1 to 128 bytes".into(),
            ));
        }
        if last_error_code
            .as_deref()
            .is_some_and(|value| value.trim().is_empty() || value.len() > 128)
        {
            return Err(StorageError::InvalidConfiguration(
                "settlement error code must contain 1 to 128 bytes".into(),
            ));
        }
        let slot_text = slot.map(|value| value.to_string());
        let signed_transaction_digest = signed_transaction_digest.map(|value| value.to_vec());
        let commitment = match next {
            SettlementStatus::Processed => Some("processed"),
            SettlementStatus::Confirmed => Some("confirmed"),
            SettlementStatus::Finalized => Some("finalized"),
            _ => None,
        };
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let updated = transaction
            .execute(
                SETTLEMENT_INTENT_TRANSITION_SQL,
                &[
                    &intent_id,
                    &merchant_id,
                    &next_status,
                    &signed_transaction_digest,
                    &signature,
                    &slot_text,
                    &commitment,
                    &last_error_code,
                    &expected_status,
                ],
            )
            .await?;
        if updated != 1 {
            transaction.rollback().await?;
            return Ok(false);
        }
        transaction
            .execute(
                "INSERT INTO settlement_intent_events
                    (intent_id, from_status, to_status, signature, slot, evidence)
                 VALUES ($1, $2, $3, $4, $5::text::numeric, $6)",
                &[
                    &intent_id,
                    &expected_status,
                    &next_status,
                    &signature,
                    &slot_text,
                    &evidence,
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn load_settlement_intent(
        &self,
        intent_id: Uuid,
        merchant_id: Uuid,
    ) -> Result<Option<SettlementIntent>, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        instruction_digest, status, signed_transaction_digest,
                        signature, slot::text, commitment, last_error_code
                 FROM settlement_intents
                 WHERE id = $1 AND merchant_id = $2",
                &[&intent_id, &merchant_id],
            )
            .await?;
        row.map(parse_settlement_intent_row).transpose()
    }

    /// Loads one tenant-bound applicability verification state for the
    /// settlement intent. This read model is evidence-only: `not_applicable`,
    /// `pending`, `retrying`, and `dead_lettered` never authorize signing or
    /// token movement.
    pub async fn load_applicability_binding_verification(
        &self,
        intent_id: Uuid,
        merchant_id: Uuid,
        stream_key: &str,
    ) -> Result<Option<ApplicabilityBindingVerificationView>, StorageError> {
        if intent_id.get_version_num() != 7 || merchant_id.get_version_num() != 7 {
            return Err(StorageError::InvalidApplicabilityBindingVerification(
                "settlement and tenant identities must be UUIDv7".to_string(),
            ));
        }
        validate_indexer_stream_key(stream_key)?;
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT intent.id, intent.signature, intent.slot::text,
                        CASE
                            WHEN verified.event_index IS NOT NULL THEN 'verified'
                            WHEN job.dead_lettered_at IS NOT NULL THEN 'dead_lettered'
                            WHEN job.claim_token IS NOT NULL THEN 'retrying'
                            WHEN job.id IS NOT NULL THEN 'pending'
                            ELSE 'not_applicable'
                        END,
                        COALESCE(verified.event_index::text, job.event_index::text),
                        COALESCE(verified.event_slot::text, job.event_slot::text),
                        verified.provider_slot::text,
                        verified.receipt, verified.binding, verified.merchant,
                        verified.mint, verified.context_id,
                        verified.attestation_digest, verified.event_evidence_digest,
                        verified.verification_commitment,
                        job.attempts::text, job.last_error_code,
                        to_char(verified.verified_at AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                        to_char(job.claimed_at AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                        to_char(job.dead_lettered_at AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')
                 FROM settlement_intents AS intent
                 LEFT JOIN LATERAL (
                     SELECT verification.event_index, verification.event_slot,
                            verification.provider_slot, verification.receipt,
                            verification.binding, verification.merchant,
                            verification.mint, verification.context_id,
                            verification.attestation_digest,
                            verification.event_evidence_digest,
                            verification.verification_commitment,
                            verification.verified_at
                     FROM applicability_binding_verifications AS verification
                     WHERE verification.stream_key = $3
                       AND verification.signature = intent.signature
                       AND verification.event_slot = intent.slot
                     ORDER BY verification.verified_at DESC, verification.id DESC
                     LIMIT 1
                 ) AS verified ON TRUE
                 LEFT JOIN LATERAL (
                     SELECT jobs.id, jobs.event_index, raw.slot AS event_slot,
                            jobs.attempts, jobs.claim_token, jobs.last_error_code,
                            jobs.claimed_at, jobs.dead_lettered_at, jobs.updated_at
                     FROM applicability_binding_verification_jobs AS jobs
                     JOIN raw_chain_evidence AS raw
                       ON raw.stream_key = jobs.stream_key
                      AND raw.signature = jobs.signature
                      AND raw.event_index = jobs.event_index
                     WHERE jobs.stream_key = $3
                       AND jobs.signature = intent.signature
                     ORDER BY jobs.updated_at DESC, jobs.id DESC
                     LIMIT 1
                 ) AS job ON TRUE
                 WHERE intent.id = $1 AND intent.merchant_id = $2",
                &[&intent_id, &merchant_id, &stream_key],
            )
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let parse_optional_u16 = |value: Option<String>, field: &str| {
            value
                .map(|value| {
                    value.parse::<u16>().map_err(|_| {
                        StorageError::InvalidApplicabilityBindingVerification(format!(
                            "stored {field} is outside the bounded range"
                        ))
                    })
                })
                .transpose()
        };
        let parse_optional_pubkey = |value: Option<Vec<u8>>, field: &str| {
            value
                .map(|value| {
                    let bytes: [u8; 32] = value.try_into().map_err(|value: Vec<u8>| {
                        StorageError::InvalidApplicabilityBindingVerification(format!(
                            "stored {field} must contain 32 bytes, got {}",
                            value.len()
                        ))
                    })?;
                    Ok::<String, StorageError>(Pubkey::new_from_array(bytes).to_string())
                })
                .transpose()
        };
        let parse_optional_hex = |value: Option<Vec<u8>>, field: &str| {
            value
                .map(|value| {
                    if value.is_empty() || value.len() > 32 {
                        return Err(StorageError::InvalidApplicabilityBindingVerification(
                            format!("stored {field} has an invalid byte length"),
                        ));
                    }
                    Ok::<String, StorageError>(
                        value.iter().map(|byte| format!("{byte:02x}")).collect(),
                    )
                })
                .transpose()
        };
        Ok(Some(ApplicabilityBindingVerificationView {
            settlement_intent_id: row.get(0),
            status: row.get(3),
            signature: row.get(1),
            event_index: parse_optional_u16(row.get(4), "event index")?,
            event_slot: row.get(5),
            provider_slot: row.get(6),
            receipt: parse_optional_pubkey(row.get(7), "receipt")?,
            binding: parse_optional_pubkey(row.get(8), "binding")?,
            merchant: parse_optional_pubkey(row.get(9), "merchant")?,
            mint: parse_optional_pubkey(row.get(10), "mint")?,
            context_id_hex: parse_optional_hex(row.get(11), "context ID")?,
            attestation_digest_hex: parse_optional_hex(row.get(12), "attestation digest")?,
            event_evidence_digest_hex: parse_optional_hex(row.get(13), "event evidence digest")?,
            verification_commitment: row.get(14),
            attempts: parse_optional_u16(row.get(15), "attempt count")?,
            last_error_code: row.get(16),
            verified_at: row.get(17),
            claimed_at: row.get(18),
            dead_lettered_at: row.get(19),
        }))
    }

    /// Resolves an adjustment's original settlement from server-owned,
    /// finalized evidence. The UUID is the durable settlement-intent ID; the
    /// receipt projection is joined by the finalized transaction signature
    /// and slot. Identical rows from multiple indexer streams collapse through
    /// DISTINCT, while conflicting rows are rejected rather than selected by
    /// ordering.
    pub async fn load_finalized_settlement_receipt_binding(
        &self,
        merchant_id: Uuid,
        settlement_id: Uuid,
        asset_id: [u8; 32],
    ) -> Result<SettlementReceiptBinding, StorageError> {
        if merchant_id.get_version_num() != 7
            || settlement_id.get_version_num() != 7
            || asset_id == [0; 32]
        {
            return Err(StorageError::InvalidSettlementReceiptBinding(
                "tenant, settlement, and asset identity must be non-zero UUIDv7/bytes".into(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT DISTINCT receipt.signature, receipt.slot::text,
                        receipt.receipt, receipt.payer, receipt.merchant,
                        receipt.mint, receipt.nonce::text,
                        receipt.gross_atomic::text, receipt.tax_atomic::text,
                        receipt.net_atomic::text, receipt.policy_digest,
                        receipt.evidence_digest
                 FROM settlement_intents intent
                 JOIN indexed_receipts receipt
                   ON receipt.signature = intent.signature
                  AND receipt.slot = intent.slot
                  AND receipt.mint = $3
                 WHERE intent.id = $1
                   AND intent.merchant_id = $2
                   AND intent.status = 'finalized'
                   AND intent.commitment = 'finalized'
                   AND intent.signature IS NOT NULL
                   AND intent.slot IS NOT NULL
                 LIMIT 2",
                &[&settlement_id, &merchant_id, &&asset_id[..]],
            )
            .await?;
        match rows.len() {
            0 => Err(StorageError::SettlementReceiptBindingNotFound),
            1 => match rows.into_iter().next() {
                Some(row) => parse_settlement_receipt_binding_row(row, settlement_id),
                None => Err(StorageError::InvalidSettlementReceiptBinding(
                    "receipt query cardinality changed unexpectedly".into(),
                )),
            },
            _ => Err(StorageError::InvalidSettlementReceiptBinding(
                "multiple conflicting finalized receipt projections matched the settlement".into(),
            )),
        }
    }

    /// Creates a short-lived, single-use wallet proof challenge. The message
    /// bytes are returned to the wallet caller but only their digest is
    /// persisted; the binding operation reconstructs and verifies the exact
    /// canonical message before consuming the challenge.
    pub async fn create_adjustment_authority_challenge(
        &self,
        command: CreateAdjustmentAuthorityChallenge,
    ) -> Result<AdjustmentAuthorityChallenge, StorageError> {
        validate_adjustment_authority_challenge_command(&command)?;
        let id = Uuid::now_v7();
        let expires_at_unix = command
            .now_unix
            .checked_add(command.ttl_seconds)
            .ok_or_else(|| {
                StorageError::InvalidAdjustmentAuthorityChallenge(
                    "challenge expiry overflows the supported clock range".into(),
                )
            })?;
        let message = authority_challenge_message(
            command.merchant_id,
            id,
            &command.subject,
            &command.role,
            expires_at_unix,
        )
        .map_err(|error| StorageError::InvalidAdjustmentAuthorityChallenge(error.to_string()))?;
        let message_digest = authority_challenge_digest(&message);
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        client
            .execute(
                "INSERT INTO settlement_adjustment_authority_challenges
                    (id, merchant_id, subject, role, message_digest,
                     expires_at, created_at)
                 VALUES ($1, $2, $3, $4, $5,
                         to_timestamp($6::double precision),
                         to_timestamp($7::double precision))",
                &[
                    &id,
                    &command.merchant_id,
                    &command.subject,
                    &command.role,
                    &&message_digest[..],
                    &(expires_at_unix as f64),
                    &(command.now_unix as f64),
                ],
            )
            .await?;
        Ok(AdjustmentAuthorityChallenge {
            id,
            merchant_id: command.merchant_id,
            subject: command.subject,
            role: command.role,
            message,
            message_digest,
            expires_at_unix,
        })
    }

    pub async fn load_adjustment_authority_challenge(
        &self,
        merchant_id: Uuid,
        challenge_id: Uuid,
    ) -> Result<AdjustmentAuthorityChallenge, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT subject, role, message_digest,
                        floor(extract(epoch FROM expires_at))::bigint
                 FROM settlement_adjustment_authority_challenges
                 WHERE id = $1 AND merchant_id = $2",
                &[&challenge_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::AdjustmentAuthorityChallengeNotFound)?;
        let subject: String = row.get(0);
        let role: String = row.get(1);
        let expires_at_unix: i64 = row.get(3);
        let message = authority_challenge_message(
            merchant_id,
            challenge_id,
            &subject,
            &role,
            expires_at_unix,
        )
        .map_err(|error| StorageError::InvalidAdjustmentAuthorityChallenge(error.to_string()))?;
        let message_digest = authority_challenge_digest(&message);
        let stored_digest: [u8; 32] = parse_digest(row.get(2)).map_err(|_| {
            StorageError::InvalidAdjustmentAuthorityChallenge(
                "stored challenge digest is not 32 bytes".into(),
            )
        })?;
        if message_digest != stored_digest {
            return Err(StorageError::InvalidAdjustmentAuthorityChallenge(
                "stored challenge message digest does not match canonical bytes".into(),
            ));
        }
        Ok(AdjustmentAuthorityChallenge {
            id: challenge_id,
            merchant_id,
            subject,
            role,
            message,
            message_digest,
            expires_at_unix,
        })
    }

    /// Verifies and consumes one wallet challenge, then atomically rotates the
    /// active subject/role binding. Signature verification itself lives at the
    /// API boundary; this method enforces the durable replay and uniqueness
    /// fences inside PostgreSQL.
    pub async fn bind_adjustment_authority(
        &self,
        command: BindAdjustmentAuthority,
    ) -> Result<AdjustmentAuthorityBinding, StorageError> {
        validate_adjustment_authority_binding_command(&command)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let challenge = transaction
            .query_opt(
                "SELECT subject, role, message_digest,
                        floor(extract(epoch FROM expires_at))::bigint,
                        consumed_at IS NOT NULL,
                        expires_at <= to_timestamp($3::double precision)
                 FROM settlement_adjustment_authority_challenges
                 WHERE id = $1 AND merchant_id = $2
                 FOR UPDATE",
                &[
                    &command.challenge_id,
                    &command.merchant_id,
                    &(command.now_unix as f64),
                ],
            )
            .await?
            .ok_or(StorageError::AdjustmentAuthorityChallengeNotFound)?;
        let subject: String = challenge.get(0);
        let role: String = challenge.get(1);
        let stored_digest: Vec<u8> = challenge.get(2);
        let expires_at_unix: i64 = challenge.get(3);
        let consumed: bool = challenge.get(4);
        let expired: bool = challenge.get(5);
        if consumed
            || expired
            || subject != command.subject
            || role != command.role
            || authority_challenge_digest(
                &authority_challenge_message(
                    command.merchant_id,
                    command.challenge_id,
                    &subject,
                    &role,
                    expires_at_unix,
                )
                .map_err(|error| {
                    StorageError::InvalidAdjustmentAuthorityChallenge(error.to_string())
                })?,
            ) != parse_digest(stored_digest).map_err(|_| {
                StorageError::InvalidAdjustmentAuthorityChallenge(
                    "stored challenge digest is not 32 bytes".into(),
                )
            })?
        {
            return Err(StorageError::AdjustmentAuthorityChallengeNotFound);
        }
        let active_key_conflict = transaction
            .query_opt(
                "SELECT 1
                 FROM settlement_adjustment_authority_bindings
                 WHERE merchant_id = $1 AND public_key = $2 AND status = 'active'
                 FOR UPDATE",
                &[&command.merchant_id, &&command.public_key[..]],
            )
            .await?
            .is_some();
        if active_key_conflict {
            return Err(StorageError::AdjustmentAuthorityConflict);
        }
        transaction
            .execute(
                "UPDATE settlement_adjustment_authority_bindings
                 SET status = 'revoked', revoked_at = now()
                 WHERE merchant_id = $1 AND subject = $2 AND role = $3
                   AND status = 'active'",
                &[&command.merchant_id, &command.subject, &command.role],
            )
            .await?;
        let id = Uuid::now_v7();
        let row = transaction
            .query_one(
                "INSERT INTO settlement_adjustment_authority_bindings
                    (id, merchant_id, subject, role, public_key, challenge_id,
                     proof_signature)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 RETURNING id, merchant_id, subject, role, public_key,
                           challenge_id, proof_signature",
                &[
                    &id,
                    &command.merchant_id,
                    &command.subject,
                    &command.role,
                    &&command.public_key[..],
                    &command.challenge_id,
                    &&command.proof_signature[..],
                ],
            )
            .await?;
        transaction
            .execute(
                "UPDATE settlement_adjustment_authority_challenges
                 SET consumed_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND consumed_at IS NULL",
                &[&command.challenge_id, &command.merchant_id],
            )
            .await?;
        let binding = parse_adjustment_authority_binding_row(row)?;
        transaction.commit().await?;
        Ok(binding)
    }

    pub async fn load_active_adjustment_authority(
        &self,
        merchant_id: Uuid,
        subject: &str,
        role: &str,
    ) -> Result<AdjustmentAuthorityBinding, StorageError> {
        if !valid_adjustment_authority_role(role) || subject.trim().is_empty() {
            return Err(StorageError::AdjustmentAuthorityBindingNotFound);
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT id, merchant_id, subject, role, public_key,
                        challenge_id, proof_signature
                 FROM settlement_adjustment_authority_bindings
                 WHERE merchant_id = $1 AND subject = $2 AND role = $3
                   AND status = 'active'",
                &[&merchant_id, &subject, &role],
            )
            .await?
            .ok_or(StorageError::AdjustmentAuthorityBindingNotFound)?;
        parse_adjustment_authority_binding_row(row)
    }

    /// Reserves one server-generated chain nonce for an approved adjustment.
    /// Retries converge on the immutable existing intent and cannot change
    /// authority or receipt identity snapshots.
    pub async fn reserve_settlement_adjustment_evidence_intent(
        &self,
        command: CreateSettlementAdjustmentEvidenceIntent,
    ) -> Result<SettlementAdjustmentEvidenceIntent, StorageError> {
        validate_adjustment_evidence_intent_command(&command)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let registry_slot = command.registry_slot.to_string();
        let inserted = transaction
            .query_opt(
                "INSERT INTO settlement_adjustment_evidence_intents
                    (adjustment_id, merchant_id, proposer, approver,
                     original_receipt, merchant, mint, token_program,
                     registry_address, registry_slot, registry_commitment,
                     registry_account_digest, request_digest)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::text::numeric, $11,
                         $12, $13)
                 ON CONFLICT (adjustment_id, merchant_id) DO NOTHING
                 RETURNING id, adjustment_id, merchant_id, nonce::text,
                           proposer, approver, original_receipt, merchant,
                           mint, token_program, registry_address,
                           registry_slot::text, registry_commitment,
                           registry_account_digest, request_digest,
                           instruction_digest, status",
                &[
                    &command.adjustment_id,
                    &command.merchant_id,
                    &&command.proposer[..],
                    &&command.approver[..],
                    &&command.original_receipt[..],
                    &&command.merchant[..],
                    &&command.mint[..],
                    &command.token_program,
                    &&command.registry_address[..],
                    &registry_slot,
                    &command.registry_commitment,
                    &&command.registry_account_digest[..],
                    &&command.request_digest[..],
                ],
            )
            .await?;
        let row = if let Some(row) = inserted {
            row
        } else {
            transaction
                .query_one(
                    "SELECT id, adjustment_id, merchant_id, nonce::text,
                            proposer, approver, original_receipt, merchant,
                            mint, token_program, registry_address,
                            registry_slot::text, registry_commitment,
                            registry_account_digest, request_digest,
                            instruction_digest, status
                     FROM settlement_adjustment_evidence_intents
                     WHERE adjustment_id = $1 AND merchant_id = $2
                     FOR UPDATE",
                    &[&command.adjustment_id, &command.merchant_id],
                )
                .await?
        };
        let intent = parse_settlement_adjustment_evidence_intent_row(row)?;
        if intent.request_digest != command.request_digest
            || intent.proposer != command.proposer
            || intent.approver != command.approver
            || intent.original_receipt != command.original_receipt
            || intent.merchant != command.merchant
            || intent.mint != command.mint
            || intent.token_program != command.token_program
            || intent.registry_address != command.registry_address
            || intent.registry_slot != command.registry_slot
            || intent.registry_commitment != command.registry_commitment
            || intent.registry_account_digest != command.registry_account_digest
        {
            return Err(StorageError::AdjustmentEvidenceIntentConflict);
        }
        transaction.commit().await?;
        Ok(intent)
    }

    pub async fn bind_settlement_adjustment_evidence_intent_digest(
        &self,
        merchant_id: Uuid,
        intent_id: Uuid,
        instruction_digest: [u8; 32],
    ) -> Result<SettlementAdjustmentEvidenceIntent, StorageError> {
        if instruction_digest == [0; 32] {
            return Err(StorageError::InvalidAdjustmentEvidenceIntent(
                "instruction digest must be non-zero".into(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let updated = client
            .execute(
                "UPDATE settlement_adjustment_evidence_intents
                 SET instruction_digest = $3
                 WHERE id = $1 AND merchant_id = $2
                   AND (instruction_digest IS NULL OR instruction_digest = $3)",
                &[&intent_id, &merchant_id, &&instruction_digest[..]],
            )
            .await?;
        if updated != 1 {
            let existing = client
                .query_opt(
                    "SELECT id, adjustment_id, merchant_id, nonce::text,
                            proposer, approver, original_receipt, merchant,
                            mint, token_program, registry_address,
                            registry_slot::text, registry_commitment,
                            registry_account_digest, request_digest,
                            instruction_digest, status
                     FROM settlement_adjustment_evidence_intents
                     WHERE id = $1 AND merchant_id = $2",
                    &[&intent_id, &merchant_id],
                )
                .await?
                .ok_or(StorageError::AdjustmentEvidenceIntentNotFound)?;
            let intent = parse_settlement_adjustment_evidence_intent_row(existing)?;
            if intent.instruction_digest != Some(instruction_digest) {
                return Err(StorageError::AdjustmentEvidenceIntentConflict);
            }
        }
        let row = client
            .query_one(
                "SELECT id, adjustment_id, merchant_id, nonce::text,
                        proposer, approver, original_receipt, merchant,
                        mint, token_program, registry_address,
                        registry_slot::text, registry_commitment,
                        registry_account_digest, request_digest,
                        instruction_digest, status
                 FROM settlement_adjustment_evidence_intents
                 WHERE id = $1 AND merchant_id = $2",
                &[&intent_id, &merchant_id],
            )
            .await?;
        parse_settlement_adjustment_evidence_intent_row(row)
    }

    pub async fn reserve_settlement_adjustment_chain_intent(
        &self,
        command: CreateSettlementAdjustmentChainIntent,
    ) -> Result<SettlementAdjustmentChainIntent, StorageError> {
        if command.evidence_intent_id.get_version_num() != 7
            || command.adjustment_id.get_version_num() != 7
            || command.merchant_id.get_version_num() != 7
            || command.instruction_digest == [0; 32]
        {
            return Err(StorageError::AdjustmentChainIntentConflict);
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "INSERT INTO settlement_adjustment_chain_intents
                    (evidence_intent_id, adjustment_id, merchant_id, instruction_digest)
                 SELECT $1, $2, $3, $4
                 WHERE EXISTS (
                    SELECT 1 FROM settlement_adjustment_evidence_intents evidence
                    WHERE evidence.id = $1 AND evidence.merchant_id = $3
                      AND evidence.adjustment_id = $2
                      AND evidence.instruction_digest = $4
                      AND evidence.status = 'prepared_unsubmitted'
                 )
                 ON CONFLICT (evidence_intent_id, merchant_id) DO NOTHING
                 RETURNING id, evidence_intent_id, adjustment_id, merchant_id,
                           instruction_digest, status, signed_transaction_digest,
                           signature, slot::text, commitment, last_error_code,
                           finalized_event_index, finalized_event_evidence_digest",
                &[
                    &command.evidence_intent_id,
                    &command.adjustment_id,
                    &command.merchant_id,
                    &&command.instruction_digest[..],
                ],
            )
            .await?;
        let row = match row {
            Some(row) => row,
            None => transaction
                .query_opt(
                    "SELECT id, evidence_intent_id, adjustment_id, merchant_id,
                            instruction_digest, status, signed_transaction_digest,
                            signature, slot::text, commitment, last_error_code,
                            finalized_event_index, finalized_event_evidence_digest
                     FROM settlement_adjustment_chain_intents
                     WHERE evidence_intent_id = $1 AND merchant_id = $2
                     FOR UPDATE",
                    &[&command.evidence_intent_id, &command.merchant_id],
                )
                .await?
                .ok_or(StorageError::AdjustmentChainIntentConflict)?,
        };
        let intent = parse_settlement_adjustment_chain_intent_row(row)?;
        if intent.adjustment_id != command.adjustment_id
            || intent.merchant_id != command.merchant_id
            || intent.instruction_digest != command.instruction_digest
        {
            return Err(StorageError::AdjustmentChainIntentConflict);
        }
        transaction.commit().await?;
        Ok(intent)
    }

    pub async fn transition_settlement_adjustment_chain_intent(
        &self,
        command: SettlementAdjustmentChainTransition,
    ) -> Result<bool, StorageError> {
        let expected = parse_settlement_status(&command.expected_status)?;
        let next = parse_settlement_status(&command.next_status)?;
        if !can_transition(expected, next) || command.evidence.to_string().len() > 64 * 1024 {
            return Err(StorageError::InvalidAdjustmentChainTransition);
        }
        if command
            .signature
            .as_deref()
            .is_some_and(|value| value.is_empty() || value.len() > 128)
            || command
                .last_error_code
                .as_deref()
                .is_some_and(|value| value.trim().is_empty() || value.len() > 128)
        {
            return Err(StorageError::InvalidAdjustmentChainTransition);
        }
        if matches!(next, SettlementStatus::Unknown) && command.last_error_code.is_none() {
            return Err(StorageError::InvalidAdjustmentChainTransition);
        }
        let has_finalized_proof = command.finalized_event_index.is_some()
            || command.finalized_event_evidence_digest.is_some();
        if next == SettlementStatus::Finalized {
            if command.signature.is_none()
                || command.slot.is_none()
                || command.finalized_event_index.is_none()
                || command
                    .finalized_event_evidence_digest
                    .is_none_or(|digest| digest == [0; 32])
            {
                return Err(StorageError::InvalidAdjustmentChainTransition);
            }
        } else if has_finalized_proof {
            return Err(StorageError::InvalidAdjustmentChainTransition);
        }
        let commitment = match next {
            SettlementStatus::Processed => Some("processed"),
            SettlementStatus::Confirmed => Some("confirmed"),
            SettlementStatus::Finalized => Some("finalized"),
            _ => None,
        };
        let slot_text = command.slot.map(|value| value.to_string());
        let finalized_event_index = command
            .finalized_event_index
            .map(i16::try_from)
            .transpose()
            .map_err(|_| StorageError::InvalidAdjustmentChainTransition)?;
        let finalized_event_evidence_digest = command
            .finalized_event_evidence_digest
            .map(|value| value.to_vec());
        let signed_digest = command
            .signed_transaction_digest
            .map(|value| value.to_vec());
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let updated = transaction
            .execute(
                "UPDATE settlement_adjustment_chain_intents
                 SET status = $3, signed_transaction_digest = COALESCE($4, signed_transaction_digest),
                     signature = COALESCE($5, signature), slot = COALESCE($6::text::numeric, slot),
                     commitment = COALESCE($7, commitment), last_error_code = $8,
                     finalized_event_index = COALESCE($9, finalized_event_index),
                     finalized_event_evidence_digest = COALESCE($10, finalized_event_evidence_digest)
                 WHERE id = $1 AND merchant_id = $2 AND status = $11",
                &[
                    &command.intent_id,
                    &command.merchant_id,
                    &command.next_status,
                    &signed_digest,
                    &command.signature,
                    &slot_text,
                    &commitment,
                    &command.last_error_code,
                    &finalized_event_index,
                    &finalized_event_evidence_digest,
                    &command.expected_status,
                ],
            )
            .await?;
        if updated != 1 {
            transaction.rollback().await?;
            return Ok(false);
        }
        transaction
            .execute(
                "INSERT INTO settlement_adjustment_chain_intent_events
                    (intent_id, merchant_id, from_status, to_status, signature, slot, evidence)
                 VALUES ($1, $2, $3, $4, $5, $6::text::numeric, $7)",
                &[
                    &command.intent_id,
                    &command.merchant_id,
                    &command.expected_status,
                    &command.next_status,
                    &command.signature,
                    &slot_text,
                    &command.evidence,
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn load_settlement_adjustment_chain_intent(
        &self,
        intent_id: Uuid,
        merchant_id: Uuid,
    ) -> Result<Option<SettlementAdjustmentChainIntent>, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT id, evidence_intent_id, adjustment_id, merchant_id,
                        instruction_digest, status, signed_transaction_digest,
                        signature, slot::text, commitment, last_error_code,
                        finalized_event_index, finalized_event_evidence_digest
                 FROM settlement_adjustment_chain_intents
                 WHERE id = $1 AND merchant_id = $2",
                &[&intent_id, &merchant_id],
            )
            .await?;
        row.map(parse_settlement_adjustment_chain_intent_row)
            .transpose()
    }

    pub async fn load_settlement_adjustment_evidence_intent(
        &self,
        intent_id: Uuid,
        merchant_id: Uuid,
    ) -> Result<Option<SettlementAdjustmentEvidenceIntent>, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT id, adjustment_id, merchant_id, nonce::text,
                        proposer, approver, original_receipt, merchant, mint,
                        token_program, registry_address, registry_slot::text,
                        registry_commitment, registry_account_digest,
                        request_digest, instruction_digest, status
                 FROM settlement_adjustment_evidence_intents
                 WHERE id = $1 AND merchant_id = $2",
                &[&intent_id, &merchant_id],
            )
            .await?;
        row.map(parse_settlement_adjustment_evidence_intent_row)
            .transpose()
    }

    pub async fn load_finalized_settlement_adjustment_record(
        &self,
        intent_id: Uuid,
        merchant_id: Uuid,
        expected: &IndexedSettlementAdjustment,
    ) -> Result<Option<IndexedSettlementAdjustment>, StorageError> {
        indexer::merge_settlement_adjustment(None, expected.clone())?;
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT indexed.signature, indexed.slot::text, indexed.event_index::text,
                        indexed.adjustment, indexed.original_receipt, indexed.merchant,
                        indexed.mint, indexed.nonce::text, indexed.kind::text,
                        indexed.gross_sign::text, indexed.gross_magnitude::text,
                        indexed.tax_sign::text, indexed.tax_magnitude::text,
                        indexed.net_sign::text, indexed.net_magnitude::text,
                        indexed.adjustment_digest, indexed.adjustment_evidence_digest,
                        indexed.evidence_digest, indexed.approver
                 FROM settlement_adjustment_chain_intents chain
                 JOIN indexed_settlement_adjustments indexed
                   ON indexed.signature = chain.signature
                 JOIN raw_chain_evidence raw
                   ON raw.stream_key = indexed.stream_key
                  AND raw.signature = indexed.signature
                  AND raw.event_index = indexed.event_index
                 WHERE chain.id = $1 AND chain.merchant_id = $2
                   AND chain.signature = $3
                   AND (chain.finalized_event_index IS NULL
                        OR indexed.event_index = chain.finalized_event_index)
                   AND (chain.finalized_event_evidence_digest IS NULL
                        OR indexed.evidence_digest = chain.finalized_event_evidence_digest)
                   AND indexed.slot = $4::text::numeric
                   AND indexed.adjustment = $5
                   AND indexed.original_receipt = $6
                   AND indexed.merchant = $7
                   AND indexed.mint = $8
                   AND indexed.nonce = $9::text::numeric
                   AND indexed.kind = $10::text::smallint
                   AND indexed.gross_sign = $11::text::smallint
                   AND indexed.gross_magnitude = $12::text::numeric
                   AND indexed.tax_sign = $13::text::smallint
                   AND indexed.tax_magnitude = $14::text::numeric
                   AND indexed.net_sign = $15::text::smallint
                   AND indexed.net_magnitude = $16::text::numeric
                   AND indexed.adjustment_digest = $17
                   AND indexed.adjustment_evidence_digest = $18
                   AND indexed.approver = $19
                   AND raw.event_discriminator = $20
                 LIMIT 2",
                &[
                    &intent_id,
                    &merchant_id,
                    &expected.signature,
                    &expected.slot.to_string(),
                    &&expected.adjustment[..],
                    &&expected.original_receipt[..],
                    &&expected.merchant[..],
                    &&expected.mint[..],
                    &expected.nonce.to_string(),
                    &expected.kind.to_string(),
                    &expected.gross_sign.to_string(),
                    &expected.gross_magnitude.to_string(),
                    &expected.tax_sign.to_string(),
                    &expected.tax_magnitude.to_string(),
                    &expected.net_sign.to_string(),
                    &expected.net_magnitude.to_string(),
                    &&expected.adjustment_digest[..],
                    &&expected.adjustment_evidence_digest[..],
                    &&expected.approver[..],
                    &&indexer::SETTLEMENT_ADJUSTMENT_RECORDED_EVENT_DISCRIMINATOR[..],
                ],
            )
            .await?;
        match rows.len() {
            0 => Ok(None),
            1 => rows
                .into_iter()
                .next()
                .map(parse_indexed_settlement_adjustment_row)
                .transpose(),
            _ => Err(StorageError::InvalidIndexerState(
                "multiple conflicting finalized adjustment projections matched".into(),
            )),
        }
    }

    /// Applies one authenticated tax-rule governance transition and records
    /// the transition evidence in the same database transaction. The
    /// lifecycle is intentionally narrow: subjects are assigned at their
    /// respective gates and the approver remains the publication authority.
    pub async fn transition_rule_set(
        &self,
        command: RuleGovernanceTransition,
    ) -> Result<RuleGovernanceResult, StorageError> {
        validate_rule_governance_command(&command)?;
        let action = rule_governance_action(&command.to_status)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;

        let row = transaction
            .query_opt(
                "SELECT status, author_subject, reviewer_subject, approver_subject,
                        governance_revision, provenance, jurisdiction
                 FROM rule_sets
                 WHERE id = $1 AND merchant_id = $2
                 FOR UPDATE",
                &[&command.rule_set_id, &command.merchant_id],
            )
            .await?
            .ok_or(StorageError::RuleGovernanceNotFound)?;
        let from_status: String = row.get(0);
        let author_subject: Option<String> = row.get(1);
        let reviewer_subject: Option<String> = row.get(2);
        let approver_subject: Option<String> = row.get(3);
        let governance_revision: i64 = row.get(4);
        let provenance: Value = row.get(5);
        let jurisdiction: String = row.get(6);

        if command.to_status == "published" {
            validate_rule_release_provenance(&provenance, &jurisdiction)?;
        }

        let (author_subject, reviewer_subject, approver_subject) = assign_governance_subjects(
            &from_status,
            &command.to_status,
            &command.actor_subject,
            author_subject,
            reviewer_subject,
            approver_subject,
        )?;
        let next_revision = governance_revision
            .checked_add(1)
            .ok_or(StorageError::InvalidRuleGovernanceTransition)?;
        let updated = transaction
            .execute(
                "UPDATE rule_sets
                 SET status = $3,
                     author_subject = $4,
                     reviewer_subject = $5,
                     approver_subject = $6,
                     governance_revision = $7,
                     updated_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND status = $8
                   AND governance_revision = $9",
                &[
                    &command.rule_set_id,
                    &command.merchant_id,
                    &command.to_status,
                    &author_subject,
                    &reviewer_subject,
                    &approver_subject,
                    &next_revision,
                    &from_status,
                    &governance_revision,
                ],
            )
            .await?;
        if updated != 1 {
            transaction.rollback().await?;
            return Err(StorageError::RuleGovernanceConflict);
        }
        transaction
            .execute(
                "INSERT INTO rule_governance_events
                    (rule_set_id, actor_subject, action, from_status, to_status, evidence)
                 VALUES ($1, $2, $3, $4, $5, $6)",
                &[
                    &command.rule_set_id,
                    &command.actor_subject,
                    &action,
                    &from_status,
                    &command.to_status,
                    &command.evidence,
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(RuleGovernanceResult {
            rule_set_id: command.rule_set_id,
            from_status,
            to_status: command.to_status,
            author_subject,
            reviewer_subject,
            approver_subject,
            governance_revision: next_revision,
        })
    }

    /// Returns one repeatable-read snapshot of the rule governance row and its
    /// append-only event history for the authenticated control plane.
    pub async fn load_rule_governance(
        &self,
        rule_set_id: Uuid,
        merchant_id: Uuid,
    ) -> Result<RuleGovernanceState, StorageError> {
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::RepeatableRead)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT merchant_id, id, external_id, version, jurisdiction, regime, status,
                        encode(rule_digest, 'hex'), author_subject, reviewer_subject,
                        approver_subject, governance_revision, effective_from::text,
                        effective_until::text
                 FROM rule_sets
                 WHERE id = $1 AND merchant_id = $2",
                &[&rule_set_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::RuleGovernanceNotFound)?;
        let events = transaction
            .query(
                "SELECT events.id, events.actor_subject, events.action,
                        events.from_status, events.to_status, events.evidence,
                        events.occurred_at::text
                 FROM rule_governance_events AS events
                 JOIN rule_sets AS rules
                   ON rules.id = events.rule_set_id
                  AND rules.merchant_id = $2
                 WHERE events.rule_set_id = $1
                 ORDER BY occurred_at ASC, id ASC",
                &[&rule_set_id, &merchant_id],
            )
            .await?
            .into_iter()
            .map(|event| RuleGovernanceEvent {
                id: event.get(0),
                actor_subject: event.get(1),
                action: event.get(2),
                from_status: event.get(3),
                to_status: event.get(4),
                evidence: event.get(5),
                occurred_at: event.get(6),
            })
            .collect();
        let state = RuleGovernanceState {
            merchant_id: row.get(0),
            rule_set_id: row.get(1),
            external_id: row.get(2),
            version: row.get(3),
            jurisdiction: row.get(4),
            regime: row.get(5),
            status: row.get(6),
            rule_digest_hex: row.get(7),
            author_subject: row.get(8),
            reviewer_subject: row.get(9),
            approver_subject: row.get(10),
            governance_revision: row.get(11),
            effective_from: row.get(12),
            effective_until: row.get(13),
            events,
        };
        transaction.commit().await?;
        Ok(state)
    }

    /// Calculates against the immutable, tenant-scoped rule document stored
    /// by governance. This is deliberately ephemeral: it creates no quote,
    /// attestation, outbox event, or settlement authority.
    pub async fn simulate_persisted_rule(
        &self,
        rule_set_id: Uuid,
        merchant_id: Uuid,
        facts: &TaxFacts,
    ) -> Result<CalculationResult, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT external_id, version, jurisdiction, status,
                        rule_document, rule_digest
                 FROM rule_sets
                 WHERE id = $1 AND merchant_id = $2",
                &[&rule_set_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::RuleSimulationNotFound)?;
        let rule_set = validate_persisted_rule_for_simulation(
            row.get(0),
            row.get(1),
            row.get(2),
            row.get(3),
            row.get(4),
            row.get::<_, Vec<u8>>(5).as_slice(),
            facts,
        )?;
        let computed_digest = rule_set.canonical_digest().map_err(|error| {
            StorageError::RuleSimulationIntegrity(format!(
                "rule digest cannot be computed: {error}"
            ))
        })?;
        self.compiled_rule_cache
            .get_or_compile(computed_digest, &rule_set)?
            .calculate(facts)
            .map_err(StorageError::from)
    }

    /// Creates an authenticated, tenant-owned jurisdiction-policy draft with
    /// its creation event and idempotency response in one transaction.
    pub async fn create_jurisdiction_policy(
        &self,
        command: CreateJurisdictionPolicyCommand,
    ) -> Result<JurisdictionPolicyDraftOutcome, StorageError> {
        validate_jurisdiction_policy_draft_command(&command)?;
        let policy_digest = command
            .policy
            .canonical_digest()
            .map_err(|error| StorageError::InvalidJurisdictionPolicyDraft(error.to_string()))?;
        let request_digest = jurisdiction_policy_request_digest(&command, policy_digest)?;
        let idempotency_key = format!("jurisdiction-policy/{}", command.idempotency_key);
        let policy_document = serde_json::to_value(&command.policy)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;

        let inserted = transaction
            .query_opt(
                "INSERT INTO idempotency_records
                    (merchant_id, idempotency_key, request_digest)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
                 RETURNING id",
                &[&command.merchant_id, &idempotency_key, &&request_digest[..]],
            )
            .await?;
        if inserted.is_none() {
            let existing = transaction
                .query_one(
                    "SELECT request_digest, response_document
                     FROM idempotency_records
                     WHERE merchant_id = $1 AND idempotency_key = $2
                     FOR UPDATE",
                    &[&command.merchant_id, &idempotency_key],
                )
                .await?;
            let existing_digest: Vec<u8> = existing.get(0);
            if existing_digest.as_slice() != request_digest {
                return Err(StorageError::IdempotencyConflict);
            }
            let stored_response: Option<Value> = existing.get(1);
            let Some(stored_response) = stored_response else {
                return Err(StorageError::IdempotencyInProgress);
            };
            let response: CreatedJurisdictionPolicy = serde_json::from_value(stored_response)
                .map_err(|_| StorageError::InvalidStoredResponse)?;
            transaction.commit().await?;
            return Ok(JurisdictionPolicyDraftOutcome::Replay(response));
        }

        let rule_row = transaction
            .query_opt(
                "SELECT status, effective_from, effective_until
                 FROM rule_sets WHERE id = $1 AND merchant_id = $2",
                &[&command.rule_set_id, &command.merchant_id],
            )
            .await?
            .ok_or(StorageError::RuleNotFound)?;
        let rule_effective_from: std::time::SystemTime = rule_row.get(1);
        let rule_effective_until: Option<std::time::SystemTime> = rule_row.get(2);
        let policy_effective_from = unix_to_system_time(command.effective_from_unix)?;
        let policy_effective_until = command
            .effective_until_unix
            .map(unix_to_system_time)
            .transpose()?;
        if policy_effective_from < rule_effective_from
            || (rule_effective_until.is_some()
                && (policy_effective_until.is_none()
                    || policy_effective_until > rule_effective_until))
        {
            return Err(StorageError::InvalidJurisdictionPolicyDraft(
                "policy effective window must be within the referenced rule release window"
                    .to_string(),
            ));
        }
        let policy_id = Uuid::now_v7();
        let inserted_policy = transaction
            .query_one(
                "INSERT INTO jurisdiction_policies
                    (id, merchant_id, rule_set_id, policy_digest, policy_document,
                     provenance, status, effective_from, effective_until,
                     author_subject, governance_revision)
                 VALUES ($1, $2, $3, $4, $5, $6, 'draft',
                         to_timestamp($7::double precision),
                         CASE WHEN $8::double precision IS NULL THEN NULL
                              ELSE to_timestamp($8::double precision) END,
                         $9, 0)
                 RETURNING id, rule_set_id, status,
                           to_char(effective_from AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                           to_char(effective_until AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                           governance_revision",
                &[
                    &policy_id,
                    &command.merchant_id,
                    &command.rule_set_id,
                    &&policy_digest[..],
                    &policy_document,
                    &command.provenance,
                    &(command.effective_from_unix as f64),
                    &command.effective_until_unix.map(|value| value as f64),
                    &command.actor_subject,
                ],
            )
            .await
            .map_err(|error| {
                if error
                    .code()
                    .is_some_and(|code| code.code() == "23505")
                {
                    StorageError::JurisdictionPolicyDraftConflict
                } else {
                    StorageError::Database(error)
                }
            })?;
        let response = CreatedJurisdictionPolicy {
            policy_id: inserted_policy.get(0),
            rule_set_id: inserted_policy.get(1),
            policy_digest,
            status: inserted_policy.get(2),
            effective_from: inserted_policy.get(3),
            effective_until: inserted_policy.get(4),
            governance_revision: inserted_policy.get(5),
        };
        transaction
            .execute(
                "INSERT INTO jurisdiction_policy_events
                    (policy_id, merchant_id, actor_subject, action,
                     from_status, to_status, evidence)
                 VALUES ($1, $2, $3, 'create', 'none', 'draft', $4)",
                &[
                    &policy_id,
                    &command.merchant_id,
                    &command.actor_subject,
                    &serde_json::json!({
                        "source": "jurisdiction-policy-authoring",
                        "policy_digest": policy_digest,
                    }),
                ],
            )
            .await?;
        let response_document = serde_json::to_value(&response)?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"jurisdiction_policy",
                    &policy_id,
                    &"tax.jurisdiction_policy.draft.created",
                    &serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": command.merchant_id,
                        "policy": response,
                    }),
                ],
            )
            .await?;
        transaction
            .execute(
                "UPDATE idempotency_records
                 SET response_status = 201, response_document = $3, completed_at = now()
                 WHERE merchant_id = $1 AND idempotency_key = $2",
                &[&command.merchant_id, &idempotency_key, &response_document],
            )
            .await?;
        transaction.commit().await?;
        Ok(JurisdictionPolicyDraftOutcome::Created(response))
    }

    pub async fn transition_jurisdiction_policy(
        &self,
        command: JurisdictionPolicyGovernanceTransition,
    ) -> Result<JurisdictionPolicyGovernanceResult, StorageError> {
        validate_jurisdiction_policy_governance_command(&command)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT status, author_subject, reviewer_subject, approver_subject,
                        governance_revision, rule_set_id, effective_from, effective_until
                 FROM jurisdiction_policies
                 WHERE id = $1 AND merchant_id = $2 FOR UPDATE",
                &[&command.policy_id, &command.merchant_id],
            )
            .await?
            .ok_or(StorageError::JurisdictionPolicyGovernanceNotFound)?;
        let from_status: String = row.get(0);
        let author_subject: Option<String> = row.get(1);
        let reviewer_subject: Option<String> = row.get(2);
        let approver_subject: Option<String> = row.get(3);
        let revision: i64 = row.get(4);
        let rule_set_id: Uuid = row.get(5);
        if command.to_status == "published" {
            let rule = transaction
                .query_opt(
                    "SELECT status, effective_from, effective_until
                     FROM rule_sets WHERE id = $1 AND merchant_id = $2",
                    &[&rule_set_id, &command.merchant_id],
                )
                .await?
                .ok_or(StorageError::RuleNotFound)?;
            let policy_from: std::time::SystemTime = row.get(6);
            let policy_until: Option<std::time::SystemTime> = row.get(7);
            let rule_status: String = rule.get(0);
            let rule_from: std::time::SystemTime = rule.get(1);
            let rule_until: Option<std::time::SystemTime> = rule.get(2);
            if rule_status != "published"
                || policy_from < rule_from
                || (rule_until.is_some() && (policy_until.is_none() || policy_until > rule_until))
            {
                return Err(StorageError::JurisdictionPolicyGovernanceConflict);
            }
        }
        let (author, reviewer, approver) = assign_jurisdiction_policy_subjects(
            &from_status,
            &command.to_status,
            &command.actor_subject,
            author_subject,
            reviewer_subject,
            approver_subject,
        )?;
        let action = jurisdiction_policy_action(&command.to_status)?;
        let next_revision = revision
            .checked_add(1)
            .ok_or(StorageError::InvalidJurisdictionPolicyTransition)?;
        transaction
            .execute(
                "UPDATE jurisdiction_policies
                 SET status = $3, author_subject = $4, reviewer_subject = $5,
                     approver_subject = $6, governance_revision = $7, updated_at = now()
                 WHERE id = $1 AND merchant_id = $2",
                &[
                    &command.policy_id,
                    &command.merchant_id,
                    &command.to_status,
                    &author,
                    &reviewer,
                    &approver,
                    &next_revision,
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO jurisdiction_policy_events
                    (policy_id, merchant_id, actor_subject, action,
                     from_status, to_status, evidence)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[
                    &command.policy_id,
                    &command.merchant_id,
                    &command.actor_subject,
                    &action,
                    &from_status,
                    &command.to_status,
                    &command.evidence,
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"jurisdiction_policy",
                    &command.policy_id,
                    &format!("tax.jurisdiction_policy.{}", action),
                    &serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": command.merchant_id,
                        "policy_id": command.policy_id,
                        "from_status": from_status,
                        "to_status": command.to_status,
                        "governance_revision": next_revision,
                    }),
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(JurisdictionPolicyGovernanceResult {
            policy_id: command.policy_id,
            from_status,
            to_status: command.to_status,
            author_subject: author,
            reviewer_subject: reviewer,
            approver_subject: approver,
            governance_revision: next_revision,
        })
    }

    pub async fn load_jurisdiction_policy_governance(
        &self,
        policy_id: Uuid,
        merchant_id: Uuid,
    ) -> Result<JurisdictionPolicyGovernanceState, StorageError> {
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::RepeatableRead)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT merchant_id, id, rule_set_id, policy_digest, policy_document,
                        provenance, status, author_subject, reviewer_subject,
                        approver_subject, governance_revision,
                        to_char(effective_from AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                        to_char(effective_until AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')
                 FROM jurisdiction_policies WHERE id = $1 AND merchant_id = $2",
                &[&policy_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::JurisdictionPolicyGovernanceNotFound)?;
        let policy_digest: Digest32 = row
            .get::<_, Vec<u8>>(3)
            .try_into()
            .map_err(|_| StorageError::JurisdictionPolicyDigestMismatch)?;
        let policy: ResolutionPolicy = serde_json::from_value(row.get(4)).map_err(|error| {
            StorageError::ReplayEvidenceIntegrity(format!("invalid policy document: {error}"))
        })?;
        if policy
            .canonical_digest()
            .map_err(|error| StorageError::ReplayEvidenceIntegrity(error.to_string()))?
            != policy_digest
        {
            return Err(StorageError::JurisdictionPolicyDigestMismatch);
        }
        let events = transaction
            .query(
                "SELECT id, actor_subject, action, from_status, to_status,
                        evidence, occurred_at::text
                 FROM jurisdiction_policy_events
                 WHERE policy_id = $1 AND merchant_id = $2
                 ORDER BY occurred_at ASC, id ASC",
                &[&policy_id, &merchant_id],
            )
            .await?
            .into_iter()
            .map(|event| JurisdictionPolicyGovernanceEvent {
                id: event.get(0),
                actor_subject: event.get(1),
                action: event.get(2),
                from_status: event.get(3),
                to_status: event.get(4),
                evidence: event.get(5),
                occurred_at: event.get(6),
            })
            .collect();
        let state = JurisdictionPolicyGovernanceState {
            merchant_id: row.get(0),
            policy_id: row.get(1),
            rule_set_id: row.get(2),
            policy_digest,
            policy,
            provenance: row.get(5),
            status: row.get(6),
            author_subject: row.get(7),
            reviewer_subject: row.get(8),
            approver_subject: row.get(9),
            governance_revision: row.get(10),
            effective_from: row.get(11),
            effective_until: row.get(12),
            events,
        };
        transaction.commit().await?;
        Ok(state)
    }

    /// Creates one tenant-owned adjustment proposal, its append-only creation
    /// evidence, and its first outbox publication record in one serializable
    /// transaction. The proposal is not a settlement authorization.
    pub async fn create_settlement_adjustment(
        &self,
        command: CreateSettlementAdjustmentCommand,
    ) -> Result<SettlementAdjustmentOutcome, StorageError> {
        validate_settlement_adjustment_command(&command)?;
        let adjustment = Adjustment::propose(command.proposal.clone())
            .map_err(|error| StorageError::InvalidSettlementAdjustment(error.to_string()))?;
        let adjustment_digest = adjustment
            .digest()
            .map_err(|error| StorageError::InvalidSettlementAdjustment(error.to_string()))?;
        let request_digest = settlement_adjustment_request_digest(&command, adjustment_digest)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let inserted = transaction
            .query_opt(
                "INSERT INTO settlement_adjustments
                    (id, merchant_id, idempotency_key, request_digest,
                     adjustment_digest, original_settlement_id, asset_id, kind,
                     gross_delta, tax_delta, net_delta, reason_code,
                     evidence_digest, source_calculation_id,
                     replacement_calculation_id, proposer_subject)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8,
                         $9::text::numeric, $10::text::numeric, $11::text::numeric,
                         $12, $13, $14, $15, $16)
                 ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
                 RETURNING id, merchant_id, idempotency_key, request_digest,
                           adjustment_digest, original_settlement_id, asset_id,
                           kind, gross_delta::text, tax_delta::text, net_delta::text,
                           reason_code, evidence_digest, source_calculation_id,
                           replacement_calculation_id, status, proposer_subject,
                           approver_subject",
                &[
                    &command.proposal.id,
                    &command.merchant_id,
                    &command.idempotency_key,
                    &&request_digest[..],
                    &&adjustment_digest[..],
                    &command.proposal.original_settlement_id,
                    &&command.proposal.asset_id[..],
                    &adjustment_kind_to_db(command.proposal.kind),
                    &command.proposal.delta.gross_atomic.to_string(),
                    &command.proposal.delta.tax_atomic.to_string(),
                    &command.proposal.delta.net_atomic.to_string(),
                    &command.proposal.reason_code,
                    &&command.proposal.evidence_digest[..],
                    &command.proposal.source_calculation_id,
                    &command.proposal.replacement_calculation_id,
                    &command.actor_subject,
                ],
            )
            .await?;
        if let Some(row) = inserted {
            let record = parse_settlement_adjustment_row(row)?;
            transaction
                .execute(
                    "INSERT INTO settlement_adjustment_events
                        (adjustment_id, merchant_id, actor_subject, action,
                         from_status, to_status, evidence)
                     VALUES ($1, $2, $3, 'propose', 'none', 'proposed', $4)",
                    &[
                        &record.id,
                        &command.merchant_id,
                        &command.actor_subject,
                        &serde_json::json!({
                            "request_digest": request_digest,
                            "adjustment_digest": adjustment_digest,
                            "evidence_digest": command.proposal.evidence_digest,
                        }),
                    ],
                )
                .await?;
            transaction
                .execute(
                    "INSERT INTO outbox_events
                        (aggregate_type, aggregate_id, event_type, payload)
                     VALUES ($1, $2, $3, $4)",
                    &[
                        &"settlement_adjustment",
                        &record.id,
                        &"tax.settlement_adjustment.proposed",
                        &serde_json::json!({
                            "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                            "merchant_id": command.merchant_id,
                            "adjustment": record,
                        }),
                    ],
                )
                .await?;
            transaction.commit().await?;
            return Ok(SettlementAdjustmentOutcome::Created(record));
        }
        let row = transaction
            .query_one(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        adjustment_digest, original_settlement_id, asset_id,
                        kind, gross_delta::text, tax_delta::text, net_delta::text,
                        reason_code, evidence_digest, source_calculation_id,
                        replacement_calculation_id, status, proposer_subject,
                        approver_subject
                 FROM settlement_adjustments
                 WHERE merchant_id = $1 AND idempotency_key = $2
                 FOR UPDATE",
                &[&command.merchant_id, &command.idempotency_key],
            )
            .await?;
        let record = parse_settlement_adjustment_row(row)?;
        if record.request_digest != request_digest {
            return Err(StorageError::SettlementAdjustmentIdempotencyConflict);
        }
        transaction.commit().await?;
        Ok(SettlementAdjustmentOutcome::Replay(record))
    }

    pub async fn load_settlement_adjustment(
        &self,
        adjustment_id: Uuid,
        merchant_id: Uuid,
    ) -> Result<SettlementAdjustmentRecord, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        adjustment_digest, original_settlement_id, asset_id,
                        kind, gross_delta::text, tax_delta::text, net_delta::text,
                        reason_code, evidence_digest, source_calculation_id,
                        replacement_calculation_id, status, proposer_subject,
                        approver_subject
                 FROM settlement_adjustments
                 WHERE id = $1 AND merchant_id = $2",
                &[&adjustment_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::SettlementAdjustmentNotFound)?;
        parse_settlement_adjustment_row(row)
    }

    pub async fn transition_settlement_adjustment(
        &self,
        command: SettlementAdjustmentTransition,
    ) -> Result<SettlementAdjustmentTransitionResult, StorageError> {
        validate_settlement_adjustment_transition_command(&command)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT status, proposer_subject, approver_subject
                 FROM settlement_adjustments
                 WHERE id = $1 AND merchant_id = $2
                 FOR UPDATE",
                &[&command.adjustment_id, &command.merchant_id],
            )
            .await?
            .ok_or(StorageError::SettlementAdjustmentNotFound)?;
        let from_status: String = row.get(0);
        let proposer_subject: String = row.get(1);
        let current_approver: Option<String> = row.get(2);
        let current_status = adjustment_status_from_db(&from_status)?;
        let target_status = adjustment_status_from_db(&command.to_status)?;
        validate_transition(
            current_status,
            target_status,
            &proposer_subject,
            current_approver.as_deref(),
            &command.actor_subject,
        )
        .map_err(|error| match error {
            settlement_adjustments::AdjustmentError::ApprovalSubjectMustDiffer => {
                StorageError::SettlementAdjustmentActorConflict
            }
            _ => StorageError::InvalidSettlementAdjustmentTransition,
        })?;
        let approver_subject = if command.to_status == "approved" {
            Some(command.actor_subject.clone())
        } else {
            current_approver
        };
        let updated = transaction
            .execute(
                "UPDATE settlement_adjustments
                 SET status = $3, approver_subject = $4, updated_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND status = $5",
                &[
                    &command.adjustment_id,
                    &command.merchant_id,
                    &command.to_status,
                    &approver_subject,
                    &from_status,
                ],
            )
            .await?;
        if updated != 1 {
            transaction.rollback().await?;
            return Err(StorageError::InvalidSettlementAdjustmentTransition);
        }
        let action = match command.to_status.as_str() {
            "approved" => "approve",
            "rejected" => "reject",
            "voided" => "void",
            _ => return Err(StorageError::InvalidSettlementAdjustmentTransition),
        };
        transaction
            .execute(
                "INSERT INTO settlement_adjustment_events
                    (adjustment_id, merchant_id, actor_subject, action,
                     from_status, to_status, evidence)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[
                    &command.adjustment_id,
                    &command.merchant_id,
                    &command.actor_subject,
                    &action,
                    &from_status,
                    &command.to_status,
                    &command.evidence,
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"settlement_adjustment",
                    &command.adjustment_id,
                    &format!("tax.settlement_adjustment.{action}"),
                    &serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": command.merchant_id,
                        "adjustment_id": command.adjustment_id,
                        "from_status": from_status,
                        "to_status": command.to_status,
                    }),
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(SettlementAdjustmentTransitionResult {
            adjustment_id: command.adjustment_id,
            from_status,
            to_status: command.to_status,
            approver_subject,
        })
    }

    /// Persists a proposed stream plan and its first publication event in one
    /// serializable transaction. The supplied plan identity is already a
    /// server-generated UUIDv7 at this boundary; callers must not accept it
    /// from a browser request.
    pub async fn create_revenue_stream_plan(
        &self,
        command: CreateRevenueStreamPlanCommand,
    ) -> Result<RevenueStreamPlanOutcome, StorageError> {
        validate_create_revenue_stream_plan_command(&command)?;
        let request_digest = revenue_stream_plan_request_digest(&command)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let spec = &command.spec;
        let end_slot = spec.end_slot.map(|value| value.to_string());
        let start_slot = spec.start_slot.to_string();
        let max_total_atomic = spec.max_total_atomic.to_string();
        let max_claim_atomic = spec.max_claim_atomic.to_string();
        let inserted = transaction
            .query_opt(
                "INSERT INTO revenue_stream_plans
                    (id, merchant_id, settlement_route_id, destination_route_id, idempotency_key,
                     request_digest, asset, recipient, plan_digest, policy_digest,
                     start_slot, end_slot, max_total_atomic, max_claim_atomic,
                     proposer_subject, operator_subject)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::text::numeric,
                         $12::text::numeric, $13::text::numeric, $14::text::numeric, $15, $16)
                 ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
                 RETURNING id, merchant_id, idempotency_key, request_digest,
                           settlement_route_id, destination_route_id, asset, recipient, plan_digest, policy_digest,
                           start_slot::text, end_slot::text,
                           max_total_atomic::text, max_claim_atomic::text,
                           status, proposer_subject, approver_subject, operator_subject,
                           accrued_total_atomic::text, released_total_atomic::text,
                           next_nonce::text",
                &[
                    &spec.id.0,
                    &command.merchant_id,
                    &spec.settlement_route_id,
                    &command.destination_route_id,
                    &command.idempotency_key,
                    &&request_digest[..],
                    &&spec.asset.0[..],
                    &&spec.recipient.0[..],
                    &&spec.plan_digest.0[..],
                    &&spec.policy_digest.0[..],
                    &start_slot,
                    &end_slot,
                    &max_total_atomic,
                    &max_claim_atomic,
                    &command.actor_subject,
                    &spec.operator_subject,
                ],
            )
            .await?;
        if let Some(row) = inserted {
            let record = parse_revenue_stream_plan_row(row)?;
            transaction
                .execute(
                    "INSERT INTO revenue_stream_plan_events
                        (plan_id, merchant_id, actor_subject, action,
                         from_status, to_status, evidence)
                     VALUES ($1, $2, $3, 'propose', 'none', 'proposed', $4)",
                    &[
                        &record.id.0,
                        &command.merchant_id,
                        &command.actor_subject,
                        &serde_json::json!({
                            "request_digest": request_digest,
                            "plan_digest": spec.plan_digest,
                        }),
                    ],
                )
                .await?;
            transaction
                .execute(
                    "INSERT INTO outbox_events
                        (aggregate_type, aggregate_id, event_type, payload)
                     VALUES ($1, $2, $3, $4)",
                    &[
                        &"revenue_stream_plan",
                        &record.id.0,
                        &"tax.revenue_stream_plan.proposed",
                        &serde_json::json!({
                            "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                            "merchant_id": command.merchant_id,
                            "plan": record.clone(),
                        }),
                    ],
                )
                .await?;
            transaction.commit().await?;
            return Ok(RevenueStreamPlanOutcome::Created(record));
        }
        let row = transaction
            .query_one(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        settlement_route_id, destination_route_id, asset, recipient, plan_digest, policy_digest,
                        start_slot::text, end_slot::text,
                        max_total_atomic::text, max_claim_atomic::text,
                        status, proposer_subject, approver_subject, operator_subject,
                        accrued_total_atomic::text, released_total_atomic::text,
                        next_nonce::text
                 FROM revenue_stream_plans
                 WHERE merchant_id = $1 AND idempotency_key = $2
                 FOR UPDATE",
                &[&command.merchant_id, &command.idempotency_key],
            )
            .await?;
        let record = parse_revenue_stream_plan_row(row)?;
        if record.request_digest != request_digest {
            return Err(StorageError::RevenueStreamPlanIdempotencyConflict);
        }
        transaction.commit().await?;
        Ok(RevenueStreamPlanOutcome::Replay(record))
    }

    pub async fn load_revenue_stream_plan(
        &self,
        plan_id: StreamPlanId,
        merchant_id: Uuid,
    ) -> Result<RevenueStreamPlanRecord, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        settlement_route_id, destination_route_id, asset, recipient, plan_digest, policy_digest,
                        start_slot::text, end_slot::text,
                        max_total_atomic::text, max_claim_atomic::text,
                        status, proposer_subject, approver_subject, operator_subject,
                        accrued_total_atomic::text, released_total_atomic::text,
                        next_nonce::text
                 FROM revenue_stream_plans
                 WHERE id = $1 AND merchant_id = $2",
                &[&plan_id.0, &merchant_id],
            )
            .await?
            .ok_or(StorageError::RevenueStreamPlanNotFound)?;
        parse_revenue_stream_plan_row(row)
    }

    pub async fn list_revenue_stream_plans(
        &self,
        merchant_id: Uuid,
        limit: usize,
    ) -> Result<Vec<RevenueStreamPlanRecord>, StorageError> {
        if merchant_id.get_version_num() != 7 || !(1..=100).contains(&limit) {
            return Err(StorageError::InvalidRevenueStream(
                "merchant identity or plan list limit is invalid".into(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        settlement_route_id, destination_route_id, asset, recipient, plan_digest, policy_digest,
                        start_slot::text, end_slot::text,
                        max_total_atomic::text, max_claim_atomic::text,
                        status, proposer_subject, approver_subject, operator_subject,
                        accrued_total_atomic::text, released_total_atomic::text,
                        next_nonce::text
                 FROM revenue_stream_plans
                 WHERE merchant_id = $1
                 ORDER BY updated_at DESC, id DESC
                 LIMIT $2",
                &[&merchant_id, &(limit as i64)],
            )
            .await?;
        rows.into_iter()
            .map(parse_revenue_stream_plan_row)
            .collect()
    }

    /// Loads a bounded set of active stream plans together with the immutable,
    /// digest-matched settlement split plan needed by the finalized accrual
    /// reconciler. This is an internal worker port; it has no browser/API
    /// inputs and never returns caller-selected account topology.
    pub async fn load_active_revenue_stream_reconciliation_contexts(
        &self,
        limit: usize,
    ) -> Result<Vec<RevenueStreamReconciliationContextRecord>, StorageError> {
        if !(1..=MAX_REVENUE_STREAM_RECONCILIATION_CONTEXTS).contains(&limit) {
            return Err(StorageError::InvalidRevenueStream(
                "revenue reconciliation context limit is invalid".into(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        settlement_route_id, destination_route_id, asset, recipient, plan_digest, policy_digest,
                        start_slot::text, end_slot::text,
                        max_total_atomic::text, max_claim_atomic::text,
                        status, proposer_subject, approver_subject, operator_subject,
                        accrued_total_atomic::text, released_total_atomic::text,
                        next_nonce::text
                 FROM revenue_stream_plans
                 WHERE status = 'active'
                 ORDER BY updated_at, id
                 LIMIT $1",
                &[&(limit as i64)],
            )
            .await?;
        let mut contexts = Vec::with_capacity(rows.len());
        for row in rows {
            let stream_plan = parse_revenue_stream_plan_row(row)?;
            let settlement_split_plan = self
                .load_active_settlement_split_plan_by_digest(
                    stream_plan.merchant_id,
                    stream_plan.spec.asset.0,
                    stream_plan.spec.plan_digest.0,
                )
                .await?;
            contexts.push(RevenueStreamReconciliationContextRecord {
                merchant_id: stream_plan.merchant_id,
                stream_plan,
                settlement_split_plan,
            });
        }
        Ok(contexts)
    }

    pub async fn transition_revenue_stream_plan(
        &self,
        command: RevenueStreamPlanTransition,
    ) -> Result<RevenueStreamPlanTransitionResult, StorageError> {
        validate_revenue_stream_plan_transition(&command)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT status, proposer_subject, approver_subject,
                        accrued_total_atomic::text, released_total_atomic::text,
                        end_slot::text
                 FROM revenue_stream_plans
                 WHERE id = $1 AND merchant_id = $2
                 FOR UPDATE",
                &[&command.plan_id.0, &command.merchant_id],
            )
            .await?
            .ok_or(StorageError::RevenueStreamPlanNotFound)?;
        let from_status: String = row.get(0);
        let proposer_subject: String = row.get(1);
        let current_approver: Option<String> = row.get(2);
        let accrued_total = parse_u64(row.get(3), "revenue stream accrued total")?;
        let released_total = parse_u64(row.get(4), "revenue stream released total")?;
        let end_slot = row
            .get::<_, Option<String>>(5)
            .as_deref()
            .map(|value| parse_u64(value, "revenue stream end slot"))
            .transpose()?;
        let approver_subject = validate_revenue_stream_plan_transition_state(
            &from_status,
            &command.to_status,
            RevenueStreamTransitionState {
                proposer_subject: &proposer_subject,
                current_approver: current_approver.as_deref(),
                actor_subject: &command.actor_subject,
                accrued_total_atomic: accrued_total,
                released_total_atomic: released_total,
                end_slot,
                observed_slot: command.observed_slot,
            },
        )?;
        let updated = transaction
            .execute(
                "UPDATE revenue_stream_plans
                 SET status = $3, approver_subject = $4, updated_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND status = $5",
                &[
                    &command.plan_id.0,
                    &command.merchant_id,
                    &command.to_status,
                    &approver_subject,
                    &from_status,
                ],
            )
            .await?;
        if updated != 1 {
            return Err(StorageError::InvalidRevenueStreamPlanTransition);
        }
        let action = revenue_stream_plan_action(&from_status, &command.to_status)?;
        transaction
            .execute(
                "INSERT INTO revenue_stream_plan_events
                    (plan_id, merchant_id, actor_subject, action,
                     from_status, to_status, evidence)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[
                    &command.plan_id.0,
                    &command.merchant_id,
                    &command.actor_subject,
                    &action,
                    &from_status,
                    &command.to_status,
                    &command.evidence,
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"revenue_stream_plan",
                    &command.plan_id.0,
                    &format!("tax.revenue_stream_plan.{action}"),
                    &serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": command.merchant_id,
                        "plan_id": command.plan_id.0,
                        "from_status": from_status,
                        "to_status": command.to_status,
                    }),
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(RevenueStreamPlanTransitionResult {
            plan_id: command.plan_id,
            from_status,
            to_status: command.to_status,
            approver_subject,
        })
    }

    /// Claims one durable finalized split job. The lease is coordination only;
    /// it is committed before the pure/application reconciliation work runs.
    pub async fn claim_revenue_stream_accrual_job(
        &self,
        stream_key: &str,
    ) -> Result<Option<RevenueStreamAccrualReconciliationJob>, StorageError> {
        validate_indexer_stream_key(stream_key)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT source_signature, evidence_digest, observed_finalized_slot::text
                 FROM revenue_stream_accrual_reconciliation_jobs
                 WHERE stream_key = $1
                   AND completed_at IS NULL
                   AND dead_lettered_at IS NULL
                   AND attempts < $3::smallint
                   AND (claim_token IS NULL
                        OR claimed_at < now() - ($2::bigint * interval '1 second'))
                 ORDER BY created_at, source_signature
                 FOR UPDATE SKIP LOCKED
                 LIMIT 1",
                &[
                    &stream_key,
                    &(REVENUE_STREAM_ACCRUAL_RECONCILIATION_LEASE.as_secs() as i64),
                    &MAX_REVENUE_STREAM_ACCRUAL_ATTEMPTS,
                ],
            )
            .await?;
        let Some(row) = row else {
            transaction.commit().await?;
            return Ok(None);
        };
        let source_signature: String = row.get(0);
        let evidence_bytes: Vec<u8> = row.get(1);
        let evidence_digest: Digest32 = evidence_bytes.try_into().map_err(|bytes: Vec<u8>| {
            StorageError::InvalidRevenueStreamAccrualJob(format!(
                "job evidence digest must contain 32 bytes, got {}",
                bytes.len()
            ))
        })?;
        let observed_finalized_slot = parse_u64(row.get(2), "revenue job finalized slot")?;
        let claim_token = Uuid::now_v7();
        let claimed = transaction
            .execute(
                "UPDATE revenue_stream_accrual_reconciliation_jobs
                 SET claim_token = $3, claimed_at = now(),
                     attempts = attempts + 1, updated_at = now()
                 WHERE stream_key = $1 AND source_signature = $2
                   AND completed_at IS NULL AND dead_lettered_at IS NULL
                   AND attempts < $4::smallint
                   AND (claim_token IS NULL
                        OR claimed_at < now() - ($5::bigint * interval '1 second'))",
                &[
                    &stream_key,
                    &source_signature,
                    &claim_token,
                    &MAX_REVENUE_STREAM_ACCRUAL_ATTEMPTS,
                    &(REVENUE_STREAM_ACCRUAL_RECONCILIATION_LEASE.as_secs() as i64),
                ],
            )
            .await?;
        if claimed != 1 {
            transaction.rollback().await?;
            return Ok(None);
        }
        let split_row = transaction
            .query_one(
                "SELECT signature, slot::text, receipt, payer, merchant, mint,
                        nonce::text, gross_atomic::text, tax_atomic::text,
                        net_atomic::text, split_plan_version::text,
                        split_plan_digest, policy_digest, evidence_digest
                 FROM indexed_split_settlements
                 WHERE stream_key = $1 AND signature = $2",
                &[&stream_key, &source_signature],
            )
            .await?;
        let split_settlement = parse_split_settlement_row(split_row)?;
        if split_settlement.evidence_digest != evidence_digest {
            return Err(StorageError::InvalidRevenueStreamAccrualJob(
                "job evidence digest does not match indexed split evidence".into(),
            ));
        }
        let receipt_row = transaction
            .query_one(
                "SELECT signature, slot::text, gross_atomic::text,
                        tax_atomic::text, net_atomic::text, evidence_digest,
                        receipt, payer, merchant, mint, nonce::text, policy_digest
                 FROM indexed_receipts
                 WHERE stream_key = $1 AND signature = $2",
                &[&stream_key, &source_signature],
            )
            .await?;
        let receipt = parse_receipt_row(receipt_row)?;
        transaction.commit().await?;
        Ok(Some(RevenueStreamAccrualReconciliationJob {
            stream_key: stream_key.to_string(),
            source_signature,
            evidence_digest,
            observed_finalized_slot,
            claim_token,
            split_settlement,
            receipt,
        }))
    }

    /// Completes exactly one claimed job after all accrual writes have
    /// succeeded. A stale worker cannot clear a newer lease.
    pub async fn complete_revenue_stream_accrual_job(
        &self,
        job: &RevenueStreamAccrualReconciliationJob,
        result_code: &str,
    ) -> Result<(), StorageError> {
        validate_revenue_stream_job_identity(job, result_code)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let updated = transaction
            .execute(
                "UPDATE revenue_stream_accrual_reconciliation_jobs
                 SET completed_at = now(), claim_token = NULL, claimed_at = NULL,
                     last_error_code = $4, updated_at = now()
                 WHERE stream_key = $1 AND source_signature = $2
                   AND claim_token = $3 AND completed_at IS NULL
                   AND dead_lettered_at IS NULL",
                &[
                    &job.stream_key,
                    &job.source_signature,
                    &job.claim_token,
                    &result_code,
                ],
            )
            .await?;
        if updated != 1 {
            transaction.rollback().await?;
            return Err(StorageError::RevenueStreamAccrualJobLeaseLost);
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Releases a failed job for bounded retry, or records explicit
    /// dead-letter evidence after the configured attempt limit.
    pub async fn fail_revenue_stream_accrual_job(
        &self,
        job: &RevenueStreamAccrualReconciliationJob,
        error_code: &str,
    ) -> Result<bool, StorageError> {
        validate_revenue_stream_job_identity(job, error_code)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let updated = transaction
            .execute(
                "UPDATE revenue_stream_accrual_reconciliation_jobs
                 SET claim_token = NULL, claimed_at = NULL, last_error_code = $4,
                     dead_lettered_at = CASE
                         WHEN attempts >= $5::smallint THEN now()
                         ELSE NULL
                     END,
                     updated_at = now()
                 WHERE stream_key = $1 AND source_signature = $2
                   AND claim_token = $3 AND completed_at IS NULL
                   AND dead_lettered_at IS NULL",
                &[
                    &job.stream_key,
                    &job.source_signature,
                    &job.claim_token,
                    &error_code,
                    &MAX_REVENUE_STREAM_ACCRUAL_ATTEMPTS,
                ],
            )
            .await?;
        if updated != 1 {
            transaction.rollback().await?;
            return Err(StorageError::RevenueStreamAccrualJobLeaseLost);
        }
        transaction.commit().await?;
        Ok(true)
    }

    /// Records only finalized settlement evidence. The plan row is locked so
    /// the accrued balance and its cap are updated atomically with the
    /// immutable evidence and outbox publication.
    pub async fn record_revenue_stream_accrual(
        &self,
        command: RecordRevenueStreamAccrualCommand,
    ) -> Result<RevenueStreamAccrualOutcome, StorageError> {
        validate_revenue_stream_actor(
            command.merchant_id,
            command.plan_id,
            &command.actor_subject,
        )?;
        validate_revenue_stream_accrual(&command.accrual)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let plan =
            load_revenue_stream_plan_for_update(&transaction, command.plan_id, command.merchant_id)
                .await?;
        if plan.status != "active" {
            return Err(StorageError::InvalidRevenueStreamPlanTransition);
        }
        validate_revenue_stream_accrual_binding(&plan, &command.accrual)?;
        let existing = transaction
            .query_opt(
                "SELECT source_signature, finalized_slot::text, receipt_digest,
                        plan_digest, policy_digest, asset, recipient,
                        gross_atomic::text, tax_atomic::text, net_atomic::text,
                        recipient_atomic::text
                 FROM revenue_stream_accruals
                 WHERE stream_plan_id = $1 AND merchant_id = $2 AND source_signature = $3
                 FOR UPDATE",
                &[
                    &command.plan_id.0,
                    &command.merchant_id,
                    &command.accrual.source_signature.0,
                ],
            )
            .await?;
        if let Some(row) = existing {
            let stored = parse_revenue_stream_accrual_row(row)?;
            if stored == command.accrual {
                transaction.commit().await?;
                return Ok(RevenueStreamAccrualOutcome::Duplicate);
            }
            return Err(StorageError::RevenueStreamEvidenceConflict);
        }
        let next_accrued = plan
            .accrued_total_atomic
            .checked_add(command.accrual.recipient_atomic)
            .ok_or(StorageError::InvalidRevenueStream(
                "accrued total exceeds u64".into(),
            ))?;
        if next_accrued > plan.spec.max_total_atomic {
            return Err(StorageError::RevenueStreamInsufficientAvailable);
        }
        transaction
            .execute(
                "INSERT INTO revenue_stream_accruals
                    (stream_plan_id, merchant_id, source_signature,
                     finalized_slot, receipt_digest, plan_digest, policy_digest,
                     asset, recipient, gross_atomic, tax_atomic, net_atomic,
                     recipient_atomic)
                 VALUES ($1, $2, $3, $4::text::numeric, $5, $6, $7, $8, $9,
                         $10::text::numeric, $11::text::numeric, $12::text::numeric,
                         $13::text::numeric)",
                &[
                    &command.plan_id.0,
                    &command.merchant_id,
                    &command.accrual.source_signature.0,
                    &command.accrual.finalized_slot.to_string(),
                    &&command.accrual.receipt_digest.0[..],
                    &&command.accrual.plan_digest.0[..],
                    &&command.accrual.policy_digest.0[..],
                    &&command.accrual.asset.0[..],
                    &&command.accrual.recipient.0[..],
                    &command.accrual.gross_atomic.to_string(),
                    &command.accrual.tax_atomic.to_string(),
                    &command.accrual.net_atomic.to_string(),
                    &command.accrual.recipient_atomic.to_string(),
                ],
            )
            .await?;
        transaction
            .execute(
                "UPDATE revenue_stream_plans
                 SET accrued_total_atomic = $3::text::numeric, updated_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND accrued_total_atomic = $4::text::numeric",
                &[
                    &command.plan_id.0,
                    &command.merchant_id,
                    &next_accrued.to_string(),
                    &plan.accrued_total_atomic.to_string(),
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"revenue_stream_plan",
                    &command.plan_id.0,
                    &"tax.revenue_stream.accrued",
                    &serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": command.merchant_id,
                        "plan_id": command.plan_id.0,
                        "source_signature": command.accrual.source_signature,
                        "recipient_atomic": command.accrual.recipient_atomic,
                        "accrued_total_atomic": next_accrued,
                    }),
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(RevenueStreamAccrualOutcome::Recorded)
    }

    /// Applies a finalized correction while reserving all pending and already
    /// released claims. A decrease can only consume currently available
    /// balance; an increase remains bounded by the immutable plan cap.
    pub async fn record_revenue_stream_correction(
        &self,
        command: RecordRevenueStreamCorrectionCommand,
    ) -> Result<(), StorageError> {
        validate_revenue_stream_actor(
            command.merchant_id,
            command.plan_id,
            &command.actor_subject,
        )?;
        validate_revenue_stream_correction(&command.correction)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let plan =
            load_revenue_stream_plan_for_update(&transaction, command.plan_id, command.merchant_id)
                .await?;
        if !matches!(plan.status.as_str(), "active" | "paused") {
            return Err(StorageError::InvalidRevenueStreamPlanTransition);
        }
        if command.correction.plan_digest != plan.spec.plan_digest {
            return Err(StorageError::InvalidRevenueStream(
                "correction plan digest does not match the stream plan".into(),
            ));
        }
        let source_exists = transaction
            .query_opt(
                "SELECT 1 FROM revenue_stream_accruals
                 WHERE stream_plan_id = $1 AND merchant_id = $2 AND source_signature = $3",
                &[
                    &command.plan_id.0,
                    &command.merchant_id,
                    &command.correction.source_signature.0,
                ],
            )
            .await?
            .is_some();
        if !source_exists {
            return Err(StorageError::RevenueStreamEvidenceConflict);
        }
        let existing = transaction
            .query_opt(
                "SELECT correction_signature, source_signature, finalized_slot::text,
                        plan_digest, decrease, amount_atomic::text
                 FROM revenue_stream_corrections
                 WHERE stream_plan_id = $1 AND merchant_id = $2 AND correction_signature = $3
                 FOR UPDATE",
                &[
                    &command.plan_id.0,
                    &command.merchant_id,
                    &command.correction.correction_signature.0,
                ],
            )
            .await?;
        if let Some(row) = existing {
            if parse_revenue_stream_correction_row(row)? == command.correction {
                transaction.commit().await?;
                return Ok(());
            }
            return Err(StorageError::RevenueStreamEvidenceConflict);
        }
        let pending_atomic = parse_u64(
            transaction
                .query_one(
                    "SELECT COALESCE(SUM(amount_atomic), 0)::text
                     FROM revenue_stream_claims
                     WHERE stream_plan_id = $1 AND merchant_id = $2 AND status = 'pending'",
                    &[&command.plan_id.0, &command.merchant_id],
                )
                .await?
                .get(0),
            "revenue stream pending claims",
        )?;
        let available = plan
            .accrued_total_atomic
            .checked_sub(plan.released_total_atomic)
            .and_then(|value| value.checked_sub(pending_atomic))
            .ok_or(StorageError::InvalidRevenueStream(
                "revenue stream balance is inconsistent".into(),
            ))?;
        let next_accrued = if command.correction.decrease {
            if command.correction.amount_atomic > available {
                return Err(StorageError::RevenueStreamInsufficientAvailable);
            }
            plan.accrued_total_atomic
                .checked_sub(command.correction.amount_atomic)
                .ok_or(StorageError::RevenueStreamInsufficientAvailable)?
        } else {
            plan.accrued_total_atomic
                .checked_add(command.correction.amount_atomic)
                .ok_or(StorageError::InvalidRevenueStream(
                    "corrected accrued total exceeds u64".into(),
                ))?
        };
        if next_accrued > plan.spec.max_total_atomic {
            return Err(StorageError::RevenueStreamInsufficientAvailable);
        }
        transaction
            .execute(
                "INSERT INTO revenue_stream_corrections
                    (stream_plan_id, merchant_id, correction_signature,
                     source_signature, finalized_slot, plan_digest, decrease,
                     amount_atomic)
                 VALUES ($1, $2, $3, $4, $5::text::numeric, $6, $7, $8::text::numeric)",
                &[
                    &command.plan_id.0,
                    &command.merchant_id,
                    &command.correction.correction_signature.0,
                    &command.correction.source_signature.0,
                    &command.correction.finalized_slot.to_string(),
                    &&command.correction.plan_digest.0[..],
                    &command.correction.decrease,
                    &command.correction.amount_atomic.to_string(),
                ],
            )
            .await?;
        transaction
            .execute(
                "UPDATE revenue_stream_plans
                 SET accrued_total_atomic = $3::text::numeric, updated_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND accrued_total_atomic = $4::text::numeric",
                &[
                    &command.plan_id.0,
                    &command.merchant_id,
                    &next_accrued.to_string(),
                    &plan.accrued_total_atomic.to_string(),
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"revenue_stream_plan",
                    &command.plan_id.0,
                    &"tax.revenue_stream.corrected",
                    &serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": command.merchant_id,
                        "plan_id": command.plan_id.0,
                        "correction_signature": command.correction.correction_signature,
                        "decrease": command.correction.decrease,
                        "amount_atomic": command.correction.amount_atomic,
                        "accrued_total_atomic": next_accrued,
                    }),
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// Reserves an available amount by allocating a monotonically increasing
    /// plan-local nonce. The returned digest is the exact binding used by the
    /// Solana release claim and is never accepted from a browser.
    pub async fn prepare_revenue_stream_claim(
        &self,
        command: PrepareRevenueStreamClaimCommand,
    ) -> Result<RevenueStreamClaimRecord, StorageError> {
        validate_revenue_stream_actor(
            command.merchant_id,
            command.plan_id,
            &command.actor_subject,
        )?;
        if command.claim_id.get_version_num() != 7
            || command.amount_atomic == 0
            || command.idempotency_key.trim().is_empty()
            || command.idempotency_key.len() > 200
            || command
                .idempotency_key
                .bytes()
                .any(|byte| byte.is_ascii_control())
        {
            return Err(StorageError::InvalidRevenueStream(
                "claim identity, idempotency, or amount is invalid".into(),
            ));
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let plan =
            load_revenue_stream_plan_for_update(&transaction, command.plan_id, command.merchant_id)
                .await?;
        if plan.status != "active" || command.amount_atomic > plan.spec.max_claim_atomic {
            return Err(StorageError::InvalidRevenueStreamPlanTransition);
        }
        let existing_by_idempotency = transaction
            .query_opt(
                "SELECT id, stream_plan_id, merchant_id, idempotency_key,
                        amount_atomic::text, nonce::text, accrual_total_atomic::text,
                        plan_digest, asset, recipient, claim_digest, status,
                        release_signature, finalized_slot::text
                 FROM revenue_stream_claims
                 WHERE merchant_id = $1 AND stream_plan_id = $2 AND idempotency_key = $3
                 FOR UPDATE",
                &[
                    &command.merchant_id,
                    &command.plan_id.0,
                    &command.idempotency_key,
                ],
            )
            .await?;
        if let Some(row) = existing_by_idempotency {
            let existing = parse_revenue_stream_claim_row(row)?;
            if existing.amount_atomic == command.amount_atomic {
                transaction.commit().await?;
                return Ok(existing);
            }
            return Err(StorageError::RevenueStreamEvidenceConflict);
        }
        let duplicate_id = transaction
            .query_opt(
                "SELECT 1 FROM revenue_stream_claims WHERE id = $1",
                &[&command.claim_id],
            )
            .await?
            .is_some();
        if duplicate_id {
            return Err(StorageError::RevenueStreamEvidenceConflict);
        }
        let pending_atomic = parse_u64(
            transaction
                .query_one(
                    "SELECT COALESCE(SUM(amount_atomic), 0)::text
                     FROM revenue_stream_claims
                     WHERE stream_plan_id = $1 AND merchant_id = $2 AND status = 'pending'",
                    &[&command.plan_id.0, &command.merchant_id],
                )
                .await?
                .get(0),
            "revenue stream pending claims",
        )?;
        let available = plan
            .accrued_total_atomic
            .checked_sub(plan.released_total_atomic)
            .and_then(|value| value.checked_sub(pending_atomic))
            .ok_or(StorageError::InvalidRevenueStream(
                "revenue stream balance is inconsistent".into(),
            ))?;
        if command.amount_atomic > available {
            return Err(StorageError::RevenueStreamInsufficientAvailable);
        }
        let nonce = plan.next_nonce;
        let next_nonce = nonce
            .checked_add(1)
            .ok_or(StorageError::InvalidRevenueStream(
                "revenue stream nonce exceeds u64".into(),
            ))?;
        let claim_digest = revenue_streaming::claim_digest(
            command.claim_id,
            command.plan_id,
            command.amount_atomic,
            nonce,
            plan.spec.plan_digest,
            plan.spec.asset,
            plan.spec.recipient,
        );
        transaction
            .execute(
                "INSERT INTO revenue_stream_claims
                    (id, stream_plan_id, merchant_id, idempotency_key, amount_atomic, nonce,
                     accrual_total_atomic, plan_digest, asset, recipient, claim_digest)
                 VALUES ($1, $2, $3, $4, $5::text::numeric, $6::text::numeric,
                         $7::text::numeric, $8, $9, $10, $11)",
                &[
                    &command.claim_id,
                    &command.plan_id.0,
                    &command.merchant_id,
                    &command.idempotency_key,
                    &command.amount_atomic.to_string(),
                    &nonce.to_string(),
                    &plan.accrued_total_atomic.to_string(),
                    &&plan.spec.plan_digest.0[..],
                    &&plan.spec.asset.0[..],
                    &&plan.spec.recipient.0[..],
                    &&claim_digest.0[..],
                ],
            )
            .await?;
        transaction
            .execute(
                "UPDATE revenue_stream_plans
                 SET next_nonce = $3::text::numeric, updated_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND next_nonce = $4::text::numeric",
                &[
                    &command.plan_id.0,
                    &command.merchant_id,
                    &next_nonce.to_string(),
                    &nonce.to_string(),
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO revenue_stream_claim_events
                    (claim_id, stream_plan_id, merchant_id, actor_subject, action, evidence)
                 VALUES ($1, $2, $3, $4, 'prepared', $5)",
                &[
                    &command.claim_id,
                    &command.plan_id.0,
                    &command.merchant_id,
                    &command.actor_subject,
                    &serde_json::json!({
                        "claim_digest": claim_digest,
                        "amount_atomic": command.amount_atomic,
                        "nonce": nonce,
                    }),
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"revenue_stream_claim",
                    &command.claim_id,
                    &"tax.revenue_stream.claim_prepared",
                    &serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": command.merchant_id,
                        "plan_id": command.plan_id.0,
                        "claim_id": command.claim_id,
                        "claim_digest": claim_digest,
                    }),
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(RevenueStreamClaimRecord {
            id: command.claim_id,
            plan_id: command.plan_id,
            merchant_id: command.merchant_id,
            idempotency_key: command.idempotency_key,
            amount_atomic: command.amount_atomic,
            nonce,
            accrual_total_atomic: plan.accrued_total_atomic,
            plan_digest: plan.spec.plan_digest.0,
            asset: plan.spec.asset.0,
            recipient: plan.spec.recipient.0,
            claim_digest: claim_digest.0,
            status: "pending".to_string(),
            release_signature: None,
            finalized_slot: None,
        })
    }

    /// Commits finalized release evidence and increments the plan's released
    /// total in the same serializable transaction. Database state only records
    /// evidence; finalized Solana state remains the movement authority.
    pub async fn finalize_revenue_stream_claim(
        &self,
        command: FinalizeRevenueStreamClaimCommand,
    ) -> Result<RevenueStreamClaimOutcome, StorageError> {
        validate_revenue_stream_actor_subject(command.merchant_id, &command.actor_subject)?;
        validate_revenue_stream_release(&command.release)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let claim_row = transaction
            .query_opt(
                "SELECT stream_plan_id, amount_atomic::text, nonce::text,
                        accrual_total_atomic::text, plan_digest, asset, recipient,
                        claim_digest, status, release_signature, finalized_slot::text
                 FROM revenue_stream_claims
                 WHERE id = $1 AND merchant_id = $2
                 FOR UPDATE",
                &[&command.release.claim_id, &command.merchant_id],
            )
            .await?
            .ok_or(StorageError::RevenueStreamClaimNotFound)?;
        let plan_id = StreamPlanId(claim_row.get(0));
        let amount_atomic = parse_u64(claim_row.get(1), "revenue stream claim amount")?;
        let nonce = parse_u64(claim_row.get(2), "revenue stream claim nonce")?;
        let accrual_total_atomic = parse_u64(claim_row.get(3), "revenue stream claim accrual")?;
        let plan_digest = parse_revenue_stream_fixed_digest(claim_row.get(4), "claim plan digest")?;
        let asset = parse_revenue_stream_fixed_digest(claim_row.get(5), "claim asset")?;
        let recipient = parse_revenue_stream_fixed_digest(claim_row.get(6), "claim recipient")?;
        let claim_digest = parse_revenue_stream_fixed_digest(claim_row.get(7), "claim digest")?;
        let status: String = claim_row.get(8);
        if status == "finalized" {
            let stored_signature = claim_row
                .get::<_, Option<Vec<u8>>>(9)
                .ok_or(StorageError::RevenueStreamEvidenceConflict)?;
            let stored_slot = claim_row
                .get::<_, Option<String>>(10)
                .as_deref()
                .map(|value| parse_u64(value, "stored release slot"))
                .transpose()?
                .ok_or(StorageError::RevenueStreamEvidenceConflict)?;
            if stored_signature == command.release.release_signature.0
                && stored_slot == command.release.finalized_slot
            {
                transaction.commit().await?;
                return Ok(RevenueStreamClaimOutcome::Duplicate);
            }
            return Err(StorageError::RevenueStreamEvidenceConflict);
        }
        if status != "pending"
            || command.release.plan_digest.0 != plan_digest
            || command.release.asset.0 != asset
            || command.release.recipient.0 != recipient
            || command.release.claim_digest.0 != claim_digest
            || command.release.amount_atomic != amount_atomic
        {
            return Err(StorageError::RevenueStreamEvidenceConflict);
        }
        let plan =
            load_revenue_stream_plan_for_update(&transaction, plan_id, command.merchant_id).await?;
        if command.release.finalized_slot < plan.spec.start_slot
            || plan
                .spec
                .end_slot
                .is_some_and(|end| command.release.finalized_slot > end)
        {
            return Err(StorageError::InvalidRevenueStream(
                "release is outside the stream window".into(),
            ));
        }
        let next_released = plan
            .released_total_atomic
            .checked_add(amount_atomic)
            .ok_or(StorageError::InvalidRevenueStream(
                "released total exceeds u64".into(),
            ))?;
        if next_released > plan.accrued_total_atomic {
            return Err(StorageError::RevenueStreamInsufficientAvailable);
        }
        transaction
            .execute(
                "UPDATE revenue_stream_claims
                 SET status = 'finalized', release_signature = $3,
                     finalized_slot = $4::text::numeric, updated_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND status = 'pending'",
                &[
                    &command.release.claim_id,
                    &command.merchant_id,
                    &command.release.release_signature.0,
                    &command.release.finalized_slot.to_string(),
                ],
            )
            .await?;
        transaction
            .execute(
                "UPDATE revenue_stream_plans
                 SET released_total_atomic = $3::text::numeric, updated_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND released_total_atomic = $4::text::numeric",
                &[
                    &plan_id.0,
                    &command.merchant_id,
                    &next_released.to_string(),
                    &plan.released_total_atomic.to_string(),
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO revenue_stream_claim_events
                    (claim_id, stream_plan_id, merchant_id, actor_subject, action, evidence)
                 VALUES ($1, $2, $3, $4, 'finalized', $5)",
                &[
                    &command.release.claim_id,
                    &plan_id.0,
                    &command.merchant_id,
                    &command.actor_subject,
                    &serde_json::json!({
                        "release_signature": command.release.release_signature,
                        "finalized_slot": command.release.finalized_slot,
                        "amount_atomic": amount_atomic,
                    }),
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"revenue_stream_claim",
                    &command.release.claim_id,
                    &"tax.revenue_stream.claim_finalized",
                    &serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": command.merchant_id,
                        "plan_id": plan_id.0,
                        "claim_id": command.release.claim_id,
                        "release_signature": command.release.release_signature,
                        "finalized_slot": command.release.finalized_slot,
                    }),
                ],
            )
            .await?;
        transaction.commit().await?;
        let _ = nonce;
        let _ = accrual_total_atomic;
        Ok(RevenueStreamClaimOutcome::Recorded)
    }

    /// Creates one tenant-owned draft, its append-only creation evidence, and
    /// its first outbox publication record in one serializable transaction.
    /// The draft is validated by the pure tax engine before any durable write.
    pub async fn create_vault_release_claim(
        &self,
        command: CreateVaultReleaseClaimCommand,
    ) -> Result<VaultReleaseClaimOutcome, StorageError> {
        validate_vault_release_claim_command(&command)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let route = transaction
            .query_opt(
                "SELECT status, mint, token_program, tax_vault,
                        mint_decimals,
                        effective_from <= now()
                            AND (effective_until IS NULL OR now() < effective_until)
                 FROM settlement_routes
                 WHERE id = $1 AND merchant_id = $2
                   AND verification_schema = 'settlement-route-verification/v2'
                 FOR SHARE",
                &[&command.route_id, &command.merchant_id],
            )
            .await?
            .ok_or(StorageError::SettlementRouteNotFound)?;
        let route_status: String = route.get(0);
        let route_effective: bool = route.get(5);
        let mint: String = route.get(1);
        let token_program: String = route.get(2);
        let tax_vault: String = route.get(3);
        let mint_decimals: i16 = route.get(4);
        if !(0..=9).contains(&mint_decimals)
            || !matches!(
                token_program.as_str(),
                SPL_TOKEN_PROGRAM_ID | TOKEN_2022_PROGRAM_ID
            )
        {
            return Err(StorageError::SettlementRouteIntegrity(
                "verified release route has unsupported asset metadata".to_string(),
            ));
        }
        if tax_vault == command.destination {
            return Err(StorageError::InvalidVaultReleaseClaim(
                "tax_vault and destination must be distinct".into(),
            ));
        }
        let request_digest =
            vault_release_request_digest(&command, &mint, &token_program, &tax_vault)?;
        if route_status != "active" || !route_effective {
            // Route suspension prevents new release proposals, but it must
            // not turn a previously accepted idempotent request into an
            // unknown outcome. The route row remains locked for this
            // transaction, so lifecycle changes cannot race this decision.
            let existing = transaction
                .query_opt(
                    "SELECT id, merchant_id, idempotency_key, request_digest, mint,
                            token_program, tax_vault, destination, claim_pda,
                            nonce::text, amount_atomic::text, reference_digest,
                            status, proposer_subject, approver_subject,
                            extract(epoch from execute_after)::bigint,
                            extract(epoch from expires_at)::bigint,
                            transaction_signature, slot::text, last_error_code
                     FROM vault_release_claims
                     WHERE merchant_id = $1 AND idempotency_key = $2
                     FOR UPDATE",
                    &[&command.merchant_id, &command.idempotency_key],
                )
                .await?;
            if let Some(row) = existing {
                let claim = parse_vault_release_claim_row(row)?;
                if claim.request_digest != request_digest {
                    return Err(StorageError::VaultReleaseClaimIdempotencyConflict);
                }
                transaction.commit().await?;
                return Ok(VaultReleaseClaimOutcome::Replay(claim));
            }
            return Err(StorageError::SettlementRouteUnavailable);
        }
        let id = Uuid::now_v7();
        let execute_after_unix = command.execute_after_unix as f64;
        let expires_at_unix = command.expires_at_unix as f64;
        let inserted = transaction
            .query_opt(
                "INSERT INTO vault_release_claims
                    (id, merchant_id, idempotency_key, request_digest, mint,
                     token_program, tax_vault, destination, nonce, amount_atomic,
                     reference_digest, proposer_subject, execute_after, expires_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::text::numeric,
                         $10::text::numeric, $11, $12,
                         to_timestamp($13::double precision),
                         to_timestamp($14::double precision))
                 ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
                 RETURNING id, merchant_id, idempotency_key, request_digest,
                           mint, token_program, tax_vault, destination, claim_pda,
                           nonce::text, amount_atomic::text, reference_digest,
                           status, proposer_subject, approver_subject,
                           extract(epoch from execute_after)::bigint,
                           extract(epoch from expires_at)::bigint,
                           transaction_signature, slot::text, last_error_code",
                &[
                    &id,
                    &command.merchant_id,
                    &command.idempotency_key,
                    &&request_digest[..],
                    &mint,
                    &token_program,
                    &tax_vault,
                    &command.destination,
                    &command.nonce.to_string(),
                    &command.amount_atomic.to_string(),
                    &&command.reference_digest[..],
                    &command.actor_subject,
                    &execute_after_unix,
                    &expires_at_unix,
                ],
            )
            .await?;
        if let Some(row) = inserted {
            let claim = parse_vault_release_claim_row(row)?;
            transaction
                .execute(
                    "INSERT INTO vault_release_claim_events
                        (claim_id, merchant_id, actor_subject, action,
                         from_status, to_status, evidence)
                     VALUES ($1, $2, $3, 'propose', 'none', 'proposed', $4)",
                    &[
                        &claim.id,
                        &command.merchant_id,
                        &command.actor_subject,
                        &serde_json::json!({
                            "request_digest": request_digest,
                            "route_id": command.route_id,
                            "reference_digest": command.reference_digest,
                        }),
                    ],
                )
                .await?;
            transaction
                .execute(
                    "INSERT INTO outbox_events
                        (aggregate_type, aggregate_id, event_type, payload)
                     VALUES ($1, $2, $3, $4)",
                    &[
                        &"vault_release_claim",
                        &claim.id,
                        &"tax.vault_release_claim.proposed",
                        &serde_json::json!({
                            "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                            "merchant_id": command.merchant_id,
                            "claim": claim,
                        }),
                    ],
                )
                .await?;
            transaction.commit().await?;
            return Ok(VaultReleaseClaimOutcome::Created(claim));
        }
        let row = transaction
            .query_one(
                "SELECT id, merchant_id, idempotency_key, request_digest, mint,
                        token_program, tax_vault, destination, claim_pda,
                        nonce::text, amount_atomic::text, reference_digest,
                        status, proposer_subject, approver_subject,
                        extract(epoch from execute_after)::bigint,
                        extract(epoch from expires_at)::bigint,
                        transaction_signature, slot::text, last_error_code
                 FROM vault_release_claims
                 WHERE merchant_id = $1 AND idempotency_key = $2
                 FOR UPDATE",
                &[&command.merchant_id, &command.idempotency_key],
            )
            .await?;
        let claim = parse_vault_release_claim_row(row)?;
        if claim.request_digest != request_digest {
            return Err(StorageError::VaultReleaseClaimIdempotencyConflict);
        }
        transaction.commit().await?;
        Ok(VaultReleaseClaimOutcome::Replay(claim))
    }

    pub async fn transition_vault_release_claim(
        &self,
        command: VaultReleaseClaimTransition,
    ) -> Result<VaultReleaseClaimTransitionResult, StorageError> {
        validate_vault_release_transition(&command)?;
        let action = vault_release_claim_action(&command.to_status)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT status, proposer_subject, approver_subject
                 FROM vault_release_claims
                 WHERE id = $1 AND merchant_id = $2
                 FOR UPDATE",
                &[&command.claim_id, &command.merchant_id],
            )
            .await?
            .ok_or(StorageError::VaultReleaseClaimNotFound)?;
        let from_status: String = row.get(0);
        let proposer_subject: String = row.get(1);
        let current_approver: Option<String> = row.get(2);
        if command.to_status == "approved" && command.actor_subject == proposer_subject {
            return Err(StorageError::VaultReleaseClaimActorConflict);
        }
        if command.to_status == "executed"
            && current_approver.as_deref() != Some(command.actor_subject.as_str())
        {
            return Err(StorageError::VaultReleaseClaimActorConflict);
        }
        let approver_subject = if command.to_status == "approved" {
            Some(command.actor_subject.clone())
        } else {
            current_approver
        };
        let slot_text = command.slot.map(|value| value.to_string());
        let updated = transaction
            .execute(
                "UPDATE vault_release_claims
                 SET status = $3,
                     claim_pda = COALESCE($4, claim_pda),
                     approver_subject = $5,
                     transaction_signature = COALESCE($6, transaction_signature),
                     slot = COALESCE($7::text::numeric, slot),
                     last_error_code = COALESCE($8, last_error_code),
                     updated_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND status = $9",
                &[
                    &command.claim_id,
                    &command.merchant_id,
                    &command.to_status,
                    &command.claim_pda,
                    &approver_subject,
                    &command.transaction_signature,
                    &slot_text,
                    &command.last_error_code,
                    &from_status,
                ],
            )
            .await?;
        if updated != 1 {
            transaction.rollback().await?;
            return Err(StorageError::InvalidVaultReleaseClaimTransition);
        }
        transaction
            .execute(
                "INSERT INTO vault_release_claim_events
                    (claim_id, merchant_id, actor_subject, action,
                     from_status, to_status, evidence)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[
                    &command.claim_id,
                    &command.merchant_id,
                    &command.actor_subject,
                    &action,
                    &from_status,
                    &command.to_status,
                    &command.evidence,
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"vault_release_claim",
                    &command.claim_id,
                    &format!("tax.vault_release_claim.{action}"),
                    &serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": command.merchant_id,
                        "claim_id": command.claim_id,
                        "from_status": from_status,
                        "to_status": command.to_status,
                    }),
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(VaultReleaseClaimTransitionResult {
            claim_id: command.claim_id,
            from_status,
            to_status: command.to_status,
            approver_subject,
        })
    }

    pub async fn load_vault_release_claim(
        &self,
        claim_id: Uuid,
        merchant_id: Uuid,
    ) -> Result<VaultReleaseClaim, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT id, merchant_id, idempotency_key, request_digest, mint,
                        token_program, tax_vault, destination, claim_pda,
                        nonce::text, amount_atomic::text, reference_digest,
                        status, proposer_subject, approver_subject,
                        extract(epoch from execute_after)::bigint,
                        extract(epoch from expires_at)::bigint,
                        transaction_signature, slot::text, last_error_code
                 FROM vault_release_claims
                 WHERE id = $1 AND merchant_id = $2",
                &[&claim_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::VaultReleaseClaimNotFound)?;
        parse_vault_release_claim_row(row)
    }

    pub async fn create_rule_draft(
        &self,
        command: CreateRuleDraftCommand,
    ) -> Result<RuleDraftOutcome, StorageError> {
        validate_rule_draft_command(&command)?;
        let rule_digest = command.rule_set.canonical_digest()?;
        let request_digest = rule_draft_request_digest(&command, rule_digest)?;
        let idempotency_key = format!("rule-draft/{}", command.idempotency_key);
        let rule_document = serde_json::to_value(RuleSetDocument::new(command.rule_set.clone()))?;
        let provenance = command.provenance.clone();
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;

        let inserted = transaction
            .query_opt(
                "INSERT INTO idempotency_records
                    (merchant_id, idempotency_key, request_digest)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
                 RETURNING id",
                &[&command.merchant_id, &idempotency_key, &&request_digest[..]],
            )
            .await?;
        if inserted.is_none() {
            let existing = transaction
                .query_one(
                    "SELECT request_digest, response_document
                     FROM idempotency_records
                     WHERE merchant_id = $1 AND idempotency_key = $2
                     FOR UPDATE",
                    &[&command.merchant_id, &idempotency_key],
                )
                .await?;
            let existing_digest: Vec<u8> = existing.get(0);
            if existing_digest.as_slice() != request_digest {
                return Err(StorageError::IdempotencyConflict);
            }
            let stored_response: Option<Value> = existing.get(1);
            let Some(stored_response) = stored_response else {
                return Err(StorageError::IdempotencyInProgress);
            };
            let response: CreatedRuleDraft = serde_json::from_value(stored_response)
                .map_err(|_| StorageError::InvalidStoredResponse)?;
            transaction.commit().await?;
            return Ok(RuleDraftOutcome::Replay(response));
        }

        let rule_id = Uuid::now_v7();
        let inserted_rule = match transaction
            .query_one(
                "INSERT INTO rule_sets
                    (id, merchant_id, external_id, version, jurisdiction, regime,
                     status, rule_digest, rule_document, provenance, effective_from,
                     effective_until, author_subject, governance_revision)
                 VALUES ($1, $2, $3, $4, $5, $6, 'draft', $7, $8, $9,
                         to_timestamp($10::double precision),
                         CASE WHEN $11::double precision IS NULL THEN NULL
                              ELSE to_timestamp($11::double precision) END,
                         $12, 0)
                 RETURNING id, external_id, version, jurisdiction, regime, status,
                           to_char(effective_from AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                           to_char(effective_until AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')",
                &[
                    &rule_id,
                    &command.merchant_id,
                    &command.external_id,
                    &i32::try_from(command.version).map_err(|_| {
                        StorageError::InvalidRuleDraft(
                            "rule version exceeds the PostgreSQL range".to_string(),
                        )
                    })?,
                    &command.jurisdiction,
                    &command.regime,
                    &&rule_digest[..],
                    &rule_document,
                    &provenance,
                    &(command.effective_from_unix as f64),
                    &command.effective_until_unix.map(|value| value as f64),
                    &command.actor_subject,
                ],
            )
            .await
        {
            Ok(row) => row,
            Err(error)
                if error
                    .code()
                    .is_some_and(|code| code.code() == "23505") =>
            {
                return Err(StorageError::RuleDraftConflict);
            }
            Err(error) => return Err(StorageError::Database(error)),
        };
        let response = CreatedRuleDraft {
            rule_set_id: inserted_rule.get(0),
            external_id: inserted_rule.get(1),
            version: inserted_rule.get(2),
            jurisdiction: inserted_rule.get(3),
            regime: inserted_rule.get(4),
            status: inserted_rule.get(5),
            rule_digest,
            effective_from: inserted_rule.get(6),
            effective_until: inserted_rule.get(7),
            governance_revision: 0,
        };
        transaction
            .execute(
                "INSERT INTO rule_governance_events
                    (rule_set_id, actor_subject, action, from_status, to_status, evidence)
                 VALUES ($1, $2, 'create', 'none', 'draft', $3)",
                &[
                    &response.rule_set_id,
                    &command.actor_subject,
                    &serde_json::json!({
                        "source": "tax-rule-authoring",
                        "rule_digest": rule_digest,
                    }),
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"rule_set",
                    &response.rule_set_id,
                    &"tax.rule.draft.created",
                    &serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": command.merchant_id,
                        "rule_set": response,
                    }),
                ],
            )
            .await?;
        let response_document = serde_json::to_value(&response)?;
        transaction
            .execute(
                "UPDATE idempotency_records
                 SET response_status = 201, response_document = $3, completed_at = now()
                 WHERE merchant_id = $1 AND idempotency_key = $2",
                &[&command.merchant_id, &idempotency_key, &response_document],
            )
            .await?;
        transaction.commit().await?;
        Ok(RuleDraftOutcome::Created(response))
    }

    pub async fn quote(&self, command: QuoteCommand) -> Result<QuoteOutcome, StorageError> {
        validate_command(&command)?;
        let request_digest = quote_request_digest(&command)?;
        for attempt in 0..MAX_QUOTE_RETRY_ATTEMPTS {
            match self.quote_once(&command, request_digest).await {
                Ok(outcome) => return Ok(outcome),
                Err(error)
                    if attempt + 1 < MAX_QUOTE_RETRY_ATTEMPTS
                        && is_retryable_quote_error(&error) =>
                {
                    tracing::warn!(
                        attempt = attempt + 1,
                        max_attempts = MAX_QUOTE_RETRY_ATTEMPTS,
                        "retrying quote transaction after a transient PostgreSQL conflict"
                    );
                    tokio::time::sleep(QUOTE_RETRY_BACKOFF * (attempt as u32 + 1)).await;
                }
                Err(error) => return Err(error),
            }
        }
        Err(StorageError::InvalidConfiguration(
            "quote retry attempt budget must be positive".to_string(),
        ))
    }

    /// Persists an authenticated, tenant-scoped document quote without
    /// widening the aggregate quote/settlement contract.
    pub async fn document_quote(
        &self,
        command: DocumentQuoteCommand,
    ) -> Result<DocumentQuoteOutcome, StorageError> {
        validate_document_quote_command(&command)?;
        let request_digest = document_quote_request_digest(&command)?;
        for attempt in 0..MAX_QUOTE_RETRY_ATTEMPTS {
            match self.document_quote_once(&command, request_digest).await {
                Ok(outcome) => return Ok(outcome),
                Err(error)
                    if attempt + 1 < MAX_QUOTE_RETRY_ATTEMPTS
                        && is_retryable_quote_error(&error) =>
                {
                    tracing::warn!(
                        attempt = attempt + 1,
                        max_attempts = MAX_QUOTE_RETRY_ATTEMPTS,
                        "retrying document quote transaction after a transient PostgreSQL conflict"
                    );
                    tokio::time::sleep(QUOTE_RETRY_BACKOFF * (attempt as u32 + 1)).await;
                }
                Err(error) => return Err(error),
            }
        }
        Err(StorageError::InvalidConfiguration(
            "document quote retry attempt budget must be positive".to_string(),
        ))
    }

    pub async fn create_applicability_context(
        &self,
        command: CreateApplicabilityContextCommand,
    ) -> Result<ApplicabilityContextOutcome, StorageError> {
        validate_create_applicability_context_command(&command)?;
        let request_digest = applicability_context_request_digest(&command)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;

        let inserted = transaction
            .query_opt(
                "INSERT INTO idempotency_records
                    (merchant_id, idempotency_key, request_digest)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
                 RETURNING id",
                &[
                    &command.merchant_id,
                    &command.idempotency_key,
                    &&request_digest[..],
                ],
            )
            .await?;
        if inserted.is_none() {
            let existing = transaction
                .query_one(
                    "SELECT request_digest, response_document
                     FROM idempotency_records
                     WHERE merchant_id = $1 AND idempotency_key = $2
                     FOR UPDATE",
                    &[&command.merchant_id, &command.idempotency_key],
                )
                .await?;
            let existing_digest: Vec<u8> = existing.get(0);
            if existing_digest.as_slice() != request_digest {
                return Err(StorageError::IdempotencyConflict);
            }
            let stored_response: Option<Value> = existing.get(1);
            let Some(stored_response) = stored_response else {
                return Err(StorageError::IdempotencyInProgress);
            };
            let response: StoredApplicabilityContext = serde_json::from_value(stored_response)
                .map_err(|_| StorageError::InvalidStoredResponse)?;
            transaction.commit().await?;
            return Ok(ApplicabilityContextOutcome::Replay(response));
        }

        let context_id = Uuid::now_v7();
        let unsigned = &command.attestation.unsigned;
        let facts_document = serde_json::to_value(&unsigned.facts)?;
        let decision_document = serde_json::to_value(&unsigned.decision)?;
        let attestation_document = serde_json::to_value(&command.attestation)?;
        let evidence_expires_at_unix = unsigned.facts.valid_until_unix;
        let policy_digest = unsigned.decision.policy_digest;
        let decision_digest = unsigned
            .decision
            .canonical_digest()
            .map_err(|error| StorageError::InvalidApplicabilityContext(error.to_string()))?;
        let stored = StoredApplicabilityContext {
            context_id,
            merchant_id: command.merchant_id,
            status: "draft".to_string(),
            policy_id: unsigned.decision.policy_id.clone(),
            policy_version: unsigned.decision.policy_version,
            policy_digest,
            facts_digest: unsigned.decision.facts_digest,
            decision_digest,
            jurisdiction_resolution_digest: unsigned.jurisdiction_resolution_digest,
            evidence_digest: unsigned.facts.evidence_digest,
            attestation_digest: command.attestation.payload_digest,
            effective_from_unix: command.effective_from_unix,
            effective_until_unix: command.effective_until_unix,
            evidence_expires_at_unix,
            provenance: command.provenance.clone(),
            attestation: command.attestation.clone(),
        };
        transaction
            .execute(
                "INSERT INTO tax_applicability_contexts
                    (id, merchant_id, policy_id, policy_version, policy_digest,
                     facts_digest, decision_digest, jurisdiction_resolution_digest,
                     evidence_digest, attestation_digest, facts_document,
                     decision_document, attestation_document, status,
                     effective_from, effective_until, evidence_expires_at,
                     provenance, author_subject)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                         $13, $14,
                         TIMESTAMPTZ 'epoch' + ($15::bigint * INTERVAL '1 second'),
                         CASE WHEN $16::bigint IS NULL THEN NULL
                              ELSE TIMESTAMPTZ 'epoch' + ($16::bigint * INTERVAL '1 second') END,
                         TIMESTAMPTZ 'epoch' + ($17::bigint * INTERVAL '1 second'),
                         $18, $19)",
                &[
                    &context_id,
                    &command.merchant_id,
                    &stored.policy_id,
                    &(stored.policy_version as i32),
                    &&stored.policy_digest[..],
                    &&stored.facts_digest[..],
                    &&stored.decision_digest[..],
                    &&stored.jurisdiction_resolution_digest[..],
                    &&stored.evidence_digest[..],
                    &&stored.attestation_digest[..],
                    &Json(facts_document),
                    &Json(decision_document),
                    &Json(attestation_document),
                    &stored.status,
                    &stored.effective_from_unix,
                    &stored.effective_until_unix,
                    &stored.evidence_expires_at_unix,
                    &Json(stored.provenance.clone()),
                    &command.actor_subject,
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO tax_applicability_context_events
                    (context_id, merchant_id, event_type, actor_subject, evidence)
                 VALUES ($1, $2, 'created', $3, $4)",
                &[
                    &context_id,
                    &command.merchant_id,
                    &command.actor_subject,
                    &Json(serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "request_digest": request_digest,
                    })),
                ],
            )
            .await?;
        let response_document = serde_json::to_value(&stored)?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"tax_applicability_context",
                    &context_id,
                    &"tax.applicability.context.created",
                    &Json(serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": command.merchant_id,
                        "context": response_document,
                    })),
                ],
            )
            .await?;
        transaction
            .execute(
                "UPDATE idempotency_records
                 SET response_status = 201, response_document = $3, completed_at = now()
                 WHERE merchant_id = $1 AND idempotency_key = $2",
                &[
                    &command.merchant_id,
                    &command.idempotency_key,
                    &Json(response_document),
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(ApplicabilityContextOutcome::Created(stored))
    }

    pub async fn transition_applicability_context(
        &self,
        transition: ApplicabilityContextTransition,
    ) -> Result<(), StorageError> {
        validate_applicability_context_transition(&transition)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT status FROM tax_applicability_contexts
                 WHERE id = $1 AND merchant_id = $2 FOR UPDATE",
                &[&transition.context_id, &transition.merchant_id],
            )
            .await?
            .ok_or(StorageError::ApplicabilityContextNotFound)?;
        let current_status: String = row.get(0);
        if current_status != transition.expected_status {
            return Err(StorageError::ApplicabilityContextTransitionConflict);
        }
        let event_type = match transition.next_status.as_str() {
            "review" => "submitted",
            "approved" => "approved",
            "published" => "published",
            "rejected" => "rejected",
            "retired" => "retired",
            _ => return Err(StorageError::ApplicabilityContextTransitionConflict),
        };
        let transition_evidence = transition.evidence.clone();
        transaction
            .execute(
                "UPDATE tax_applicability_contexts
                 SET status = $3,
                     reviewer_subject = CASE WHEN $3 = 'approved' THEN $4 ELSE reviewer_subject END,
                     approver_subject = CASE WHEN $3 = 'published' THEN $4 ELSE approver_subject END,
                     updated_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND status = $5",
                &[
                    &transition.context_id,
                    &transition.merchant_id,
                    &transition.next_status,
                    &transition.actor_subject,
                    &transition.expected_status,
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO tax_applicability_context_events
                    (context_id, merchant_id, event_type, actor_subject, evidence)
                 VALUES ($1, $2, $3, $4, $5)",
                &[
                    &transition.context_id,
                    &transition.merchant_id,
                    &event_type,
                    &transition.actor_subject,
                    &Json(transition_evidence.clone()),
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"tax_applicability_context",
                    &transition.context_id,
                    &format!("tax.applicability.context.{event_type}"),
                    &Json(serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": transition.merchant_id,
                        "context_id": transition.context_id,
                        "from_status": transition.expected_status,
                        "to_status": transition.next_status,
                        "actor_subject": transition.actor_subject,
                        "evidence": transition_evidence,
                    })),
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn load_applicability_context(
        &self,
        merchant_id: Uuid,
        context_id: Uuid,
        now_unix: i64,
        require_published: bool,
    ) -> Result<StoredApplicabilityContext, StorageError> {
        if merchant_id.is_nil() || context_id.is_nil() || now_unix <= 0 {
            return Err(StorageError::InvalidApplicabilityContext(
                "context lookup identity or time is invalid".into(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT status, policy_id, policy_version, policy_digest,
                        facts_digest, decision_digest,
                        jurisdiction_resolution_digest, evidence_digest,
                        attestation_digest,
                        floor(extract(epoch FROM effective_from))::bigint,
                        CASE WHEN effective_until IS NULL THEN NULL
                             ELSE floor(extract(epoch FROM effective_until))::bigint END,
                        floor(extract(epoch FROM evidence_expires_at))::bigint,
                        facts_document, decision_document, attestation_document,
                        provenance
                 FROM tax_applicability_contexts
                 WHERE id = $1 AND merchant_id = $2
                   AND ($4::boolean = false OR status = 'published')
                   AND effective_from <= TIMESTAMPTZ 'epoch' + ($3::bigint * INTERVAL '1 second')
                   AND (effective_until IS NULL OR effective_until > TIMESTAMPTZ 'epoch' + ($3::bigint * INTERVAL '1 second'))
                   AND evidence_expires_at > TIMESTAMPTZ 'epoch' + ($3::bigint * INTERVAL '1 second')",
                &[&context_id, &merchant_id, &now_unix, &require_published],
            )
            .await?
            .ok_or(StorageError::ApplicabilityContextNotFound)?;
        parse_stored_applicability_context(context_id, merchant_id, row)
    }

    /// Loads the complete tenant-owned projection for governance/read paths.
    /// Unlike quote binding, operator history must remain inspectable after a
    /// context is retired or its source evidence expires. Callers must still
    /// authenticate the merchant boundary before invoking this port.
    pub async fn load_applicability_context_for_operator(
        &self,
        merchant_id: Uuid,
        context_id: Uuid,
    ) -> Result<StoredApplicabilityContext, StorageError> {
        if merchant_id.is_nil() || context_id.is_nil() {
            return Err(StorageError::InvalidApplicabilityContext(
                "context lookup identity is invalid".into(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT status, policy_id, policy_version, policy_digest,
                        facts_digest, decision_digest,
                        jurisdiction_resolution_digest, evidence_digest,
                        attestation_digest,
                        floor(extract(epoch FROM effective_from))::bigint,
                        CASE WHEN effective_until IS NULL THEN NULL
                             ELSE floor(extract(epoch FROM effective_until))::bigint END,
                        floor(extract(epoch FROM evidence_expires_at))::bigint,
                        facts_document, decision_document, attestation_document,
                        provenance
                 FROM tax_applicability_contexts
                 WHERE id = $1 AND merchant_id = $2",
                &[&context_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::ApplicabilityContextNotFound)?;
        parse_stored_applicability_context(context_id, merchant_id, row)
    }

    async fn document_quote_once(
        &self,
        command: &DocumentQuoteCommand,
        request_digest: Digest32,
    ) -> Result<DocumentQuoteOutcome, StorageError> {
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let inserted = transaction
            .query_opt(
                "INSERT INTO idempotency_records
                    (merchant_id, idempotency_key, request_digest)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
                 RETURNING id",
                &[
                    &command.merchant_id,
                    &command.idempotency_key,
                    &&request_digest[..],
                ],
            )
            .await?;
        if inserted.is_none() {
            let existing = transaction
                .query_one(
                    "SELECT request_digest, response_document
                     FROM idempotency_records
                     WHERE merchant_id = $1 AND idempotency_key = $2
                     FOR UPDATE",
                    &[&command.merchant_id, &command.idempotency_key],
                )
                .await?;
            let existing_digest: Vec<u8> = existing.get(0);
            if existing_digest.as_slice() != request_digest {
                return Err(StorageError::IdempotencyConflict);
            }
            let stored_response: Option<Value> = existing.get(1);
            let Some(stored_response) = stored_response else {
                return Err(StorageError::IdempotencyInProgress);
            };
            let response: StoredDocumentQuote = serde_json::from_value(stored_response)
                .map_err(|_| StorageError::InvalidStoredResponse)?;
            transaction.commit().await?;
            return Ok(DocumentQuoteOutcome::Replay(response));
        }

        let applicability_binding =
            load_quote_applicability_binding(&transaction, command.applicability_context_id)
                .await?;

        let rule_row = transaction
            .query_opt(
                "SELECT r.id, r.rule_document, r.rule_digest,
                        jp.policy_document, jp.policy_digest
                 FROM rule_sets r
                 LEFT JOIN jurisdiction_policies jp
                   ON jp.rule_set_id = r.id
                  AND jp.merchant_id = r.merchant_id
                  AND jp.status = 'published'
                  AND jp.effective_from <= now()
                  AND (jp.effective_until IS NULL OR now() < jp.effective_until)
                 WHERE r.merchant_id = $1
                   AND r.external_id = $2
                   AND r.version = $3
                   AND r.jurisdiction = $4
                   AND r.regime = $5
                   AND r.status = 'published'
                   AND r.effective_from <= now()
                   AND (r.effective_until IS NULL OR now() < r.effective_until)
                 ORDER BY r.effective_from DESC
                 LIMIT 1",
                &[
                    &command.merchant_id,
                    &command.rule_set_id,
                    &(command.rule_version as i32),
                    &command.jurisdiction,
                    &command.regime,
                ],
            )
            .await?;
        let Some(rule_row) = rule_row else {
            return Err(StorageError::RuleNotFound);
        };
        let database_rule_id: Uuid = rule_row.get(0);
        let rule_document: Value = rule_row.get(1);
        let stored_rule_digest: Vec<u8> = rule_row.get(2);
        let policy_document: Option<Value> = rule_row.get(3);
        let stored_policy_digest: Option<Vec<u8>> = rule_row.get(4);
        let policy_document = policy_document.ok_or(StorageError::JurisdictionPolicyNotFound)?;
        let stored_policy_digest =
            stored_policy_digest.ok_or(StorageError::JurisdictionPolicyNotFound)?;
        let policy: ResolutionPolicy = serde_json::from_value(policy_document)?;
        let computed_policy_digest = policy.canonical_digest().map_err(|error| {
            StorageError::ReplayEvidenceIntegrity(format!(
                "jurisdiction policy is invalid: {error}"
            ))
        })?;
        if stored_policy_digest.as_slice() != computed_policy_digest {
            return Err(StorageError::JurisdictionPolicyDigestMismatch);
        }
        let rule_set = decode_persisted_rule_document(rule_document)?;
        if rule_set.id != command.rule_set_id || rule_set.version != command.rule_version {
            return Err(StorageError::RuleIdentityMismatch);
        }
        let computed_rule_digest = rule_set.canonical_digest()?;
        if stored_rule_digest.as_slice() != computed_rule_digest {
            return Err(StorageError::RuleDigestMismatch);
        }
        let compiled_rule = self
            .compiled_rule_cache
            .get_or_compile(computed_rule_digest, &rule_set)?;
        let result = compiled_rule.calculate_document_with_jurisdiction_resolution(
            &command.facts,
            &policy,
            &command.jurisdiction_candidates,
        )?;
        if result.jurisdiction != command.jurisdiction
            || result.jurisdiction != command.facts.jurisdiction
        {
            return Err(StorageError::JurisdictionMismatch);
        }
        let mut persisted_facts = command.facts.clone();
        persisted_facts.jurisdiction_resolution = result.jurisdiction_resolution.clone();
        let (calculation_id, expires_at_unix) = insert_document_calculation(
            &transaction,
            command,
            &persisted_facts,
            database_rule_id,
            &result,
            &CalculationInsertOptions {
                engine_version: &self.engine_version,
                evidence_cipher: self.evidence_cipher.as_ref(),
                applicability_binding: applicability_binding.as_ref(),
            },
        )
        .await?;
        let response = StoredDocumentQuote {
            calculation_id,
            status: CALCULATION_STATUS.to_string(),
            expires_at_unix,
            schema: command.schema.clone(),
            applicability_context_id: applicability_binding.map(|binding| binding.context_id),
            applicability_attestation_digest: applicability_binding
                .map(|binding| binding.attestation_digest),
            result,
        };
        let response_document = serde_json::to_value(&response)?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"tax_document_calculation",
                    &calculation_id,
                    &"tax.document_calculation.created",
                    &outbox_payload(command.merchant_id, response_document.clone()),
                ],
            )
            .await?;
        transaction
            .execute(
                "UPDATE idempotency_records
                 SET response_status = 200,
                     response_document = $3,
                     completed_at = now()
                 WHERE merchant_id = $1 AND idempotency_key = $2",
                &[
                    &command.merchant_id,
                    &command.idempotency_key,
                    &response_document,
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(DocumentQuoteOutcome::Created(response))
    }

    pub async fn verify_document_quote_replay(
        &self,
        merchant_id: Uuid,
        calculation_id: Uuid,
    ) -> Result<DocumentQuoteReplayVerification, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT dc.status, dc.document_schema, dc.engine_version,
                        dc.facts_digest, dc.result_document,
                        dc.evidence_refs, dc.facts_ciphertext, dc.facts_nonce,
                        dc.facts_key_id,
                        rs.rule_document, rs.rule_digest
                 FROM tax_document_calculations dc
                 JOIN rule_sets rs ON rs.id = dc.rule_set_id
                    AND rs.merchant_id = dc.merchant_id
                 WHERE dc.id = $1 AND dc.merchant_id = $2",
                &[&calculation_id, &merchant_id],
            )
            .await?;
        let Some(row) = row else {
            if client
                .query_opt(
                    "SELECT 1 FROM tax_calculations
                     WHERE id = $1 AND merchant_id = $2",
                    &[&calculation_id, &merchant_id],
                )
                .await?
                .is_some()
            {
                return Err(StorageError::AggregateCalculationScopeMismatch);
            }
            return Err(StorageError::DocumentQuoteNotFound);
        };
        let status: String = row.get(0);
        let schema: String = row.get(1);
        let engine_version: String = row.get(2);
        let facts_digest_bytes: Vec<u8> = row.get(3);
        let result_document: Value = row.get(4);
        let evidence_refs: Value = row.get(5);
        let ciphertext: Vec<u8> = row.get(6);
        let nonce: Vec<u8> = row.get(7);
        let key_id: String = row.get(8);
        let facts_digest: Digest32 = facts_digest_bytes.clone().try_into().map_err(|_| {
            StorageError::DocumentReplayEvidenceIntegrity(
                "facts digest is not 32 bytes".to_string(),
            )
        })?;
        let facts = self
            .evidence_cipher
            .as_ref()
            .ok_or(StorageError::EvidenceCipherUnavailable)?
            .decrypt_facts(merchant_id, &facts_digest, &key_id, &nonce, &ciphertext)
            .map_err(|error| StorageError::DocumentReplayEvidenceIntegrity(error.to_string()))?;
        let rule_set = decode_persisted_rule_document(row.get(9))
            .map_err(|error| StorageError::DocumentReplayEvidenceIntegrity(error.to_string()))?;
        let stored_rule_digest: Digest32 = row.get::<_, Vec<u8>>(10).try_into().map_err(|_| {
            StorageError::DocumentReplayEvidenceIntegrity("rule digest is not 32 bytes".into())
        })?;
        let document_facts: TaxDocumentFacts = serde_json::from_value(facts)
            .map_err(|error| StorageError::DocumentReplayEvidenceIntegrity(error.to_string()))?;
        let result: TaxDocumentResult = serde_json::from_value(result_document)
            .map_err(|error| StorageError::DocumentReplayEvidenceIntegrity(error.to_string()))?;
        verify_replayed_document_fx_evidence(merchant_id, &document_facts, &evidence_refs)?;
        if schema != TAX_DOCUMENT_SCHEMA_V1
            || engine_version != self.engine_version
            || document_facts.canonical_digest()? != facts_digest
            || rule_set.canonical_digest()? != stored_rule_digest
            || result.facts_digest != facts_digest
            || result.rule_digest != stored_rule_digest
        {
            return Err(StorageError::DocumentReplayEvidenceIntegrity(
                "document evidence binding does not match persisted digests".into(),
            ));
        }
        result
            .verify_replay(&rule_set, &document_facts)
            .map_err(|error| StorageError::DocumentReplayEvidenceIntegrity(error.to_string()))?;
        Ok(DocumentQuoteReplayVerification {
            calculation_id,
            status,
            schema,
            engine_version,
            facts_digest,
            rule_digest: stored_rule_digest,
        })
    }

    async fn quote_once(
        &self,
        command: &QuoteCommand,
        request_digest: [u8; 32],
    ) -> Result<QuoteOutcome, StorageError> {
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let inserted = transaction
            .query_opt(
                "INSERT INTO idempotency_records
                    (merchant_id, idempotency_key, request_digest)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
                 RETURNING id",
                &[
                    &command.merchant_id,
                    &command.idempotency_key,
                    &&request_digest[..],
                ],
            )
            .await?;

        if inserted.is_none() {
            let existing = transaction
                .query_one(
                    "SELECT request_digest, response_document
                     FROM idempotency_records
                     WHERE merchant_id = $1 AND idempotency_key = $2
                     FOR UPDATE",
                    &[&command.merchant_id, &command.idempotency_key],
                )
                .await?;
            let existing_digest: Vec<u8> = existing.get(0);
            if existing_digest.as_slice() != request_digest {
                return Err(StorageError::IdempotencyConflict);
            }
            let stored_response: Option<Value> = existing.get(1);
            let Some(stored_response) = stored_response else {
                return Err(StorageError::IdempotencyInProgress);
            };
            let response: StoredQuote = serde_json::from_value(stored_response)
                .map_err(|_| StorageError::InvalidStoredResponse)?;
            transaction.commit().await?;
            return Ok(QuoteOutcome::Replay(response));
        }

        let applicability_binding =
            load_quote_applicability_binding(&transaction, command.applicability_context_id)
                .await?;

        let rule_row = transaction
            .query_opt(
                "SELECT r.id, r.rule_document, r.rule_digest,
                        jp.policy_document, jp.policy_digest
                 FROM rule_sets r
                 LEFT JOIN jurisdiction_policies jp
                   ON jp.rule_set_id = r.id
                  AND jp.merchant_id = r.merchant_id
                  AND jp.status = 'published'
                  AND jp.effective_from <= now()
                  AND (jp.effective_until IS NULL OR now() < jp.effective_until)
                 WHERE r.merchant_id = $1
                   AND r.external_id = $2
                   AND r.version = $3
                   AND r.jurisdiction = $4
                   AND r.regime = $5
                   AND r.status = 'published'
                   AND r.effective_from <= now()
                   AND (r.effective_until IS NULL OR now() < r.effective_until)
                 ORDER BY r.effective_from DESC
                 LIMIT 1",
                &[
                    &command.merchant_id,
                    &command.rule_set_id,
                    &(command.rule_version as i32),
                    &command.jurisdiction,
                    &command.regime,
                ],
            )
            .await?;
        let Some(rule_row) = rule_row else {
            return Err(StorageError::RuleNotFound);
        };
        let database_rule_id: Uuid = rule_row.get(0);
        let rule_document: Value = rule_row.get(1);
        let stored_rule_digest: Vec<u8> = rule_row.get(2);
        let policy_document: Option<Value> = rule_row.get(3);
        let stored_policy_digest: Option<Vec<u8>> = rule_row.get(4);
        let policy_document = policy_document.ok_or(StorageError::JurisdictionPolicyNotFound)?;
        let stored_policy_digest =
            stored_policy_digest.ok_or(StorageError::JurisdictionPolicyNotFound)?;
        let policy: ResolutionPolicy = serde_json::from_value(policy_document)?;
        let computed_policy_digest = policy.canonical_digest().map_err(|error| {
            StorageError::ReplayEvidenceIntegrity(format!(
                "jurisdiction policy is invalid: {error}"
            ))
        })?;
        if stored_policy_digest.as_slice() != computed_policy_digest {
            return Err(StorageError::JurisdictionPolicyDigestMismatch);
        }
        let rule_set = decode_persisted_rule_document(rule_document)?;
        if rule_set.id != command.rule_set_id || rule_set.version != command.rule_version {
            return Err(StorageError::RuleIdentityMismatch);
        }
        let computed_rule_digest = rule_set.canonical_digest()?;
        if stored_rule_digest.as_slice() != computed_rule_digest {
            return Err(StorageError::RuleDigestMismatch);
        }

        let compiled_rule = self
            .compiled_rule_cache
            .get_or_compile(computed_rule_digest, &rule_set)?;
        let result = compiled_rule.calculate_with_jurisdiction_resolution(
            &command.facts,
            &policy,
            &command.jurisdiction_candidates,
        )?;
        if result.jurisdiction != command.jurisdiction
            || result.jurisdiction != command.facts.jurisdiction
        {
            return Err(StorageError::JurisdictionMismatch);
        }
        let mut persisted_facts = command.facts.clone();
        persisted_facts.jurisdiction_resolution = result.jurisdiction_resolution.clone();
        let (calculation_id, expires_at_unix) = insert_calculation(
            &transaction,
            command,
            &persisted_facts,
            database_rule_id,
            &result,
            &CalculationInsertOptions {
                engine_version: &self.engine_version,
                evidence_cipher: self.evidence_cipher.as_ref(),
                applicability_binding: applicability_binding.as_ref(),
            },
        )
        .await?;
        let response = StoredQuote {
            calculation_id,
            status: CALCULATION_STATUS.to_string(),
            expires_at_unix,
            applicability_context_id: applicability_binding.map(|binding| binding.context_id),
            applicability_attestation_digest: applicability_binding
                .map(|binding| binding.attestation_digest),
            result,
        };
        let response_document = serde_json::to_value(&response)?;
        let event_payload = outbox_payload(command.merchant_id, response_document.clone());
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"tax_calculation",
                    &calculation_id,
                    &"tax.calculation.created",
                    &event_payload,
                ],
            )
            .await?;
        transaction
            .execute(
                "UPDATE idempotency_records
                 SET response_status = 200,
                     response_document = $3,
                     completed_at = now()
                 WHERE merchant_id = $1 AND idempotency_key = $2",
                &[
                    &command.merchant_id,
                    &command.idempotency_key,
                    &response_document,
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(QuoteOutcome::Created(response))
    }

    /// Loads only the authenticated merchant's persisted quote and enforces
    /// the database-owned status and expiry before settlement compilation.
    pub async fn load_quote_for_settlement(
        &self,
        merchant_id: Uuid,
        calculation_id: Uuid,
    ) -> Result<StoredQuote, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT status, result_document,
                        floor(extract(epoch FROM expires_at))::bigint,
                        expires_at <= now(), applicability_context_id,
                        applicability_attestation_digest
                 FROM tax_calculations
                 WHERE id = $1 AND merchant_id = $2",
                &[&calculation_id, &merchant_id],
            )
            .await?;
        let Some(row) = row else {
            if client
                .query_opt(
                    "SELECT 1 FROM tax_document_calculations
                     WHERE id = $1 AND merchant_id = $2",
                    &[&calculation_id, &merchant_id],
                )
                .await?
                .is_some()
            {
                return Err(StorageError::DocumentCalculationScopeMismatch);
            }
            return Err(StorageError::QuoteNotFound);
        };
        let status: String = row.get(0);
        if status != CALCULATION_STATUS && status != ATTESTED_CALCULATION_STATUS {
            return Err(StorageError::QuoteNotSettleable);
        }
        let expired: bool = row.get(3);
        if expired {
            return Err(StorageError::QuoteExpired);
        }
        let result: CalculationResult =
            serde_json::from_value(row.get(1)).map_err(|_| StorageError::InvalidStoredResponse)?;
        let applicability_context_id: Option<Uuid> = row.get(4);
        let applicability_attestation_digest = row
            .get::<_, Option<Vec<u8>>>(5)
            .map(parse_digest)
            .transpose()?;
        Ok(StoredQuote {
            calculation_id,
            status,
            expires_at_unix: row.get(2),
            applicability_context_id,
            applicability_attestation_digest,
            result,
        })
    }

    /// Creates a public guest-checkout handoff only from a live quote and an
    /// active, tenant-owned route. The token digest is the only secret
    /// material persisted by this method.
    pub async fn create_checkout_session(
        &self,
        command: CreateCheckoutSessionCommand,
    ) -> Result<CheckoutSession, StorageError> {
        if command.token_digest == [0; 32] {
            return Err(StorageError::CheckoutSessionIntegrity(
                "checkout token digest must not be zero".into(),
            ));
        }
        if command.expires_at_unix <= 0 {
            return Err(StorageError::CheckoutSessionIntegrity(
                "checkout expiry must be a positive Unix timestamp".into(),
            ));
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let quote_exists = transaction
            .query_opt(
                "SELECT 1
                 FROM tax_calculations
                 WHERE id = $1 AND merchant_id = $2
                   AND status IN ('calculated_unattested', 'attested')
                   AND expires_at > now()",
                &[&command.calculation_id, &command.merchant_id],
            )
            .await?
            .is_some();
        if !quote_exists {
            return Err(StorageError::QuoteNotFound);
        }
        let route_exists = transaction
            .query_opt(
                "SELECT 1
                 FROM settlement_routes
                 WHERE id = $1 AND merchant_id = $2 AND status = 'active'
                   AND effective_from <= now()
                   AND (effective_until IS NULL OR effective_until > now())",
                &[&command.route_id, &command.merchant_id],
            )
            .await?
            .is_some();
        if !route_exists {
            return Err(StorageError::SettlementRouteUnavailable);
        }
        let row = transaction
            .query_one(
                "INSERT INTO checkout_sessions
                    (merchant_id, calculation_id, route_id, token_digest, expires_at)
                 VALUES ($1, $2, $3, $4, to_timestamp($5::double precision))
                 RETURNING id, merchant_id, calculation_id, route_id, status,
                           floor(extract(epoch FROM expires_at))::bigint,
                           settlement_intent_id",
                &[
                    &command.merchant_id,
                    &command.calculation_id,
                    &command.route_id,
                    &&command.token_digest[..],
                    &command.expires_at_unix,
                ],
            )
            .await?;
        let session = parse_checkout_session_row(row)?;
        transaction.commit().await?;
        Ok(session)
    }

    /// Loads a guest-checkout session with its opaque token proof. Unknown
    /// session IDs and expired sessions are intentionally not enumerable.
    pub async fn load_checkout_session(
        &self,
        session_id: Uuid,
        token_digest: [u8; 32],
    ) -> Result<CheckoutSession, StorageError> {
        if token_digest == [0; 32] {
            return Err(StorageError::CheckoutSessionTokenInvalid);
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT id, merchant_id, calculation_id, route_id, token_digest,
                        status, floor(extract(epoch FROM expires_at))::bigint,
                        expires_at <= now(), settlement_intent_id
                 FROM checkout_sessions
                 WHERE id = $1",
                &[&session_id],
            )
            .await?;
        let Some(row) = row else {
            return Err(StorageError::CheckoutSessionNotFound);
        };
        let stored_digest: Vec<u8> = row.get(4);
        if stored_digest.as_slice() != token_digest.as_slice() {
            return Err(StorageError::CheckoutSessionTokenInvalid);
        }
        if row.get::<_, bool>(7) {
            return Err(StorageError::CheckoutSessionExpired);
        }
        if row.get::<_, String>(5) != "active" {
            return Err(StorageError::CheckoutSessionIntentConflict);
        }
        parse_checkout_session_row_from_public(row)
    }

    /// Creates or replays the one settlement intent allowed for a public
    /// checkout session. The session row is locked before intent creation so
    /// concurrent customer clicks cannot produce independent intents.
    pub async fn create_checkout_settlement_intent(
        &self,
        session_id: Uuid,
        merchant_id: Uuid,
        idempotency_key: &str,
        request_digest: [u8; 32],
        instruction_digest: [u8; 32],
    ) -> Result<SettlementIntentOutcome, StorageError> {
        if idempotency_key.trim().is_empty() || idempotency_key.len() > 200 {
            return Err(StorageError::InvalidConfiguration(
                "settlement idempotency key must contain 1 to 200 bytes".into(),
            ));
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let session = transaction
            .query_opt(
                "SELECT merchant_id, expires_at <= now(), settlement_intent_id
                 FROM checkout_sessions
                 WHERE id = $1
                 FOR UPDATE",
                &[&session_id],
            )
            .await?
            .ok_or(StorageError::CheckoutSessionNotFound)?;
        let session_merchant: Uuid = session.get(0);
        if session_merchant != merchant_id {
            return Err(StorageError::CheckoutSessionTokenInvalid);
        }
        if session.get::<_, bool>(1) {
            return Err(StorageError::CheckoutSessionExpired);
        }
        if let Some(intent_id) = session.get::<_, Option<Uuid>>(2) {
            let row = transaction
                .query_one(
                    "SELECT id, merchant_id, idempotency_key, request_digest,
                            instruction_digest, status, signed_transaction_digest,
                            signature, slot::text, commitment, last_error_code
                     FROM settlement_intents
                     WHERE id = $1 AND merchant_id = $2",
                    &[&intent_id, &merchant_id],
                )
                .await
                .map_err(|_| {
                    StorageError::CheckoutSessionIntegrity(
                        "session references a missing settlement intent".into(),
                    )
                })?;
            let intent = parse_settlement_intent_row(row)?;
            if intent.request_digest != request_digest
                || intent.instruction_digest != instruction_digest
            {
                return Err(StorageError::CheckoutSessionIntentConflict);
            }
            transaction.commit().await?;
            return Ok(SettlementIntentOutcome::Replay(intent));
        }
        let inserted = transaction
            .query_opt(
                "INSERT INTO settlement_intents
                    (merchant_id, checkout_session_id, idempotency_key,
                     request_digest, instruction_digest)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
                 RETURNING id, merchant_id, idempotency_key, request_digest,
                           instruction_digest, status, signed_transaction_digest,
                           signature, slot::text, commitment, last_error_code",
                &[
                    &merchant_id,
                    &session_id,
                    &idempotency_key,
                    &&request_digest[..],
                    &&instruction_digest[..],
                ],
            )
            .await?;
        let Some(row) = inserted else {
            let row = transaction
                .query_one(
                    "SELECT id, merchant_id, idempotency_key, request_digest,
                            instruction_digest, status, signed_transaction_digest,
                            signature, slot::text, commitment, last_error_code
                     FROM settlement_intents
                     WHERE merchant_id = $1 AND idempotency_key = $2
                     FOR UPDATE",
                    &[&merchant_id, &idempotency_key],
                )
                .await?;
            let intent = parse_settlement_intent_row(row)?;
            if intent.request_digest != request_digest
                || intent.instruction_digest != instruction_digest
            {
                return Err(StorageError::IdempotencyConflict);
            }
            transaction.commit().await?;
            return Ok(SettlementIntentOutcome::Replay(intent));
        };
        let intent = parse_settlement_intent_row(row)?;
        transaction
            .execute(
                "UPDATE checkout_sessions
                 SET settlement_intent_id = $2, updated_at = now()
                 WHERE id = $1 AND merchant_id = $3 AND settlement_intent_id IS NULL",
                &[&session_id, &intent.id, &merchant_id],
            )
            .await?;
        transaction.commit().await?;
        Ok(SettlementIntentOutcome::Created(intent))
    }

    /// Loads only a tenant-scoped document quote for document-attestation
    /// binding. This type is deliberately not accepted by settlement loaders.
    pub async fn load_document_quote_for_attestation(
        &self,
        merchant_id: Uuid,
        calculation_id: Uuid,
    ) -> Result<StoredDocumentQuote, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT status, document_schema, result_document,
                        floor(extract(epoch FROM expires_at))::bigint,
                        expires_at <= now(), applicability_context_id,
                        applicability_attestation_digest
                 FROM tax_document_calculations
                 WHERE id = $1 AND merchant_id = $2",
                &[&calculation_id, &merchant_id],
            )
            .await?;
        let Some(row) = row else {
            if client
                .query_opt(
                    "SELECT 1 FROM tax_calculations
                     WHERE id = $1 AND merchant_id = $2",
                    &[&calculation_id, &merchant_id],
                )
                .await?
                .is_some()
            {
                return Err(StorageError::AggregateCalculationScopeMismatch);
            }
            return Err(StorageError::DocumentQuoteNotFound);
        };
        let status: String = row.get(0);
        if status != CALCULATION_STATUS && status != ATTESTED_CALCULATION_STATUS {
            return Err(StorageError::QuoteNotSettleable);
        }
        if row.get::<_, bool>(4) {
            return Err(StorageError::QuoteExpired);
        }
        let result: TaxDocumentResult =
            serde_json::from_value(row.get(2)).map_err(|_| StorageError::InvalidStoredResponse)?;
        let applicability_context_id: Option<Uuid> = row.get(5);
        let applicability_attestation_digest = row
            .get::<_, Option<Vec<u8>>>(6)
            .map(parse_digest)
            .transpose()?;
        Ok(StoredDocumentQuote {
            calculation_id,
            status,
            expires_at_unix: row.get(3),
            schema: row.get(1),
            applicability_context_id,
            applicability_attestation_digest,
            result,
        })
    }

    /// Confirms that a calculation referenced by an approved adjustment has a
    /// durable non-simulation evidence row. Expiry is intentionally ignored:
    /// historical adjustment work may occur after a checkout quote expires,
    /// but it may never reference an unknown or simulation-only calculation.
    pub async fn ensure_calculation_evidence_exists(
        &self,
        merchant_id: Uuid,
        calculation_id: Uuid,
    ) -> Result<(), StorageError> {
        for attempt in 0..MAX_EVIDENCE_LOOKUP_ATTEMPTS {
            match self
                .ensure_calculation_evidence_exists_once(merchant_id, calculation_id)
                .await
            {
                Ok(()) => return Ok(()),
                Err(error)
                    if attempt + 1 < MAX_EVIDENCE_LOOKUP_ATTEMPTS
                        && is_retryable_evidence_lookup_error(&error) =>
                {
                    tracing::warn!(
                        attempt = attempt + 1,
                        max_attempts = MAX_EVIDENCE_LOOKUP_ATTEMPTS,
                        error = %error,
                        "retrying calculation evidence lookup after a transient database error"
                    );
                    tokio::time::sleep(EVIDENCE_LOOKUP_BACKOFF * (attempt as u32 + 1)).await;
                }
                Err(error) => {
                    tracing::error!(
                        merchant_id = %merchant_id,
                        calculation_id = %calculation_id,
                        error = %error,
                        "calculation evidence lookup failed"
                    );
                    return Err(error);
                }
            }
        }
        Err(StorageError::InvalidConfiguration(
            "calculation evidence lookup retry attempt budget must be positive".to_string(),
        ))
    }

    async fn ensure_calculation_evidence_exists_once(
        &self,
        merchant_id: Uuid,
        calculation_id: Uuid,
    ) -> Result<(), StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let exists = client
            .query_opt(
                "SELECT 1
                 FROM tax_calculations
                 WHERE id = $1 AND merchant_id = $2
                   AND status IN ('calculated_unattested', 'attested')",
                &[&calculation_id, &merchant_id],
            )
            .await?
            .is_some();
        if exists {
            Ok(())
        } else if client
            .query_opt(
                "SELECT 1 FROM tax_document_calculations
                 WHERE id = $1 AND merchant_id = $2",
                &[&calculation_id, &merchant_id],
            )
            .await?
            .is_some()
        {
            Err(StorageError::DocumentCalculationScopeMismatch)
        } else {
            Err(StorageError::QuoteNotFound)
        }
    }

    /// Persists one externally signed, server-bound calculation attestation.
    /// The evidence, attested calculation status, and first outbox event are
    /// committed atomically. Issuer private keys never enter this adapter.
    pub async fn record_calculation_attestation(
        &self,
        merchant_id: Uuid,
        calculation_id: Uuid,
        attestation: &CalculationAttestation,
    ) -> Result<StoredCalculationAttestation, StorageError> {
        attestation
            .verify()
            .map_err(|error| StorageError::InvalidAttestation(error.to_string()))?;
        let attestation_document = serde_json::to_value(attestation)?;
        let attestation_document_param = Json(attestation_document.clone());
        if serde_json::to_vec(&attestation_document)?.len() > 64 * 1024 {
            return Err(StorageError::AttestationEvidenceTooLarge);
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;

        let calculation = transaction
            .query_opt(
                "SELECT status, expires_at <= now()
                 FROM tax_calculations
                 WHERE id = $1 AND merchant_id = $2
                 FOR UPDATE",
                &[&calculation_id, &merchant_id],
            )
            .await?;
        let Some(calculation) = calculation else {
            if transaction
                .query_opt(
                    "SELECT 1 FROM tax_document_calculations
                     WHERE id = $1 AND merchant_id = $2
                     FOR SHARE",
                    &[&calculation_id, &merchant_id],
                )
                .await?
                .is_some()
            {
                return Err(StorageError::DocumentCalculationScopeMismatch);
            }
            return Err(StorageError::QuoteNotFound);
        };
        let calculation_status: String = calculation.get(0);
        if calculation_status != CALCULATION_STATUS
            && calculation_status != ATTESTED_CALCULATION_STATUS
        {
            return Err(StorageError::QuoteNotSettleable);
        }
        if calculation.get::<_, bool>(1) {
            return Err(StorageError::QuoteExpired);
        }

        let issued_at = attestation.unsigned.issued_at_unix;
        let expires_at = attestation.unsigned.expires_at_unix;
        let inserted = transaction
            .query_opt(
                "INSERT INTO tax_calculation_attestations
                    (merchant_id, calculation_id, payload_digest, issuer_public_key,
                     attestation_document, issued_at, expires_at)
                 VALUES ($1, $2, $3, $4, $5,
                         TIMESTAMPTZ 'epoch' + ($6::bigint * INTERVAL '1 second'),
                         TIMESTAMPTZ 'epoch' + ($7::bigint * INTERVAL '1 second'))
                 ON CONFLICT (merchant_id, calculation_id) DO NOTHING
                 RETURNING id",
                &[
                    &merchant_id,
                    &calculation_id,
                    &&attestation.payload_digest[..],
                    &&attestation.issuer_public_key[..],
                    &attestation_document_param,
                    &issued_at,
                    &expires_at,
                ],
            )
            .await?;

        if inserted.is_none() {
            let row = transaction
                .query_one(
                    "SELECT payload_digest, issuer_public_key,
                            floor(extract(epoch FROM expires_at))::bigint,
                            attestation_document
                     FROM tax_calculation_attestations
                     WHERE merchant_id = $1 AND calculation_id = $2
                     FOR UPDATE",
                    &[&merchant_id, &calculation_id],
                )
                .await?;
            let stored_digest: Vec<u8> = row.get(0);
            if stored_digest.as_slice() != attestation.payload_digest.as_slice() {
                return Err(StorageError::AttestationConflict);
            }
            let stored = parse_stored_calculation_attestation(
                calculation_id,
                stored_digest,
                row.get(1),
                row.get(2),
                row.get(3),
            )?;
            if stored.attestation != *attestation {
                return Err(StorageError::AttestationConflict);
            }
            transaction.commit().await?;
            return Ok(stored);
        }

        transaction
            .execute(
                "UPDATE tax_calculations
                 SET status = $3
                 WHERE id = $1 AND merchant_id = $2 AND status = $4",
                &[
                    &calculation_id,
                    &merchant_id,
                    &ATTESTED_CALCULATION_STATUS,
                    &CALCULATION_STATUS,
                ],
            )
            .await?;
        let event_payload = serde_json::json!({
            "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
            "merchant_id": merchant_id,
            "calculation_id": calculation_id,
            "attestation": attestation_document,
        });
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"tax_calculation",
                    &calculation_id,
                    &"tax.calculation.attested",
                    &event_payload,
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(StoredCalculationAttestation {
            calculation_id,
            payload_digest: attestation.payload_digest,
            issuer_public_key: attestation.issuer_public_key,
            expires_at_unix: attestation.unsigned.expires_at_unix,
            attestation: attestation.clone(),
        })
    }

    /// Persists one externally signed, document-scoped attestation. The
    /// document evidence, status transition, and first outbox event commit
    /// atomically; aggregate attestation rows are never touched.
    pub async fn record_document_calculation_attestation(
        &self,
        merchant_id: Uuid,
        calculation_id: Uuid,
        asset_mint: Digest32,
        attestation: &DocumentCalculationAttestation,
    ) -> Result<StoredDocumentCalculationAttestation, StorageError> {
        attestation
            .verify()
            .map_err(|error| StorageError::InvalidAttestation(error.to_string()))?;
        let attestation_document = serde_json::to_value(attestation)?;
        let attestation_document_param = Json(attestation_document.clone());
        if serde_json::to_vec(&attestation_document)?.len() > 64 * 1024 {
            return Err(StorageError::AttestationEvidenceTooLarge);
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;

        let calculation = transaction
            .query_opt(
                "SELECT status, expires_at <= now(), document_schema,
                        document_id, engine_version, facts_digest, rule_digest,
                        gross_atomic::text, total_tax_atomic::text,
                        total_net_atomic::text, result_document
                 FROM tax_document_calculations
                 WHERE id = $1 AND merchant_id = $2
                 FOR UPDATE",
                &[&calculation_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::DocumentQuoteNotFound)?;
        let calculation_status: String = calculation.get(0);
        if calculation_status != CALCULATION_STATUS
            && calculation_status != ATTESTED_CALCULATION_STATUS
        {
            return Err(StorageError::QuoteNotSettleable);
        }
        if calculation.get::<_, bool>(1) {
            return Err(StorageError::QuoteExpired);
        }
        let document_schema: String = calculation.get(2);
        let document_id: String = calculation.get(3);
        let engine_version: String = calculation.get(4);
        let facts_digest: Digest32 = calculation.get::<_, Vec<u8>>(5).try_into().map_err(|_| {
            StorageError::InvalidAttestation("stored document facts digest is not 32 bytes".into())
        })?;
        let rule_digest: Digest32 = calculation.get::<_, Vec<u8>>(6).try_into().map_err(|_| {
            StorageError::InvalidAttestation("stored document rule digest is not 32 bytes".into())
        })?;
        let gross_atomic = parse_attestation_amount(calculation.get(7))?;
        let tax_atomic = parse_attestation_amount(calculation.get(8))?;
        let net_atomic = parse_attestation_amount(calculation.get(9))?;
        let result: TaxDocumentResult = serde_json::from_value(calculation.get(10))
            .map_err(|error| StorageError::InvalidAttestation(error.to_string()))?;
        if result.document_id != document_id
            || result.engine_version != engine_version
            || result.facts_digest != facts_digest
            || result.rule_digest != rule_digest
            || result.gross_atomic != i128::from(gross_atomic)
            || result.total_tax_atomic != i128::from(tax_atomic)
            || result.total_net_atomic != i128::from(net_atomic)
        {
            return Err(StorageError::InvalidAttestation(
                "stored document result does not match its binding columns".into(),
            ));
        }
        let binding = DocumentAttestationBinding {
            document_schema: &document_schema,
            merchant_id: merchant_identity_digest(merchant_id.as_bytes()),
            document_id: &document_id,
            engine_version: &engine_version,
            rule_set_id: &result.rule_set_id,
            rule_version: result.rule_version,
            jurisdiction: &result.jurisdiction,
            currency: &result.currency,
            asset_mint,
            facts_digest,
            rule_digest,
            gross_atomic,
            tax_atomic,
            net_atomic,
            treatment_outcome_digest: policy::treatment_outcome_binding(
                attestation.unsigned.schema_version,
                result.treatment_outcome.treatment,
                result.treatment_outcome.arithmetic_tax_atomic,
                result.treatment_outcome.collected_tax_atomic,
                result.treatment_outcome.self_assessed_tax_atomic,
                result.treatment_outcome.withheld_tax_atomic,
                result.treatment_outcome.taxable_base_atomic,
            )
            .map_err(|error| StorageError::InvalidAttestation(error.to_string()))?,
        };
        attestation
            .verify_binding(&binding)
            .map_err(|error| StorageError::InvalidAttestation(error.to_string()))?;

        let issued_at = attestation.unsigned.issued_at_unix;
        let expires_at = attestation.unsigned.expires_at_unix;
        let inserted = transaction
            .query_opt(
                "INSERT INTO tax_document_calculation_attestations
                    (merchant_id, calculation_id, payload_digest, issuer_public_key,
                     attestation_document, issued_at, expires_at)
                 VALUES ($1, $2, $3, $4, $5,
                         TIMESTAMPTZ 'epoch' + ($6::bigint * INTERVAL '1 second'),
                         TIMESTAMPTZ 'epoch' + ($7::bigint * INTERVAL '1 second'))
                 ON CONFLICT (merchant_id, calculation_id) DO NOTHING
                 RETURNING id",
                &[
                    &merchant_id,
                    &calculation_id,
                    &&attestation.payload_digest[..],
                    &&attestation.issuer_public_key[..],
                    &attestation_document_param,
                    &issued_at,
                    &expires_at,
                ],
            )
            .await?;

        if inserted.is_none() {
            let row = transaction
                .query_one(
                    "SELECT payload_digest, issuer_public_key,
                            floor(extract(epoch FROM expires_at))::bigint,
                            attestation_document
                     FROM tax_document_calculation_attestations
                     WHERE merchant_id = $1 AND calculation_id = $2
                     FOR UPDATE",
                    &[&merchant_id, &calculation_id],
                )
                .await?;
            let stored = parse_stored_document_calculation_attestation(
                calculation_id,
                row.get(0),
                row.get(1),
                row.get(2),
                row.get(3),
            )?;
            if stored.attestation != *attestation {
                return Err(StorageError::AttestationConflict);
            }
            transaction.commit().await?;
            return Ok(stored);
        }

        transaction
            .execute(
                "UPDATE tax_document_calculations
                 SET status = $3
                 WHERE id = $1 AND merchant_id = $2 AND status = $4",
                &[
                    &calculation_id,
                    &merchant_id,
                    &ATTESTED_CALCULATION_STATUS,
                    &CALCULATION_STATUS,
                ],
            )
            .await?;
        let event_payload = serde_json::json!({
            "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
            "merchant_id": merchant_id,
            "calculation_id": calculation_id,
            "attestation": attestation_document,
        });
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"tax_document_calculation",
                    &calculation_id,
                    &"tax.document_calculation.attested",
                    &event_payload,
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(StoredDocumentCalculationAttestation {
            calculation_id,
            payload_digest: attestation.payload_digest,
            issuer_public_key: attestation.issuer_public_key,
            expires_at_unix: attestation.unsigned.expires_at_unix,
            attestation: attestation.clone(),
        })
    }

    pub async fn load_document_calculation_attestation(
        &self,
        merchant_id: Uuid,
        calculation_id: Uuid,
    ) -> Result<StoredDocumentCalculationAttestation, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT payload_digest, issuer_public_key,
                        floor(extract(epoch FROM expires_at))::bigint,
                        attestation_document
                 FROM tax_document_calculation_attestations
                 WHERE merchant_id = $1 AND calculation_id = $2",
                &[&merchant_id, &calculation_id],
            )
            .await?
            .ok_or(StorageError::AttestationNotFound)?;
        parse_stored_document_calculation_attestation(
            calculation_id,
            row.get(0),
            row.get(1),
            row.get(2),
            row.get(3),
        )
    }

    pub async fn load_calculation_attestation(
        &self,
        merchant_id: Uuid,
        calculation_id: Uuid,
    ) -> Result<StoredCalculationAttestation, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT payload_digest, issuer_public_key,
                        floor(extract(epoch FROM expires_at))::bigint,
                        attestation_document
                 FROM tax_calculation_attestations
                 WHERE merchant_id = $1 AND calculation_id = $2",
                &[&merchant_id, &calculation_id],
            )
            .await?
            .ok_or(StorageError::AttestationNotFound)?;
        parse_stored_calculation_attestation(
            calculation_id,
            row.get(0),
            row.get(1),
            row.get(2),
            row.get(3),
        )
    }

    /// Replays a persisted quote from database-owned normalized facts and the
    /// immutable published rule release. This endpoint is evidence-only: it
    /// does not authorize settlement or return client-supplied inputs.
    pub async fn verify_quote_replay(
        &self,
        merchant_id: Uuid,
        calculation_id: Uuid,
    ) -> Result<QuoteReplayVerification, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT tc.status, tc.engine_version, tc.facts_digest,
                        tc.result_document, tc.facts_document,
                        tc.evidence_refs, tc.facts_ciphertext, tc.facts_nonce,
                        tc.facts_key_id,
                        rs.rule_document, rs.rule_digest
                 FROM tax_calculations AS tc
                 JOIN rule_sets AS rs ON rs.id = tc.rule_set_id
                    AND rs.merchant_id = tc.merchant_id
                 WHERE tc.id = $1 AND tc.merchant_id = $2",
                &[&calculation_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::QuoteNotFound)?;

        let status: String = row.get(0);
        let engine_version: String = row.get(1);
        let facts_digest_bytes: Vec<u8> = row.get(2);
        let facts_digest: Digest32 = facts_digest_bytes.clone().try_into().map_err(|_| {
            StorageError::ReplayEvidenceIntegrity("facts digest is not 32 bytes".into())
        })?;
        let result_document: Value = row.get(3);
        let plaintext_facts: Option<Value> = row.get(4);
        let evidence_refs: Value = row.get(5);
        let ciphertext: Option<Vec<u8>> = row.get(6);
        let nonce: Option<Vec<u8>> = row.get(7);
        let key_id: Option<String> = row.get(8);
        let facts_document = match (plaintext_facts, ciphertext, nonce, key_id) {
            (Some(_), Some(_), _, _) => {
                return Err(StorageError::ReplayEvidenceIntegrity(
                    "replay facts cannot contain plaintext and ciphertext together".into(),
                ));
            }
            (Some(plaintext), None, None, None) => Some(plaintext),
            (None, Some(ciphertext), Some(nonce), Some(key_id)) => Some(
                self.evidence_cipher
                    .as_ref()
                    .ok_or(StorageError::EvidenceCipherUnavailable)?
                    .decrypt_facts(merchant_id, &facts_digest, &key_id, &nonce, &ciphertext)?,
            ),
            (None, None, None, None) => None,
            _ => {
                return Err(StorageError::EvidenceIntegrity(
                    "encrypted replay facts have incomplete columns".into(),
                ));
            }
        };
        let rule_document: Value = row.get(9);
        let rule_digest: Vec<u8> = row.get(10);
        verify_persisted_calculation_evidence(
            merchant_id,
            calculation_id,
            status,
            engine_version,
            facts_digest_bytes,
            result_document,
            facts_document,
            evidence_refs,
            rule_document,
            rule_digest,
            &self.engine_version,
        )
    }

    /// Appends an operational audit record for access to normalized tax
    /// evidence. The audit row contains identity, purpose, outcome, and the
    /// facts digest only; decrypted facts never cross this boundary.
    pub async fn record_evidence_access_audit(
        &self,
        command: EvidenceAccessAuditCommand,
    ) -> Result<(), StorageError> {
        if command.actor_subject.is_empty() || command.actor_subject.len() > 256 {
            return Err(StorageError::InvalidConfiguration(
                "evidence audit actor subject must contain 1 to 256 bytes".into(),
            ));
        }
        if command.purpose.is_empty() || command.purpose.len() > 128 {
            return Err(StorageError::InvalidConfiguration(
                "evidence audit purpose must contain 1 to 128 bytes".into(),
            ));
        }
        if command.outcome.is_empty() || command.outcome.len() > 64 {
            return Err(StorageError::InvalidConfiguration(
                "evidence audit outcome must contain 1 to 64 bytes".into(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        client
            .execute(
                "INSERT INTO evidence_access_audit_events
                    (merchant_id, calculation_id, actor_subject, request_id,
                     purpose, outcome, facts_digest)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[
                    &command.merchant_id,
                    &command.calculation_id,
                    &command.actor_subject,
                    &command.request_id,
                    &command.purpose,
                    &command.outcome,
                    &command.facts_digest.as_ref().map(|digest| &digest[..]),
                ],
            )
            .await?;
        Ok(())
    }

    /// Lists a bounded, tenant-scoped projection of sensitive evidence access.
    /// This is an audit read model only; it never returns decrypted facts or
    /// authorizes replay or settlement.
    pub async fn list_evidence_access_audit_events(
        &self,
        merchant_id: Uuid,
        calculation_id: Option<Uuid>,
        limit: usize,
    ) -> Result<Vec<EvidenceAccessAuditEventView>, StorageError> {
        validate_evidence_access_audit_limit(limit)?;
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let limit = i64::try_from(limit).map_err(|_| {
            StorageError::InvalidConfiguration(
                "evidence access audit list limit is too large".into(),
            )
        })?;
        let rows = match calculation_id {
            Some(calculation_id) => {
                client
                    .query(
                        "SELECT id, calculation_id, actor_subject, request_id,
                                purpose, outcome, facts_digest,
                                to_char(occurred_at AT TIME ZONE 'UTC',
                                        'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')
                         FROM evidence_access_audit_events
                         WHERE merchant_id = $1 AND calculation_id = $2
                         ORDER BY occurred_at DESC, id DESC
                         LIMIT $3",
                        &[&merchant_id, &calculation_id, &limit],
                    )
                    .await?
            }
            None => {
                client
                    .query(
                        "SELECT id, calculation_id, actor_subject, request_id,
                                purpose, outcome, facts_digest,
                                to_char(occurred_at AT TIME ZONE 'UTC',
                                        'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')
                         FROM evidence_access_audit_events
                         WHERE merchant_id = $1
                         ORDER BY occurred_at DESC, id DESC
                         LIMIT $2",
                        &[&merchant_id, &limit],
                    )
                    .await?
            }
        };
        rows.into_iter()
            .map(|row| {
                Ok(EvidenceAccessAuditEventView {
                    id: row.get(0),
                    calculation_id: row.get(1),
                    actor_subject: row.get(2),
                    request_id: row.get(3),
                    purpose: row.get(4),
                    outcome: row.get(5),
                    facts_digest: row
                        .get::<_, Option<Vec<u8>>>(6)
                        .map(parse_digest)
                        .transpose()?,
                    occurred_at: row.get(7),
                })
            })
            .collect()
    }

    /// Creates one immutable, tenant-scoped refund funding topology. The row
    /// is configuration evidence only: it is not active until a separate
    /// finalized Solana topology verifier records verification evidence.
    pub async fn create_refund_funding_configuration(
        &self,
        command: CreateRefundFundingConfigurationCommand,
    ) -> Result<RefundFundingConfigurationOutcome, StorageError> {
        validate_refund_funding_configuration_command(&command)?;
        let configuration_digest = refund_funding_configuration_digest(&command);
        let request_digest =
            refund_funding_configuration_request_digest(&command, configuration_digest);
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;

        let id = Uuid::now_v7();
        let from_unix = command.effective_from_unix.to_string();
        let until_unix = command.effective_until_unix.map(|value| value.to_string());
        let inserted = transaction
            .query_opt(
                "INSERT INTO refund_funding_configurations
                    (id, merchant_id, idempotency_key, request_digest,
                     configuration_digest, configuration_key, version, status,
                     merchant_account, mint, tax_vault, vault_authority,
                     refund_funder, refund_funder_subject, net_source, token_program, proposer_subject,
                     effective_from, effective_until)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, 'proposed', $8, $9,
                         $10, $11, $12, $13, $14, $15, $16,
                         to_timestamp($17::double precision),
                         CASE WHEN $18::text IS NULL THEN NULL
                              ELSE to_timestamp($18::double precision) END)
                 ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
                 RETURNING id, merchant_id, idempotency_key, request_digest,
                           configuration_digest, configuration_key, version,
                           status, merchant_account, mint, tax_vault,
                           vault_authority, refund_funder, refund_funder_subject, net_source,
                           token_program, proposer_subject, approver_subject,
                           verification_digest, verification_slot::text,
                           verification_commitment, verification_evidence,
                           verifier_version,
                           to_char(verified_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                           to_char(effective_from AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                           to_char(effective_until AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')",
                &[
                    &id,
                    &command.merchant_id,
                    &command.idempotency_key,
                    &&request_digest[..],
                    &&configuration_digest[..],
                    &command.configuration_key,
                    &command.version,
                    &command.merchant_account,
                    &command.mint,
                    &command.tax_vault,
                    &command.vault_authority,
                    &command.refund_funder,
                    &command.refund_funder_subject,
                    &command.net_source,
                    &command.token_program,
                    &command.actor_subject,
                    &from_unix,
                    &until_unix,
                ],
            )
            .await?;
        if let Some(row) = inserted {
            let configuration = parse_refund_funding_configuration_row(row)?;
            transaction
                .execute(
                    "INSERT INTO refund_funding_configuration_events
                        (configuration_id, merchant_id, actor_subject, action,
                         from_status, to_status, evidence)
                     VALUES ($1, $2, $3, 'propose', 'none', 'proposed', $4)",
                    &[
                        &configuration.id,
                        &command.merchant_id,
                        &command.actor_subject,
                        &serde_json::json!({
                            "request_digest": request_digest,
                            "configuration_digest": configuration_digest,
                        }),
                    ],
                )
                .await?;
            transaction
                .execute(
                    "INSERT INTO outbox_events
                        (aggregate_type, aggregate_id, event_type, payload)
                     VALUES ($1, $2, $3, $4)",
                    &[
                        &"refund_funding_configuration",
                        &configuration.id,
                        &"tax.refund_funding_configuration.proposed",
                        &serde_json::json!({
                            "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                            "merchant_id": command.merchant_id,
                            "configuration": configuration,
                        }),
                    ],
                )
                .await?;
            transaction.commit().await?;
            return Ok(RefundFundingConfigurationOutcome::Created(configuration));
        }

        let row = transaction
            .query_one(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        configuration_digest, configuration_key, version,
                        status, merchant_account, mint, tax_vault,
                        vault_authority, refund_funder, refund_funder_subject, net_source,
                        token_program, proposer_subject, approver_subject,
                        verification_digest, verification_slot::text,
                        verification_commitment, verification_evidence,
                        verifier_version,
                        to_char(verified_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                        to_char(effective_from AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                        to_char(effective_until AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')
                 FROM refund_funding_configurations
                 WHERE merchant_id = $1 AND idempotency_key = $2
                 FOR UPDATE",
                &[&command.merchant_id, &command.idempotency_key],
            )
            .await?;
        let configuration = parse_refund_funding_configuration_row(row)?;
        if configuration.request_digest != request_digest {
            return Err(StorageError::RefundFundingConfigurationIdempotencyConflict);
        }
        transaction.commit().await?;
        Ok(RefundFundingConfigurationOutcome::Replay(configuration))
    }

    pub async fn load_refund_funding_configuration(
        &self,
        configuration_id: Uuid,
        merchant_id: Uuid,
    ) -> Result<RefundFundingConfiguration, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        configuration_digest, configuration_key, version,
                        status, merchant_account, mint, tax_vault,
                        vault_authority, refund_funder, refund_funder_subject, net_source,
                        token_program, proposer_subject, approver_subject,
                        verification_digest, verification_slot::text,
                        verification_commitment, verification_evidence,
                        verifier_version,
                        to_char(verified_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                        to_char(effective_from AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                        to_char(effective_until AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')
                 FROM refund_funding_configurations
                 WHERE id = $1 AND merchant_id = $2",
                &[&configuration_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::RefundFundingConfigurationNotFound)?;
        parse_refund_funding_configuration_row(row)
    }

    /// Resolves exactly one currently effective, finalized-verified funding
    /// configuration for a merchant/mint/token-program tuple. Multiple
    /// active rows are an authority conflict, not a reason to pick by order.
    pub async fn load_active_refund_funding_configuration(
        &self,
        merchant_id: Uuid,
        mint: &str,
        token_program: &str,
    ) -> Result<RefundFundingConfiguration, StorageError> {
        if merchant_id.get_version_num() != 7
            || mint.trim().is_empty()
            || token_program.trim().is_empty()
        {
            return Err(StorageError::RefundFundingConfigurationNotFound);
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        configuration_digest, configuration_key, version,
                        status, merchant_account, mint, tax_vault,
                        vault_authority, refund_funder, refund_funder_subject, net_source,
                        token_program, proposer_subject, approver_subject,
                        verification_digest, verification_slot::text,
                        verification_commitment, verification_evidence,
                        verifier_version,
                        to_char(verified_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                        to_char(effective_from AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                        to_char(effective_until AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')
                 FROM refund_funding_configurations
                 WHERE merchant_id = $1 AND mint = $2 AND token_program = $3
                   AND status = 'active'
                   AND effective_from <= now()
                   AND (effective_until IS NULL OR effective_until > now())
                 ORDER BY version DESC, id
                 LIMIT 2",
                &[&merchant_id, &mint, &token_program],
            )
            .await?;
        match rows.len() {
            0 => Err(StorageError::RefundFundingConfigurationNotFound),
            1 => parse_refund_funding_configuration_row(rows.into_iter().next().ok_or_else(
                || {
                    StorageError::InvalidRefundFundingConfiguration(
                        "active funding row cardinality changed unexpectedly".into(),
                    )
                },
            )?),
            _ => Err(StorageError::RefundFundingConfigurationVerificationInvalid(
                "multiple effective active funding configurations match the mint and token program"
                    .into(),
            )),
        }
    }

    /// Requires a finalized adjustment-record event that exactly matches the
    /// approved adjustment and its immutable evidence intent. This returns
    /// the on-chain adjustment PDA needed by the separate refund-claim flow.
    pub async fn load_finalized_settlement_adjustment_binding(
        &self,
        merchant_id: Uuid,
        adjustment_id: Uuid,
    ) -> Result<FinalizedSettlementAdjustmentBinding, StorageError> {
        if merchant_id.get_version_num() != 7 || adjustment_id.get_version_num() != 7 {
            return Err(StorageError::InvalidSettlementAdjustment(
                "merchant and adjustment identities must be UUIDv7".into(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT DISTINCT indexed.adjustment, indexed.original_receipt,
                        indexed.nonce::text, indexed.signature, indexed.slot::text,
                        indexed.adjustment_digest, indexed.adjustment_evidence_digest
                 FROM settlement_adjustment_chain_intents chain
                 JOIN settlement_adjustment_evidence_intents evidence
                   ON evidence.id = chain.evidence_intent_id
                  AND evidence.merchant_id = chain.merchant_id
                 JOIN settlement_adjustments adjustment
                   ON adjustment.id = chain.adjustment_id
                  AND adjustment.merchant_id = chain.merchant_id
                 JOIN indexed_settlement_adjustments indexed
                   ON indexed.original_receipt = evidence.original_receipt
                  AND indexed.merchant = evidence.merchant
                  AND indexed.mint = evidence.mint
                  AND indexed.approver = evidence.approver
                  AND indexed.adjustment_digest = adjustment.adjustment_digest
                  AND indexed.adjustment_evidence_digest = adjustment.evidence_digest
                 JOIN raw_chain_evidence raw
                   ON raw.signature = indexed.signature
                  AND raw.event_index = indexed.event_index
                 WHERE chain.adjustment_id = $1
                   AND chain.merchant_id = $2
                   AND chain.status = 'finalized'
                   AND chain.commitment = 'finalized'
                   AND chain.signature IS NOT NULL
                   AND raw.event_discriminator = $3
                 LIMIT 2",
                &[
                    &adjustment_id,
                    &merchant_id,
                    &&indexer::SETTLEMENT_ADJUSTMENT_RECORDED_EVENT_DISCRIMINATOR[..],
                ],
            )
            .await?;
        match rows.len() {
            0 => Err(StorageError::SettlementAdjustmentNotFound),
            1 => parse_finalized_settlement_adjustment_binding(
                rows.into_iter().next().ok_or_else(|| {
                    StorageError::InvalidSettlementAdjustment(
                        "finalized adjustment row cardinality changed unexpectedly".into(),
                    )
                })?,
            ),
            _ => Err(StorageError::InvalidSettlementAdjustment(
                "multiple conflicting finalized adjustment records matched".into(),
            )),
        }
    }

    pub async fn load_refund_claim_intent_by_idempotency(
        &self,
        merchant_id: Uuid,
        idempotency_key: &str,
    ) -> Result<Option<RefundClaimIntent>, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        adjustment_id, funding_configuration_id,
                        receipt_signature, receipt_slot::text, mint, token_program,
                        gross_atomic::text, tax_atomic::text, net_atomic::text,
                        claim_nonce::text, claim, instruction_digest,
                        artifact_digest, status, signed_transaction_digest,
                        signature, submitted_slot::text, commitment, last_error_code,
                        unsigned_artifact
                 FROM refund_claim_intents
                 WHERE merchant_id = $1 AND idempotency_key = $2",
                &[&merchant_id, &idempotency_key],
            )
            .await?;
        row.map(parse_refund_claim_intent_row).transpose()
    }

    pub async fn load_refund_claim_intent(
        &self,
        intent_id: Uuid,
        merchant_id: Uuid,
    ) -> Result<Option<RefundClaimIntent>, StorageError> {
        if intent_id.get_version_num() != 7 || merchant_id.get_version_num() != 7 {
            return Err(StorageError::RefundClaimIntentNotFound);
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        adjustment_id, funding_configuration_id,
                        receipt_signature, receipt_slot::text, mint, token_program,
                        gross_atomic::text, tax_atomic::text, net_atomic::text,
                        claim_nonce::text, claim, instruction_digest,
                        artifact_digest, status, signed_transaction_digest,
                        signature, submitted_slot::text, commitment, last_error_code,
                        unsigned_artifact
                 FROM refund_claim_intents
                 WHERE id = $1 AND merchant_id = $2",
                &[&intent_id, &merchant_id],
            )
            .await?;
        row.map(parse_refund_claim_intent_row).transpose()
    }

    /// Claims one oldest submitted claim whose finalized evidence still needs
    /// reconciliation. The short lease is committed before external RPC
    /// work, so replicas do not hold a database lock across the network. A
    /// stale lease is reclaimable after the bounded crash window.
    pub async fn load_pending_refund_claim_reconciliation(
        &self,
    ) -> Result<Option<RefundClaimReconciliationCandidate>, StorageError> {
        const LEASE_SECONDS: i64 = 60;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        adjustment_id, funding_configuration_id,
                        receipt_signature, receipt_slot::text, mint, token_program,
                        gross_atomic::text, tax_atomic::text, net_atomic::text,
                        claim_nonce::text, claim, instruction_digest,
                        artifact_digest, status, signed_transaction_digest,
                        signature, submitted_slot::text, commitment, last_error_code,
                        unsigned_artifact
                 FROM refund_claim_intents
                 WHERE signature IS NOT NULL
                   AND status IN ('submitting', 'submitted', 'processed', 'confirmed', 'unknown')
                   AND (reconciliation_claim_token IS NULL
                        OR reconciliation_claimed_at < now() - ($1::bigint * interval '1 second'))
                 ORDER BY updated_at, id
                 FOR UPDATE SKIP LOCKED
                 LIMIT 1",
                &[&LEASE_SECONDS],
            )
            .await?;
        let Some(row) = row else {
            transaction.commit().await?;
            return Ok(None);
        };
        let intent = parse_refund_claim_intent_row(row)?;
        let artifact = serde_json::from_value::<RefundArtifact>(intent.unsigned_artifact.clone())
            .map_err(|_| {
            StorageError::InvalidRefundClaimIntent(
                "persisted refund claim artifact is not valid".into(),
            )
        })?;
        if artifact.merchant_id != intent.merchant_id
            || artifact.adjustment_id != intent.adjustment_id
            || artifact.funding_configuration_id != intent.funding_configuration_id
            || artifact.instruction_digest != intent.instruction_digest
            || artifact.artifact_digest != intent.artifact_digest
            || artifact.instruction.claim != Pubkey::new_from_array(intent.claim).to_string()
            || artifact.instruction.claim_nonce != intent.claim_nonce
        {
            return Err(StorageError::InvalidRefundClaimIntent(
                "persisted refund claim artifact is not bound to its intent".into(),
            ));
        }
        let claim_token = Uuid::now_v7();
        let updated = transaction
            .execute(
                "UPDATE refund_claim_intents
                 SET reconciliation_claim_token = $2,
                     reconciliation_claimed_at = now()
                 WHERE id = $1",
                &[&intent.id, &claim_token],
            )
            .await?;
        if updated != 1 {
            transaction.rollback().await?;
            return Ok(None);
        }
        transaction.commit().await?;
        Ok(Some(RefundClaimReconciliationCandidate {
            intent,
            artifact,
            claim_token,
        }))
    }

    /// Loads and validates the immutable claim-created event retained during
    /// finalized targeted recovery. This is the signature-to-claim binding
    /// used by claim reconciliation; a finalized account snapshot alone is
    /// insufficient because it does not identify which transaction created it.
    pub async fn load_finalized_refund_claim_evidence(
        &self,
        stream_key: &str,
        signature: &str,
        expected: &RefundClaimEvidenceExpectation,
    ) -> Result<FinalizedRefundClaimEvidence, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT signature, slot::text, event_index::text,
                        event_discriminator, event_data, evidence_digest
                 FROM raw_chain_evidence
                 WHERE stream_key = $1 AND signature = $2
                   AND event_discriminator = $3
                 ORDER BY event_index
                 LIMIT 2",
                &[
                    &stream_key,
                    &signature,
                    &&indexer::ADJUSTMENT_REFUND_CLAIM_CREATED_EVENT_DISCRIMINATOR[..],
                ],
            )
            .await?;
        let mut matches = Vec::new();
        for row in rows {
            let evidence = parse_raw_evidence_row(row)?;
            let decoded = decode_refund_claim_created_event(
                &evidence.signature,
                evidence.slot,
                evidence.event_index,
                &evidence.event_data,
                evidence.evidence_digest,
            )
            .map_err(|error| StorageError::InvalidIndexerState(error.to_string()))?;
            if decoded.claim == expected.claim
                && decoded.adjustment == expected.adjustment
                && decoded.original_receipt == expected.original_receipt
                && decoded.merchant == expected.merchant
                && decoded.mint == expected.mint
                && decoded.nonce == expected.nonce
                && decoded.gross_magnitude == expected.gross_magnitude
                && decoded.tax_magnitude == expected.tax_magnitude
                && decoded.net_magnitude == expected.net_magnitude
                && decoded.refund_funder == expected.refund_funder
                && decoded.refund_destination == expected.refund_destination
                && decoded.proposer == expected.proposer
                && decoded.reference_digest == expected.reference_digest
                && decoded.execute_after_unix == expected.execute_after_unix
                && decoded.expires_at_unix == expected.expires_at_unix
            {
                matches.push(decoded);
            }
        }
        match matches.len() {
            1 => matches.pop().ok_or_else(|| {
                StorageError::InvalidIndexerState(
                    "refund claim evidence match disappeared unexpectedly".into(),
                )
            }),
            0 => Err(StorageError::InvalidIndexerState(
                "finalized refund claim event did not match the durable intent".into(),
            )),
            _ => Err(StorageError::InvalidIndexerState(
                "multiple finalized refund claim events matched the durable intent".into(),
            )),
        }
    }

    /// Loads the finalized cancellation event retained by the indexer. A
    /// cancellation projection is accepted only when every immutable identity
    /// field matches the server-owned cancellation intent.
    pub async fn load_finalized_refund_claim_cancellation_evidence(
        &self,
        stream_key: &str,
        signature: &str,
        expected: &RefundClaimCancellationEvidenceExpectation,
    ) -> Result<FinalizedRefundClaimCancellationEvidence, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT signature, slot::text, event_index::text,
                        event_discriminator, event_data, evidence_digest
                 FROM raw_chain_evidence
                 WHERE stream_key = $1 AND signature = $2
                   AND event_discriminator = $3
                 ORDER BY event_index
                 LIMIT 2",
                &[
                    &stream_key,
                    &signature,
                    &&indexer::ADJUSTMENT_REFUND_CLAIM_CANCELLED_EVENT_DISCRIMINATOR[..],
                ],
            )
            .await?;
        let mut matches = Vec::new();
        for row in rows {
            let evidence = parse_raw_evidence_row(row)?;
            let decoded = decode_refund_claim_cancelled_event(
                &evidence.signature,
                evidence.slot,
                evidence.event_index,
                &evidence.event_data,
                evidence.evidence_digest,
            )
            .map_err(|error| StorageError::InvalidIndexerState(error.to_string()))?;
            if decoded.claim == expected.claim
                && decoded.adjustment == expected.adjustment
                && decoded.merchant == expected.merchant
                && decoded.mint == expected.mint
                && decoded.nonce == expected.nonce
                && decoded.governance_authority == expected.governance_authority
                && decoded.reference_digest == expected.reference_digest
            {
                matches.push(decoded);
            }
        }
        match matches.len() {
            1 => matches.pop().ok_or_else(|| {
                StorageError::InvalidIndexerState(
                    "refund claim cancellation evidence match disappeared unexpectedly".into(),
                )
            }),
            0 => Err(StorageError::InvalidIndexerState(
                "finalized refund claim cancellation event did not match the durable intent".into(),
            )),
            _ => Err(StorageError::InvalidIndexerState(
                "multiple finalized refund claim cancellation events matched the durable intent"
                    .into(),
            )),
        }
    }

    /// Applies one compare-and-set refund claim lifecycle transition and
    /// records its event and outbox publication atomically. The transport
    /// fields are never accepted from a browser without the exact signed-byte
    /// verifier running before this method.
    pub async fn transition_refund_claim_intent(
        &self,
        command: RefundClaimIntentTransition,
    ) -> Result<bool, StorageError> {
        let expected = parse_settlement_status(&command.expected_status)?;
        let next = parse_settlement_status(&command.next_status)?;
        if !can_transition(expected, next) || command.evidence.to_string().len() > 64 * 1024 {
            return Err(StorageError::InvalidRefundClaimIntent(
                "refund claim lifecycle transition is invalid".into(),
            ));
        }
        if command
            .signature
            .as_deref()
            .is_some_and(|value| value.is_empty() || value.len() > 128)
            || command
                .last_error_code
                .as_deref()
                .is_some_and(|value| value.trim().is_empty() || value.len() > 128)
        {
            return Err(StorageError::InvalidRefundClaimIntent(
                "refund claim transport evidence is invalid".into(),
            ));
        }
        if matches!(next, SettlementStatus::Unknown) && command.last_error_code.is_none() {
            return Err(StorageError::InvalidRefundClaimIntent(
                "unknown refund claim status requires an error code".into(),
            ));
        }
        let commitment = match next {
            SettlementStatus::Processed => Some("processed"),
            SettlementStatus::Confirmed => Some("confirmed"),
            SettlementStatus::Finalized => Some("finalized"),
            _ => None,
        };
        let slot_text = command.slot.map(|value| value.to_string());
        let signed_digest = command
            .signed_transaction_digest
            .map(|value| value.to_vec());
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let updated = transaction
            .execute(
                "UPDATE refund_claim_intents
                 SET status = $3,
                     signed_transaction_digest = COALESCE($4, signed_transaction_digest),
                     signature = COALESCE($5, signature),
                     submitted_slot = COALESCE($6::text::numeric, submitted_slot),
                     commitment = COALESCE($7, commitment),
                     last_error_code = $8,
                     reconciliation_claim_token = CASE
                         WHEN $10::uuid IS NOT NULL AND $3 <> 'finalized'
                             THEN reconciliation_claim_token
                         ELSE NULL
                     END,
                     reconciliation_claimed_at = CASE
                         WHEN $10::uuid IS NOT NULL AND $3 <> 'finalized'
                             THEN reconciliation_claimed_at
                         ELSE NULL
                     END,
                     updated_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND status = $9
                   AND ($10::uuid IS NULL OR reconciliation_claim_token = $10)",
                &[
                    &command.intent_id,
                    &command.merchant_id,
                    &command.next_status,
                    &signed_digest,
                    &command.signature,
                    &slot_text,
                    &commitment,
                    &command.last_error_code,
                    &command.expected_status,
                    &command.reconciliation_claim_token,
                ],
            )
            .await?;
        if updated != 1 {
            transaction.rollback().await?;
            return Ok(false);
        }
        transaction
            .execute(
                "INSERT INTO refund_claim_intent_events
                    (intent_id, merchant_id, from_status, to_status, signature, slot, evidence)
                 VALUES ($1, $2, $3, $4, $5, $6::text::numeric, $7)",
                &[
                    &command.intent_id,
                    &command.merchant_id,
                    &command.expected_status,
                    &command.next_status,
                    &command.signature,
                    &slot_text,
                    &command.evidence,
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"refund_claim_intent",
                    &command.intent_id,
                    &"tax.refund_claim_intent.status_changed",
                    &serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": command.merchant_id,
                        "intent_id": command.intent_id,
                        "from_status": command.expected_status,
                        "to_status": command.next_status,
                        "signature": command.signature,
                        "slot": slot_text,
                    }),
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn list_refund_funding_configurations(
        &self,
        merchant_id: Uuid,
    ) -> Result<Vec<RefundFundingConfiguration>, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        configuration_digest, configuration_key, version,
                        status, merchant_account, mint, tax_vault,
                        vault_authority, refund_funder, refund_funder_subject, net_source,
                        token_program, proposer_subject, approver_subject,
                        verification_digest, verification_slot::text,
                        verification_commitment, verification_evidence,
                        verifier_version,
                        to_char(verified_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                        to_char(effective_from AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                        to_char(effective_until AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')
                 FROM refund_funding_configurations
                 WHERE merchant_id = $1
                 ORDER BY configuration_key, version DESC, id
                 LIMIT 64",
                &[&merchant_id],
            )
            .await?;
        rows.into_iter()
            .map(parse_refund_funding_configuration_row)
            .collect()
    }

    pub async fn transition_refund_funding_configuration(
        &self,
        command: RefundFundingConfigurationTransition,
    ) -> Result<RefundFundingConfigurationTransitionResult, StorageError> {
        validate_refund_funding_configuration_transition(&command)?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT status, proposer_subject, approver_subject
                 FROM refund_funding_configurations
                 WHERE id = $1 AND merchant_id = $2
                 FOR UPDATE",
                &[&command.configuration_id, &command.merchant_id],
            )
            .await?
            .ok_or(StorageError::RefundFundingConfigurationNotFound)?;
        let from_status: String = row.get(0);
        let proposer_subject: String = row.get(1);
        let current_approver: Option<String> = row.get(2);
        let approver_subject = match (from_status.as_str(), command.to_status.as_str()) {
            ("proposed", "approved") => {
                if command.actor_subject == proposer_subject {
                    return Err(StorageError::RefundFundingConfigurationGovernanceConflict);
                }
                Some(command.actor_subject.clone())
            }
            ("active", "suspended" | "retired") | ("suspended", "retired") => {
                if current_approver.as_deref() != Some(command.actor_subject.as_str()) {
                    return Err(StorageError::RefundFundingConfigurationGovernanceConflict);
                }
                current_approver.clone()
            }
            _ => return Err(StorageError::InvalidRefundFundingConfigurationTransition),
        };
        let updated = transaction
            .execute(
                "UPDATE refund_funding_configurations
                 SET status = $3, approver_subject = $4, updated_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND status = $5",
                &[
                    &command.configuration_id,
                    &command.merchant_id,
                    &command.to_status,
                    &approver_subject,
                    &from_status,
                ],
            )
            .await?;
        if updated != 1 {
            return Err(StorageError::InvalidRefundFundingConfigurationTransition);
        }
        let action = match command.to_status.as_str() {
            "approved" => "approve",
            "suspended" => "suspend",
            "retired" => "retire",
            _ => return Err(StorageError::InvalidRefundFundingConfigurationTransition),
        };
        transaction
            .execute(
                "INSERT INTO refund_funding_configuration_events
                    (configuration_id, merchant_id, actor_subject, action,
                     from_status, to_status, evidence)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[
                    &command.configuration_id,
                    &command.merchant_id,
                    &command.actor_subject,
                    &action,
                    &from_status,
                    &command.to_status,
                    &command.evidence,
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"refund_funding_configuration",
                    &command.configuration_id,
                    &format!("tax.refund_funding_configuration.{action}"),
                    &serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": command.merchant_id,
                        "configuration_id": command.configuration_id,
                        "from_status": from_status,
                        "to_status": command.to_status,
                    }),
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(RefundFundingConfigurationTransitionResult {
            configuration_id: command.configuration_id,
            from_status,
            to_status: command.to_status,
            approver_subject,
        })
    }

    /// Claims an approved funding configuration for server-owned finalized
    /// topology verification. The claim is a short durable lease: a second
    /// verifier skips an unexpired pending lease, while a crashed owner can
    /// be reclaimed after the bounded lease window.
    pub async fn begin_refund_funding_verification(
        &self,
        merchant_id: Uuid,
        configuration_id: Uuid,
        actor_subject: &str,
    ) -> Result<Option<Uuid>, StorageError> {
        if actor_subject.is_empty() || actor_subject.len() > 256 {
            return Err(StorageError::InvalidRefundFundingConfiguration(
                "verification actor subject must contain 1 to 256 bytes".to_string(),
            ));
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT status,
                        COALESCE(
                            verification_lease_claimed_at > now()
                                - ($3::bigint * interval '1 second'),
                            false
                        )
                 FROM refund_funding_configurations
                 WHERE id = $1 AND merchant_id = $2 FOR UPDATE",
                &[
                    &configuration_id,
                    &merchant_id,
                    &(REFUND_FUNDING_VERIFICATION_LEASE.as_secs() as i64),
                ],
            )
            .await?
            .ok_or(StorageError::RefundFundingConfigurationNotFound)?;
        let status: String = row.get(0);
        let lease_active: bool = row.get(1);
        if status == "pending_verification" && lease_active {
            transaction.commit().await?;
            return Ok(None);
        }
        if status != "approved" && status != "pending_verification" {
            return Err(StorageError::RefundFundingConfigurationActivationConflict);
        }
        let claim_token = Uuid::now_v7();
        transaction
            .execute(
                "UPDATE refund_funding_configurations
                 SET status = CASE WHEN status = 'approved'
                                  THEN 'pending_verification' ELSE status END,
                     verification_lease_token = $3,
                     verification_lease_claimed_at = now(),
                     updated_at = now()
                 WHERE id = $1 AND merchant_id = $2
                   AND status IN ('approved', 'pending_verification')",
                &[&configuration_id, &merchant_id, &claim_token],
            )
            .await?;
        let (from_status, action) = if status == "approved" {
            ("approved", "begin_verification")
        } else {
            ("pending_verification", "reclaim_verification")
        };
        let evidence = serde_json::json!({
            "verifier_version": REFUND_FUNDING_VERIFIER_VERSION,
            "lease_token": claim_token,
            "lease_seconds": REFUND_FUNDING_VERIFICATION_LEASE.as_secs(),
        });
        transaction
            .execute(
                "INSERT INTO refund_funding_configuration_events
                    (configuration_id, merchant_id, actor_subject, action,
                     from_status, to_status, evidence)
                 VALUES ($1, $2, $3, $4, $5, 'pending_verification', $6)",
                &[
                    &configuration_id,
                    &merchant_id,
                    &actor_subject,
                    &action,
                    &from_status,
                    &evidence,
                ],
            )
            .await?;
        if status == "approved" {
            transaction
                .execute(
                    "INSERT INTO outbox_events
                        (aggregate_type, aggregate_id, event_type, payload)
                     VALUES ($1, $2, $3, $4)",
                    &[
                        &"refund_funding_configuration",
                        &configuration_id,
                        &"tax.refund_funding_configuration.verification_started",
                        &serde_json::json!({
                            "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                            "merchant_id": merchant_id,
                            "configuration_id": configuration_id,
                            "lease_token": claim_token,
                        }),
                    ],
                )
                .await?;
        }
        transaction.commit().await?;
        Ok(Some(claim_token))
    }

    /// Releases only the currently owned verification lease after a bounded
    /// provider failure. A stale worker cannot clear a newer owner's lease.
    pub async fn release_refund_funding_verification(
        &self,
        merchant_id: Uuid,
        configuration_id: Uuid,
        claim_token: Uuid,
    ) -> Result<bool, StorageError> {
        if claim_token.get_version_num() != 7 {
            return Err(StorageError::InvalidRefundFundingConfiguration(
                "verification lease token must be UUIDv7".to_string(),
            ));
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let updated = transaction
            .execute(
                "UPDATE refund_funding_configurations
                 SET verification_lease_token = NULL,
                     verification_lease_claimed_at = NULL,
                     updated_at = now()
                 WHERE id = $1 AND merchant_id = $2
                   AND status = 'pending_verification'
                   AND verification_lease_token = $3",
                &[&configuration_id, &merchant_id, &claim_token],
            )
            .await?;
        transaction.commit().await?;
        Ok(updated == 1)
    }

    /// Allocates a server-owned claim nonce. Gaps are safe and intentional:
    /// a process crash after allocation cannot make an already signed claim
    /// reusable. The durable claim-intent idempotency record is the replay
    /// authority, while this counter only prevents PDA collisions.
    pub async fn reserve_refund_claim_nonce(
        &self,
        merchant_id: Uuid,
        adjustment_id: Uuid,
    ) -> Result<u64, StorageError> {
        if merchant_id.get_version_num() != 7 || adjustment_id.get_version_num() != 7 {
            return Err(StorageError::InvalidRefundClaimIntent(
                "merchant and adjustment identities must be UUIDv7".to_string(),
            ));
        }
        for attempt in 0..MAX_REFUND_INTENT_RETRY_ATTEMPTS {
            match self
                .reserve_refund_claim_nonce_once(merchant_id, adjustment_id)
                .await
            {
                Ok(nonce) => return Ok(nonce),
                Err(error)
                    if attempt + 1 < MAX_REFUND_INTENT_RETRY_ATTEMPTS
                        && is_retryable_refund_intent_error(&error) =>
                {
                    tracing::warn!(
                        attempt = attempt + 1,
                        max_attempts = MAX_REFUND_INTENT_RETRY_ATTEMPTS,
                        "retrying refund claim nonce allocation after a transient PostgreSQL conflict"
                    );
                    tokio::time::sleep(REFUND_INTENT_RETRY_BACKOFF * (attempt as u32 + 1)).await;
                }
                Err(error) => return Err(error),
            }
        }
        Err(StorageError::InvalidConfiguration(
            "refund intent retry attempt budget must be positive".to_string(),
        ))
    }

    async fn reserve_refund_claim_nonce_once(
        &self,
        merchant_id: Uuid,
        adjustment_id: Uuid,
    ) -> Result<u64, StorageError> {
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "INSERT INTO refund_claim_nonce_counters
                    (merchant_id, adjustment_id, next_nonce)
                 VALUES ($1, $2, 2)
                 ON CONFLICT (merchant_id, adjustment_id) DO UPDATE
                    SET next_nonce = refund_claim_nonce_counters.next_nonce + 1,
                        updated_at = now()
                    WHERE refund_claim_nonce_counters.next_nonce < 18446744073709551615
                 RETURNING (next_nonce - 1)::text",
                &[&merchant_id, &adjustment_id],
            )
            .await?;
        let nonce = row
            .map(|value| parse_u64(value.get(0), "refund claim nonce"))
            .transpose()?
            .ok_or(StorageError::RefundClaimNonceAllocationConflict)?;
        transaction.commit().await?;
        Ok(nonce)
    }

    /// Persists one server-owned claim artifact and its first lifecycle event
    /// atomically. The caller must have already loaded finalized receipt
    /// evidence, an active verified funding configuration, and the wallet
    /// bindings. This method never accepts browser-provided topology.
    pub async fn create_refund_claim_intent(
        &self,
        command: CreateRefundClaimIntentCommand,
    ) -> Result<RefundClaimIntent, StorageError> {
        validate_refund_claim_intent_command(&command)?;
        for attempt in 0..MAX_REFUND_INTENT_RETRY_ATTEMPTS {
            match self.create_refund_claim_intent_once(&command).await {
                Ok(intent) => return Ok(intent),
                Err(error)
                    if attempt + 1 < MAX_REFUND_INTENT_RETRY_ATTEMPTS
                        && is_retryable_refund_intent_error(&error) =>
                {
                    tracing::warn!(
                        attempt = attempt + 1,
                        max_attempts = MAX_REFUND_INTENT_RETRY_ATTEMPTS,
                        "retrying refund claim intent after a transient PostgreSQL conflict"
                    );
                    tokio::time::sleep(REFUND_INTENT_RETRY_BACKOFF * (attempt as u32 + 1)).await;
                }
                Err(error) => return Err(error),
            }
        }
        Err(StorageError::InvalidConfiguration(
            "refund intent retry attempt budget must be positive".to_string(),
        ))
    }

    async fn create_refund_claim_intent_once(
        &self,
        command: &CreateRefundClaimIntentCommand,
    ) -> Result<RefundClaimIntent, StorageError> {
        let unsigned_artifact = command
            .artifact
            .unsigned_artifact()
            .map_err(|error| StorageError::InvalidRefundClaimIntent(error.to_string()))?;
        let claim = Pubkey::from_str(&command.artifact.instruction.claim)
            .map_err(|_| StorageError::InvalidRefundClaimIntent("claim PDA is invalid".into()))?;
        let claim_bytes = claim.to_bytes();
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let inputs = &command.inputs;
        let artifact = &command.artifact;
        let intent_id = Uuid::now_v7();
        let inserted = transaction
            .query_opt(
                "INSERT INTO refund_claim_intents
                    (id, merchant_id, idempotency_key, request_digest,
                     adjustment_id, funding_configuration_id,
                     funding_configuration_digest, receipt_signature, receipt_slot,
                     original_receipt, payer, merchant, mint, token_program,
                     proposer, refund_funder, refund_funder_subject, tax_vault,
                     net_source, refund_destination, claim_nonce,
                     gross_atomic, tax_atomic, net_atomic, reference_digest,
                     execute_after_unix, expires_at_unix, claim,
                     instruction_digest, artifact_digest, unsigned_artifact)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::text::numeric,
                         $10, $11, $12, $13, $14, $15, $16, $17, $18, $19,
                         $20, $21::text::numeric, $22::text::numeric,
                         $23::text::numeric, $24::text::numeric, $25,
                         $26, $27, $28, $29, $30, $31)
                 ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
                RETURNING id, merchant_id, idempotency_key, request_digest,
                           adjustment_id, funding_configuration_id,
                           receipt_signature, receipt_slot::text, mint, token_program,
                           gross_atomic::text, tax_atomic::text, net_atomic::text,
                           claim_nonce::text, claim, instruction_digest,
                           artifact_digest, status, signed_transaction_digest,
                           signature, submitted_slot::text, commitment, last_error_code,
                           unsigned_artifact",
                &[
                    &intent_id,
                    &command.merchant_id,
                    &command.idempotency_key,
                    &&command.request_digest[..],
                    &inputs.adjustment_id,
                    &inputs.funding_configuration_id,
                    &&inputs.funding_configuration_digest[..],
                    &inputs.receipt_signature,
                    &inputs.receipt_slot.to_string(),
                    &&inputs.original_receipt[..],
                    &&inputs.payer[..],
                    &&inputs.merchant[..],
                    &&inputs.mint[..],
                    &inputs.token_program,
                    &inputs.proposer,
                    &inputs.refund_funder,
                    &inputs.refund_funder_subject,
                    &inputs.tax_vault,
                    &inputs.net_source,
                    &artifact.refund_destination,
                    &inputs.claim_nonce.to_string(),
                    &inputs.gross_atomic.to_string(),
                    &inputs.tax_atomic.to_string(),
                    &inputs.net_atomic.to_string(),
                    &&inputs.reference_digest[..],
                    &inputs.execute_after_unix,
                    &inputs.expires_at_unix,
                    &&claim_bytes[..],
                    &&artifact.instruction_digest[..],
                    &&artifact.artifact_digest[..],
                    &unsigned_artifact,
                ],
            )
            .await?;
        if let Some(row) = inserted {
            let intent = parse_refund_claim_intent_row(row)?;
            transaction
                .execute(
                    "INSERT INTO refund_claim_intent_events
                        (intent_id, merchant_id, from_status, to_status, evidence)
                     VALUES ($1, $2, 'none', 'prepared_unsubmitted', $3)",
                    &[
                        &intent.id,
                        &command.merchant_id,
                        &serde_json::json!({
                            "request_digest": command.request_digest,
                            "artifact_digest": artifact.artifact_digest,
                        }),
                    ],
                )
                .await?;
            transaction
                .execute(
                    "INSERT INTO outbox_events
                        (aggregate_type, aggregate_id, event_type, payload)
                     VALUES ($1, $2, $3, $4)",
                    &[
                        &"refund_claim_intent",
                        &intent.id,
                        &"tax.refund_claim_intent.prepared",
                        &serde_json::json!({
                            "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                            "merchant_id": command.merchant_id,
                            "intent": intent,
                        }),
                    ],
                )
                .await?;
            transaction.commit().await?;
            return Ok(intent);
        }
        let row = transaction
            .query_one(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        adjustment_id, funding_configuration_id,
                        receipt_signature, receipt_slot::text, mint, token_program,
                        gross_atomic::text, tax_atomic::text, net_atomic::text,
                        claim_nonce::text, claim, instruction_digest,
                        artifact_digest, status, signed_transaction_digest,
                        signature, submitted_slot::text, commitment, last_error_code,
                        unsigned_artifact
                 FROM refund_claim_intents
                 WHERE merchant_id = $1 AND idempotency_key = $2
                 FOR UPDATE",
                &[&command.merchant_id, &command.idempotency_key],
            )
            .await?;
        let intent = parse_refund_claim_intent_row(row)?;
        if intent.request_digest != command.request_digest {
            return Err(StorageError::RefundClaimIntentIdempotencyConflict);
        }
        transaction.commit().await?;
        Ok(intent)
    }

    pub async fn load_refund_cancellation_intent_by_idempotency(
        &self,
        merchant_id: Uuid,
        idempotency_key: &str,
    ) -> Result<Option<RefundCancellationIntent>, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        claim_intent_id, adjustment_id, merchant, mint, adjustment,
                        claim_nonce::text, claim, governance_authority,
                        governance_subject, instruction_digest, artifact_digest,
                        status, signed_transaction_digest, signature,
                        submitted_slot::text, commitment, last_error_code,
                        unsigned_artifact
                 FROM refund_claim_cancellation_intents
                 WHERE merchant_id = $1 AND idempotency_key = $2",
                &[&merchant_id, &idempotency_key],
            )
            .await?;
        row.map(parse_refund_cancellation_intent_row).transpose()
    }

    pub async fn load_refund_cancellation_intent(
        &self,
        intent_id: Uuid,
        merchant_id: Uuid,
    ) -> Result<Option<RefundCancellationIntent>, StorageError> {
        if intent_id.get_version_num() != 7 || merchant_id.get_version_num() != 7 {
            return Err(StorageError::RefundCancellationIntentNotFound);
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        claim_intent_id, adjustment_id, merchant, mint, adjustment,
                        claim_nonce::text, claim, governance_authority,
                        governance_subject, instruction_digest, artifact_digest,
                        status, signed_transaction_digest, signature,
                        submitted_slot::text, commitment, last_error_code,
                        unsigned_artifact
                 FROM refund_claim_cancellation_intents
                 WHERE id = $1 AND merchant_id = $2",
                &[&intent_id, &merchant_id],
            )
            .await?;
        row.map(parse_refund_cancellation_intent_row).transpose()
    }

    /// Claims one submitted cancellation whose finalized event still needs
    /// reconciliation. The lease is committed before external RPC work and
    /// is fenced again by the final transition.
    pub async fn load_pending_refund_claim_cancellation_reconciliation(
        &self,
    ) -> Result<Option<RefundClaimCancellationReconciliationCandidate>, StorageError> {
        const LEASE_SECONDS: i64 = 60;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        claim_intent_id, adjustment_id, merchant, mint, adjustment,
                        claim_nonce::text, claim, governance_authority,
                        governance_subject, instruction_digest, artifact_digest,
                        status, signed_transaction_digest, signature,
                        submitted_slot::text, commitment, last_error_code,
                        unsigned_artifact
                 FROM refund_claim_cancellation_intents
                 WHERE signature IS NOT NULL
                   AND status IN ('submitting', 'submitted', 'processed', 'confirmed', 'unknown')
                   AND (reconciliation_claim_token IS NULL
                        OR reconciliation_claimed_at < now() - ($1::bigint * interval '1 second'))
                 ORDER BY updated_at, id
                 FOR UPDATE SKIP LOCKED
                 LIMIT 1",
                &[&LEASE_SECONDS],
            )
            .await?;
        let Some(row) = row else {
            transaction.commit().await?;
            return Ok(None);
        };
        let intent = parse_refund_cancellation_intent_row(row)?;
        let claim_token = Uuid::now_v7();
        let updated = transaction
            .execute(
                "UPDATE refund_claim_cancellation_intents
                 SET reconciliation_claim_token = $2,
                     reconciliation_claimed_at = now()
                 WHERE id = $1",
                &[&intent.id, &claim_token],
            )
            .await?;
        if updated != 1 {
            transaction.rollback().await?;
            return Ok(None);
        }
        transaction.commit().await?;
        Ok(Some(RefundClaimCancellationReconciliationCandidate {
            intent,
            claim_token,
        }))
    }

    /// Derives cancellation event binding from the immutable cancellation
    /// intent and the immutable refund-claim artifact. No browser field is
    /// accepted by this boundary.
    pub async fn load_refund_claim_cancellation_evidence_expectation(
        &self,
        intent: &RefundCancellationIntent,
    ) -> Result<RefundClaimCancellationEvidenceExpectation, StorageError> {
        let claim = self
            .load_refund_claim_intent(intent.claim_intent_id, intent.merchant_id)
            .await?
            .ok_or_else(|| {
                StorageError::InvalidRefundCancellationIntent(
                    "the cancellation's bound refund claim is missing".into(),
                )
            })?;
        let claim_artifact = serde_json::from_value::<RefundArtifact>(
            claim.unsigned_artifact.clone(),
        )
        .map_err(|_| {
            StorageError::InvalidRefundCancellationIntent(
                "the cancellation's bound refund claim artifact is invalid".into(),
            )
        })?;
        let reference_digest = refund_claim_evidence_expectation(&claim, &claim_artifact)
            .map_err(|error| StorageError::InvalidRefundCancellationIntent(error.to_string()))?
            .reference_digest;
        let governance_authority =
            Pubkey::from_str(&intent.governance_authority).map_err(|_| {
                StorageError::InvalidRefundCancellationIntent(
                    "persisted governance authority is invalid".into(),
                )
            })?;
        Ok(RefundClaimCancellationEvidenceExpectation {
            claim: intent.claim,
            adjustment: intent.adjustment,
            merchant: intent.merchant,
            mint: intent.mint,
            nonce: intent.claim_nonce,
            governance_authority: governance_authority.to_bytes(),
            reference_digest,
        })
    }

    pub async fn create_refund_cancellation_intent(
        &self,
        command: CreateRefundCancellationIntentCommand,
    ) -> Result<RefundCancellationIntent, StorageError> {
        validate_refund_cancellation_intent_command(&command)?;
        let unsigned_artifact = command
            .artifact
            .unsigned_artifact()
            .map_err(|error| StorageError::InvalidRefundCancellationIntent(error.to_string()))?;
        let claim = Pubkey::from_str(&command.artifact.instruction.claim).map_err(|_| {
            StorageError::InvalidRefundCancellationIntent("claim PDA is invalid".into())
        })?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let intent_id = Uuid::now_v7();
        let inputs = &command.inputs;
        let inserted = transaction
            .query_opt(
                "INSERT INTO refund_claim_cancellation_intents
                    (id, merchant_id, idempotency_key, request_digest,
                     claim_intent_id, adjustment_id, merchant, mint, adjustment,
                     claim_nonce, claim, governance_authority, governance_subject,
                     instruction_digest, artifact_digest, unsigned_artifact)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::text::numeric,
                         $11, $12, $13, $14, $15, $16)
                 ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
                 RETURNING id, merchant_id, idempotency_key, request_digest,
                           claim_intent_id, adjustment_id, merchant, mint, adjustment,
                           claim_nonce::text, claim, governance_authority,
                           governance_subject, instruction_digest, artifact_digest,
                           status, signed_transaction_digest, signature,
                           submitted_slot::text, commitment, last_error_code,
                           unsigned_artifact",
                &[
                    &intent_id,
                    &command.merchant_id,
                    &command.idempotency_key,
                    &&command.request_digest[..],
                    &inputs.claim_intent_id,
                    &inputs.adjustment_id,
                    &&inputs.merchant[..],
                    &&inputs.mint[..],
                    &&inputs.adjustment[..],
                    &inputs.claim_nonce.to_string(),
                    &&claim.to_bytes()[..],
                    &inputs.governance_authority,
                    &inputs.governance_subject,
                    &&command.artifact.instruction_digest[..],
                    &&command.artifact.artifact_digest[..],
                    &unsigned_artifact,
                ],
            )
            .await?;
        if let Some(row) = inserted {
            let intent = parse_refund_cancellation_intent_row(row)?;
            transaction
                .execute(
                    "INSERT INTO refund_claim_cancellation_intent_events
                        (intent_id, merchant_id, from_status, to_status, evidence)
                     VALUES ($1, $2, 'none', 'prepared_unsubmitted', $3)",
                    &[
                        &intent.id,
                        &command.merchant_id,
                        &serde_json::json!({
                            "request_digest": command.request_digest,
                            "artifact_digest": command.artifact.artifact_digest,
                        }),
                    ],
                )
                .await?;
            transaction
                .execute(
                    "INSERT INTO outbox_events
                        (aggregate_type, aggregate_id, event_type, payload)
                     VALUES ($1, $2, $3, $4)",
                    &[
                        &"refund_claim_cancellation_intent",
                        &intent.id,
                        &"tax.refund_claim_cancellation_intent.prepared",
                        &serde_json::json!({
                            "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                            "merchant_id": command.merchant_id,
                            "intent": intent,
                        }),
                    ],
                )
                .await?;
            transaction.commit().await?;
            return Ok(intent);
        }
        let row = transaction
            .query_one(
                "SELECT id, merchant_id, idempotency_key, request_digest,
                        claim_intent_id, adjustment_id, merchant, mint, adjustment,
                        claim_nonce::text, claim, governance_authority,
                        governance_subject, instruction_digest, artifact_digest,
                        status, signed_transaction_digest, signature,
                        submitted_slot::text, commitment, last_error_code,
                        unsigned_artifact
                 FROM refund_claim_cancellation_intents
                 WHERE merchant_id = $1 AND idempotency_key = $2
                 FOR UPDATE",
                &[&command.merchant_id, &command.idempotency_key],
            )
            .await?;
        let intent = parse_refund_cancellation_intent_row(row)?;
        transaction.commit().await?;
        if intent.request_digest != command.request_digest {
            return Err(StorageError::RefundCancellationIntentIdempotencyConflict);
        }
        Ok(intent)
    }

    pub async fn transition_refund_cancellation_intent(
        &self,
        command: RefundCancellationIntentTransition,
    ) -> Result<bool, StorageError> {
        let expected = parse_settlement_status(&command.expected_status)?;
        let next = parse_settlement_status(&command.next_status)?;
        if !can_transition(expected, next) || command.evidence.to_string().len() > 64 * 1024 {
            return Err(StorageError::InvalidRefundCancellationIntent(
                "refund cancellation lifecycle transition is invalid".into(),
            ));
        }
        if matches!(next, SettlementStatus::Unknown) && command.last_error_code.is_none() {
            return Err(StorageError::InvalidRefundCancellationIntent(
                "unknown refund cancellation status requires an error code".into(),
            ));
        }
        let commitment = match next {
            SettlementStatus::Processed => Some("processed"),
            SettlementStatus::Confirmed => Some("confirmed"),
            SettlementStatus::Finalized => Some("finalized"),
            _ => None,
        };
        let slot_text = command.slot.map(|value| value.to_string());
        let signed_digest = command
            .signed_transaction_digest
            .map(|value| value.to_vec());
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let updated = transaction
            .execute(
                "UPDATE refund_claim_cancellation_intents
                 SET status = $3,
                     signed_transaction_digest = COALESCE($4, signed_transaction_digest),
                     signature = COALESCE($5, signature),
                     submitted_slot = COALESCE($6::text::numeric, submitted_slot),
                     commitment = COALESCE($7, commitment),
                     last_error_code = $8,
                     updated_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND status = $9",
                &[
                    &command.intent_id,
                    &command.merchant_id,
                    &command.next_status,
                    &signed_digest,
                    &command.signature,
                    &slot_text,
                    &commitment,
                    &command.last_error_code,
                    &command.expected_status,
                ],
            )
            .await?;
        if updated != 1 {
            transaction.rollback().await?;
            return Ok(false);
        }
        transaction
            .execute(
                "INSERT INTO refund_claim_cancellation_intent_events
                    (intent_id, merchant_id, from_status, to_status, signature, slot, evidence)
                 VALUES ($1, $2, $3, $4, $5, $6::text::numeric, $7)",
                &[
                    &command.intent_id,
                    &command.merchant_id,
                    &command.expected_status,
                    &command.next_status,
                    &command.signature,
                    &slot_text,
                    &command.evidence,
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(true)
    }

    /// Atomically records finalized cancellation evidence and converges the
    /// original refund claim to `cancelled`. The claim transition is guarded
    /// by a transaction-local database setting in migration 0056 so an
    /// ordinary API update cannot forge a finalized-to-cancelled transition.
    pub async fn finalize_refund_claim_cancellation(
        &self,
        command: RefundCancellationFinalization,
    ) -> Result<bool, StorageError> {
        if command.expected_cancellation_status.trim().is_empty()
            || command.signature.trim().is_empty()
            || command.signature.len() > 128
            || command.slot == 0
            || command.evidence.to_string().len() > 64 * 1024
        {
            return Err(StorageError::InvalidRefundCancellationIntent(
                "finalized cancellation evidence is invalid".into(),
            ));
        }
        let expected = parse_settlement_status(&command.expected_cancellation_status)?;
        if !can_transition(expected, SettlementStatus::Finalized) {
            return Err(StorageError::InvalidRefundCancellationIntent(
                "cancellation must converge to finalized from its durable transport state".into(),
            ));
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms'; SET LOCAL app.refund_claim_cancellation_reconciliation = 'true';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let cancellation_updated = transaction
            .execute(
                "UPDATE refund_claim_cancellation_intents
                 SET status = 'finalized', signature = COALESCE(signature, $4),
                     submitted_slot = COALESCE(submitted_slot, $5::text::numeric),
                     commitment = 'finalized', last_error_code = NULL,
                     reconciliation_claim_token = NULL,
                     reconciliation_claimed_at = NULL,
                     updated_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND status = $3
                   AND (($6::uuid IS NOT NULL AND reconciliation_claim_token = $6::uuid)
                     OR ($6::uuid IS NULL AND (reconciliation_claim_token IS NULL
                         OR reconciliation_claimed_at < now() - interval '60 seconds'))) ",
                &[
                    &command.cancellation_intent_id,
                    &command.merchant_id,
                    &command.expected_cancellation_status,
                    &command.signature,
                    &command.slot.to_string(),
                    &command.reconciliation_claim_token,
                ],
            )
            .await?;
        if cancellation_updated != 1 {
            transaction.rollback().await?;
            return Ok(false);
        }
        let claim_updated = transaction
            .execute(
                "UPDATE refund_claim_intents
                 SET status = 'cancelled', updated_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND status = 'finalized'",
                &[&command.claim_intent_id, &command.merchant_id],
            )
            .await?;
        if claim_updated != 1 {
            transaction.rollback().await?;
            return Err(StorageError::InvalidRefundCancellationIntent(
                "refund claim was not finalized or was already cancelled".into(),
            ));
        }
        transaction
            .execute(
                "INSERT INTO refund_claim_cancellation_intent_events
                    (intent_id, merchant_id, from_status, to_status, signature, slot, evidence)
                 VALUES ($1, $2, $3, 'finalized', $4, $5::text::numeric, $6)",
                &[
                    &command.cancellation_intent_id,
                    &command.merchant_id,
                    &command.expected_cancellation_status,
                    &command.signature,
                    &command.slot.to_string(),
                    &command.evidence,
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO refund_claim_intent_events
                    (intent_id, merchant_id, from_status, to_status, signature, slot, evidence)
                 VALUES ($1, $2, 'finalized', 'cancelled', $3, $4::text::numeric, $5)",
                &[
                    &command.claim_intent_id,
                    &command.merchant_id,
                    &command.signature,
                    &command.slot.to_string(),
                    &command.evidence,
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4), ($5, $6, $7, $8)",
                &[
                    &"refund_claim_cancellation_intent",
                    &command.cancellation_intent_id,
                    &"tax.refund_claim_cancellation_intent.finalized",
                    &serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": command.merchant_id,
                        "intent_id": command.cancellation_intent_id,
                        "claim_intent_id": command.claim_intent_id,
                        "signature": command.signature,
                        "slot": command.slot,
                        "evidence": command.evidence,
                    }),
                    &"refund_claim_intent",
                    &command.claim_intent_id,
                    &"tax.refund_claim_intent.cancelled",
                    &serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": command.merchant_id,
                        "intent_id": command.claim_intent_id,
                        "from_status": "finalized",
                        "to_status": "cancelled",
                        "signature": command.signature,
                        "slot": command.slot,
                    }),
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn load_refund_execute_intent_by_idempotency(
        &self,
        merchant_id: Uuid,
        idempotency_key: &str,
    ) -> Result<Option<RefundExecuteIntent>, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let query = REFUND_EXECUTE_INTENT_SELECT_SQL
            .to_string()
            .replace("{where}", "WHERE merchant_id = $1 AND idempotency_key = $2");
        let row = client
            .query_opt(&query, &[&merchant_id, &idempotency_key])
            .await?;
        row.map(parse_refund_execute_intent_row).transpose()
    }

    pub async fn load_refund_execute_intent(
        &self,
        intent_id: Uuid,
        merchant_id: Uuid,
    ) -> Result<Option<RefundExecuteIntent>, StorageError> {
        if intent_id.get_version_num() != 7 || merchant_id.get_version_num() != 7 {
            return Err(StorageError::RefundExecuteIntentNotFound);
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let query = REFUND_EXECUTE_INTENT_SELECT_SQL
            .to_string()
            .replace("{where}", "WHERE id = $1 AND merchant_id = $2");
        let row = client
            .query_opt(&query, &[&intent_id, &merchant_id])
            .await?;
        row.map(parse_refund_execute_intent_row).transpose()
    }

    pub async fn create_refund_execute_intent(
        &self,
        command: CreateRefundExecuteIntentCommand,
    ) -> Result<RefundExecuteIntent, StorageError> {
        validate_refund_execute_intent_command(&command)?;
        for attempt in 0..MAX_REFUND_INTENT_RETRY_ATTEMPTS {
            match self.create_refund_execute_intent_once(&command).await {
                Ok(intent) => return Ok(intent),
                Err(error)
                    if attempt + 1 < MAX_REFUND_INTENT_RETRY_ATTEMPTS
                        && is_retryable_refund_intent_error(&error) =>
                {
                    tracing::warn!(
                        attempt = attempt + 1,
                        max_attempts = MAX_REFUND_INTENT_RETRY_ATTEMPTS,
                        "retrying refund execute intent after a transient PostgreSQL conflict"
                    );
                    tokio::time::sleep(REFUND_INTENT_RETRY_BACKOFF * (attempt as u32 + 1)).await;
                }
                Err(error) => return Err(error),
            }
        }
        Err(StorageError::InvalidConfiguration(
            "refund intent retry attempt budget must be positive".to_string(),
        ))
    }

    async fn create_refund_execute_intent_once(
        &self,
        command: &CreateRefundExecuteIntentCommand,
    ) -> Result<RefundExecuteIntent, StorageError> {
        let unsigned_artifact = command
            .artifact
            .unsigned_artifact()
            .map_err(|error| StorageError::InvalidRefundExecuteIntent(error.to_string()))?;
        let claim = Pubkey::from_str(&command.artifact.instruction.claim)
            .map_err(|_| StorageError::InvalidRefundExecuteIntent("claim PDA is invalid".into()))?;
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let inputs = &command.inputs;
        let artifact = &command.artifact;
        let intent_id = Uuid::now_v7();
        let inserted = transaction
            .query_opt(
                "INSERT INTO refund_execute_intents
                    (id, merchant_id, idempotency_key, request_digest,
                     claim_intent_id, funding_configuration_id,
                     funding_configuration_digest, claim, adjustment_id,
                     original_receipt, merchant, mint, token_program,
                     approver, refund_funder, refund_funder_subject,
                     instruction_digest, artifact_digest, unsigned_artifact)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                         $12, $13, $14, $15, $16, $17, $18, $19)
                 ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
                 RETURNING id, merchant_id, idempotency_key, request_digest,
                           claim_intent_id, funding_configuration_id,
                           funding_configuration_digest, claim, adjustment_id,
                           original_receipt, merchant, mint, token_program,
                           approver, refund_funder, refund_funder_subject,
                           instruction_digest, artifact_digest, status,
                           signed_transaction_digest, signature,
                           submitted_slot::text, commitment, last_error_code,
                           unsigned_artifact",
                &[
                    &intent_id,
                    &command.merchant_id,
                    &command.idempotency_key,
                    &&command.request_digest[..],
                    &command.claim_intent_id,
                    &inputs.funding_configuration_id,
                    &&inputs.funding_configuration_digest[..],
                    &&claim.to_bytes()[..],
                    &inputs.adjustment_id,
                    &&inputs.original_receipt[..],
                    &&inputs.merchant[..],
                    &&inputs.mint[..],
                    &inputs.token_program,
                    &inputs.approver,
                    &inputs.refund_funder,
                    &inputs.refund_funder_subject,
                    &&artifact.instruction_digest[..],
                    &&artifact.artifact_digest[..],
                    &unsigned_artifact,
                ],
            )
            .await?;
        if let Some(row) = inserted {
            let intent = parse_refund_execute_intent_row(row)?;
            transaction
                .execute(
                    "INSERT INTO refund_execute_intent_events
                        (intent_id, merchant_id, from_status, to_status, evidence)
                     VALUES ($1, $2, 'none', 'prepared_unsubmitted', $3)",
                    &[
                        &intent.id,
                        &command.merchant_id,
                        &serde_json::json!({
                            "request_digest": command.request_digest,
                            "artifact_digest": artifact.artifact_digest,
                            "claim_intent_id": command.claim_intent_id,
                        }),
                    ],
                )
                .await?;
            transaction
                .execute(
                    "INSERT INTO outbox_events
                        (aggregate_type, aggregate_id, event_type, payload)
                     VALUES ($1, $2, $3, $4)",
                    &[
                        &"refund_execute_intent",
                        &intent.id,
                        &"tax.refund_execute_intent.prepared",
                        &serde_json::json!({
                            "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                            "merchant_id": command.merchant_id,
                            "intent": intent,
                        }),
                    ],
                )
                .await?;
            transaction.commit().await?;
            return Ok(intent);
        }
        let row = transaction
            .query_one(
                &REFUND_EXECUTE_INTENT_SELECT_SQL.to_string().replace(
                    "{where}",
                    "WHERE merchant_id = $1 AND idempotency_key = $2 FOR UPDATE",
                ),
                &[&command.merchant_id, &command.idempotency_key],
            )
            .await?;
        let intent = parse_refund_execute_intent_row(row)?;
        if intent.request_digest != command.request_digest {
            return Err(StorageError::RefundExecuteIntentIdempotencyConflict);
        }
        transaction.commit().await?;
        Ok(intent)
    }

    pub async fn transition_refund_execute_intent(
        &self,
        command: RefundExecuteIntentTransition,
    ) -> Result<bool, StorageError> {
        let expected = parse_settlement_status(&command.expected_status)?;
        let next = parse_settlement_status(&command.next_status)?;
        if !can_transition(expected, next) || command.evidence.to_string().len() > 64 * 1024 {
            return Err(StorageError::InvalidRefundExecuteIntent(
                "refund execute lifecycle transition is invalid".into(),
            ));
        }
        if command
            .signature
            .as_deref()
            .is_some_and(|value| value.is_empty() || value.len() > 128)
            || command
                .last_error_code
                .as_deref()
                .is_some_and(|value| value.trim().is_empty() || value.len() > 128)
        {
            return Err(StorageError::InvalidRefundExecuteIntent(
                "refund execute transport evidence is invalid".into(),
            ));
        }
        if matches!(next, SettlementStatus::Unknown) && command.last_error_code.is_none() {
            return Err(StorageError::InvalidRefundExecuteIntent(
                "unknown refund execute status requires an error code".into(),
            ));
        }
        let commitment = match next {
            SettlementStatus::Processed => Some("processed"),
            SettlementStatus::Confirmed => Some("confirmed"),
            SettlementStatus::Finalized => Some("finalized"),
            _ => None,
        };
        let slot_text = command.slot.map(|value| value.to_string());
        let signed_digest = command
            .signed_transaction_digest
            .map(|value| value.to_vec());
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let updated = transaction
            .execute(
                "UPDATE refund_execute_intents
                 SET status = $3,
                     signed_transaction_digest = COALESCE($4, signed_transaction_digest),
                     signature = COALESCE($5, signature),
                     submitted_slot = COALESCE($6::text::numeric, submitted_slot),
                     commitment = COALESCE($7, commitment),
                     last_error_code = $8,
                     updated_at = now()
                 WHERE id = $1 AND merchant_id = $2 AND status = $9",
                &[
                    &command.intent_id,
                    &command.merchant_id,
                    &command.next_status,
                    &signed_digest,
                    &command.signature,
                    &slot_text,
                    &commitment,
                    &command.last_error_code,
                    &command.expected_status,
                ],
            )
            .await?;
        if updated != 1 {
            transaction.rollback().await?;
            return Ok(false);
        }
        transaction
            .execute(
                "INSERT INTO refund_execute_intent_events
                    (intent_id, merchant_id, from_status, to_status, signature, slot, evidence)
                 VALUES ($1, $2, $3, $4, $5, $6::text::numeric, $7)",
                &[
                    &command.intent_id,
                    &command.merchant_id,
                    &command.expected_status,
                    &command.next_status,
                    &command.signature,
                    &slot_text,
                    &command.evidence,
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"refund_execute_intent",
                    &command.intent_id,
                    &"tax.refund_execute_intent.status_changed",
                    &serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": command.merchant_id,
                        "intent_id": command.intent_id,
                        "from_status": command.expected_status,
                        "to_status": command.next_status,
                        "signature": command.signature,
                        "slot": slot_text,
                    }),
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn list_refund_funding_configurations_for_verification(
        &self,
        limit: usize,
    ) -> Result<Vec<RefundFundingVerificationCandidate>, StorageError> {
        if limit == 0 || limit > 64 {
            return Err(StorageError::InvalidConfiguration(
                "refund funding verification batch must be between 1 and 64".to_string(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT id, merchant_id, version, configuration_digest,
                        merchant_account, mint, tax_vault, vault_authority,
                        refund_funder, refund_funder_subject, net_source, token_program
                 FROM refund_funding_configurations
                 WHERE status = 'approved'
                    OR (status = 'pending_verification'
                        AND (verification_lease_token IS NULL
                             OR verification_lease_claimed_at < now()
                                - ($2::bigint * interval '1 second')))
                 ORDER BY updated_at, id
                 LIMIT $1",
                &[
                    &(limit as i64),
                    &(REFUND_FUNDING_VERIFICATION_LEASE.as_secs() as i64),
                ],
            )
            .await?;
        rows.into_iter()
            .map(|row| {
                Ok(RefundFundingVerificationCandidate {
                    id: row.get(0),
                    merchant_id: row.get(1),
                    version: row.get(2),
                    configuration_digest: parse_digest(row.get(3))?,
                    merchant_account: row.get(4),
                    mint: row.get(5),
                    tax_vault: row.get(6),
                    vault_authority: row.get(7),
                    refund_funder: row.get(8),
                    refund_funder_subject: row.get(9),
                    net_source: row.get(10),
                    token_program: row.get(11),
                })
            })
            .collect()
    }

    pub async fn load_refund_funding_verification_metrics(
        &self,
    ) -> Result<RefundFundingVerificationMetricsSnapshot, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_one(
                "SELECT
                    count(*) FILTER (
                        WHERE status IN ('approved', 'pending_verification')
                    )::bigint,
                    count(*) FILTER (
                        WHERE status = 'pending_verification'
                          AND verification_lease_token IS NOT NULL
                          AND verification_lease_claimed_at > now()
                              - ($1::bigint * interval '1 second')
                    )::bigint,
                    count(*) FILTER (
                        WHERE status = 'pending_verification'
                          AND verification_lease_token IS NOT NULL
                          AND verification_lease_claimed_at <= now()
                              - ($1::bigint * interval '1 second')
                    )::bigint
                 FROM refund_funding_configurations",
                &[&(REFUND_FUNDING_VERIFICATION_LEASE.as_secs() as i64)],
            )
            .await?;
        Ok(RefundFundingVerificationMetricsSnapshot {
            pending: row.get::<_, i64>(0).max(0) as u64,
            leases_active: row.get::<_, i64>(1).max(0) as u64,
            leases_expired: row.get::<_, i64>(2).max(0) as u64,
        })
    }

    /// Atomically stores one complete finalized topology proof and activates
    /// only the exact pending configuration it was produced for.
    pub async fn activate_refund_funding_configuration(
        &self,
        merchant_id: Uuid,
        configuration_id: Uuid,
        claim_token: Uuid,
        verification: &indexer::RefundFundingTopologyVerification,
    ) -> Result<bool, StorageError> {
        if verification.configuration_id != configuration_id
            || verification.configuration_version <= 0
            || verification.verification_slot == 0
            || verification.verification_digest == [0; 32]
            || verification.accounts.len() != 5
            || verification.verification_commitment != "finalized"
            || claim_token.get_version_num() != 7
        {
            return Err(StorageError::RefundFundingConfigurationVerificationInvalid(
                "verification identity, commitment, account count, slot, or digest is invalid"
                    .to_string(),
            ));
        }
        let evidence = serde_json::to_value(verification)?;
        let evidence_bytes = serde_json::to_vec(&evidence)?;
        if evidence_bytes.is_empty() || evidence_bytes.len() > 32 * 1024 {
            return Err(StorageError::RefundFundingConfigurationVerificationInvalid(
                "verification evidence exceeds the bounded persistence limit".to_string(),
            ));
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT status, version, configuration_digest,
                        merchant_account, mint, tax_vault, vault_authority,
                        refund_funder, net_source, token_program,
                        refund_funder_subject, verification_digest,
                        verification_lease_token
                 FROM refund_funding_configurations
                 WHERE id = $1 AND merchant_id = $2 FOR UPDATE",
                &[&configuration_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::RefundFundingConfigurationNotFound)?;
        let status: String = row.get(0);
        let version: i32 = row.get(1);
        let funder_subject: Option<String> = row.get(10);
        let existing_digest: Option<Vec<u8>> = row.get(11);
        let stored_claim_token: Option<Uuid> = row.get(12);
        if version != verification.configuration_version
            || row.get::<_, Vec<u8>>(2).as_slice() != verification.configuration_digest
            || row.get::<_, String>(3) != verification.merchant_account
            || row.get::<_, String>(4) != verification.mint
            || row.get::<_, String>(5) != verification.tax_vault
            || row.get::<_, String>(6) != verification.vault_authority
            || row.get::<_, String>(7) != verification.refund_funder
            || row.get::<_, String>(8) != verification.net_source
            || row.get::<_, String>(9) != verification.token_program
        {
            return Err(StorageError::RefundFundingConfigurationVerificationInvalid(
                "verification does not match the stored funding configuration".to_string(),
            ));
        }
        let funder_subject = funder_subject.ok_or_else(|| {
            StorageError::RefundFundingConfigurationVerificationInvalid(
                "reviewed funding configuration has no refund-funder subject".to_string(),
            )
        })?;
        let binding = transaction
            .query_opt(
                "SELECT public_key
                 FROM settlement_adjustment_authority_bindings
                 WHERE merchant_id = $1 AND subject = $2
                   AND role = 'refund_funder' AND status = 'active'",
                &[&merchant_id, &funder_subject],
            )
            .await?
            .ok_or_else(|| {
                StorageError::RefundFundingConfigurationVerificationInvalid(
                    "refund-funder wallet subject has no active wallet binding".to_string(),
                )
            })?;
        let bound_funder: Vec<u8> = binding.get(0);
        let configured_funder = Pubkey::from_str(&verification.refund_funder).map_err(|_| {
            StorageError::RefundFundingConfigurationVerificationInvalid(
                "configured refund-funder address is invalid".to_string(),
            )
        })?;
        if bound_funder.as_slice() != configured_funder.as_ref() {
            return Err(StorageError::RefundFundingConfigurationVerificationInvalid(
                "refund-funder wallet binding does not match the configured key".to_string(),
            ));
        }
        if status == "active"
            && existing_digest.as_deref() == Some(&verification.verification_digest[..])
        {
            transaction.commit().await?;
            return Ok(false);
        }
        if status != "pending_verification" || stored_claim_token != Some(claim_token) {
            return Err(StorageError::RefundFundingConfigurationActivationConflict);
        }
        let updated = transaction
            .execute(
                "UPDATE refund_funding_configurations
                 SET status = 'active', verification_digest = $3,
                     verification_slot = $4::text::numeric,
                     verification_commitment = $5,
                     verification_evidence = $6,
                     verifier_version = $7,
                     verification_lease_token = NULL,
                     verification_lease_claimed_at = NULL,
                     verified_at = now(), updated_at = now()
                 WHERE id = $1 AND merchant_id = $2
                   AND status = 'pending_verification'
                   AND verification_lease_token = $8",
                &[
                    &configuration_id,
                    &merchant_id,
                    &&verification.verification_digest[..],
                    &verification.verification_slot.to_string(),
                    &verification.verification_commitment,
                    &evidence,
                    &REFUND_FUNDING_VERIFIER_VERSION,
                    &claim_token,
                ],
            )
            .await?;
        if updated != 1 {
            transaction.rollback().await?;
            return Ok(false);
        }
        transaction
            .execute(
                "INSERT INTO refund_funding_configuration_events
                    (configuration_id, merchant_id, actor_subject, action,
                     from_status, to_status, evidence)
                 VALUES ($1, $2, $3, 'activate', 'pending_verification',
                         'active', $4)",
                &[
                    &configuration_id,
                    &merchant_id,
                    &REFUND_FUNDING_VERIFIER_ACTOR,
                    &evidence,
                ],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO outbox_events
                    (aggregate_type, aggregate_id, event_type, payload)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &"refund_funding_configuration",
                    &configuration_id,
                    &"tax.refund_funding_configuration.active",
                    &serde_json::json!({
                        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                        "merchant_id": merchant_id,
                        "configuration_id": configuration_id,
                        "verification_digest": verification.verification_digest,
                    }),
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn begin_settlement_route_verification_run(
        &self,
        run_kind: &str,
        batch_size: usize,
        operator_approval_id: Option<Uuid>,
    ) -> Result<Uuid, StorageError> {
        validate_settlement_route_verification_run_start(run_kind, operator_approval_id)?;
        if !(1..=64).contains(&batch_size) {
            return Err(StorageError::InvalidConfiguration(
                "settlement route verification run batch must be between 1 and 64".to_string(),
            ));
        }
        let run_id = Uuid::now_v7();
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        client
            .execute(
                "INSERT INTO settlement_route_verification_runs
                    (id, run_kind, status, batch_size, operator_approval_id)
                 VALUES ($1, $2, 'running', $3, $4)",
                &[
                    &run_id,
                    &run_kind,
                    &(batch_size as i16),
                    &operator_approval_id,
                ],
            )
            .await?;
        Ok(run_id)
    }

    pub async fn complete_settlement_route_verification_run(
        &self,
        run_id: Uuid,
        summary: SettlementRouteVerificationRunSummary,
    ) -> Result<(), StorageError> {
        validate_settlement_route_verification_run_summary(summary)?;
        let status = if summary.failed == 0 {
            "completed"
        } else {
            "completed_with_failures"
        };
        let failure_code = (summary.failed > 0).then_some("route_attempts_failed");
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let updated = client
            .execute(
                "UPDATE settlement_route_verification_runs
                 SET status = $2, examined_count = $3, activated_count = $4,
                     already_active_count = $5, failed_count = $6,
                     failure_code = $7, completed_at = now(), updated_at = now()
                 WHERE id = $1 AND status = 'running'",
                &[
                    &run_id,
                    &status,
                    &(summary.examined as i32),
                    &(summary.activated as i32),
                    &(summary.already_active as i32),
                    &(summary.failed as i32),
                    &failure_code,
                ],
            )
            .await?;
        if updated != 1 {
            return Err(StorageError::SettlementRouteVerificationRunConflict);
        }
        Ok(())
    }

    pub async fn fail_settlement_route_verification_run(
        &self,
        run_id: Uuid,
        failure_code: &str,
    ) -> Result<(), StorageError> {
        if failure_code.is_empty()
            || failure_code.len() > 128
            || !failure_code
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(StorageError::InvalidConfiguration(
                "settlement route verification failure code is invalid".to_string(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let updated = client
            .execute(
                "UPDATE settlement_route_verification_runs
                 SET status = 'failed', failure_code = $2,
                     completed_at = now(), updated_at = now()
                 WHERE id = $1 AND status = 'running'",
                &[&run_id, &failure_code],
            )
            .await?;
        if updated != 1 {
            return Err(StorageError::SettlementRouteVerificationRunConflict);
        }
        Ok(())
    }

    /// Returns a bounded batch of routes that require a separately authorized
    /// finalized chain verification. This read is intentionally not used by
    /// checkout or settlement compilation.
    pub async fn list_settlement_routes_for_verification(
        &self,
        limit: usize,
    ) -> Result<Vec<SettlementRouteVerificationCandidate>, StorageError> {
        if limit == 0 || limit > 64 {
            return Err(StorageError::InvalidConfiguration(
                "route verification batch must be between 1 and 64".to_string(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT id, merchant_id, route_key, version, merchant_account,
                        mint, tax_vault, net_destination, token_program
                 FROM settlement_routes
                 WHERE status IN ('pending_verification', 'suspended')
                 ORDER BY updated_at, id
                 LIMIT $1",
                &[&(limit as i64)],
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| SettlementRouteVerificationCandidate {
                id: row.get(0),
                merchant_id: row.get(1),
                route_key: row.get(2),
                version: row.get(3),
                merchant_account: row.get(4),
                mint: row.get(5),
                tax_vault: row.get(6),
                net_destination: row.get(7),
                token_program: row.get(8),
            })
            .collect())
    }

    /// Atomically records chain evidence and promotes a pending/suspended
    /// route. A repeated identical verification is idempotent; a different
    /// proof cannot overwrite an already active or retired route.
    pub async fn activate_settlement_route(
        &self,
        merchant_id: Uuid,
        route_id: Uuid,
        verification: &indexer::SettlementRouteVerification,
    ) -> Result<bool, StorageError> {
        if verification.route_id != route_id
            || verification.route_version <= 0
            || verification.verification_digest == [0; 32]
            || verification.accounts.len() != 5
            || verification.verification_schema != SETTLEMENT_ROUTE_VERIFICATION_SCHEMA
            || verification.asset_config.trim().is_empty()
            || verification.asset_kind != STABLECOIN_ASSET_KIND
            || !verification.asset_enabled
            || verification.mint_decimals > 9
            || !matches!(
                verification.verification_commitment,
                "finalized" | "local_synthetic"
            )
        {
            return Err(StorageError::SettlementRouteVerificationInvalid(
                "verification identity, commitment, account count, or digest is invalid"
                    .to_string(),
            ));
        }
        let mint = Pubkey::from_str(verification.mint.trim()).map_err(|_| {
            StorageError::SettlementRouteVerificationInvalid(
                "verification mint is not a canonical public key".to_string(),
            )
        })?;
        let (expected_asset_config, expected_asset_config_bump) = Pubkey::find_program_address(
            &[b"asset-config", mint.as_ref()],
            &settlement_client::settlement_program_id(),
        );
        if verification.asset_config != expected_asset_config.to_string() {
            return Err(StorageError::SettlementRouteVerificationInvalid(
                "verification asset admission PDA does not match the route mint".to_string(),
            ));
        }
        if verification.asset_config_bump != expected_asset_config_bump {
            return Err(StorageError::SettlementRouteVerificationInvalid(
                "verification asset admission PDA bump does not match the route mint".to_string(),
            ));
        }
        let evidence = serde_json::to_value(verification)?;
        let evidence_bytes = serde_json::to_vec(&evidence)?;
        if evidence_bytes.is_empty() || evidence_bytes.len() > 32 * 1024 {
            return Err(StorageError::SettlementRouteVerificationInvalid(
                "verification evidence exceeds the bounded persistence limit".to_string(),
            ));
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;

        let row = transaction
            .query_opt(
                "SELECT version, status, verification_digest,
                        merchant_account, mint, tax_vault, net_destination, token_program
                 FROM settlement_routes
                 WHERE id = $1 AND merchant_id = $2
                 FOR UPDATE",
                &[&route_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::SettlementRouteNotFound)?;
        let version: i32 = row.get(0);
        let status: String = row.get(1);
        let existing_digest: Option<Vec<u8>> = row.get(2);
        if version != verification.route_version {
            return Err(StorageError::SettlementRouteVerificationInvalid(
                "verification was produced for a different route version".to_string(),
            ));
        }
        let route_configuration_matches = row.get::<_, String>(3)
            == verification.merchant_account.as_str()
            && row.get::<_, String>(4) == verification.mint.as_str()
            && row.get::<_, String>(5) == verification.tax_vault.as_str()
            && row.get::<_, String>(6) == verification.net_destination.as_str()
            && row.get::<_, String>(7) == verification.token_program.as_str();
        if !route_configuration_matches {
            return Err(StorageError::SettlementRouteVerificationInvalid(
                "verification was produced for a different route account configuration".to_string(),
            ));
        }
        if status == "active"
            && existing_digest.as_deref() == Some(&verification.verification_digest[..])
        {
            transaction.commit().await?;
            return Ok(false);
        }
        if !matches!(status.as_str(), "pending_verification" | "suspended") {
            return Err(StorageError::SettlementRouteActivationConflict);
        }

        transaction
            .execute(
                "INSERT INTO settlement_route_verifications
                    (route_id, merchant_id, route_version, verification_digest,
                     verification_slot, verification_commitment, verification_schema, evidence)
                 VALUES ($1, $2, $3, $4, $5::text::numeric, $6, $7, $8)
                 ON CONFLICT (route_id, verification_digest) DO NOTHING",
                &[
                    &route_id,
                    &merchant_id,
                    &verification.route_version,
                    &&verification.verification_digest[..],
                    &verification.verification_slot.to_string(),
                    &verification.verification_commitment,
                    &verification.verification_schema,
                    &evidence,
                ],
            )
            .await?;
        let mint_decimals = i16::from(verification.mint_decimals);
        let updated = transaction
            .execute(
                "UPDATE settlement_routes
                 SET status = 'active', verification_digest = $2,
                     verification_slot = $3::text::numeric,
                     verification_commitment = $4,
                     verification_evidence = $5, mint_decimals = $6,
                     asset_config = $7, asset_kind = $8, asset_enabled = $9,
                     asset_config_bump = $10, verification_schema = $11,
                     verified_at = now(), updated_at = now()
                 WHERE id = $1 AND merchant_id = $12
                   AND status IN ('pending_verification', 'suspended')",
                &[
                    &route_id,
                    &&verification.verification_digest[..],
                    &verification.verification_slot.to_string(),
                    &verification.verification_commitment,
                    &evidence,
                    &mint_decimals,
                    &verification.asset_config,
                    &i16::from(verification.asset_kind),
                    &verification.asset_enabled,
                    &i16::from(verification.asset_config_bump),
                    &verification.verification_schema,
                    &merchant_id,
                ],
            )
            .await?;
        if updated != 1 {
            return Err(StorageError::SettlementRouteActivationConflict);
        }
        transaction.commit().await?;
        Ok(true)
    }

    /// Returns the authenticated merchant's active route. The route table is
    /// configuration authority only; Solana remains authoritative for account
    /// ownership and token-account compatibility.
    pub async fn load_active_settlement_route(
        &self,
        merchant_id: Uuid,
        route_id: Uuid,
    ) -> Result<SettlementRoute, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT route_key, version, status, merchant_account, mint,
                        tax_vault, net_destination, token_program, mint_decimals,
                        effective_from <= now()
                            AND (effective_until IS NULL OR now() < effective_until)
                 FROM settlement_routes
                 WHERE id = $1 AND merchant_id = $2
                   AND verification_schema = 'settlement-route-verification/v2'",
                &[&route_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::SettlementRouteNotFound)?;
        let status: String = row.get(2);
        let effective: bool = row.get(9);
        if status != "active" || !effective {
            return Err(StorageError::SettlementRouteUnavailable);
        }
        Ok(SettlementRoute {
            id: route_id,
            route_key: row.get(0),
            version: row.get(1),
            merchant_account: row.get(3),
            mint: row.get(4),
            mint_decimals: row.get::<_, i16>(8).try_into().map_err(|_| {
                StorageError::SettlementRouteIntegrity(
                    "active route mint decimals are outside the supported range".to_string(),
                )
            })?,
            tax_vault: row.get(5),
            net_destination: row.get(6),
            token_program: row.get(7),
        })
    }

    /// Resolves the one active verified route matching finalized receipt
    /// identity. An adjustment compiler must not accept a route ID or token
    /// program from the browser because the original settlement proposal does
    /// not carry a route authority.
    pub async fn load_active_settlement_route_for_chain_identity(
        &self,
        merchant_id: Uuid,
        merchant_account: &str,
        mint: &str,
    ) -> Result<SettlementRoute, StorageError> {
        if merchant_account.len() < 32
            || merchant_account.len() > 44
            || mint.len() < 32
            || mint.len() > 44
        {
            return Err(StorageError::SettlementRouteUnavailable);
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT id, route_key, version, merchant_account, mint,
                        tax_vault, net_destination, token_program, mint_decimals
                 FROM settlement_routes
                 WHERE merchant_id = $1
                   AND status = 'active'
                   AND verification_schema = 'settlement-route-verification/v2'
                   AND asset_kind = 1 AND asset_enabled = true
                   AND asset_config IS NOT NULL AND asset_config_bump IS NOT NULL
                   AND effective_from <= now()
                   AND (effective_until IS NULL OR now() < effective_until)
                   AND merchant_account = $2
                   AND mint = $3
                 ORDER BY version DESC, id
                 LIMIT 2",
                &[&merchant_id, &merchant_account, &mint],
            )
            .await?;
        match rows.len() {
            0 => Err(StorageError::SettlementRouteNotFound),
            1 => match rows.into_iter().next() {
                Some(row) => Ok(SettlementRoute {
                    id: row.get(0),
                    route_key: row.get(1),
                    version: row.get(2),
                    merchant_account: row.get(3),
                    mint: row.get(4),
                    mint_decimals: row.get::<_, i16>(8).try_into().map_err(|_| {
                        StorageError::SettlementRouteIntegrity(
                            "active route mint decimals are outside the supported range"
                                .to_string(),
                        )
                    })?,
                    tax_vault: row.get(5),
                    net_destination: row.get(6),
                    token_program: row.get(7),
                }),
                None => Err(StorageError::SettlementRouteUnavailable),
            },
            _ => Err(StorageError::SettlementRouteUnavailable),
        }
    }

    /// Resolves exactly one active verified route for a tenant and asset.
    /// Revenue-stream plans use this server-owned lookup because the browser
    /// supplies the asset identity but must not select merchant-side accounts.
    pub async fn load_active_settlement_route_for_mint(
        &self,
        merchant_id: Uuid,
        mint: &str,
    ) -> Result<SettlementRoute, StorageError> {
        if merchant_id.get_version_num() != 7 || mint.len() < 32 || mint.len() > 44 {
            return Err(StorageError::SettlementRouteUnavailable);
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT id, route_key, version, merchant_account, mint,
                        tax_vault, net_destination, token_program, mint_decimals
                 FROM settlement_routes
                 WHERE merchant_id = $1
                   AND status = 'active'
                   AND verification_schema = 'settlement-route-verification/v2'
                   AND asset_kind = 1 AND asset_enabled = true
                   AND asset_config IS NOT NULL AND asset_config_bump IS NOT NULL
                   AND effective_from <= now()
                   AND (effective_until IS NULL OR now() < effective_until)
                   AND mint = $2
                 ORDER BY version DESC, id
                 LIMIT 2",
                &[&merchant_id, &mint],
            )
            .await?;
        match rows.len() {
            0 => Err(StorageError::SettlementRouteNotFound),
            1 => match rows.into_iter().next() {
                Some(row) => Ok(SettlementRoute {
                    id: row.get(0),
                    route_key: row.get(1),
                    version: row.get(2),
                    merchant_account: row.get(3),
                    mint: row.get(4),
                    mint_decimals: row.get::<_, i16>(8).try_into().map_err(|_| {
                        StorageError::SettlementRouteIntegrity(
                            "active route mint decimals are outside the supported range"
                                .to_string(),
                        )
                    })?,
                    tax_vault: row.get(5),
                    net_destination: row.get(6),
                    token_program: row.get(7),
                }),
                None => Err(StorageError::SettlementRouteUnavailable),
            },
            _ => Err(StorageError::SettlementRouteUnavailable),
        }
    }

    /// Returns a bounded batch of server-owned revenue payout destinations
    /// that require a separate finalized chain verification pass.
    pub async fn list_revenue_stream_destination_routes_for_verification(
        &self,
        limit: usize,
    ) -> Result<Vec<RevenueStreamDestinationRouteVerificationCandidate>, StorageError> {
        if limit == 0 || limit > 64 {
            return Err(StorageError::InvalidConfiguration(
                "revenue destination verification batch must be between 1 and 64".to_string(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT id, merchant_id, route_key, version, recipient_owner,
                        mint, destination_token_account, token_program
                 FROM revenue_stream_destination_routes
                 WHERE status IN ('pending_verification', 'suspended')
                 ORDER BY updated_at, id
                 LIMIT $1",
                &[&(limit as i64)],
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| RevenueStreamDestinationRouteVerificationCandidate {
                id: row.get(0),
                merchant_id: row.get(1),
                route_key: row.get(2),
                version: row.get(3),
                recipient_owner: row.get(4),
                mint: row.get(5),
                destination_token_account: row.get(6),
                token_program: row.get(7),
            })
            .collect())
    }

    /// Atomically records finalized destination-account evidence and promotes
    /// a pending/suspended payout route. The route remains inactive when its
    /// chain configuration or evidence identity does not match.
    pub async fn activate_revenue_stream_destination_route(
        &self,
        merchant_id: Uuid,
        route_id: Uuid,
        verification: &indexer::RevenueStreamDestinationRouteVerification,
    ) -> Result<bool, StorageError> {
        if verification.route_id != route_id
            || verification.route_version <= 0
            || verification.verification_digest == [0; 32]
            || verification.accounts.len() != 2
            || verification.mint_decimals > 9
            || !matches!(
                verification.verification_commitment,
                "finalized" | "local_synthetic"
            )
        {
            return Err(
                StorageError::RevenueStreamDestinationRouteVerificationInvalid(
                    "verification identity, commitment, account count, or digest is invalid"
                        .to_string(),
                ),
            );
        }
        let evidence = serde_json::to_value(verification)?;
        let evidence_bytes = serde_json::to_vec(&evidence)?;
        if evidence_bytes.is_empty() || evidence_bytes.len() > 32 * 1024 {
            return Err(
                StorageError::RevenueStreamDestinationRouteVerificationInvalid(
                    "verification evidence exceeds the bounded persistence limit".to_string(),
                ),
            );
        }
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;
        transaction
            .batch_execute(&format!(
                "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms';",
                DEFAULT_LOCK_TIMEOUT.as_millis(),
                DEFAULT_STATEMENT_TIMEOUT.as_millis()
            ))
            .await?;
        let row = transaction
            .query_opt(
                "SELECT version, status, verification_digest,
                        recipient_owner, mint, destination_token_account, token_program
                 FROM revenue_stream_destination_routes
                 WHERE id = $1 AND merchant_id = $2
                 FOR UPDATE",
                &[&route_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::RevenueStreamDestinationRouteNotFound)?;
        let version: i32 = row.get(0);
        let status: String = row.get(1);
        let existing_digest: Option<Vec<u8>> = row.get(2);
        if version != verification.route_version {
            return Err(
                StorageError::RevenueStreamDestinationRouteVerificationInvalid(
                    "verification was produced for a different route version".to_string(),
                ),
            );
        }
        let route_matches = row.get::<_, String>(3) == verification.recipient_owner
            && row.get::<_, String>(4) == verification.mint
            && row.get::<_, String>(5) == verification.destination_token_account
            && row.get::<_, String>(6) == verification.token_program;
        if !route_matches {
            return Err(
                StorageError::RevenueStreamDestinationRouteVerificationInvalid(
                    "verification was produced for a different destination configuration"
                        .to_string(),
                ),
            );
        }
        if status == "active"
            && existing_digest.as_deref() == Some(&verification.verification_digest[..])
        {
            transaction.commit().await?;
            return Ok(false);
        }
        if !matches!(status.as_str(), "pending_verification" | "suspended") {
            return Err(StorageError::RevenueStreamDestinationRouteActivationConflict);
        }
        transaction
            .execute(
                "INSERT INTO revenue_stream_destination_route_verifications
                    (route_id, merchant_id, route_version, verification_digest,
                     verification_slot, verification_commitment, evidence, verifier_version)
                 VALUES ($1, $2, $3, $4, $5::text::numeric, $6, $7, $8)
                 ON CONFLICT (route_id, verification_digest) DO NOTHING",
                &[
                    &route_id,
                    &merchant_id,
                    &verification.route_version,
                    &&verification.verification_digest[..],
                    &verification.verification_slot.to_string(),
                    &verification.verification_commitment,
                    &evidence,
                    &"indexer-route-verifier-v1",
                ],
            )
            .await?;
        let updated = transaction
            .execute(
                "UPDATE revenue_stream_destination_routes
                 SET status = 'active', verification_digest = $2,
                     verification_slot = $3::text::numeric,
                     verification_commitment = $4, verification_evidence = $5,
                     verifier_version = $6, verified_at = now(), updated_at = now()
                 WHERE id = $1 AND merchant_id = $7
                   AND status IN ('pending_verification', 'suspended')",
                &[
                    &route_id,
                    &&verification.verification_digest[..],
                    &verification.verification_slot.to_string(),
                    &verification.verification_commitment,
                    &evidence,
                    &"indexer-route-verifier-v1",
                    &merchant_id,
                ],
            )
            .await?;
        if updated != 1 {
            return Err(StorageError::RevenueStreamDestinationRouteActivationConflict);
        }
        transaction.commit().await?;
        Ok(true)
    }

    /// Resolves exactly one active, effective, finalized-verified destination
    /// for a tenant, recipient owner, and mint. This lookup is outside the
    /// wallet signing path and never derives or creates an ATA.
    pub async fn load_active_revenue_stream_destination_route(
        &self,
        merchant_id: Uuid,
        recipient_owner: &str,
        mint: &str,
    ) -> Result<RevenueStreamDestinationRoute, StorageError> {
        if merchant_id.get_version_num() != 7
            || recipient_owner.len() < 32
            || recipient_owner.len() > 44
            || mint.len() < 32
            || mint.len() > 44
        {
            return Err(StorageError::RevenueStreamDestinationRouteUnavailable);
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT id, merchant_id, route_key, version, recipient_owner,
                        mint, destination_token_account, token_program,
                        verification_digest, verification_slot::text,
                        verification_commitment, verification_evidence
                 FROM revenue_stream_destination_routes
                 WHERE merchant_id = $1
                   AND recipient_owner = $2
                   AND mint = $3
                   AND status = 'active'
                   AND verification_commitment = 'finalized'
                   AND effective_from <= now()
                   AND (effective_until IS NULL OR now() < effective_until)
                 ORDER BY version DESC, id
                 LIMIT 2",
                &[&merchant_id, &recipient_owner, &mint],
            )
            .await?;
        match rows.len() {
            0 => Err(StorageError::RevenueStreamDestinationRouteNotFound),
            1 => rows
                .into_iter()
                .next()
                .ok_or(StorageError::RevenueStreamDestinationRouteUnavailable)
                .and_then(parse_revenue_stream_destination_route_row),
            _ => Err(StorageError::RevenueStreamDestinationRouteUnavailable),
        }
    }

    /// Resolves one active destination by its server-issued reference. This
    /// is the only destination lookup used by revenue-plan creation; the
    /// caller never supplies the recipient token account or token program.
    pub async fn load_active_revenue_stream_destination_route_by_id(
        &self,
        merchant_id: Uuid,
        route_id: Uuid,
    ) -> Result<RevenueStreamDestinationRoute, StorageError> {
        if merchant_id.get_version_num() != 7 || route_id.get_version_num() != 7 {
            return Err(StorageError::RevenueStreamDestinationRouteUnavailable);
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT id, merchant_id, route_key, version, recipient_owner,
                        mint, destination_token_account, token_program,
                        verification_digest, verification_slot::text,
                        verification_commitment, verification_evidence
                 FROM revenue_stream_destination_routes
                 WHERE id = $1 AND merchant_id = $2
                   AND status = 'active'
                   AND verification_commitment = 'finalized'
                   AND effective_from <= now()
                   AND (effective_until IS NULL OR now() < effective_until)",
                &[&route_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::RevenueStreamDestinationRouteNotFound)?;
        parse_revenue_stream_destination_route_row(row)
    }

    /// Lists only selectable payout destinations for the authenticated
    /// merchant. The result is bounded and contains no inactive or
    /// unverified topology.
    pub async fn list_active_revenue_stream_destination_references(
        &self,
        merchant_id: Uuid,
        limit: usize,
    ) -> Result<Vec<RevenueStreamDestinationReference>, StorageError> {
        if merchant_id.get_version_num() != 7 || !(1..=64).contains(&limit) {
            return Err(StorageError::InvalidConfiguration(
                "revenue destination reference limit is invalid".to_string(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT id, route_key, version, recipient_owner, mint,
                        token_program, verification_slot::text
                 FROM revenue_stream_destination_routes
                 WHERE merchant_id = $1
                   AND status = 'active'
                   AND verification_commitment = 'finalized'
                   AND effective_from <= now()
                   AND (effective_until IS NULL OR now() < effective_until)
                 ORDER BY route_key, version DESC, id
                 LIMIT $2",
                &[&merchant_id, &(limit as i64)],
            )
            .await?;
        rows.into_iter()
            .map(|row| {
                let id: Uuid = row.get(0);
                if id.get_version_num() != 7 {
                    return Err(StorageError::InvalidRevenueStream(
                        "stored destination reference identity must be UUIDv7".into(),
                    ));
                }
                Ok(RevenueStreamDestinationReference {
                    id,
                    route_key: row.get(1),
                    version: row.get(2),
                    recipient_owner: row.get(3),
                    mint: row.get(4),
                    token_program: row.get(5),
                    verification_slot: parse_u64(row.get(6), "destination verification slot")?,
                })
            })
            .collect()
    }

    /// Lists only currently active routes for the authenticated merchant.
    /// This is a bounded read for UI selection and does not expose inactive
    /// configuration as a possible checkout option.
    pub async fn list_active_settlement_routes(
        &self,
        merchant_id: Uuid,
    ) -> Result<Vec<SettlementRoute>, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT id, route_key, version, merchant_account, mint,
                        tax_vault, net_destination, token_program, mint_decimals
                 FROM settlement_routes
                 WHERE merchant_id = $1
                   AND status = 'active'
                   AND verification_schema = 'settlement-route-verification/v2'
                   AND asset_kind = 1 AND asset_enabled = true
                   AND asset_config IS NOT NULL AND asset_config_bump IS NOT NULL
                   AND effective_from <= now()
                   AND (effective_until IS NULL OR now() < effective_until)
                 ORDER BY route_key, version DESC, id
                 LIMIT 64",
                &[&merchant_id],
            )
            .await?;
        rows.into_iter()
            .map(|row| {
                Ok(SettlementRoute {
                    id: row.get(0),
                    route_key: row.get(1),
                    version: row.get(2),
                    merchant_account: row.get(3),
                    mint: row.get(4),
                    mint_decimals: row.get::<_, i16>(8).try_into().map_err(|_| {
                        StorageError::SettlementRouteIntegrity(
                            "active route mint decimals are outside the supported range"
                                .to_string(),
                        )
                    })?,
                    tax_vault: row.get(5),
                    net_destination: row.get(6),
                    token_program: row.get(7),
                })
            })
            .collect::<Result<Vec<_>, StorageError>>()
    }

    /// Loads a server-owned, active Split Plan under the authenticated tenant
    /// boundary. The JSON document is revalidated and its canonical digest is
    /// recomputed before it can reach a compiler.
    pub async fn load_active_settlement_split_plan(
        &self,
        merchant_id: Uuid,
        plan_id: Uuid,
    ) -> Result<SettlementSplitPlan, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT plan_key, version, status, merchant_account, mint,
                        plan_digest, recipients,
                        effective_from <= now()
                            AND (effective_until IS NULL OR now() < effective_until)
                 FROM settlement_split_plans
                 WHERE id = $1 AND merchant_id = $2",
                &[&plan_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::SettlementSplitPlanNotFound)?;
        let status: String = row.get(2);
        let effective: bool = row.get(7);
        if status != "active" || !effective {
            return Err(StorageError::SettlementSplitPlanUnavailable);
        }
        let version: i32 = row.get(1);
        if version <= 0 {
            return Err(StorageError::SettlementSplitPlanIntegrity(
                "plan version must be positive".to_string(),
            ));
        }
        let plan_digest_bytes: Vec<u8> = row.get(5);
        let plan_digest: Digest32 = plan_digest_bytes.try_into().map_err(|bytes: Vec<u8>| {
            StorageError::SettlementSplitPlanIntegrity(format!(
                "plan digest must contain 32 bytes, got {}",
                bytes.len()
            ))
        })?;
        let recipients: Vec<split_plan::SplitRecipient> =
            serde_json::from_value(row.get::<_, Value>(6))?;
        validate_persisted_split_plan(
            plan_id,
            row.get(3),
            row.get(4),
            version,
            plan_digest,
            recipients,
            row.get(0),
        )
    }

    /// Loads an active split plan only when its canonical digest, merchant
    /// tenant, and mint bytes exactly match a server-owned revenue plan.
    pub async fn load_active_settlement_split_plan_by_digest(
        &self,
        merchant_id: Uuid,
        mint: [u8; 32],
        plan_digest: [u8; 32],
    ) -> Result<SettlementSplitPlan, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT id, plan_key, version, status, merchant_account, mint,
                        plan_digest, recipients,
                        effective_from <= now()
                            AND (effective_until IS NULL OR now() < effective_until)
                 FROM settlement_split_plans
                 WHERE merchant_id = $1
                   AND mint = $2
                   AND plan_digest = $3
                 ORDER BY version DESC, id
                 LIMIT 2",
                &[&merchant_id, &&mint[..], &&plan_digest[..]],
            )
            .await?;
        let row = match rows.as_slice() {
            [] => return Err(StorageError::SettlementSplitPlanNotFound),
            [row] => row,
            _ => {
                return Err(StorageError::SettlementSplitPlanIntegrity(
                    "multiple active split plans share the revenue plan digest".to_string(),
                ));
            }
        };
        let status: String = row.get(3);
        let effective: bool = row.get(8);
        if status != "active" || !effective {
            return Err(StorageError::SettlementSplitPlanUnavailable);
        }
        let version: i32 = row.get(2);
        if version <= 0 {
            return Err(StorageError::SettlementSplitPlanIntegrity(
                "plan version must be positive".to_string(),
            ));
        }
        let plan_digest_bytes: Vec<u8> = row.get(6);
        let stored_digest: Digest32 = plan_digest_bytes.try_into().map_err(|bytes: Vec<u8>| {
            StorageError::SettlementSplitPlanIntegrity(format!(
                "plan digest must contain 32 bytes, got {}",
                bytes.len()
            ))
        })?;
        let recipients: Vec<split_plan::SplitRecipient> =
            serde_json::from_value(row.get::<_, Value>(7))?;
        validate_persisted_split_plan(
            row.get(0),
            row.get(4),
            row.get(5),
            version,
            stored_digest,
            recipients,
            row.get(1),
        )
    }

    /// Lists only active, currently effective Split Plans for the authenticated
    /// merchant. Every row is canonicalized and digest-validated before it can
    /// reach a checkout review surface.
    pub async fn list_active_settlement_split_plans(
        &self,
        merchant_id: Uuid,
    ) -> Result<Vec<SettlementSplitPlan>, StorageError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT id, plan_key, version, merchant_account, mint,
                        plan_digest, recipients
                 FROM settlement_split_plans
                 WHERE merchant_id = $1
                   AND status = 'active'
                   AND effective_from <= now()
                   AND (effective_until IS NULL OR now() < effective_until)
                 ORDER BY plan_key, version DESC, id
                 LIMIT 64",
                &[&merchant_id],
            )
            .await?;
        rows.into_iter()
            .map(|row| {
                let plan_digest_bytes: Vec<u8> = row.get(5);
                let plan_digest: Digest32 =
                    plan_digest_bytes.try_into().map_err(|bytes: Vec<u8>| {
                        StorageError::SettlementSplitPlanIntegrity(format!(
                            "plan digest must contain 32 bytes, got {}",
                            bytes.len()
                        ))
                    })?;
                let recipients: Vec<split_plan::SplitRecipient> =
                    serde_json::from_value(row.get::<_, Value>(6))?;
                validate_persisted_split_plan(
                    row.get(0),
                    row.get(3),
                    row.get(4),
                    row.get(2),
                    plan_digest,
                    recipients,
                    row.get(1),
                )
            })
            .collect()
    }

    /// Loads one published, effective, canonical-validated policy by its
    /// server-issued reference. The policy document is never accepted from a
    /// revenue-plan browser request.
    pub async fn load_active_revenue_stream_policy_reference(
        &self,
        merchant_id: Uuid,
        policy_id: Uuid,
    ) -> Result<RevenueStreamPolicyReference, StorageError> {
        if merchant_id.get_version_num() != 7 || policy_id.get_version_num() != 7 {
            return Err(StorageError::JurisdictionPolicyNotFound);
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let row = client
            .query_opt(
                "SELECT jp.id, jp.rule_set_id, r.external_id, r.version,
                        r.jurisdiction, r.regime, jp.policy_digest,
                        jp.policy_document,
                        to_char(jp.effective_from AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                        to_char(jp.effective_until AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')
                 FROM jurisdiction_policies jp
                 JOIN rule_sets r
                   ON r.id = jp.rule_set_id
                  AND r.merchant_id = jp.merchant_id
                 WHERE jp.id = $1
                   AND jp.merchant_id = $2
                   AND jp.status = 'published'
                   AND jp.effective_from <= now()
                   AND (jp.effective_until IS NULL OR now() < jp.effective_until)
                   AND r.status = 'published'
                   AND r.effective_from <= now()
                   AND (r.effective_until IS NULL OR now() < r.effective_until)",
                &[&policy_id, &merchant_id],
            )
            .await?
            .ok_or(StorageError::JurisdictionPolicyNotFound)?;
        parse_revenue_stream_policy_reference(row)
    }

    /// Lists only published, effective, canonical-validated policies for the
    /// revenue-plan selector. Both the policy and its published rule release
    /// must be current; the result is deliberately bounded.
    pub async fn list_active_revenue_stream_policy_references(
        &self,
        merchant_id: Uuid,
        limit: usize,
    ) -> Result<Vec<RevenueStreamPolicyReference>, StorageError> {
        if merchant_id.get_version_num() != 7 || !(1..=64).contains(&limit) {
            return Err(StorageError::InvalidConfiguration(
                "revenue policy reference limit is invalid".to_string(),
            ));
        }
        let client = self
            .pool
            .get()
            .await
            .map_err(|error| StorageError::Pool(error.to_string()))?;
        let rows = client
            .query(
                "SELECT jp.id, jp.rule_set_id, r.external_id, r.version,
                        r.jurisdiction, r.regime, jp.policy_digest,
                        jp.policy_document,
                        to_char(jp.effective_from AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                        to_char(jp.effective_until AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"')
                 FROM jurisdiction_policies jp
                 JOIN rule_sets r
                   ON r.id = jp.rule_set_id
                  AND r.merchant_id = jp.merchant_id
                 WHERE jp.merchant_id = $1
                   AND jp.status = 'published'
                   AND jp.effective_from <= now()
                   AND (jp.effective_until IS NULL OR now() < jp.effective_until)
                   AND r.status = 'published'
                   AND r.effective_from <= now()
                   AND (r.effective_until IS NULL OR now() < r.effective_until)
                 ORDER BY r.jurisdiction, r.regime, r.version DESC, jp.id
                 LIMIT $2",
                &[&merchant_id, &(limit as i64)],
            )
            .await?;
        rows.into_iter()
            .map(parse_revenue_stream_policy_reference)
            .collect()
    }
}

fn validate_evidence_access_audit_limit(limit: usize) -> Result<(), StorageError> {
    if !(1..=MAX_EVIDENCE_ACCESS_AUDIT_LIST_LIMIT).contains(&limit) {
        return Err(StorageError::InvalidConfiguration(format!(
            "evidence access audit list limit must be between 1 and {MAX_EVIDENCE_ACCESS_AUDIT_LIST_LIMIT}"
        )));
    }
    Ok(())
}

async fn persist_revenue_stream_accrual_jobs(
    transaction: &deadpool_postgres::tokio_postgres::Transaction<'_>,
    stream_key: &str,
    page: &FinalizedPage,
) -> Result<(), StorageError> {
    for split in &page.split_settlements {
        transaction
            .execute(
                "INSERT INTO revenue_stream_accrual_reconciliation_jobs
                    (stream_key, source_signature, evidence_digest, observed_finalized_slot)
                 VALUES ($1, $2, $3, $4::text::numeric)
                 ON CONFLICT (stream_key, source_signature) DO NOTHING",
                &[
                    &stream_key,
                    &split.signature,
                    &&split.evidence_digest[..],
                    &page.observed_finalized_slot.to_string(),
                ],
            )
            .await?;
    }
    Ok(())
}

fn validate_persisted_split_plan(
    id: Uuid,
    merchant_account: String,
    mint: String,
    version: i32,
    plan_digest: Digest32,
    recipients: Vec<split_plan::SplitRecipient>,
    plan_key: String,
) -> Result<SettlementSplitPlan, StorageError> {
    let plan = split_plan::SplitPlan::new(
        merchant_account.clone(),
        mint.clone(),
        u64::try_from(version).map_err(|_| {
            StorageError::SettlementSplitPlanIntegrity(
                "plan version cannot be represented as u64".to_string(),
            )
        })?,
        recipients.clone(),
    )
    .map_err(|error| StorageError::SettlementSplitPlanIntegrity(error.to_string()))?;
    if plan
        .digest()
        .map_err(|error| StorageError::SettlementSplitPlanIntegrity(error.to_string()))?
        != plan_digest
    {
        return Err(StorageError::SettlementSplitPlanIntegrity(
            "plan digest does not match canonical recipients".to_string(),
        ));
    }
    Ok(SettlementSplitPlan {
        id,
        plan_key,
        version,
        merchant_account,
        mint,
        plan_digest,
        recipients,
    })
}

fn outbox_payload(merchant_id: Uuid, quote: Value) -> Value {
    serde_json::json!({
        "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
        "merchant_id": merchant_id,
        "quote": quote,
    })
}

fn verify_fx_evidence_projection(
    merchant_id: Uuid,
    pre_resolution_facts_digest: Digest32,
    conversion: &tax_engine::FxConversionEvidence,
    evidence: &PersistedFxEvidence,
) -> Result<(), StorageError> {
    if evidence.pre_resolution_facts_digest != pre_resolution_facts_digest {
        return Err(StorageError::FxEvidenceIntegrity(
            "pre-resolution facts digest does not match the signed envelope".to_string(),
        ));
    }
    let expected_key =
        ed25519_dalek::VerifyingKey::from_bytes(&evidence.envelope.provider_public_key).map_err(
            |_| StorageError::FxEvidenceIntegrity("signed FX provider key is invalid".to_string()),
        )?;
    let signed_conversion = evidence
        .envelope
        .verify_integrity(
            &expected_key,
            merchant_identity_digest(merchant_id.as_bytes()),
            pre_resolution_facts_digest,
        )
        .map_err(|error| StorageError::FxEvidenceIntegrity(error.to_string()))?;
    if signed_conversion != *conversion {
        return Err(StorageError::FxEvidenceIntegrity(
            "signed FX projection does not match normalized facts".to_string(),
        ));
    }
    Ok(())
}

fn verify_persisted_fx_evidence(
    merchant_id: Uuid,
    pre_resolution_facts: &TaxFacts,
    persisted_facts: &TaxFacts,
    evidence: &PersistedFxEvidence,
) -> Result<(), StorageError> {
    if pre_resolution_facts.jurisdiction_resolution.is_some()
        || pre_resolution_facts.fx_conversion.is_some()
    {
        return Err(StorageError::FxEvidenceIntegrity(
            "pre-resolution facts must not contain server evidence".to_string(),
        ));
    }
    let pre_resolution_facts_digest = pre_resolution_facts.canonical_digest()?;
    let conversion = persisted_facts.fx_conversion.as_ref().ok_or_else(|| {
        StorageError::FxEvidenceIntegrity(
            "FX evidence exists without an FX conversion projection".to_string(),
        )
    })?;
    verify_fx_evidence_projection(
        merchant_id,
        pre_resolution_facts_digest,
        conversion,
        evidence,
    )
}

fn verify_persisted_document_fx_evidence(
    merchant_id: Uuid,
    pre_resolution_facts: &TaxDocumentFacts,
    persisted_facts: &TaxDocumentFacts,
    evidence: &PersistedFxEvidence,
) -> Result<(), StorageError> {
    if pre_resolution_facts.jurisdiction_resolution.is_some()
        || pre_resolution_facts.fx_conversion.is_some()
    {
        return Err(StorageError::FxEvidenceIntegrity(
            "pre-resolution document facts must not contain server evidence".to_string(),
        ));
    }
    let pre_resolution_facts_digest = pre_resolution_facts.canonical_digest()?;
    let conversion = persisted_facts.fx_conversion.as_ref().ok_or_else(|| {
        StorageError::FxEvidenceIntegrity(
            "FX evidence exists without an FX conversion projection".to_string(),
        )
    })?;
    verify_fx_evidence_projection(
        merchant_id,
        pre_resolution_facts_digest,
        conversion,
        evidence,
    )
}

fn parse_fx_evidence_ref(value: &Value) -> Result<PersistedFxEvidence, StorageError> {
    if !value.is_object() || serde_json::to_vec(value)?.len() > MAX_FX_EVIDENCE_DOCUMENT_BYTES {
        return Err(StorageError::FxEvidenceIntegrity(
            "persisted FX evidence reference is outside the bounded object shape".to_string(),
        ));
    }
    serde_json::from_value(value.clone()).map_err(|error| {
        StorageError::FxEvidenceIntegrity(format!("persisted FX evidence is invalid: {error}"))
    })
}

fn verify_replayed_fx_evidence(
    merchant_id: Uuid,
    persisted_facts: &TaxFacts,
    evidence_refs: &Value,
) -> Result<(), StorageError> {
    let evidence_value = evidence_refs.get("fx_evidence");
    match (persisted_facts.fx_conversion.is_some(), evidence_value) {
        (false, None) => Ok(()),
        (false, Some(_)) => Err(StorageError::FxEvidenceIntegrity(
            "persisted FX envelope exists without an FX projection".to_string(),
        )),
        (true, None) => Err(StorageError::FxEvidenceIntegrity(
            "persisted FX projection is missing its signed envelope".to_string(),
        )),
        (true, Some(value)) => {
            let evidence = parse_fx_evidence_ref(value)?;
            let mut pre_resolution_facts = persisted_facts.clone();
            pre_resolution_facts.jurisdiction_resolution = None;
            pre_resolution_facts.fx_conversion = None;
            verify_persisted_fx_evidence(
                merchant_id,
                &pre_resolution_facts,
                persisted_facts,
                &evidence,
            )
        }
    }
}

fn verify_replayed_document_fx_evidence(
    merchant_id: Uuid,
    persisted_facts: &TaxDocumentFacts,
    evidence_refs: &Value,
) -> Result<(), StorageError> {
    let evidence_value = evidence_refs.get("fx_evidence");
    match (persisted_facts.fx_conversion.is_some(), evidence_value) {
        (false, None) => Ok(()),
        (false, Some(_)) => Err(StorageError::DocumentReplayEvidenceIntegrity(
            "persisted FX envelope exists without a document FX projection".to_string(),
        )),
        (true, None) => Err(StorageError::DocumentReplayEvidenceIntegrity(
            "persisted document FX projection is missing its signed envelope".to_string(),
        )),
        (true, Some(value)) => {
            let evidence = parse_fx_evidence_ref(value).map_err(|error| {
                StorageError::DocumentReplayEvidenceIntegrity(error.to_string())
            })?;
            let mut pre_resolution_facts = persisted_facts.clone();
            pre_resolution_facts.jurisdiction_resolution = None;
            pre_resolution_facts.fx_conversion = None;
            verify_persisted_document_fx_evidence(
                merchant_id,
                &pre_resolution_facts,
                persisted_facts,
                &evidence,
            )
            .map_err(|error| StorageError::DocumentReplayEvidenceIntegrity(error.to_string()))
        }
    }
}

fn build_fx_evidence_ref(
    merchant_id: Uuid,
    pre_resolution_facts: &TaxFacts,
    persisted_facts: &TaxFacts,
    evidence: Option<&PersistedFxEvidence>,
) -> Result<Option<Value>, StorageError> {
    match (persisted_facts.fx_conversion.as_ref(), evidence) {
        (None, None) => Ok(None),
        (None, Some(_)) => Err(StorageError::FxEvidenceIntegrity(
            "FX evidence exists without an FX conversion projection".to_string(),
        )),
        (Some(_), None) => Err(StorageError::FxEvidenceIntegrity(
            "FX conversion projection is missing its signed envelope".to_string(),
        )),
        (Some(_), Some(evidence)) => {
            verify_persisted_fx_evidence(
                merchant_id,
                pre_resolution_facts,
                persisted_facts,
                evidence,
            )?;
            let document = serde_json::to_value(evidence)?;
            if serde_json::to_vec(&document)?.len() > MAX_FX_EVIDENCE_DOCUMENT_BYTES {
                return Err(StorageError::FxEvidenceIntegrity(
                    "signed FX evidence exceeds the bounded persistence limit".to_string(),
                ));
            }
            Ok(Some(document))
        }
    }
}

fn build_document_fx_evidence_ref(
    merchant_id: Uuid,
    pre_resolution_facts: &TaxDocumentFacts,
    persisted_facts: &TaxDocumentFacts,
    evidence: Option<&PersistedFxEvidence>,
) -> Result<Option<Value>, StorageError> {
    match (persisted_facts.fx_conversion.as_ref(), evidence) {
        (None, None) => Ok(None),
        (None, Some(_)) => Err(StorageError::FxEvidenceIntegrity(
            "FX evidence exists without a document FX conversion projection".to_string(),
        )),
        (Some(_), None) => Err(StorageError::FxEvidenceIntegrity(
            "document FX conversion projection is missing its signed envelope".to_string(),
        )),
        (Some(_), Some(evidence)) => {
            verify_persisted_document_fx_evidence(
                merchant_id,
                pre_resolution_facts,
                persisted_facts,
                evidence,
            )?;
            let document = serde_json::to_value(evidence)?;
            if serde_json::to_vec(&document)?.len() > MAX_FX_EVIDENCE_DOCUMENT_BYTES {
                return Err(StorageError::FxEvidenceIntegrity(
                    "signed FX evidence exceeds the bounded persistence limit".to_string(),
                ));
            }
            Ok(Some(document))
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct QuoteApplicabilityBinding {
    context_id: Uuid,
    attestation_digest: Digest32,
}

struct CalculationInsertOptions<'a> {
    engine_version: &'a str,
    evidence_cipher: Option<&'a EvidenceCipher>,
    applicability_binding: Option<&'a QuoteApplicabilityBinding>,
}

async fn load_quote_applicability_binding(
    transaction: &deadpool_postgres::Transaction<'_>,
    context_id: Option<Uuid>,
) -> Result<Option<QuoteApplicabilityBinding>, StorageError> {
    let Some(context_id) = context_id else {
        return Ok(None);
    };
    let row = transaction
        .query_opt(
            "SELECT attestation_digest
             FROM tax_applicability_contexts
             WHERE id = $1
               AND status = 'published'
               AND effective_from <= now()
               AND (effective_until IS NULL OR effective_until > now())
               AND evidence_expires_at > now()",
            &[&context_id],
        )
        .await?
        .ok_or(StorageError::ApplicabilityContextNotFound)?;
    let attestation_digest = parse_digest(row.get(0))?;
    Ok(Some(QuoteApplicabilityBinding {
        context_id,
        attestation_digest,
    }))
}

async fn insert_calculation(
    transaction: &deadpool_postgres::Transaction<'_>,
    command: &QuoteCommand,
    persisted_facts: &TaxFacts,
    database_rule_id: Uuid,
    result: &CalculationResult,
    options: &CalculationInsertOptions<'_>,
) -> Result<(Uuid, i64), StorageError> {
    let total_tax = result.total_tax_atomic.to_string();
    let total_net = result.total_net_atomic.to_string();
    let result_document = serde_json::to_value(result)?;
    let evidence_cipher = options
        .evidence_cipher
        .ok_or(StorageError::EvidenceCipherUnavailable)?;
    let fx_evidence = build_fx_evidence_ref(
        command.merchant_id,
        &command.facts,
        persisted_facts,
        command.fx_evidence.as_ref(),
    )?;
    let encrypted_facts = evidence_cipher.encrypt_facts(
        command.merchant_id,
        &result.facts_digest,
        persisted_facts,
    )?;
    let mut evidence_refs = serde_json::json!({
        "facts_digest": result.facts_digest,
        "facts_ciphertext": "tax_calculations.facts_ciphertext",
        "facts_key_id": encrypted_facts.key_id,
        "rule_digest": result.rule_digest,
        "engine_version": options.engine_version,
    });
    if let Some(binding) = options.applicability_binding
        && let Some(object) = evidence_refs.as_object_mut()
    {
        object.insert(
            "applicability_context_id".to_string(),
            serde_json::json!(binding.context_id.to_string()),
        );
        object.insert(
            "applicability_attestation_digest".to_string(),
            serde_json::json!(binding.attestation_digest),
        );
    }
    if let Some(fx_evidence) = fx_evidence
        && let Some(object) = evidence_refs.as_object_mut()
    {
        object.insert("fx_evidence".to_string(), fx_evidence);
    }
    let row = transaction
        .query_one(
            "INSERT INTO tax_calculations
                (merchant_id, rule_set_id, facts_digest, engine_version, currency,
                jurisdiction, total_tax_atomic, total_net_atomic, status,
                result_document, facts_document, facts_ciphertext, facts_nonce,
                facts_key_id, evidence_refs, expires_at, applicability_context_id,
                applicability_attestation_digest)
             VALUES ($1, $2, $3, $4, $5, $6, $7::text::numeric, $8::text::numeric,
                     $9, $10, NULL, $11, $12, $13, $14,
                     now() + ($15::bigint * interval '1 second'), $16, $17)
             RETURNING id, floor(extract(epoch FROM expires_at))::bigint",
            &[
                &command.merchant_id,
                &database_rule_id,
                &&result.facts_digest[..],
                &options.engine_version,
                &command.facts.currency,
                &command.facts.jurisdiction,
                &total_tax,
                &total_net,
                &CALCULATION_STATUS,
                &result_document,
                &&encrypted_facts.ciphertext,
                &&encrypted_facts.nonce[..],
                &encrypted_facts.key_id,
                &evidence_refs,
                &(DEFAULT_QUOTE_TTL.as_secs() as i64),
                &options
                    .applicability_binding
                    .map(|binding| binding.context_id),
                &options
                    .applicability_binding
                    .map(|binding| binding.attestation_digest.to_vec()),
            ],
        )
        .await?;
    Ok((row.get(0), row.get(1)))
}

async fn insert_document_calculation(
    transaction: &deadpool_postgres::Transaction<'_>,
    command: &DocumentQuoteCommand,
    persisted_facts: &TaxDocumentFacts,
    database_rule_id: Uuid,
    result: &TaxDocumentResult,
    options: &CalculationInsertOptions<'_>,
) -> Result<(Uuid, i64), StorageError> {
    let gross = result.gross_atomic.to_string();
    let total_tax = result.total_tax_atomic.to_string();
    let total_net = result.total_net_atomic.to_string();
    let result_document = serde_json::to_value(result)?;
    let evidence_cipher = options
        .evidence_cipher
        .ok_or(StorageError::EvidenceCipherUnavailable)?;
    let fx_evidence = build_document_fx_evidence_ref(
        command.merchant_id,
        &command.facts,
        persisted_facts,
        command.fx_evidence.as_ref(),
    )?;
    let encrypted_facts = evidence_cipher.encrypt_document_facts(
        command.merchant_id,
        &result.facts_digest,
        persisted_facts,
    )?;
    let mut evidence_refs = serde_json::json!({
        "schema": command.schema,
        "facts_digest": result.facts_digest,
        "facts_ciphertext": "tax_document_calculations.facts_ciphertext",
        "facts_key_id": encrypted_facts.key_id,
        "rule_digest": result.rule_digest,
        "engine_version": options.engine_version,
    });
    if let Some(binding) = options.applicability_binding
        && let Some(object) = evidence_refs.as_object_mut()
    {
        object.insert(
            "applicability_context_id".to_string(),
            serde_json::json!(binding.context_id.to_string()),
        );
        object.insert(
            "applicability_attestation_digest".to_string(),
            serde_json::json!(binding.attestation_digest),
        );
    }
    if let Some(fx_evidence) = fx_evidence
        && let Some(object) = evidence_refs.as_object_mut()
    {
        object.insert("fx_evidence".to_string(), fx_evidence);
    }
    let row = transaction
        .query_one(
            "INSERT INTO tax_document_calculations
                (merchant_id, rule_set_id, document_schema, document_id,
                 facts_digest, rule_digest, engine_version, currency, jurisdiction,
                 gross_atomic, total_tax_atomic, total_net_atomic, status,
                 result_document, facts_ciphertext, facts_nonce, facts_key_id,
                 evidence_refs, expires_at, applicability_context_id,
                 applicability_attestation_digest)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9,
                     $10::text::numeric, $11::text::numeric, $12::text::numeric,
                     $13, $14, $15, $16, $17, $18,
                     now() + ($19::bigint * interval '1 second'), $20, $21)
             RETURNING id, floor(extract(epoch FROM expires_at))::bigint",
            &[
                &command.merchant_id,
                &database_rule_id,
                &command.schema,
                &result.document_id,
                &&result.facts_digest[..],
                &&result.rule_digest[..],
                &options.engine_version,
                &result.currency,
                &result.jurisdiction,
                &gross,
                &total_tax,
                &total_net,
                &CALCULATION_STATUS,
                &result_document,
                &&encrypted_facts.ciphertext,
                &&encrypted_facts.nonce[..],
                &encrypted_facts.key_id,
                &evidence_refs,
                &(DEFAULT_QUOTE_TTL.as_secs() as i64),
                &options
                    .applicability_binding
                    .map(|binding| binding.context_id),
                &options
                    .applicability_binding
                    .map(|binding| binding.attestation_digest.to_vec()),
            ],
        )
        .await?;
    Ok((row.get(0), row.get(1)))
}

fn decode_persisted_rule_document(document: Value) -> Result<RuleSet, StorageError> {
    if document.get("schema").is_some() {
        let envelope: RuleSetDocument = serde_json::from_value(document)?;
        envelope.validate()?;
        return Ok(envelope.rule_set);
    }

    // Existing rows are immutable evidence. This branch is deliberately
    // read-only compatibility for historical bare RuleSet documents; all new
    // publication writes use RuleSetDocument above.
    Ok(serde_json::from_value(document)?)
}

fn validate_persisted_rule_for_simulation(
    external_id: String,
    version: i32,
    jurisdiction: String,
    status: String,
    document: Value,
    stored_digest: &[u8],
    facts: &TaxFacts,
) -> Result<RuleSet, StorageError> {
    if status == "retired" {
        return Err(StorageError::RuleSimulationUnavailable);
    }
    if facts.jurisdiction != jurisdiction {
        return Err(StorageError::RuleSimulationJurisdictionMismatch);
    }
    let rule_set = decode_persisted_rule_document(document).map_err(|error| {
        StorageError::RuleSimulationIntegrity(format!("rule document is invalid: {error}"))
    })?;
    let persisted_version = u32::try_from(version).map_err(|_| {
        StorageError::RuleSimulationIntegrity("rule version is invalid".to_string())
    })?;
    if rule_set.id != external_id || rule_set.version != persisted_version {
        return Err(StorageError::RuleSimulationIntegrity(
            "rule document identity does not match its persisted selector".to_string(),
        ));
    }
    let computed_digest = rule_set.canonical_digest().map_err(|error| {
        StorageError::RuleSimulationIntegrity(format!("rule digest cannot be computed: {error}"))
    })?;
    if stored_digest != computed_digest {
        return Err(StorageError::RuleSimulationIntegrity(
            "rule digest does not match the canonical persisted document".to_string(),
        ));
    }
    Ok(rule_set)
}

#[allow(clippy::too_many_arguments)]
fn verify_persisted_calculation_evidence(
    merchant_id: Uuid,
    calculation_id: Uuid,
    status: String,
    stored_engine_version: String,
    stored_facts_digest: Vec<u8>,
    result_document: Value,
    facts_document: Option<Value>,
    evidence_refs: Value,
    rule_document: Value,
    stored_rule_digest: Vec<u8>,
    expected_engine_version: &str,
) -> Result<QuoteReplayVerification, StorageError> {
    let facts_document = facts_document.ok_or(StorageError::ReplayEvidenceUnavailable)?;
    let facts: TaxFacts = serde_json::from_value(facts_document)
        .map_err(|error| StorageError::ReplayEvidenceIntegrity(error.to_string()))?;
    verify_replayed_fx_evidence(merchant_id, &facts, &evidence_refs)
        .map_err(|error| StorageError::ReplayEvidenceIntegrity(error.to_string()))?;
    let rule_set = decode_persisted_rule_document(rule_document)
        .map_err(|error| StorageError::ReplayEvidenceIntegrity(error.to_string()))?;
    let result: CalculationResult = serde_json::from_value(result_document)
        .map_err(|error| StorageError::ReplayEvidenceIntegrity(error.to_string()))?;
    let facts_digest: Digest32 = stored_facts_digest.try_into().map_err(|_| {
        StorageError::ReplayEvidenceIntegrity("facts digest is not 32 bytes".to_string())
    })?;
    let rule_digest: Digest32 = stored_rule_digest.try_into().map_err(|_| {
        StorageError::ReplayEvidenceIntegrity("rule digest is not 32 bytes".to_string())
    })?;

    if stored_engine_version != expected_engine_version {
        return Err(StorageError::ReplayEvidenceIntegrity(
            "stored engine version does not match the running service".to_string(),
        ));
    }
    if result.engine_version != stored_engine_version {
        return Err(StorageError::ReplayEvidenceIntegrity(
            "result engine version does not match its evidence column".to_string(),
        ));
    }
    if facts.canonical_digest()? != facts_digest {
        return Err(StorageError::ReplayEvidenceIntegrity(
            "facts document does not match its digest".to_string(),
        ));
    }
    if rule_set.canonical_digest()? != rule_digest {
        return Err(StorageError::ReplayEvidenceIntegrity(
            "rule document does not match its digest".to_string(),
        ));
    }
    if result.facts_digest != facts_digest || result.rule_digest != rule_digest {
        return Err(StorageError::ReplayEvidenceIntegrity(
            "result digests do not match persisted evidence".to_string(),
        ));
    }
    if result.rule_set_id != rule_set.id || result.rule_version != rule_set.version {
        return Err(StorageError::ReplayEvidenceIntegrity(
            "result rule identity does not match the published rule release".to_string(),
        ));
    }
    result
        .verify_replay(&rule_set, &facts)
        .map_err(|error| StorageError::ReplayEvidenceIntegrity(error.to_string()))?;

    Ok(QuoteReplayVerification {
        calculation_id,
        status,
        engine_version: stored_engine_version,
        facts_digest,
        rule_digest,
    })
}

pub fn quote_request_digest(command: &QuoteCommand) -> Result<[u8; 32], StorageError> {
    let facts_digest = quote_request_facts_digest(&command.facts, command.fx_evidence.as_ref())?;
    let candidate_digest =
        candidate_set_digest(&command.jurisdiction_candidates).map_err(|error| {
            StorageError::InvalidConfiguration(format!("jurisdiction evidence is invalid: {error}"))
        })?;
    let mut hasher = Sha256::new();
    hasher.update(b"solana-tax-protocol/quote-request/v1");
    write_text(&mut hasher, &command.rule_set_id)?;
    hasher.update(command.rule_version.to_le_bytes());
    write_text(&mut hasher, &command.jurisdiction)?;
    write_text(&mut hasher, &command.regime)?;
    hasher.update(facts_digest);
    hasher.update(candidate_digest);
    if let Some(context_id) = command.applicability_context_id {
        hasher.update(b"solana-tax-protocol/quote-applicability-context/v1");
        hasher.update(context_id.as_bytes());
    }
    if let Some(evidence) = command.fx_evidence.as_ref() {
        hasher.update(b"solana-tax-protocol/quote-fx-source/v1");
        write_text(&mut hasher, &evidence.envelope.unsigned.source_currency)?;
    }
    Ok(hasher.finalize().into())
}

pub fn document_quote_request_digest(
    command: &DocumentQuoteCommand,
) -> Result<Digest32, StorageError> {
    let facts_digest =
        document_quote_request_facts_digest(&command.facts, command.fx_evidence.as_ref())?;
    let candidate_digest =
        candidate_set_digest(&command.jurisdiction_candidates).map_err(|error| {
            StorageError::InvalidDocumentQuote(format!("jurisdiction evidence is invalid: {error}"))
        })?;
    let mut hasher = Sha256::new();
    hasher.update(b"solana-tax-protocol/document-quote-request/v1");
    write_text(&mut hasher, &command.schema)?;
    write_text(&mut hasher, &command.rule_set_id)?;
    hasher.update(command.rule_version.to_le_bytes());
    write_text(&mut hasher, &command.jurisdiction)?;
    write_text(&mut hasher, &command.regime)?;
    hasher.update(facts_digest);
    hasher.update(candidate_digest);
    if let Some(context_id) = command.applicability_context_id {
        hasher.update(b"solana-tax-protocol/document-quote-applicability-context/v1");
        hasher.update(context_id.as_bytes());
    }
    if let Some(evidence) = command.fx_evidence.as_ref() {
        hasher.update(b"solana-tax-protocol/document-quote-fx-source/v1");
        write_text(&mut hasher, &evidence.envelope.unsigned.source_currency)?;
    }
    Ok(hasher.finalize().into())
}

fn quote_request_facts_digest(
    facts: &TaxFacts,
    evidence: Option<&PersistedFxEvidence>,
) -> Result<Digest32, StorageError> {
    let Some(evidence) = evidence else {
        return Ok(facts.canonical_digest()?);
    };
    let mut pre_resolution_facts = facts.clone();
    pre_resolution_facts.jurisdiction_resolution = None;
    pre_resolution_facts.fx_conversion = None;
    let computed = pre_resolution_facts.canonical_digest()?;
    if computed != evidence.pre_resolution_facts_digest {
        return Err(StorageError::FxEvidenceIntegrity(
            "FX request digest is not bound to the pre-resolution facts".to_string(),
        ));
    }
    Ok(computed)
}

fn document_quote_request_facts_digest(
    facts: &TaxDocumentFacts,
    evidence: Option<&PersistedFxEvidence>,
) -> Result<Digest32, StorageError> {
    let Some(evidence) = evidence else {
        return Ok(facts.canonical_digest()?);
    };
    let mut pre_resolution_facts = facts.clone();
    pre_resolution_facts.jurisdiction_resolution = None;
    pre_resolution_facts.fx_conversion = None;
    let computed = pre_resolution_facts.canonical_digest()?;
    if computed != evidence.pre_resolution_facts_digest {
        return Err(StorageError::FxEvidenceIntegrity(
            "FX document request digest is not bound to the pre-resolution facts".to_string(),
        ));
    }
    Ok(computed)
}

fn validate_document_quote_command(command: &DocumentQuoteCommand) -> Result<(), StorageError> {
    if command
        .applicability_context_id
        .is_some_and(|context_id| context_id.is_nil())
    {
        return Err(StorageError::InvalidDocumentQuote(
            "applicability context reference must not be nil".to_string(),
        ));
    }
    if command.merchant_id.is_nil()
        || command.idempotency_key.trim().is_empty()
        || command.idempotency_key.len() > 200
        || command.schema != TAX_DOCUMENT_SCHEMA_V1
        || command.rule_set_id.trim().is_empty()
        || command.rule_version == 0
        || command.jurisdiction.trim().is_empty()
        || command.regime.trim().is_empty()
        || command.facts.jurisdiction != command.jurisdiction
    {
        return Err(StorageError::InvalidDocumentQuote(
            "merchant, idempotency, schema, selector, or jurisdiction fields are invalid"
                .to_string(),
        ));
    }
    command
        .facts
        .validate()
        .map_err(|error| StorageError::InvalidDocumentQuote(error.to_string()))?;
    candidate_set_digest(&command.jurisdiction_candidates).map_err(|error| {
        StorageError::InvalidDocumentQuote(format!("jurisdiction evidence is invalid: {error}"))
    })?;
    i32::try_from(command.rule_version).map_err(|_| {
        StorageError::InvalidDocumentQuote("rule version exceeds the PostgreSQL range".to_string())
    })?;
    Ok(())
}

pub fn validate_create_applicability_context_command(
    command: &CreateApplicabilityContextCommand,
) -> Result<(), StorageError> {
    if command.merchant_id.is_nil()
        || command.idempotency_key.trim().is_empty()
        || command.idempotency_key.len() > 256
        || command
            .idempotency_key
            .bytes()
            .any(|byte| byte.is_ascii_control())
        || command.actor_subject.trim().is_empty()
        || command.actor_subject.len() > 256
        || command
            .actor_subject
            .bytes()
            .any(|byte| byte.is_ascii_control())
        || command.effective_from_unix <= 0
        || command
            .effective_until_unix
            .is_some_and(|until| until <= command.effective_from_unix)
        || serde_json::to_vec(&command.provenance)
            .map(|bytes| bytes.len() > MAX_APPLICABILITY_CONTEXT_PROVENANCE_BYTES)
            .unwrap_or(true)
    {
        return Err(StorageError::InvalidApplicabilityContext(
            "context identity, lifecycle window, actor, or provenance is invalid".into(),
        ));
    }
    command
        .attestation
        .verify()
        .map_err(|error| StorageError::InvalidApplicabilityContext(error.to_string()))?;
    let unsigned = &command.attestation.unsigned;
    if unsigned.merchant_id != merchant_identity_digest(command.merchant_id.as_bytes()) {
        return Err(StorageError::InvalidApplicabilityContext(
            "attestation merchant identity does not match the tenant".into(),
        ));
    }
    if command.effective_from_unix < unsigned.issued_at_unix
        || command
            .effective_until_unix
            .is_some_and(|until| until > unsigned.expires_at_unix)
    {
        return Err(StorageError::InvalidApplicabilityContext(
            "context effective window exceeds signed attestation validity".into(),
        ));
    }
    Ok(())
}

pub fn applicability_context_request_digest(
    command: &CreateApplicabilityContextCommand,
) -> Result<Digest32, StorageError> {
    validate_create_applicability_context_command(command)?;
    let canonical = serde_json::to_vec(&serde_json::json!({
        "schema": "tax-applicability-context/v1",
        "merchant_id": command.merchant_id,
        "idempotency_key": command.idempotency_key,
        "attestation": command.attestation,
        "effective_from_unix": command.effective_from_unix,
        "effective_until_unix": command.effective_until_unix,
        "provenance": command.provenance,
        "actor_subject": command.actor_subject,
    }))?;
    Ok(Sha256::digest(canonical).into())
}

pub fn validate_applicability_context_transition(
    transition: &ApplicabilityContextTransition,
) -> Result<(), StorageError> {
    if transition.context_id.is_nil()
        || transition.merchant_id.is_nil()
        || transition.actor_subject.trim().is_empty()
        || transition.actor_subject.len() > 256
        || transition
            .actor_subject
            .bytes()
            .any(|byte| byte.is_ascii_control())
        || serde_json::to_vec(&transition.evidence)
            .map(|bytes| bytes.len() > MAX_APPLICABILITY_CONTEXT_PROVENANCE_BYTES)
            .unwrap_or(true)
    {
        return Err(StorageError::InvalidApplicabilityContext(
            "transition identity, actor, or evidence is invalid".into(),
        ));
    }
    let valid = matches!(
        (
            transition.expected_status.as_str(),
            transition.next_status.as_str()
        ),
        ("draft", "review")
            | ("draft", "rejected")
            | ("review", "approved")
            | ("review", "rejected")
            | ("approved", "published")
            | ("approved", "rejected")
            | ("published", "retired")
    );
    if !valid {
        return Err(StorageError::ApplicabilityContextTransitionConflict);
    }
    Ok(())
}

fn validate_settlement_adjustment_command(
    command: &CreateSettlementAdjustmentCommand,
) -> Result<(), StorageError> {
    if command.merchant_id.is_nil()
        || command.proposal.merchant_id != command.merchant_id
        || command.idempotency_key.trim().is_empty()
        || command.idempotency_key.len() > 200
        || command.actor_subject.trim().is_empty()
        || command.actor_subject.len() > 256
        || command
            .actor_subject
            .bytes()
            .any(|byte| byte.is_ascii_control())
        || command.proposal.proposed_by.trim() != command.actor_subject.trim()
    {
        return Err(StorageError::InvalidSettlementAdjustment(
            "merchant, idempotency, or authenticated proposer identity is invalid".to_string(),
        ));
    }
    Ok(())
}

fn validate_settlement_adjustment_transition_command(
    command: &SettlementAdjustmentTransition,
) -> Result<(), StorageError> {
    if command.merchant_id.is_nil()
        || command.adjustment_id.is_nil()
        || command.actor_subject.trim().is_empty()
        || command.actor_subject.len() > 256
        || command
            .actor_subject
            .bytes()
            .any(|byte| byte.is_ascii_control())
        || !matches!(
            command.to_status.as_str(),
            "approved" | "rejected" | "voided"
        )
        || !command.evidence.is_object()
        || serde_json::to_vec(&command.evidence)
            .map(|bytes| bytes.len() > MAX_VAULT_RELEASE_EVIDENCE_BYTES)
            .unwrap_or(true)
    {
        return Err(StorageError::InvalidSettlementAdjustment(
            "transition identity, status, actor, or evidence is invalid".to_string(),
        ));
    }
    Ok(())
}

fn settlement_adjustment_request_digest(
    command: &CreateSettlementAdjustmentCommand,
    adjustment_digest: Digest32,
) -> Result<Digest32, StorageError> {
    let mut hasher = Sha256::new();
    hasher.update(b"solana-tax-protocol/settlement-adjustment-request/v1");
    hasher.update(command.merchant_id.as_bytes());
    write_text(&mut hasher, &command.idempotency_key)?;
    write_text(&mut hasher, &command.actor_subject)?;
    hasher.update(adjustment_digest);
    Ok(hasher.finalize().into())
}

fn adjustment_kind_to_db(kind: AdjustmentKind) -> &'static str {
    match kind {
        AdjustmentKind::Refund => "refund",
        AdjustmentKind::Reversal => "reversal",
        AdjustmentKind::Correction => "correction",
        AdjustmentKind::Chargeback => "chargeback",
        AdjustmentKind::RemittanceAdjustment => "remittance_adjustment",
    }
}

fn adjustment_kind_from_db(value: &str) -> Result<AdjustmentKind, StorageError> {
    match value {
        "refund" => Ok(AdjustmentKind::Refund),
        "reversal" => Ok(AdjustmentKind::Reversal),
        "correction" => Ok(AdjustmentKind::Correction),
        "chargeback" => Ok(AdjustmentKind::Chargeback),
        "remittance_adjustment" => Ok(AdjustmentKind::RemittanceAdjustment),
        _ => Err(StorageError::InvalidSettlementAdjustment(
            "stored adjustment kind is invalid".to_string(),
        )),
    }
}

fn adjustment_status_from_db(value: &str) -> Result<AdjustmentStatus, StorageError> {
    match value {
        "proposed" => Ok(AdjustmentStatus::Proposed),
        "approved" => Ok(AdjustmentStatus::Approved),
        "rejected" => Ok(AdjustmentStatus::Rejected),
        "voided" => Ok(AdjustmentStatus::Voided),
        _ => Err(StorageError::InvalidSettlementAdjustmentTransition),
    }
}

fn rule_draft_request_digest(
    command: &CreateRuleDraftCommand,
    rule_digest: Digest32,
) -> Result<Digest32, StorageError> {
    let mut hasher = Sha256::new();
    hasher.update(b"solana-tax-protocol/rule-draft/v1");
    write_text(&mut hasher, &command.external_id)?;
    hasher.update(command.version.to_le_bytes());
    write_text(&mut hasher, &command.jurisdiction)?;
    write_text(&mut hasher, &command.regime)?;
    hasher.update(command.effective_from_unix.to_le_bytes());
    match command.effective_until_unix {
        Some(value) => {
            hasher.update([1]);
            hasher.update(value.to_le_bytes());
        }
        None => hasher.update([0]),
    }
    hasher.update(rule_digest);
    let provenance = serde_json::to_vec(&command.provenance)?;
    let provenance_len = u32::try_from(provenance.len()).map_err(|_| {
        StorageError::InvalidRuleDraft("provenance document is too large".to_string())
    })?;
    hasher.update(provenance_len.to_le_bytes());
    hasher.update(provenance);
    Ok(hasher.finalize().into())
}

fn validate_rule_draft_command(command: &CreateRuleDraftCommand) -> Result<(), StorageError> {
    if command.actor_subject.trim().is_empty() || command.actor_subject.len() > 256 {
        return Err(StorageError::InvalidRuleDraft(
            "author subject must contain 1 to 256 bytes".to_string(),
        ));
    }
    if command.idempotency_key.trim().is_empty() || command.idempotency_key.len() > 180 {
        return Err(StorageError::InvalidRuleDraft(
            "idempotency key must contain 1 to 180 bytes".to_string(),
        ));
    }
    for (field, value, max_bytes) in [
        ("external ID", command.external_id.as_str(), 128_usize),
        ("jurisdiction", command.jurisdiction.as_str(), 64_usize),
        ("tax regime", command.regime.as_str(), 64_usize),
    ] {
        if value.trim().is_empty() || value.len() > max_bytes || value.contains('\0') {
            return Err(StorageError::InvalidRuleDraft(format!(
                "{field} must contain 1 to {max_bytes} bytes"
            )));
        }
    }
    if command.version == 0 || i32::try_from(command.version).is_err() {
        return Err(StorageError::InvalidRuleDraft(
            "rule version must be a positive PostgreSQL-compatible integer".to_string(),
        ));
    }
    if command.effective_from_unix <= 0 || command.effective_from_unix > 4_102_444_800 {
        return Err(StorageError::InvalidRuleDraft(
            "effective_from_unix must be between 1970 and 2100".to_string(),
        ));
    }
    if let Some(effective_until) = command.effective_until_unix
        && (effective_until <= command.effective_from_unix || effective_until > 4_102_444_800)
    {
        return Err(StorageError::InvalidRuleDraft(
            "effective_until_unix must be after effective_from_unix and no later than 2100"
                .to_string(),
        ));
    }
    if command.rule_set.id != command.external_id || command.rule_set.version != command.version {
        return Err(StorageError::InvalidRuleDraft(
            "rule_set identity must match external_id and version".to_string(),
        ));
    }
    if !command.provenance.is_object() {
        return Err(StorageError::InvalidRuleDraft(
            "provenance must be a JSON object".to_string(),
        ));
    }
    if serde_json::to_vec(&command.provenance)?.len() > MAX_RULE_DRAFT_PROVENANCE_BYTES {
        return Err(StorageError::InvalidRuleDraft(
            "provenance exceeds the bounded size".to_string(),
        ));
    }
    Ok(())
}

fn write_text(hasher: &mut Sha256, value: &str) -> Result<(), StorageError> {
    let length = u32::try_from(value.len()).map_err(|_| {
        StorageError::InvalidConfiguration("quote selector text is too long".to_string())
    })?;
    hasher.update(length.to_le_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}

fn validate_command(command: &QuoteCommand) -> Result<(), StorageError> {
    if command
        .applicability_context_id
        .is_some_and(|context_id| context_id.is_nil())
    {
        return Err(StorageError::InvalidConfiguration(
            "applicability context reference must not be nil".to_string(),
        ));
    }
    if command.idempotency_key.trim().is_empty() || command.idempotency_key.len() > 200 {
        return Err(StorageError::InvalidConfiguration(
            "idempotency key must contain 1 to 200 bytes".to_string(),
        ));
    }
    if command.rule_set_id.trim().is_empty()
        || command.jurisdiction.trim().is_empty()
        || command.regime.trim().is_empty()
        || command.rule_version == 0
    {
        return Err(StorageError::InvalidConfiguration(
            "rule selector fields are required".to_string(),
        ));
    }
    if command.facts.jurisdiction != command.jurisdiction {
        return Err(StorageError::InvalidConfiguration(
            "facts jurisdiction must match the quote selector".to_string(),
        ));
    }
    candidate_set_digest(&command.jurisdiction_candidates).map_err(|error| {
        StorageError::InvalidConfiguration(format!("jurisdiction evidence is invalid: {error}"))
    })?;
    if i32::try_from(command.rule_version).is_err() {
        return Err(StorageError::InvalidConfiguration(
            "rule version exceeds the PostgreSQL selector range".to_string(),
        ));
    }
    Ok(())
}

fn validate_rule_governance_command(
    command: &RuleGovernanceTransition,
) -> Result<(), StorageError> {
    if command.actor_subject.trim().is_empty() || command.actor_subject.len() > 256 {
        return Err(StorageError::InvalidConfiguration(
            "rule governance actor subject must contain 1 to 256 bytes".to_string(),
        ));
    }
    if rule_governance_action(&command.to_status).is_err() {
        return Err(StorageError::InvalidRuleGovernanceTransition);
    }
    if !command.evidence.is_object() {
        return Err(StorageError::InvalidRuleGovernanceEvidence);
    }
    if serde_json::to_vec(&command.evidence)?.len() > MAX_RULE_GOVERNANCE_EVIDENCE_BYTES {
        return Err(StorageError::RuleGovernanceEvidenceTooLarge);
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleReleaseProvenanceV1 {
    schema: String,
    source_reference: String,
    source_digest_hex: String,
    legal_tax_owner: String,
    jurisdiction_scope: String,
    reviewed_at_unix: i64,
    review_reference: String,
}

fn validate_rule_release_provenance(
    provenance: &Value,
    rule_jurisdiction: &str,
) -> Result<(), StorageError> {
    let envelope: RuleReleaseProvenanceV1 =
        serde_json::from_value(provenance.clone()).map_err(|_| {
            StorageError::InvalidRuleReleaseProvenance(
                "expected the exact tax-rule-provenance/v1 envelope".to_string(),
            )
        })?;
    if envelope.schema != "tax-rule-provenance/v1" {
        return Err(StorageError::InvalidRuleReleaseProvenance(
            "schema must be tax-rule-provenance/v1".to_string(),
        ));
    }
    for (field, value, max_bytes) in [
        (
            "source_reference",
            envelope.source_reference.as_str(),
            512_usize,
        ),
        (
            "legal_tax_owner",
            envelope.legal_tax_owner.as_str(),
            256_usize,
        ),
        (
            "jurisdiction_scope",
            envelope.jurisdiction_scope.as_str(),
            64_usize,
        ),
        (
            "review_reference",
            envelope.review_reference.as_str(),
            512_usize,
        ),
    ] {
        if value.trim().is_empty() || value.len() > max_bytes || value.contains('\0') {
            return Err(StorageError::InvalidRuleReleaseProvenance(format!(
                "{field} must contain 1 to {max_bytes} bytes"
            )));
        }
    }
    if envelope.source_digest_hex.len() != 64
        || !envelope
            .source_digest_hex
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(StorageError::InvalidRuleReleaseProvenance(
            "source_digest_hex must contain exactly 64 hexadecimal characters".to_string(),
        ));
    }
    if envelope.jurisdiction_scope != rule_jurisdiction {
        return Err(StorageError::InvalidRuleReleaseProvenance(
            "jurisdiction_scope must match the persisted rule jurisdiction".to_string(),
        ));
    }
    if envelope.reviewed_at_unix <= 0 || envelope.reviewed_at_unix > 4_102_444_800 {
        return Err(StorageError::InvalidRuleReleaseProvenance(
            "reviewed_at_unix must be between 1970 and 2100".to_string(),
        ));
    }
    Ok(())
}

fn validate_jurisdiction_policy_draft_command(
    command: &CreateJurisdictionPolicyCommand,
) -> Result<(), StorageError> {
    if command.actor_subject.trim().is_empty() || command.actor_subject.len() > 256 {
        return Err(StorageError::InvalidJurisdictionPolicyDraft(
            "author subject must contain 1 to 256 bytes".to_string(),
        ));
    }
    if command.idempotency_key.trim().is_empty() || command.idempotency_key.len() > 180 {
        return Err(StorageError::InvalidJurisdictionPolicyDraft(
            "idempotency key must contain 1 to 180 bytes".to_string(),
        ));
    }
    if command.effective_from_unix <= 0 || command.effective_from_unix > 4_102_444_800 {
        return Err(StorageError::InvalidJurisdictionPolicyDraft(
            "effective_from_unix must be between 1970 and 2100".to_string(),
        ));
    }
    if let Some(effective_until) = command.effective_until_unix
        && (effective_until <= command.effective_from_unix || effective_until > 4_102_444_800)
    {
        return Err(StorageError::InvalidJurisdictionPolicyDraft(
            "effective_until_unix must be after effective_from_unix and no later than 2100"
                .to_string(),
        ));
    }
    command
        .policy
        .canonical_digest()
        .map_err(|error| StorageError::InvalidJurisdictionPolicyDraft(error.to_string()))?;
    if !command.provenance.is_object() {
        return Err(StorageError::InvalidJurisdictionPolicyDraft(
            "provenance must be a JSON object".to_string(),
        ));
    }
    if serde_json::to_vec(&command.provenance)?.len() > MAX_JURISDICTION_POLICY_PROVENANCE_BYTES {
        return Err(StorageError::InvalidJurisdictionPolicyDraft(
            "provenance exceeds the bounded size".to_string(),
        ));
    }
    Ok(())
}

fn jurisdiction_policy_request_digest(
    command: &CreateJurisdictionPolicyCommand,
    policy_digest: Digest32,
) -> Result<Digest32, StorageError> {
    let mut hasher = Sha256::new();
    hasher.update(b"solana-tax-protocol/jurisdiction-policy/v1");
    hasher.update(command.rule_set_id.as_bytes());
    hasher.update(policy_digest);
    hasher.update(command.effective_from_unix.to_le_bytes());
    match command.effective_until_unix {
        Some(value) => {
            hasher.update([1]);
            hasher.update(value.to_le_bytes());
        }
        None => hasher.update([0]),
    }
    let provenance = serde_json::to_vec(&command.provenance)?;
    let provenance_len = u32::try_from(provenance.len()).map_err(|_| {
        StorageError::InvalidJurisdictionPolicyDraft("provenance document is too large".into())
    })?;
    hasher.update(provenance_len.to_le_bytes());
    hasher.update(provenance);
    Ok(hasher.finalize().into())
}

fn validate_jurisdiction_policy_governance_command(
    command: &JurisdictionPolicyGovernanceTransition,
) -> Result<(), StorageError> {
    if command.actor_subject.trim().is_empty() || command.actor_subject.len() > 256 {
        return Err(StorageError::InvalidJurisdictionPolicyTransition);
    }
    if jurisdiction_policy_action(&command.to_status).is_err() {
        return Err(StorageError::InvalidJurisdictionPolicyTransition);
    }
    if !command.evidence.is_object() {
        return Err(StorageError::InvalidJurisdictionPolicyEvidence);
    }
    if serde_json::to_vec(&command.evidence)?.len() > MAX_RULE_GOVERNANCE_EVIDENCE_BYTES {
        return Err(StorageError::JurisdictionPolicyEvidenceTooLarge);
    }
    Ok(())
}

fn validate_vault_release_claim_command(
    command: &CreateVaultReleaseClaimCommand,
) -> Result<(), StorageError> {
    if command.actor_subject.trim().is_empty() || command.actor_subject.len() > 256 {
        return Err(StorageError::InvalidVaultReleaseClaim(
            "proposer subject must contain 1 to 256 bytes".into(),
        ));
    }
    if command.idempotency_key.trim().is_empty() || command.idempotency_key.len() > 200 {
        return Err(StorageError::InvalidVaultReleaseClaim(
            "idempotency key must contain 1 to 200 bytes".into(),
        ));
    }
    if command.route_id.get_version_num() != 7 {
        return Err(StorageError::InvalidVaultReleaseClaim(
            "route_id must be a UUIDv7 verified payment route".into(),
        ));
    }
    for (field, value) in [("destination", command.destination.as_str())] {
        if value.trim().len() < 32 || value.trim().len() > 64 || value != value.trim() {
            return Err(StorageError::InvalidVaultReleaseClaim(format!(
                "{field} must contain a bounded Solana address"
            )));
        }
    }
    if command.amount_atomic == 0 {
        return Err(StorageError::InvalidVaultReleaseClaim(
            "amount_atomic must be positive".into(),
        ));
    }
    if command.reference_digest == [0; 32] {
        return Err(StorageError::InvalidVaultReleaseClaim(
            "reference_digest must not be all zeroes".into(),
        ));
    }
    if command.execute_after_unix <= 0
        || command.expires_at_unix <= command.execute_after_unix
        || command
            .expires_at_unix
            .checked_sub(command.execute_after_unix)
            .is_none_or(|window| window as u64 > MAX_VAULT_RELEASE_WINDOW.as_secs())
    {
        return Err(StorageError::InvalidVaultReleaseClaim(
            "release execution window is invalid or exceeds 90 days".into(),
        ));
    }
    Ok(())
}

fn vault_release_request_digest(
    command: &CreateVaultReleaseClaimCommand,
    mint: &str,
    token_program: &str,
    tax_vault: &str,
) -> Result<Digest32, StorageError> {
    let mut hasher = Sha256::new();
    hasher.update(b"solana-tax-protocol/vault-release-claim/v2");
    hasher.update(command.merchant_id.as_bytes());
    hasher.update(command.route_id.as_bytes());
    write_projection_text(&mut hasher, &command.idempotency_key);
    write_projection_text(&mut hasher, mint);
    write_projection_text(&mut hasher, token_program);
    write_projection_text(&mut hasher, tax_vault);
    write_projection_text(&mut hasher, &command.destination);
    hasher.update(command.nonce.to_le_bytes());
    hasher.update(command.amount_atomic.to_le_bytes());
    hasher.update(command.reference_digest);
    hasher.update(command.execute_after_unix.to_le_bytes());
    hasher.update(command.expires_at_unix.to_le_bytes());
    Ok(hasher.finalize().into())
}

fn validate_vault_release_transition(
    command: &VaultReleaseClaimTransition,
) -> Result<(), StorageError> {
    if command.actor_subject.trim().is_empty() || command.actor_subject.len() > 256 {
        return Err(StorageError::InvalidVaultReleaseClaim(
            "actor subject must contain 1 to 256 bytes".into(),
        ));
    }
    if vault_release_claim_action(&command.to_status).is_err() {
        return Err(StorageError::InvalidVaultReleaseClaimTransition);
    }
    if !command.evidence.is_object() {
        return Err(StorageError::InvalidVaultReleaseEvidence);
    }
    if serde_json::to_vec(&command.evidence)?.len() > MAX_VAULT_RELEASE_EVIDENCE_BYTES {
        return Err(StorageError::VaultReleaseEvidenceTooLarge);
    }
    if command
        .claim_pda
        .as_deref()
        .is_some_and(|value| value.trim().len() < 32 || value.len() > 64)
        || command
            .transaction_signature
            .as_deref()
            .is_some_and(|value| value.trim().is_empty() || value.len() > 128)
        || command
            .last_error_code
            .as_deref()
            .is_some_and(|value| value.trim().is_empty() || value.len() > 128)
    {
        return Err(StorageError::InvalidVaultReleaseClaim(
            "release evidence identifiers are outside their bounds".into(),
        ));
    }
    match command.to_status.as_str() {
        "approved" | "cancelled" => {
            if command.claim_pda.is_some()
                || command.transaction_signature.is_some()
                || command.slot.is_some()
                || command.last_error_code.is_some()
            {
                return Err(StorageError::InvalidVaultReleaseClaim(
                    "approval and cancellation cannot include chain evidence".into(),
                ));
            }
        }
        "onchain_created" => {
            if command.claim_pda.is_none()
                || command.transaction_signature.is_none()
                || command.slot.is_none()
            {
                return Err(StorageError::InvalidVaultReleaseClaim(
                    "on-chain creation requires claim PDA, transaction signature, and slot".into(),
                ));
            }
        }
        "executed" => {
            if command.transaction_signature.is_none() || command.slot.is_none() {
                return Err(StorageError::InvalidVaultReleaseClaim(
                    "execution requires transaction signature and slot".into(),
                ));
            }
        }
        "unknown" => {
            if command.last_error_code.is_none() {
                return Err(StorageError::InvalidVaultReleaseClaim(
                    "unknown chain state requires a bounded error code".into(),
                ));
            }
        }
        _ => return Err(StorageError::InvalidVaultReleaseClaimTransition),
    }
    Ok(())
}

fn vault_release_claim_action(status: &str) -> Result<&'static str, StorageError> {
    match status {
        "approved" => Ok("approve"),
        "onchain_created" => Ok("chain_created"),
        "executed" => Ok("execute"),
        "cancelled" => Ok("cancel"),
        "unknown" => Ok("unknown"),
        _ => Err(StorageError::InvalidVaultReleaseClaimTransition),
    }
}

fn jurisdiction_policy_action(status: &str) -> Result<&'static str, StorageError> {
    match status {
        "reviewed" => Ok("review"),
        "approved" => Ok("approve"),
        "published" => Ok("publish"),
        "retired" => Ok("retire"),
        "blocked" => Ok("block"),
        _ => Err(StorageError::InvalidJurisdictionPolicyTransition),
    }
}

fn assign_jurisdiction_policy_subjects(
    from_status: &str,
    to_status: &str,
    actor_subject: &str,
    author_subject: Option<String>,
    reviewer_subject: Option<String>,
    approver_subject: Option<String>,
) -> Result<GovernanceSubjects, StorageError> {
    let valid_transition = matches!(
        (from_status, to_status),
        ("draft", "reviewed")
            | ("draft", "blocked")
            | ("reviewed", "approved")
            | ("reviewed", "blocked")
            | ("approved", "published")
            | ("approved", "blocked")
            | ("published", "retired")
    );
    if !valid_transition {
        return Err(StorageError::InvalidJurisdictionPolicyTransition);
    }
    let author = author_subject;
    let mut reviewer = reviewer_subject;
    let mut approver = approver_subject;
    match to_status {
        "reviewed" => {
            if author.is_none()
                || reviewer
                    .as_deref()
                    .is_some_and(|subject| subject != actor_subject)
                || author.as_deref() == Some(actor_subject)
            {
                return Err(StorageError::JurisdictionPolicyGovernanceConflict);
            }
            reviewer = Some(actor_subject.to_string());
        }
        "approved" => {
            if author.is_none()
                || reviewer.is_none()
                || approver
                    .as_deref()
                    .is_some_and(|subject| subject != actor_subject)
                || author.as_deref() == Some(actor_subject)
                || reviewer.as_deref() == Some(actor_subject)
            {
                return Err(StorageError::JurisdictionPolicyGovernanceConflict);
            }
            approver = Some(actor_subject.to_string());
        }
        "published" | "retired" => {
            if author.is_none() || reviewer.is_none() || approver.as_deref() != Some(actor_subject)
            {
                return Err(StorageError::JurisdictionPolicyGovernanceConflict);
            }
        }
        "blocked" => {
            if author.is_none() {
                return Err(StorageError::JurisdictionPolicyGovernanceConflict);
            }
        }
        _ => return Err(StorageError::InvalidJurisdictionPolicyTransition),
    }
    if (author.is_some() && reviewer.is_some() && author == reviewer)
        || (author.is_some() && approver.is_some() && author == approver)
        || (reviewer.is_some() && approver.is_some() && reviewer == approver)
    {
        return Err(StorageError::JurisdictionPolicyGovernanceConflict);
    }
    Ok((author, reviewer, approver))
}

fn unix_to_system_time(value: i64) -> Result<std::time::SystemTime, StorageError> {
    if value <= 0 {
        return Err(StorageError::InvalidJurisdictionPolicyDraft(
            "timestamp must be positive".to_string(),
        ));
    }
    std::time::UNIX_EPOCH
        .checked_add(Duration::from_secs(value as u64))
        .ok_or_else(|| {
            StorageError::InvalidJurisdictionPolicyDraft("timestamp overflow".to_string())
        })
}

fn rule_governance_action(status: &str) -> Result<&'static str, StorageError> {
    match status {
        "simulated" => Ok("simulate"),
        "reviewed" => Ok("review"),
        "approved" => Ok("approve"),
        "published" => Ok("publish"),
        "retired" => Ok("retire"),
        _ => Err(StorageError::InvalidRuleGovernanceTransition),
    }
}

fn assign_governance_subjects(
    from_status: &str,
    to_status: &str,
    actor_subject: &str,
    author_subject: Option<String>,
    reviewer_subject: Option<String>,
    approver_subject: Option<String>,
) -> Result<GovernanceSubjects, StorageError> {
    let valid_transition = matches!(
        (from_status, to_status),
        ("draft", "simulated")
            | ("simulated", "reviewed")
            | ("reviewed", "approved")
            | ("approved", "published")
            | ("published", "retired")
    );
    if !valid_transition {
        return Err(StorageError::InvalidRuleGovernanceTransition);
    }

    let mut author = author_subject;
    let mut reviewer = reviewer_subject;
    let mut approver = approver_subject;
    match to_status {
        "simulated" => {
            if author
                .as_deref()
                .is_some_and(|subject| subject != actor_subject)
            {
                return Err(StorageError::RuleGovernanceConflict);
            }
            author = Some(actor_subject.to_string());
        }
        "reviewed" => {
            if author.is_none()
                || reviewer
                    .as_deref()
                    .is_some_and(|subject| subject != actor_subject)
                || author.as_deref() == Some(actor_subject)
            {
                return Err(StorageError::RuleGovernanceConflict);
            }
            reviewer = Some(actor_subject.to_string());
        }
        "approved" => {
            if author.is_none()
                || reviewer.is_none()
                || approver
                    .as_deref()
                    .is_some_and(|subject| subject != actor_subject)
                || author.as_deref() == Some(actor_subject)
                || reviewer.as_deref() == Some(actor_subject)
            {
                return Err(StorageError::RuleGovernanceConflict);
            }
            approver = Some(actor_subject.to_string());
        }
        "published" | "retired" => {
            if author.is_none() || reviewer.is_none() || approver.as_deref() != Some(actor_subject)
            {
                return Err(StorageError::RuleGovernanceConflict);
            }
        }
        _ => return Err(StorageError::InvalidRuleGovernanceTransition),
    }
    if (author.is_some() && reviewer.is_some() && author == reviewer)
        || (author.is_some() && approver.is_some() && author == approver)
        || (reviewer.is_some() && approver.is_some() && reviewer == approver)
    {
        return Err(StorageError::RuleGovernanceConflict);
    }
    Ok((author, reviewer, approver))
}

fn validate_outbox_options(batch_size: usize, lease: Duration) -> Result<(), StorageError> {
    if batch_size == 0 || batch_size > MAX_OUTBOX_BATCH_SIZE {
        return Err(StorageError::InvalidConfiguration(format!(
            "outbox batch size must be between 1 and {MAX_OUTBOX_BATCH_SIZE}"
        )));
    }
    if lease.is_zero() || lease > Duration::from_secs(300) {
        return Err(StorageError::InvalidConfiguration(
            "outbox lease must be between 1 millisecond and 300 seconds".to_string(),
        ));
    }
    Ok(())
}

fn validate_replay_gap(gap: &ReplayGap) -> Result<(), StorageError> {
    if gap.range.end_slot < gap.range.start_slot
        || gap
            .range
            .end_slot
            .checked_sub(gap.range.start_slot)
            .and_then(|length| length.checked_add(1))
            .is_none_or(|length| length > MAX_SCAN_RANGE_SLOTS)
    {
        return Err(StorageError::InvalidReplayState(
            "replay gap range is outside the bounded slot domain".to_string(),
        ));
    }
    if gap.attempts > MAX_REPLAY_ATTEMPTS {
        return Err(StorageError::InvalidReplayState(
            "replay gap attempts exceed the bounded retry limit".to_string(),
        ));
    }
    Ok(())
}

fn validate_replay_options(batch_size: usize, lease: Duration) -> Result<(), StorageError> {
    if batch_size == 0 || batch_size > MAX_REPLAY_BATCH_SIZE {
        return Err(StorageError::InvalidConfiguration(format!(
            "replay batch size must be between 1 and {MAX_REPLAY_BATCH_SIZE}"
        )));
    }
    if lease.is_zero() || lease > Duration::from_secs(300) {
        return Err(StorageError::InvalidConfiguration(
            "replay lease must be between 1 millisecond and 300 seconds".to_string(),
        ));
    }
    Ok(())
}

fn validate_replay_error_code(error_code: &str) -> Result<(), StorageError> {
    if error_code.trim().is_empty() || error_code.len() > 128 {
        return Err(StorageError::InvalidConfiguration(
            "replay error code must contain 1 to 128 bytes".to_string(),
        ));
    }
    Ok(())
}

fn validate_replay_status_filter(status: Option<&str>) -> Result<(), StorageError> {
    if let Some(status) = status
        && !matches!(status, "queued" | "leased" | "completed" | "dead_lettered")
    {
        return Err(StorageError::InvalidReplayOperatorInput(
            "replay-gap status filter is unsupported".into(),
        ));
    }
    Ok(())
}

fn validate_replay_operator_input(subject: &str, note: &str) -> Result<(), StorageError> {
    if subject.trim().is_empty() || subject.len() > MAX_REPLAY_OPERATOR_SUBJECT_BYTES {
        return Err(StorageError::InvalidReplayOperatorInput(format!(
            "operator subject must contain 1 to {MAX_REPLAY_OPERATOR_SUBJECT_BYTES} bytes"
        )));
    }
    if note.trim().is_empty() || note.len() > MAX_REPLAY_OPERATOR_NOTE_BYTES {
        return Err(StorageError::InvalidReplayOperatorInput(format!(
            "operator note must contain 1 to {MAX_REPLAY_OPERATOR_NOTE_BYTES} bytes"
        )));
    }
    Ok(())
}

fn validate_targeted_recovery_audit(
    command: &TargetedRecoveryAuditCommand,
) -> Result<(), StorageError> {
    validate_indexer_stream_key(&command.stream_key)?;
    validate_replay_operator_input(&command.operator_subject, &command.note)?;
    if command.request_id.is_nil()
        || command.target_address.trim().is_empty()
        || command.target_address.len() > 64
    {
        return Err(StorageError::InvalidReplayOperatorInput(
            "targeted recovery identity is invalid".to_string(),
        ));
    }
    if indexer::canonical_signature(&command.target_signature).is_err() {
        return Err(StorageError::InvalidReplayOperatorInput(
            "targeted recovery signature is invalid".to_string(),
        ));
    }
    if !matches!(
        command.outcome.as_str(),
        "requested" | "recovered" | "failed"
    ) {
        return Err(StorageError::InvalidReplayOperatorInput(
            "targeted recovery outcome is unsupported".to_string(),
        ));
    }
    if command
        .pages_scanned
        .is_some_and(|pages| !(1..=64).contains(&pages))
    {
        return Err(StorageError::InvalidReplayOperatorInput(
            "targeted recovery page count is outside the bounded range".to_string(),
        ));
    }
    if command
        .error_code
        .as_ref()
        .is_some_and(|code| code.trim().is_empty() || code.len() > MAX_REPLAY_OPERATOR_NOTE_BYTES)
    {
        return Err(StorageError::InvalidReplayOperatorInput(
            "targeted recovery error code is invalid".to_string(),
        ));
    }
    if (command.outcome == "failed") != command.error_code.is_some() {
        return Err(StorageError::InvalidReplayOperatorInput(
            "targeted recovery outcome and error code do not match".to_string(),
        ));
    }
    if command.outcome == "recovered" && (command.pages_scanned.is_none() || command.slot.is_none())
    {
        return Err(StorageError::InvalidReplayOperatorInput(
            "recovered targeted recovery requires page and slot evidence".to_string(),
        ));
    }
    Ok(())
}

fn replay_reason_to_db(reason: ReplayGapReason) -> &'static str {
    match reason {
        ReplayGapReason::IncompleteProviderPage => "incomplete_provider_page",
        ReplayGapReason::ProviderDisagreement => "provider_disagreement",
        ReplayGapReason::ProjectionCommitFailure => "projection_commit_failure",
        ReplayGapReason::OperatorRequested => "operator_requested",
    }
}

fn replay_reason_from_db(value: &str) -> Result<ReplayGapReason, StorageError> {
    match value {
        "incomplete_provider_page" => Ok(ReplayGapReason::IncompleteProviderPage),
        "provider_disagreement" => Ok(ReplayGapReason::ProviderDisagreement),
        "projection_commit_failure" => Ok(ReplayGapReason::ProjectionCommitFailure),
        "operator_requested" => Ok(ReplayGapReason::OperatorRequested),
        _ => Err(StorageError::InvalidReplayReason(value.to_string())),
    }
}

fn validate_indexer_stream_key(stream_key: &str) -> Result<(), StorageError> {
    if stream_key.trim().is_empty() || stream_key.len() > 200 {
        return Err(StorageError::InvalidIndexerStreamKey);
    }
    Ok(())
}

fn parse_settlement_status(value: &str) -> Result<SettlementStatus, StorageError> {
    match value {
        "prepared_unsubmitted" => Ok(SettlementStatus::PreparedUnsubmitted),
        "signed" => Ok(SettlementStatus::Signed),
        "submitting" => Ok(SettlementStatus::Submitting),
        "submitted" => Ok(SettlementStatus::Submitted),
        "processed" => Ok(SettlementStatus::Processed),
        "confirmed" => Ok(SettlementStatus::Confirmed),
        "finalized" => Ok(SettlementStatus::Finalized),
        "cancelled" => Ok(SettlementStatus::Cancelled),
        "rejected" => Ok(SettlementStatus::Rejected),
        "unknown" => Ok(SettlementStatus::Unknown),
        _ => Err(StorageError::InvalidSettlementStatus(value.to_string())),
    }
}

fn parse_digest(value: Vec<u8>) -> Result<[u8; 32], StorageError> {
    value
        .try_into()
        .map_err(|_| StorageError::InvalidSettlementDigest)
}

fn parse_stored_calculation_attestation(
    calculation_id: Uuid,
    payload_digest: Vec<u8>,
    issuer_public_key: Vec<u8>,
    expires_at_unix: i64,
    document: Value,
) -> Result<StoredCalculationAttestation, StorageError> {
    let payload_digest: Digest32 = payload_digest.try_into().map_err(|_| {
        StorageError::InvalidAttestation("stored payload digest is not 32 bytes".to_string())
    })?;
    let issuer_public_key: [u8; 32] = issuer_public_key.try_into().map_err(|_| {
        StorageError::InvalidAttestation("stored issuer public key is not 32 bytes".to_string())
    })?;
    let attestation: CalculationAttestation = serde_json::from_value(document)
        .map_err(|error| StorageError::InvalidAttestation(error.to_string()))?;
    attestation
        .verify()
        .map_err(|error| StorageError::InvalidAttestation(error.to_string()))?;
    if attestation.payload_digest != payload_digest
        || attestation.issuer_public_key != issuer_public_key
        || attestation.unsigned.expires_at_unix != expires_at_unix
    {
        return Err(StorageError::InvalidAttestation(
            "stored attestation columns do not match its signed document".to_string(),
        ));
    }
    Ok(StoredCalculationAttestation {
        calculation_id,
        payload_digest,
        issuer_public_key,
        expires_at_unix,
        attestation,
    })
}

fn parse_stored_document_calculation_attestation(
    calculation_id: Uuid,
    payload_digest: Vec<u8>,
    issuer_public_key: Vec<u8>,
    expires_at_unix: i64,
    document: Value,
) -> Result<StoredDocumentCalculationAttestation, StorageError> {
    let payload_digest: Digest32 = payload_digest.try_into().map_err(|_| {
        StorageError::InvalidAttestation(
            "stored document payload digest is not 32 bytes".to_string(),
        )
    })?;
    let issuer_public_key: [u8; 32] = issuer_public_key.try_into().map_err(|_| {
        StorageError::InvalidAttestation(
            "stored document issuer public key is not 32 bytes".to_string(),
        )
    })?;
    let attestation: DocumentCalculationAttestation = serde_json::from_value(document)
        .map_err(|error| StorageError::InvalidAttestation(error.to_string()))?;
    attestation
        .verify()
        .map_err(|error| StorageError::InvalidAttestation(error.to_string()))?;
    if attestation.payload_digest != payload_digest
        || attestation.issuer_public_key != issuer_public_key
        || attestation.unsigned.expires_at_unix != expires_at_unix
    {
        return Err(StorageError::InvalidAttestation(
            "stored document attestation columns do not match its signed document".to_string(),
        ));
    }
    Ok(StoredDocumentCalculationAttestation {
        calculation_id,
        payload_digest,
        issuer_public_key,
        expires_at_unix,
        attestation,
    })
}

fn parse_stored_applicability_context(
    context_id: Uuid,
    merchant_id: Uuid,
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<StoredApplicabilityContext, StorageError> {
    let status: String = row.get(0);
    let policy_id: String = row.get(1);
    let policy_version: i32 = row.get(2);
    let policy_digest = parse_digest(row.get(3))?;
    let facts_digest = parse_digest(row.get(4))?;
    let decision_digest = parse_digest(row.get(5))?;
    let jurisdiction_resolution_digest = parse_digest(row.get(6))?;
    let evidence_digest = parse_digest(row.get(7))?;
    let attestation_digest = parse_digest(row.get(8))?;
    let effective_from_unix: i64 = row.get(9);
    let effective_until_unix: Option<i64> = row.get(10);
    let evidence_expires_at_unix: i64 = row.get(11);
    let facts_document: Value = row.get(12);
    let decision_document: Value = row.get(13);
    let attestation_document: Value = row.get(14);
    let provenance: Value = row.get(15);
    if policy_version <= 0 {
        return Err(StorageError::InvalidApplicabilityContext(
            "stored applicability policy version is invalid".into(),
        ));
    }
    let attestation: ApplicabilityAttestation = serde_json::from_value(attestation_document)
        .map_err(|error| StorageError::InvalidApplicabilityContext(error.to_string()))?;
    attestation
        .verify()
        .map_err(|error| StorageError::InvalidApplicabilityContext(error.to_string()))?;
    let unsigned = &attestation.unsigned;
    let expected_merchant = merchant_identity_digest(merchant_id.as_bytes());
    let expected_facts = serde_json::to_value(&unsigned.facts)?;
    let expected_decision = serde_json::to_value(&unsigned.decision)?;
    let expected_facts_digest = unsigned
        .facts
        .canonical_digest()
        .map_err(|error| StorageError::InvalidApplicabilityContext(error.to_string()))?;
    let expected_decision_digest = unsigned
        .decision
        .canonical_digest()
        .map_err(|error| StorageError::InvalidApplicabilityContext(error.to_string()))?;
    if unsigned.merchant_id != expected_merchant
        || policy_id != unsigned.decision.policy_id
        || policy_version as u32 != unsigned.decision.policy_version
        || policy_digest != unsigned.decision.policy_digest
        || facts_digest != expected_facts_digest
        || decision_digest != expected_decision_digest
        || unsigned.decision.facts_digest != facts_digest
        || jurisdiction_resolution_digest != unsigned.jurisdiction_resolution_digest
        || evidence_digest != unsigned.facts.evidence_digest
        || attestation_digest != attestation.payload_digest
        || effective_from_unix < unsigned.issued_at_unix
        || effective_until_unix.is_some_and(|until| until > unsigned.expires_at_unix)
        || evidence_expires_at_unix != unsigned.facts.valid_until_unix
        || facts_document != expected_facts
        || decision_document != expected_decision
    {
        return Err(StorageError::InvalidApplicabilityContext(
            "stored applicability context columns do not match signed evidence".into(),
        ));
    }
    Ok(StoredApplicabilityContext {
        context_id,
        merchant_id,
        status,
        policy_id,
        policy_version: policy_version as u32,
        policy_digest,
        facts_digest,
        decision_digest,
        jurisdiction_resolution_digest,
        evidence_digest,
        attestation_digest,
        effective_from_unix,
        effective_until_unix,
        evidence_expires_at_unix,
        provenance,
        attestation,
    })
}

fn parse_attestation_amount(value: String) -> Result<u64, StorageError> {
    value.parse::<u64>().map_err(|_| {
        StorageError::InvalidAttestation(
            "stored document amount is outside the attestation range".to_string(),
        )
    })
}

fn parse_settlement_intent_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<SettlementIntent, StorageError> {
    let signed_transaction_digest: Option<Vec<u8>> = row.get(6);
    let slot: Option<String> = row.get(8);
    Ok(SettlementIntent {
        id: row.get(0),
        merchant_id: row.get(1),
        idempotency_key: row.get(2),
        request_digest: parse_digest(row.get(3))?,
        instruction_digest: parse_digest(row.get(4))?,
        status: row.get(5),
        signed_transaction_digest: signed_transaction_digest.map(parse_digest).transpose()?,
        signature: row.get(7),
        slot: slot
            .as_deref()
            .map(|value| parse_u64(value, "settlement slot"))
            .transpose()?,
        commitment: row.get(9),
        last_error_code: row.get(10),
    })
}

fn parse_checkout_session_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<CheckoutSession, StorageError> {
    Ok(CheckoutSession {
        id: row.get(0),
        merchant_id: row.get(1),
        calculation_id: row.get(2),
        route_id: row.get(3),
        status: row.get(4),
        expires_at_unix: row.get(5),
        settlement_intent_id: row.get(6),
    })
}

fn parse_checkout_session_row_from_public(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<CheckoutSession, StorageError> {
    Ok(CheckoutSession {
        id: row.get(0),
        merchant_id: row.get(1),
        calculation_id: row.get(2),
        route_id: row.get(3),
        status: row.get(5),
        expires_at_unix: row.get(6),
        settlement_intent_id: row.get(8),
    })
}

fn parse_document_payment_authorization_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<DocumentPaymentAuthorizationRecord, StorageError> {
    let source_attestation_digest = parse_digest(row.get(6))?;
    let payment_scope_digest = parse_digest(row.get(7))?;
    let scope: PaymentScope = serde_json::from_value(row.get(8)).map_err(|error| {
        StorageError::DocumentPaymentAuthorizationIntegrity(format!(
            "scope document is invalid: {error}"
        ))
    })?;
    let gross_atomic = parse_u64(row.get::<_, String>(9).as_str(), "document gross amount")?;
    let tax_atomic = parse_u64(row.get::<_, String>(10).as_str(), "document tax amount")?;
    let net_atomic = parse_u64(row.get::<_, String>(11).as_str(), "document net amount")?;
    if source_attestation_digest == [0; 32]
        || payment_scope_digest == [0; 32]
        || gross_atomic == 0
        || tax_atomic.checked_add(net_atomic) != Some(gross_atomic)
    {
        return Err(StorageError::DocumentPaymentAuthorizationIntegrity(
            "stored document payment evidence is not conserved".into(),
        ));
    }
    Ok(DocumentPaymentAuthorizationRecord {
        id: row.get(0),
        intent_id: row.get(1),
        merchant_id: row.get(2),
        calculation_id: row.get(3),
        route_id: row.get(4),
        mint: row.get(5),
        source_attestation_digest,
        payment_scope_digest,
        scope,
        gross_atomic,
        tax_atomic,
        net_atomic,
    })
}

fn parse_settlement_adjustment_chain_intent_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<SettlementAdjustmentChainIntent, StorageError> {
    let signed_transaction_digest: Option<Vec<u8>> = row.get(6);
    let slot: Option<String> = row.get(8);
    let instruction_digest = parse_digest(row.get(4))?;
    if instruction_digest == [0; 32] {
        return Err(StorageError::AdjustmentChainIntentConflict);
    }
    let finalized_event_index: Option<i16> = row.get(11);
    let finalized_event_evidence_digest = row
        .get::<_, Option<Vec<u8>>>(12)
        .map(parse_digest)
        .transpose()?;
    if finalized_event_index.is_some_and(|value| value < 0)
        || (finalized_event_index.is_some() && finalized_event_evidence_digest.is_none())
        || (finalized_event_index.is_none() && finalized_event_evidence_digest.is_some())
        || finalized_event_evidence_digest.is_some_and(|digest| digest == [0; 32])
    {
        return Err(StorageError::AdjustmentChainIntentConflict);
    }
    Ok(SettlementAdjustmentChainIntent {
        id: row.get(0),
        evidence_intent_id: row.get(1),
        adjustment_id: row.get(2),
        merchant_id: row.get(3),
        instruction_digest,
        status: row.get(5),
        signed_transaction_digest: signed_transaction_digest.map(parse_digest).transpose()?,
        signature: row.get(7),
        slot: slot
            .as_deref()
            .map(|value| parse_u64(value, "adjustment chain slot"))
            .transpose()?,
        commitment: row.get(9),
        last_error_code: row.get(10),
        finalized_event_index: finalized_event_index.map(|value| value as u16),
        finalized_event_evidence_digest,
    })
}

fn parse_settlement_receipt_binding_row(
    row: deadpool_postgres::tokio_postgres::Row,
    settlement_id: Uuid,
) -> Result<SettlementReceiptBinding, StorageError> {
    let signature: String = row.get(0);
    if signature.trim().is_empty() || signature.len() > 128 {
        return Err(StorageError::InvalidSettlementReceiptBinding(
            "finalized receipt signature is invalid".into(),
        ));
    }
    let slot = parse_u64(row.get(1), "finalized receipt slot")?;
    let receipt = parse_optional_binding_digest(row.get(2), "receipt PDA")?;
    let payer = parse_optional_binding_digest(row.get(3), "receipt payer")?;
    let merchant = parse_optional_binding_digest(row.get(4), "receipt merchant")?;
    let mint = parse_optional_binding_digest(row.get(5), "receipt mint")?;
    let nonce = parse_u64(row.get(6), "receipt nonce")?;
    let gross_atomic = parse_u64(row.get(7), "receipt gross amount")?;
    let tax_atomic = parse_u64(row.get(8), "receipt tax amount")?;
    let net_atomic = parse_u64(row.get(9), "receipt net amount")?;
    let policy_digest = parse_optional_binding_digest(row.get(10), "receipt policy digest")?;
    let evidence_digest = parse_digest(row.get(11)).map_err(|_| {
        StorageError::InvalidSettlementReceiptBinding(
            "receipt evidence digest is not 32 bytes".into(),
        )
    })?;
    if receipt == [0; 32]
        || payer == [0; 32]
        || merchant == [0; 32]
        || mint == [0; 32]
        || nonce == 0
        || policy_digest == [0; 32]
        || evidence_digest == [0; 32]
        || gross_atomic == 0
        || tax_atomic.checked_add(net_atomic) != Some(gross_atomic)
    {
        return Err(StorageError::InvalidSettlementReceiptBinding(
            "finalized receipt projection failed identity or conservation validation".into(),
        ));
    }
    Ok(SettlementReceiptBinding {
        settlement_id,
        signature,
        slot,
        receipt,
        payer,
        merchant,
        mint,
        nonce,
        gross_atomic,
        tax_atomic,
        net_atomic,
        policy_digest,
        evidence_digest,
    })
}

fn parse_adjustment_authority_binding_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<AdjustmentAuthorityBinding, StorageError> {
    let public_key: [u8; 32] = row.get::<_, Vec<u8>>(4).try_into().map_err(|_| {
        StorageError::InvalidAdjustmentAuthorityChallenge(
            "stored authority public key is not 32 bytes".into(),
        )
    })?;
    let proof_signature: [u8; 64] = row.get::<_, Vec<u8>>(6).try_into().map_err(|_| {
        StorageError::InvalidAdjustmentAuthorityChallenge(
            "stored authority proof signature is not 64 bytes".into(),
        )
    })?;
    if public_key == [0; 32] {
        return Err(StorageError::InvalidAdjustmentAuthorityChallenge(
            "stored authority public key is zero".into(),
        ));
    }
    Ok(AdjustmentAuthorityBinding {
        id: row.get(0),
        merchant_id: row.get(1),
        subject: row.get(2),
        role: row.get(3),
        public_key,
        challenge_id: row.get(5),
        proof_signature,
    })
}

fn parse_settlement_adjustment_evidence_intent_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<SettlementAdjustmentEvidenceIntent, StorageError> {
    let digest = |index: usize, field: &str| -> Result<[u8; 32], StorageError> {
        row.get::<_, Vec<u8>>(index).try_into().map_err(|_| {
            StorageError::InvalidAdjustmentEvidenceIntent(format!("stored {field} is not 32 bytes"))
        })
    };
    let proposer = digest(4, "proposer")?;
    let approver = digest(5, "approver")?;
    let original_receipt = digest(6, "original receipt")?;
    let merchant = digest(7, "merchant")?;
    let mint = digest(8, "mint")?;
    let registry_address = digest(10, "registry address")?;
    let registry_slot = parse_u64(row.get(11), "adjustment registry slot")?;
    let registry_commitment: String = row.get(12);
    let registry_account_digest = digest(13, "registry account digest")?;
    let request_digest = digest(14, "request digest")?;
    let instruction_digest: Option<Vec<u8>> = row.get(15);
    let instruction_digest = instruction_digest
        .map(|value| {
            value.try_into().map_err(|_| {
                StorageError::InvalidAdjustmentEvidenceIntent(
                    "stored instruction digest is not 32 bytes".into(),
                )
            })
        })
        .transpose()?;
    let nonce = parse_u64(row.get(3), "adjustment evidence nonce")?;
    let token_program: String = row.get(9);
    let status: String = row.get(16);
    if nonce == 0
        || proposer == [0; 32]
        || approver == [0; 32]
        || proposer == approver
        || original_receipt == [0; 32]
        || merchant == [0; 32]
        || mint == [0; 32]
        || registry_address == [0; 32]
        || registry_slot == 0
        || registry_account_digest == [0; 32]
        || request_digest == [0; 32]
        || token_program.len() < 32
        || token_program.len() > 44
        || registry_commitment != "finalized"
        || status != "prepared_unsubmitted"
    {
        return Err(StorageError::InvalidAdjustmentEvidenceIntent(
            "stored adjustment evidence intent failed identity validation".into(),
        ));
    }
    Ok(SettlementAdjustmentEvidenceIntent {
        id: row.get(0),
        adjustment_id: row.get(1),
        merchant_id: row.get(2),
        nonce,
        proposer,
        approver,
        original_receipt,
        merchant,
        mint,
        token_program,
        registry_address,
        registry_slot,
        registry_commitment,
        registry_account_digest,
        request_digest,
        instruction_digest,
        status,
    })
}

fn parse_optional_binding_digest(
    value: Option<Vec<u8>>,
    field: &str,
) -> Result<[u8; 32], StorageError> {
    let value = value.ok_or_else(|| {
        StorageError::InvalidSettlementReceiptBinding(format!(
            "finalized receipt projection is missing {field}"
        ))
    })?;
    value.try_into().map_err(|_| {
        StorageError::InvalidSettlementReceiptBinding(format!(
            "finalized receipt projection {field} is not 32 bytes"
        ))
    })
}

fn parse_vault_reconciliation_run_view_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<VaultReconciliationRunView, StorageError> {
    Ok(VaultReconciliationRunView {
        id: row.get(0),
        status: row.get(1),
        opening_balance_atomic: row.get(2),
        receipt_tax_atomic: row.get(3),
        release_atomic: row.get(4),
        expected_balance_atomic: row.get(5),
        observed_balance_atomic: row.get(6),
        observed_slot: row.get(7),
        error_code: row.get(8),
        created_at: row.get(9),
        stale: row.get(10),
    })
}

fn parse_vault_reconciliation_target_view_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<VaultReconciliationTargetView, StorageError> {
    let latest_run = row
        .get::<_, Option<Uuid>>(12)
        .map(|id| VaultReconciliationRunView {
            id,
            status: row.get(13),
            opening_balance_atomic: row.get(14),
            receipt_tax_atomic: row.get(15),
            release_atomic: row.get(16),
            expected_balance_atomic: row.get(17),
            observed_balance_atomic: row.get(18),
            observed_slot: row.get(19),
            error_code: row.get(20),
            created_at: row.get(21),
            stale: row.get(22),
        });
    Ok(VaultReconciliationTargetView {
        target: VaultReconciliationTarget {
            id: row.get(0),
            merchant_id: row.get(1),
            route_id: row.get(2),
            stream_key: row.get(3),
            mint: row.get(4),
            token_program: row.get(5),
            tax_vault: row.get(6),
            opening_balance_atomic: parse_u64(row.get(7), "opening vault balance")?,
            opening_balance_slot: parse_u64(row.get(8), "opening vault slot")?,
            opening_reference: row.get(9),
            receipt_tax_atomic: parse_u64(row.get(10), "receipt tax aggregate")?,
            release_atomic: parse_u64(row.get(11), "vault release aggregate")?,
        },
        latest_run,
    })
}

fn validate_revenue_stream_actor(
    merchant_id: Uuid,
    plan_id: StreamPlanId,
    actor_subject: &str,
) -> Result<(), StorageError> {
    if plan_id.0.get_version_num() != 7 {
        return Err(StorageError::InvalidRevenueStream(
            "stream plan ID must be UUIDv7".into(),
        ));
    }
    validate_revenue_stream_actor_subject(merchant_id, actor_subject)
}

fn validate_revenue_stream_actor_subject(
    merchant_id: Uuid,
    actor_subject: &str,
) -> Result<(), StorageError> {
    if merchant_id.get_version_num() != 7
        || actor_subject.trim().is_empty()
        || actor_subject.len() > 256
        || actor_subject.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(StorageError::InvalidRevenueStream(
            "merchant identity or authenticated actor is invalid".into(),
        ));
    }
    Ok(())
}

fn validate_revenue_stream_job_identity(
    job: &RevenueStreamAccrualReconciliationJob,
    result_code: &str,
) -> Result<(), StorageError> {
    validate_indexer_stream_key(&job.stream_key)?;
    if job.claim_token.get_version_num() != 7
        || job.source_signature.trim().is_empty()
        || job.source_signature.len() > 128
        || result_code.trim().is_empty()
        || result_code.len() > MAX_REVENUE_STREAM_RECONCILIATION_ERROR_BYTES
        || result_code.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(StorageError::InvalidRevenueStreamAccrualJob(
            "job lease or result identity is invalid".into(),
        ));
    }
    Ok(())
}

fn validate_revenue_stream_accrual(accrual: &FinalizedAccrual) -> Result<(), StorageError> {
    if accrual.source_signature.0.len() != 64
        || accrual.source_signature.0.iter().all(|byte| *byte == 0)
        || accrual.finalized_slot == 0
        || accrual.gross_atomic == 0
        || accrual.recipient_atomic == 0
        || accrual.receipt_digest.0.iter().all(|byte| *byte == 0)
        || accrual.plan_digest.0.iter().all(|byte| *byte == 0)
        || accrual.policy_digest.0.iter().all(|byte| *byte == 0)
        || accrual.asset.0.iter().all(|byte| *byte == 0)
        || accrual.recipient.0.iter().all(|byte| *byte == 0)
    {
        return Err(StorageError::InvalidRevenueStream(
            "finalized accrual identity or amount is invalid".into(),
        ));
    }
    let expected_gross = accrual.tax_atomic.checked_add(accrual.net_atomic).ok_or(
        StorageError::InvalidRevenueStream("accrual amount exceeds u64".into()),
    )?;
    if expected_gross != accrual.gross_atomic || accrual.recipient_atomic > accrual.net_atomic {
        return Err(StorageError::InvalidRevenueStream(
            "accrual amounts do not conserve".into(),
        ));
    }
    Ok(())
}

fn validate_revenue_stream_accrual_binding(
    plan: &RevenueStreamPlanRecord,
    accrual: &FinalizedAccrual,
) -> Result<(), StorageError> {
    if !plan
        .spec
        .end_slot
        .is_none_or(|end| accrual.finalized_slot <= end)
        || accrual.finalized_slot < plan.spec.start_slot
        || accrual.plan_digest != plan.spec.plan_digest
        || accrual.policy_digest != plan.spec.policy_digest
        || accrual.asset != plan.spec.asset
        || accrual.recipient != plan.spec.recipient
    {
        return Err(StorageError::InvalidRevenueStream(
            "finalized accrual is outside the stream binding or window".into(),
        ));
    }
    Ok(())
}

fn validate_revenue_stream_correction(
    correction: &FinalizedCorrection,
) -> Result<(), StorageError> {
    if correction.correction_signature.0.len() != 64
        || correction
            .correction_signature
            .0
            .iter()
            .all(|byte| *byte == 0)
        || correction.source_signature.0.len() != 64
        || correction.source_signature.0.iter().all(|byte| *byte == 0)
        || correction.finalized_slot == 0
        || correction.plan_digest.0.iter().all(|byte| *byte == 0)
        || correction.amount_atomic == 0
    {
        return Err(StorageError::InvalidRevenueStream(
            "finalized correction identity or amount is invalid".into(),
        ));
    }
    Ok(())
}

fn validate_revenue_stream_release(release: &FinalizedRelease) -> Result<(), StorageError> {
    if release.claim_id.get_version_num() != 7
        || release.release_signature.0.len() != 64
        || release.release_signature.0.iter().all(|byte| *byte == 0)
        || release.finalized_slot == 0
        || release.claim_digest.0.iter().all(|byte| *byte == 0)
        || release.plan_digest.0.iter().all(|byte| *byte == 0)
        || release.asset.0.iter().all(|byte| *byte == 0)
        || release.recipient.0.iter().all(|byte| *byte == 0)
        || release.amount_atomic == 0
    {
        return Err(StorageError::InvalidRevenueStream(
            "finalized release identity or amount is invalid".into(),
        ));
    }
    Ok(())
}

async fn load_revenue_stream_plan_for_update(
    transaction: &deadpool_postgres::Transaction<'_>,
    plan_id: StreamPlanId,
    merchant_id: Uuid,
) -> Result<RevenueStreamPlanRecord, StorageError> {
    let row = transaction
        .query_opt(
            "SELECT id, merchant_id, idempotency_key, request_digest,
                    settlement_route_id, destination_route_id, asset, recipient, plan_digest, policy_digest,
                    start_slot::text, end_slot::text,
                    max_total_atomic::text, max_claim_atomic::text,
                    status, proposer_subject, approver_subject, operator_subject,
                    accrued_total_atomic::text, released_total_atomic::text,
                    next_nonce::text
             FROM revenue_stream_plans
             WHERE id = $1 AND merchant_id = $2
             FOR UPDATE",
            &[&plan_id.0, &merchant_id],
        )
        .await?
        .ok_or(StorageError::RevenueStreamPlanNotFound)?;
    parse_revenue_stream_plan_row(row)
}

fn parse_revenue_stream_accrual_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<FinalizedAccrual, StorageError> {
    let source_signature = row.get::<_, Vec<u8>>(0);
    if source_signature.len() != 64 || source_signature.iter().all(|byte| *byte == 0) {
        return Err(StorageError::InvalidRevenueStream(
            "stored source signature is invalid".into(),
        ));
    }
    Ok(FinalizedAccrual {
        source_signature: RevenueStreamSignature(source_signature),
        finalized_slot: parse_u64(row.get(1), "stored accrual slot")?,
        receipt_digest: RevenueStreamDigest(parse_revenue_stream_fixed_digest(
            row.get(2),
            "stored receipt digest",
        )?),
        plan_digest: RevenueStreamDigest(parse_revenue_stream_fixed_digest(
            row.get(3),
            "stored accrual plan digest",
        )?),
        policy_digest: RevenueStreamDigest(parse_revenue_stream_fixed_digest(
            row.get(4),
            "stored accrual policy digest",
        )?),
        asset: AssetId(parse_revenue_stream_fixed_digest(
            row.get(5),
            "stored accrual asset",
        )?),
        recipient: RecipientId(parse_revenue_stream_fixed_digest(
            row.get(6),
            "stored accrual recipient",
        )?),
        gross_atomic: parse_u64(row.get(7), "stored accrual gross")?,
        tax_atomic: parse_u64(row.get(8), "stored accrual tax")?,
        net_atomic: parse_u64(row.get(9), "stored accrual net")?,
        recipient_atomic: parse_u64(row.get(10), "stored accrual recipient amount")?,
    })
}

fn parse_revenue_stream_correction_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<FinalizedCorrection, StorageError> {
    let parse_signature =
        |index: usize, field: &str| -> Result<RevenueStreamSignature, StorageError> {
            let value: Vec<u8> = row.get(index);
            if value.len() != 64 || value.iter().all(|byte| *byte == 0) {
                return Err(StorageError::InvalidRevenueStream(format!(
                    "stored {field} is invalid"
                )));
            }
            Ok(RevenueStreamSignature(value))
        };
    Ok(FinalizedCorrection {
        correction_signature: parse_signature(0, "correction signature")?,
        source_signature: parse_signature(1, "correction source signature")?,
        finalized_slot: parse_u64(row.get(2), "stored correction slot")?,
        plan_digest: RevenueStreamDigest(parse_revenue_stream_fixed_digest(
            row.get(3),
            "stored correction plan digest",
        )?),
        decrease: row.get(4),
        amount_atomic: parse_u64(row.get(5), "stored correction amount")?,
    })
}

fn parse_revenue_stream_claim_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<RevenueStreamClaimRecord, StorageError> {
    let plan_id = StreamPlanId(row.get(1));
    if plan_id.0.get_version_num() != 7 {
        return Err(StorageError::InvalidRevenueStream(
            "stored claim plan ID must be UUIDv7".into(),
        ));
    }
    let id: Uuid = row.get(0);
    if id.get_version_num() != 7 {
        return Err(StorageError::InvalidRevenueStream(
            "stored claim ID must be UUIDv7".into(),
        ));
    }
    let status: String = row.get(11);
    if !matches!(status.as_str(), "pending" | "finalized" | "cancelled") {
        return Err(StorageError::InvalidRevenueStream(
            "stored claim status is invalid".into(),
        ));
    }
    let release_signature = row.get::<_, Option<Vec<u8>>>(12);
    if release_signature
        .as_ref()
        .is_some_and(|value| value.len() != 64 || value.iter().all(|byte| *byte == 0))
    {
        return Err(StorageError::InvalidRevenueStream(
            "stored release signature is invalid".into(),
        ));
    }
    let finalized_slot = row
        .get::<_, Option<String>>(13)
        .as_deref()
        .map(|value| parse_u64(value, "stored claim finalized slot"))
        .transpose()?;
    if (status == "finalized") != (release_signature.is_some() && finalized_slot.is_some()) {
        return Err(StorageError::InvalidRevenueStream(
            "stored claim finality state is invalid".into(),
        ));
    }
    Ok(RevenueStreamClaimRecord {
        id,
        plan_id,
        merchant_id: row.get(2),
        idempotency_key: row.get(3),
        amount_atomic: parse_u64(row.get(4), "stored claim amount")?,
        nonce: parse_u64(row.get(5), "stored claim nonce")?,
        accrual_total_atomic: parse_u64(row.get(6), "stored claim accrual total")?,
        plan_digest: parse_revenue_stream_fixed_digest(row.get(7), "stored claim plan digest")?,
        asset: parse_revenue_stream_fixed_digest(row.get(8), "stored claim asset")?,
        recipient: parse_revenue_stream_fixed_digest(row.get(9), "stored claim recipient")?,
        claim_digest: parse_revenue_stream_fixed_digest(row.get(10), "stored claim digest")?,
        status,
        release_signature,
        finalized_slot,
    })
}

fn parse_revenue_stream_plan_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<RevenueStreamPlanRecord, StorageError> {
    let id: Uuid = row.get(0);
    let merchant_id: Uuid = row.get(1);
    if id.get_version_num() != 7 || merchant_id.get_version_num() != 7 {
        return Err(StorageError::InvalidRevenueStream(
            "stored stream plan identities must be UUIDv7".into(),
        ));
    }
    let destination_route_id: Option<Uuid> = row.get(5);
    if destination_route_id.is_some_and(|value| value.get_version_num() != 7) {
        return Err(StorageError::InvalidRevenueStream(
            "stored revenue destination route ID must be UUIDv7".into(),
        ));
    }
    let spec = StreamPlanSpec {
        id: StreamPlanId(id),
        merchant_id,
        settlement_route_id: row.get(4),
        operator_subject: row.get(17),
        asset: AssetId(parse_revenue_stream_fixed_digest(row.get(6), "asset")?),
        recipient: RecipientId(parse_revenue_stream_fixed_digest(row.get(7), "recipient")?),
        plan_digest: RevenueStreamDigest(parse_revenue_stream_fixed_digest(
            row.get(8),
            "plan digest",
        )?),
        policy_digest: RevenueStreamDigest(parse_revenue_stream_fixed_digest(
            row.get(9),
            "policy digest",
        )?),
        start_slot: parse_u64(row.get(10), "revenue stream start slot")?,
        end_slot: row
            .get::<_, Option<String>>(11)
            .as_deref()
            .map(|value| parse_u64(value, "revenue stream end slot"))
            .transpose()?,
        max_total_atomic: parse_u64(row.get(12), "revenue stream total cap")?,
        max_claim_atomic: parse_u64(row.get(13), "revenue stream claim cap")?,
    };
    StreamPlan::new(spec.clone()).map_err(|error| {
        StorageError::InvalidRevenueStream(format!(
            "stored stream plan specification is invalid: {error}"
        ))
    })?;
    let status: String = row.get(14);
    if !matches!(
        status.as_str(),
        "proposed" | "approved" | "active" | "paused" | "cancelled" | "completed"
    ) {
        return Err(StorageError::InvalidRevenueStream(
            "stored stream plan status is invalid".into(),
        ));
    }
    let proposer_subject: String = row.get(15);
    let approver_subject: Option<String> = row.get(16);
    let operator_subject: Option<String> = row.get(17);
    if proposer_subject.trim().is_empty()
        || proposer_subject.len() > 256
        || proposer_subject.bytes().any(|byte| byte.is_ascii_control())
        || matches!(
            status.as_str(),
            "approved" | "active" | "paused" | "cancelled" | "completed"
        ) && approver_subject.is_none()
    {
        return Err(StorageError::InvalidRevenueStream(
            "stored stream plan authority state is invalid".into(),
        ));
    }
    if let Some(approver) = approver_subject.as_deref()
        && (approver.trim().is_empty()
            || approver.len() > 256
            || approver.bytes().any(|byte| byte.is_ascii_control())
            || approver == proposer_subject)
    {
        return Err(StorageError::InvalidRevenueStream(
            "stored stream plan approver is invalid".into(),
        ));
    }
    if let Some(operator) = operator_subject.as_deref()
        && (operator.trim().is_empty()
            || operator.len() > 256
            || operator.bytes().any(|byte| byte.is_ascii_control())
            || operator == proposer_subject)
    {
        return Err(StorageError::InvalidRevenueStream(
            "stored stream plan operator is invalid".into(),
        ));
    }
    let accrued_total_atomic = parse_u64(row.get(18), "revenue stream accrued total")?;
    let released_total_atomic = parse_u64(row.get(19), "revenue stream released total")?;
    let next_nonce = parse_u64(row.get(20), "revenue stream next nonce")?;
    if accrued_total_atomic > spec.max_total_atomic
        || released_total_atomic > accrued_total_atomic
        || next_nonce == 0
    {
        return Err(StorageError::InvalidRevenueStream(
            "stored stream plan totals or nonce are invalid".into(),
        ));
    }
    Ok(RevenueStreamPlanRecord {
        id: spec.id,
        merchant_id,
        idempotency_key: row.get(2),
        request_digest: parse_revenue_stream_fixed_digest(row.get(3), "request digest")?,
        destination_route_id,
        spec,
        status,
        proposer_subject,
        approver_subject,
        accrued_total_atomic,
        released_total_atomic,
        next_nonce,
    })
}

fn parse_revenue_stream_destination_route_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<RevenueStreamDestinationRoute, StorageError> {
    let id: Uuid = row.get(0);
    let merchant_id: Uuid = row.get(1);
    if id.get_version_num() != 7 || merchant_id.get_version_num() != 7 {
        return Err(StorageError::InvalidRevenueStream(
            "stored revenue destination route identities must be UUIDv7".into(),
        ));
    }
    let verification_digest = row
        .get::<_, Option<Vec<u8>>>(8)
        .ok_or_else(|| {
            StorageError::RevenueStreamDestinationRouteVerificationInvalid(
                "active route has no verification digest".into(),
            )
        })?
        .try_into()
        .map_err(|_| {
            StorageError::RevenueStreamDestinationRouteVerificationInvalid(
                "active route verification digest must contain 32 bytes".into(),
            )
        })?;
    let verification_commitment: String = row.get(10);
    if verification_commitment != "finalized" {
        return Err(StorageError::RevenueStreamDestinationRouteUnavailable);
    }
    let verification_evidence: Value = row.get(11);
    if !verification_evidence.is_object() {
        return Err(
            StorageError::RevenueStreamDestinationRouteVerificationInvalid(
                "active route verification evidence must be an object".into(),
            ),
        );
    }
    Ok(RevenueStreamDestinationRoute {
        id,
        merchant_id,
        route_key: row.get(2),
        version: row.get(3),
        recipient_owner: row.get(4),
        mint: row.get(5),
        destination_token_account: row.get(6),
        token_program: row.get(7),
        verification_digest: RevenueStreamDigest(verification_digest),
        verification_slot: parse_u64(row.get(9), "revenue destination verification slot")?,
        verification_commitment,
        verification_evidence,
    })
}

fn parse_revenue_stream_policy_reference(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<RevenueStreamPolicyReference, StorageError> {
    let id: Uuid = row.get(0);
    let rule_set_id: Uuid = row.get(1);
    if id.get_version_num() != 7 || rule_set_id.get_version_num() != 7 {
        return Err(StorageError::InvalidRevenueStream(
            "stored revenue policy reference identities must be UUIDv7".into(),
        ));
    }
    let policy_digest: Digest32 = row
        .get::<_, Vec<u8>>(6)
        .try_into()
        .map_err(|_| StorageError::JurisdictionPolicyDigestMismatch)?;
    let policy: ResolutionPolicy = serde_json::from_value(row.get(7)).map_err(|error| {
        StorageError::ReplayEvidenceIntegrity(format!(
            "revenue policy reference document is invalid: {error}"
        ))
    })?;
    let computed_digest = policy.canonical_digest().map_err(|error| {
        StorageError::ReplayEvidenceIntegrity(format!(
            "revenue policy reference canonicalization failed: {error}"
        ))
    })?;
    if computed_digest != policy_digest {
        return Err(StorageError::JurisdictionPolicyDigestMismatch);
    }
    Ok(RevenueStreamPolicyReference {
        id,
        rule_set_id,
        rule_external_id: row.get(2),
        rule_version: row.get(3),
        jurisdiction: row.get(4),
        regime: row.get(5),
        policy_digest,
        effective_from: row.get(8),
        effective_until: row.get(9),
    })
}

fn parse_revenue_stream_fixed_digest(
    value: Vec<u8>,
    field: &str,
) -> Result<[u8; 32], StorageError> {
    let digest: [u8; 32] = value.try_into().map_err(|_| {
        StorageError::InvalidRevenueStream(format!("stored {field} must contain 32 bytes"))
    })?;
    if digest.iter().all(|byte| *byte == 0) {
        return Err(StorageError::InvalidRevenueStream(format!(
            "stored {field} must not be zero"
        )));
    }
    Ok(digest)
}

fn validate_revenue_stream_plan_transition(
    command: &RevenueStreamPlanTransition,
) -> Result<(), StorageError> {
    if command.merchant_id.get_version_num() != 7
        || command.plan_id.0.get_version_num() != 7
        || command.actor_subject.trim().is_empty()
        || command.actor_subject.len() > 256
        || command
            .actor_subject
            .bytes()
            .any(|byte| byte.is_ascii_control())
        || !matches!(
            command.to_status.as_str(),
            "approved" | "active" | "paused" | "cancelled" | "completed"
        )
        || !command.evidence.is_object()
    {
        return Err(StorageError::InvalidRevenueStream(
            "transition identity, actor, status, or evidence is invalid".into(),
        ));
    }
    if serde_json::to_vec(&command.evidence)?.len() > MAX_REVENUE_STREAM_EVIDENCE_BYTES {
        return Err(StorageError::InvalidRevenueStream(
            "transition evidence exceeds the bounded size".into(),
        ));
    }
    if command.to_status == "completed" && command.observed_slot.is_none() {
        return Err(StorageError::InvalidRevenueStream(
            "completion requires an observed finalized slot".into(),
        ));
    }
    if command.to_status != "completed" && command.observed_slot.is_some() {
        return Err(StorageError::InvalidRevenueStream(
            "observed finalized slot is only valid for completion".into(),
        ));
    }
    if command.observed_slot == Some(0) {
        return Err(StorageError::InvalidRevenueStream(
            "observed finalized slot must be positive".into(),
        ));
    }
    Ok(())
}

struct RevenueStreamTransitionState<'a> {
    proposer_subject: &'a str,
    current_approver: Option<&'a str>,
    actor_subject: &'a str,
    accrued_total_atomic: u64,
    released_total_atomic: u64,
    end_slot: Option<u64>,
    observed_slot: Option<u64>,
}

fn validate_revenue_stream_plan_transition_state(
    from_status: &str,
    to_status: &str,
    state: RevenueStreamTransitionState<'_>,
) -> Result<Option<String>, StorageError> {
    let allowed = matches!(
        (from_status, to_status),
        ("proposed", "approved")
            | ("approved", "active")
            | ("approved", "cancelled")
            | ("active", "paused")
            | ("active", "cancelled")
            | ("active", "completed")
            | ("paused", "active")
            | ("paused", "cancelled")
            | ("paused", "completed")
    );
    if !allowed {
        return Err(StorageError::InvalidRevenueStreamPlanTransition);
    }
    if to_status == "approved" {
        if state.actor_subject == state.proposer_subject {
            return Err(StorageError::RevenueStreamPlanActorConflict);
        }
        return Ok(Some(state.actor_subject.to_string()));
    }
    let approver = state
        .current_approver
        .ok_or(StorageError::InvalidRevenueStreamPlanTransition)?;
    if state.actor_subject != approver {
        return Err(StorageError::RevenueStreamPlanActorConflict);
    }
    if to_status == "completed"
        && (state.released_total_atomic != state.accrued_total_atomic
            || state
                .end_slot
                .is_none_or(|end_slot| state.observed_slot.unwrap_or_default() < end_slot))
    {
        return Err(StorageError::RevenueStreamInsufficientAvailable);
    }
    Ok(Some(approver.to_string()))
}

fn revenue_stream_plan_action(
    from_status: &str,
    to_status: &str,
) -> Result<&'static str, StorageError> {
    match (from_status, to_status) {
        ("proposed", "approved") => Ok("approve"),
        ("approved", "active") => Ok("activate"),
        ("active", "paused") => Ok("pause"),
        ("paused", "active") => Ok("resume"),
        (_, "cancelled") => Ok("cancel"),
        ("active" | "paused", "completed") => Ok("complete"),
        _ => Err(StorageError::InvalidRevenueStreamPlanTransition),
    }
}

fn parse_vault_release_claim_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<VaultReleaseClaim, StorageError> {
    Ok(VaultReleaseClaim {
        id: row.get(0),
        merchant_id: row.get(1),
        idempotency_key: row.get(2),
        request_digest: parse_digest(row.get(3))?,
        mint: row.get(4),
        token_program: row.get(5),
        tax_vault: row.get(6),
        destination: row.get(7),
        claim_pda: row.get(8),
        nonce: parse_u64(row.get(9), "vault release nonce")?,
        amount_atomic: parse_u64(row.get(10), "vault release amount")?,
        reference_digest: parse_digest(row.get(11))?,
        status: row.get(12),
        proposer_subject: row.get(13),
        approver_subject: row.get(14),
        execute_after_unix: row.get(15),
        expires_at_unix: row.get(16),
        transaction_signature: row.get(17),
        slot: row
            .get::<_, Option<String>>(18)
            .as_deref()
            .map(|value| parse_u64(value, "vault release slot"))
            .transpose()?,
        last_error_code: row.get(19),
    })
}

fn parse_settlement_adjustment_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<SettlementAdjustmentRecord, StorageError> {
    let id: Uuid = row.get(0);
    let merchant_id: Uuid = row.get(1);
    let asset_id: [u8; 32] = row.get::<_, Vec<u8>>(6).try_into().map_err(|_| {
        StorageError::InvalidSettlementAdjustment("asset ID is not 32 bytes".to_string())
    })?;
    let kind = adjustment_kind_from_db(row.get(7))?;
    let delta = AdjustmentDelta {
        gross_atomic: parse_i128(row.get(8), "gross adjustment delta")?,
        tax_atomic: parse_i128(row.get(9), "tax adjustment delta")?,
        net_atomic: parse_i128(row.get(10), "net adjustment delta")?,
    };
    delta
        .validate()
        .map_err(|error| StorageError::InvalidSettlementAdjustment(error.to_string()))?;
    let adjustment_digest = parse_digest(row.get(4))?;
    let proposal = Adjustment::propose(AdjustmentProposal {
        id,
        merchant_id,
        original_settlement_id: row.get(5),
        asset_id,
        kind,
        delta,
        reason_code: row.get(11),
        evidence_digest: parse_digest(row.get(12))?,
        source_calculation_id: row.get(13),
        replacement_calculation_id: row.get(14),
        proposed_by: row.get(16),
    })
    .map_err(|error| StorageError::InvalidSettlementAdjustment(error.to_string()))?;
    if proposal
        .digest()
        .map_err(|error| StorageError::InvalidSettlementAdjustment(error.to_string()))?
        != adjustment_digest
    {
        return Err(StorageError::InvalidSettlementAdjustment(
            "stored adjustment digest does not match canonical evidence".to_string(),
        ));
    }
    Ok(SettlementAdjustmentRecord {
        id,
        merchant_id,
        idempotency_key: row.get(2),
        request_digest: parse_digest(row.get(3))?,
        adjustment_digest,
        original_settlement_id: row.get(5),
        asset_id,
        kind,
        delta,
        reason_code: row.get(11),
        evidence_digest: parse_digest(row.get(12))?,
        source_calculation_id: row.get(13),
        replacement_calculation_id: row.get(14),
        status: row.get(15),
        proposer_subject: row.get(16),
        approver_subject: row.get(17),
    })
}

fn parse_refund_funding_configuration_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<RefundFundingConfiguration, StorageError> {
    let parse_optional_digest =
        |index: usize, field: &str| -> Result<Option<Digest32>, StorageError> {
            row.get::<_, Option<Vec<u8>>>(index)
                .map(|value| {
                    value.try_into().map_err(|_| {
                        StorageError::InvalidRefundFundingConfiguration(format!(
                            "{field} is not 32 bytes"
                        ))
                    })
                })
                .transpose()
        };
    Ok(RefundFundingConfiguration {
        id: row.get(0),
        merchant_id: row.get(1),
        idempotency_key: row.get(2),
        request_digest: parse_digest(row.get(3))?,
        configuration_digest: parse_digest(row.get(4))?,
        configuration_key: row.get(5),
        version: row.get(6),
        status: row.get(7),
        merchant_account: row.get(8),
        mint: row.get(9),
        tax_vault: row.get(10),
        vault_authority: row.get(11),
        refund_funder: row.get(12),
        refund_funder_subject: row.get(13),
        net_source: row.get(14),
        token_program: row.get(15),
        proposer_subject: row.get(16),
        approver_subject: row.get(17),
        verification_digest: parse_optional_digest(18, "verification digest")?,
        verification_slot: row.get(19),
        verification_commitment: row.get(20),
        verification_evidence: row.get(21),
        verifier_version: row.get(22),
        verified_at: row.get(23),
        effective_from: row.get(24),
        effective_until: row.get(25),
    })
}

fn parse_refund_claim_intent_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<RefundClaimIntent, StorageError> {
    let mint: [u8; 32] = row.get::<_, Vec<u8>>(8).try_into().map_err(|_| {
        StorageError::InvalidRefundClaimIntent("stored mint is not 32 bytes".into())
    })?;
    let claim: [u8; 32] = row.get::<_, Vec<u8>>(14).try_into().map_err(|_| {
        StorageError::InvalidRefundClaimIntent("stored claim PDA is not 32 bytes".into())
    })?;
    Ok(RefundClaimIntent {
        id: row.get(0),
        merchant_id: row.get(1),
        idempotency_key: row.get(2),
        request_digest: parse_digest(row.get(3))?,
        adjustment_id: row.get(4),
        funding_configuration_id: row.get(5),
        receipt_signature: row.get(6),
        receipt_slot: parse_u64(row.get(7), "receipt slot")?,
        mint,
        token_program: row.get(9),
        gross_atomic: parse_u64(row.get(10), "gross amount")?,
        tax_atomic: parse_u64(row.get(11), "tax amount")?,
        net_atomic: parse_u64(row.get(12), "net amount")?,
        claim_nonce: parse_u64(row.get(13), "claim nonce")?,
        claim,
        instruction_digest: parse_digest(row.get(15))?,
        artifact_digest: parse_digest(row.get(16))?,
        status: row.get(17),
        signed_transaction_digest: row
            .get::<_, Option<Vec<u8>>>(18)
            .map(|value| {
                value.try_into().map_err(|_| {
                    StorageError::InvalidRefundClaimIntent(
                        "stored signed transaction digest is not 32 bytes".into(),
                    )
                })
            })
            .transpose()?,
        signature: row.get(19),
        submitted_slot: row
            .get::<_, Option<String>>(20)
            .as_ref()
            .map(|value| parse_u64(value, "submitted slot"))
            .transpose()?,
        commitment: row.get(21),
        last_error_code: row.get(22),
        unsigned_artifact: row.get(23),
    })
}

fn parse_refund_cancellation_intent_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<RefundCancellationIntent, StorageError> {
    let bytes = |index: usize, field: &str| -> Result<[u8; 32], StorageError> {
        row.get::<_, Vec<u8>>(index).try_into().map_err(|_| {
            StorageError::InvalidRefundCancellationIntent(format!("stored {field} is not 32 bytes"))
        })
    };
    Ok(RefundCancellationIntent {
        id: row.get(0),
        merchant_id: row.get(1),
        idempotency_key: row.get(2),
        request_digest: parse_digest(row.get(3))?,
        claim_intent_id: row.get(4),
        adjustment_id: row.get(5),
        merchant: bytes(6, "merchant")?,
        mint: bytes(7, "mint")?,
        adjustment: bytes(8, "adjustment")?,
        claim_nonce: parse_u64(row.get(9), "refund cancellation claim nonce")?,
        claim: bytes(10, "claim PDA")?,
        governance_authority: row.get(11),
        governance_subject: row.get(12),
        instruction_digest: parse_digest(row.get(13))?,
        artifact_digest: parse_digest(row.get(14))?,
        status: row.get(15),
        signed_transaction_digest: row
            .get::<_, Option<Vec<u8>>>(16)
            .map(|value| {
                value.try_into().map_err(|_| {
                    StorageError::InvalidRefundCancellationIntent(
                        "stored signed transaction digest is not 32 bytes".into(),
                    )
                })
            })
            .transpose()?,
        signature: row.get(17),
        submitted_slot: row
            .get::<_, Option<String>>(18)
            .as_ref()
            .map(|value| parse_u64(value, "refund cancellation submitted slot"))
            .transpose()?,
        commitment: row.get(19),
        last_error_code: row.get(20),
        unsigned_artifact: row.get(21),
    })
}

fn parse_refund_execute_intent_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<RefundExecuteIntent, StorageError> {
    let bytes = |index: usize, field: &str| -> Result<[u8; 32], StorageError> {
        row.get::<_, Vec<u8>>(index).try_into().map_err(|_| {
            StorageError::InvalidRefundExecuteIntent(format!("stored {field} is not 32 bytes"))
        })
    };
    Ok(RefundExecuteIntent {
        id: row.get(0),
        merchant_id: row.get(1),
        idempotency_key: row.get(2),
        request_digest: parse_digest(row.get(3))?,
        claim_intent_id: row.get(4),
        funding_configuration_id: row.get(5),
        funding_configuration_digest: parse_digest(row.get(6))?,
        claim: bytes(7, "claim PDA")?,
        adjustment_id: row.get(8),
        original_receipt: bytes(9, "original receipt")?,
        merchant: bytes(10, "merchant")?,
        mint: bytes(11, "mint")?,
        token_program: row.get(12),
        approver: row.get(13),
        refund_funder: row.get(14),
        refund_funder_subject: row.get(15),
        instruction_digest: parse_digest(row.get(16))?,
        artifact_digest: parse_digest(row.get(17))?,
        status: row.get(18),
        signed_transaction_digest: row
            .get::<_, Option<Vec<u8>>>(19)
            .map(|value| {
                value.try_into().map_err(|_| {
                    StorageError::InvalidRefundExecuteIntent(
                        "stored signed transaction digest is not 32 bytes".into(),
                    )
                })
            })
            .transpose()?,
        signature: row.get(20),
        submitted_slot: row
            .get::<_, Option<String>>(21)
            .as_ref()
            .map(|value| parse_u64(value, "submitted slot"))
            .transpose()?,
        commitment: row.get(22),
        last_error_code: row.get(23),
        unsigned_artifact: row.get(24),
    })
}

fn parse_finalized_settlement_adjustment_binding(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<FinalizedSettlementAdjustmentBinding, StorageError> {
    let digest = |index: usize, field: &str| -> Result<Digest32, StorageError> {
        row.get::<_, Vec<u8>>(index).try_into().map_err(|_| {
            StorageError::InvalidSettlementAdjustment(format!(
                "finalized adjustment {field} is not 32 bytes"
            ))
        })
    };
    let adjustment = digest(0, "PDA")?;
    let original_receipt = digest(1, "receipt")?;
    let adjustment_digest = digest(5, "digest")?;
    let evidence_digest = digest(6, "evidence digest")?;
    let nonce = parse_u64(row.get(2), "finalized adjustment nonce")?;
    let slot = parse_u64(row.get(4), "finalized adjustment slot")?;
    let signature: String = row.get(3);
    if adjustment == [0; 32]
        || original_receipt == [0; 32]
        || adjustment_digest == [0; 32]
        || evidence_digest == [0; 32]
        || nonce == 0
        || slot == 0
        || signature.trim().is_empty()
        || signature.len() > 128
    {
        return Err(StorageError::InvalidSettlementAdjustment(
            "finalized adjustment identity is incomplete".into(),
        ));
    }
    Ok(FinalizedSettlementAdjustmentBinding {
        adjustment,
        original_receipt,
        nonce,
        signature,
        slot,
        adjustment_digest,
        evidence_digest,
    })
}

fn validate_refund_cancellation_intent_command(
    command: &CreateRefundCancellationIntentCommand,
) -> Result<(), StorageError> {
    let inputs = &command.inputs;
    let artifact = &command.artifact;
    validate_cancellation_artifact(inputs, artifact)
        .map_err(|error| StorageError::InvalidRefundCancellationIntent(error.to_string()))?;
    if command.merchant_id.get_version_num() != 7
        || inputs.merchant_id != command.merchant_id
        || inputs.claim_intent_id.get_version_num() != 7
        || inputs.adjustment_id.get_version_num() != 7
        || command.request_digest == [0; 32]
        || command.request_digest != cancellation_request_digest(inputs)
        || command.idempotency_key.trim().is_empty()
        || command.idempotency_key.len() > 200
        || command.idempotency_key != command.idempotency_key.trim()
        || artifact.schema != "refund-claim-cancellation-artifact/v1"
        || artifact.merchant_id != command.merchant_id
        || artifact.claim_intent_id != inputs.claim_intent_id
        || artifact.adjustment_id != inputs.adjustment_id
        || artifact.merchant != Pubkey::new_from_array(inputs.merchant).to_string()
        || artifact.mint != Pubkey::new_from_array(inputs.mint).to_string()
        || artifact.adjustment != Pubkey::new_from_array(inputs.adjustment).to_string()
        || artifact.governance_authority != inputs.governance_authority
        || artifact.governance_subject != inputs.governance_subject
        || artifact.instruction.status != "prepared_unsubmitted"
        || artifact.instruction.program_id != settlement_client::SETTLEMENT_PROGRAM_ID
        || artifact.instruction.claim_nonce != inputs.claim_nonce
        || artifact.instruction.accounts.len() != 5
        || artifact.instruction.accounts[0].pubkey != inputs.governance_authority
    {
        return Err(StorageError::InvalidRefundCancellationIntent(
            "cancellation intent identity or artifact is inconsistent".into(),
        ));
    }
    let unsigned_artifact = artifact
        .unsigned_artifact()
        .map_err(|error| StorageError::InvalidRefundCancellationIntent(error.to_string()))?;
    let artifact_bytes = serde_json::to_vec(&unsigned_artifact)?;
    if artifact_bytes.is_empty() || artifact_bytes.len() > 65_536 {
        return Err(StorageError::InvalidRefundCancellationIntent(
            "unsigned refund cancellation artifact exceeds the bounded persistence limit".into(),
        ));
    }
    Ok(())
}

fn validate_refund_claim_intent_command(
    command: &CreateRefundClaimIntentCommand,
) -> Result<(), StorageError> {
    let inputs = &command.inputs;
    let artifact = &command.artifact;
    let gross_atomic = inputs.gross_atomic.to_string();
    let tax_atomic = inputs.tax_atomic.to_string();
    let net_atomic = inputs.net_atomic.to_string();
    validate_claim_artifact(inputs, artifact)
        .map_err(|error| StorageError::InvalidRefundClaimIntent(error.to_string()))?;
    if command.merchant_id.get_version_num() != 7
        || inputs.merchant_id != command.merchant_id
        || inputs.adjustment_id.get_version_num() != 7
        || inputs.funding_configuration_id.get_version_num() != 7
        || command.request_digest == [0; 32]
        || command.request_digest != claim_request_digest(inputs)
        || command.idempotency_key.trim().is_empty()
        || command.idempotency_key.len() > 200
        || command.idempotency_key != command.idempotency_key.trim()
        || inputs.claim_nonce == 0
        || inputs.receipt_slot == 0
        || inputs.gross_atomic == 0
        || inputs.tax_atomic.checked_add(inputs.net_atomic) != Some(inputs.gross_atomic)
        || artifact.schema != "refund-movement-artifact/v1"
        || artifact.merchant_id != command.merchant_id
        || artifact.adjustment_id != inputs.adjustment_id
        || artifact.funding_configuration_id != inputs.funding_configuration_id
        || artifact.funding_configuration_digest != inputs.funding_configuration_digest
        || artifact.receipt_signature != inputs.receipt_signature
        || artifact.receipt_slot != Some(inputs.receipt_slot)
        || artifact.authority_subject != inputs.proposer
        || artifact.proposer_subject != inputs.proposer_subject
        || artifact.refund_funder_subject != inputs.refund_funder_subject
        || artifact.proposer != inputs.proposer
        || artifact.refund_funder != inputs.refund_funder
        || artifact.merchant != Pubkey::new_from_array(inputs.merchant).to_string()
        || artifact.mint != Pubkey::new_from_array(inputs.mint).to_string()
        || artifact.token_program != inputs.token_program
        || artifact.tax_vault != inputs.tax_vault
        || artifact.net_source != inputs.net_source
        || artifact.gross_atomic.as_deref() != Some(gross_atomic.as_str())
        || artifact.tax_atomic.as_deref() != Some(tax_atomic.as_str())
        || artifact.net_atomic.as_deref() != Some(net_atomic.as_str())
        || artifact.execute_after_unix != Some(inputs.execute_after_unix)
        || artifact.expires_at_unix != Some(inputs.expires_at_unix)
        || artifact.instruction.status != "prepared_unsubmitted"
        || artifact.instruction.claim_nonce != inputs.claim_nonce
        || artifact.instruction.program_id != settlement_client::SETTLEMENT_PROGRAM_ID
    {
        return Err(StorageError::InvalidRefundClaimIntent(
            "claim intent identity, artifact, or financial inputs are inconsistent".into(),
        ));
    }
    let unsigned_artifact = artifact
        .unsigned_artifact()
        .map_err(|error| StorageError::InvalidRefundClaimIntent(error.to_string()))?;
    let artifact_bytes = serde_json::to_vec(&unsigned_artifact)?;
    if artifact_bytes.is_empty() || artifact_bytes.len() > 65_536 {
        return Err(StorageError::InvalidRefundClaimIntent(
            "unsigned refund artifact exceeds the bounded persistence limit".into(),
        ));
    }
    Ok(())
}

fn validate_refund_execute_intent_command(
    command: &CreateRefundExecuteIntentCommand,
) -> Result<(), StorageError> {
    let inputs = &command.inputs;
    let artifact = &command.artifact;
    validate_execute_artifact(inputs, artifact)
        .map_err(|error| StorageError::InvalidRefundExecuteIntent(error.to_string()))?;
    if command.merchant_id.get_version_num() != 7
        || inputs.merchant_id != command.merchant_id
        || inputs.claim_intent_id != command.claim_intent_id
        || inputs.adjustment_id.get_version_num() != 7
        || inputs.funding_configuration_id.get_version_num() != 7
        || inputs.claim_intent_id.get_version_num() != 7
        || command.claim_intent_id.get_version_num() != 7
        || command.request_digest == [0; 32]
        || command.request_digest != execute_request_digest(inputs)
        || command.idempotency_key.trim().is_empty()
        || command.idempotency_key.len() > 200
        || command.idempotency_key != command.idempotency_key.trim()
        || inputs.claim_signature.trim().is_empty()
        || inputs.claim_slot == 0
        || inputs.receipt_slot == 0
        || inputs.claim_nonce == 0
        || artifact.schema != "refund-movement-artifact/v1"
        || artifact.merchant_id != command.merchant_id
        || artifact.adjustment_id != inputs.adjustment_id
        || artifact.funding_configuration_id != inputs.funding_configuration_id
        || artifact.funding_configuration_digest != inputs.funding_configuration_digest
        || artifact.receipt_signature != inputs.receipt_signature
        || artifact.receipt_slot != Some(inputs.receipt_slot)
        || artifact.claim_signature.as_deref() != Some(inputs.claim_signature.as_str())
        || artifact.claim_slot != Some(inputs.claim_slot)
        || artifact.authority_subject != inputs.approver
        || !artifact.proposer.is_empty()
        || !artifact.proposer_subject.is_empty()
        || artifact.refund_funder != inputs.refund_funder
        || artifact.refund_funder_subject != inputs.refund_funder_subject
        || artifact.merchant != Pubkey::new_from_array(inputs.merchant).to_string()
        || artifact.mint != Pubkey::new_from_array(inputs.mint).to_string()
        || artifact.token_program != inputs.token_program
        || artifact.tax_vault != inputs.tax_vault
        || artifact.net_source != inputs.net_source
        || artifact.gross_atomic.is_some()
        || artifact.tax_atomic.is_some()
        || artifact.net_atomic.is_some()
        || artifact.execute_after_unix.is_some()
        || artifact.expires_at_unix.is_some()
        || artifact.instruction.status != "prepared_unsubmitted"
        || artifact.instruction.claim_nonce != inputs.claim_nonce
        || artifact.instruction.program_id != settlement_client::SETTLEMENT_PROGRAM_ID
        || artifact
            .instruction
            .accounts
            .first()
            .map(|account| account.pubkey.as_str())
            != Some(inputs.approver.as_str())
        || artifact
            .instruction
            .accounts
            .get(1)
            .map(|account| account.pubkey.as_str())
            != Some(inputs.refund_funder.as_str())
    {
        return Err(StorageError::InvalidRefundExecuteIntent(
            "execute intent identity, artifact, or finalized claim evidence is inconsistent".into(),
        ));
    }
    let unsigned_artifact = artifact
        .unsigned_artifact()
        .map_err(|error| StorageError::InvalidRefundExecuteIntent(error.to_string()))?;
    let artifact_bytes = serde_json::to_vec(&unsigned_artifact)?;
    if artifact_bytes.is_empty() || artifact_bytes.len() > 65_536 {
        return Err(StorageError::InvalidRefundExecuteIntent(
            "unsigned refund execute artifact exceeds the bounded persistence limit".into(),
        ));
    }
    Ok(())
}

fn validate_refund_funding_configuration_command(
    command: &CreateRefundFundingConfigurationCommand,
) -> Result<(), StorageError> {
    if command.merchant_id.get_version_num() != 7 {
        return Err(StorageError::InvalidRefundFundingConfiguration(
            "merchant ID must be UUIDv7".into(),
        ));
    }
    if command.actor_subject.trim().is_empty()
        || command.actor_subject.len() > 256
        || command
            .actor_subject
            .bytes()
            .any(|byte| byte.is_ascii_control())
    {
        return Err(StorageError::InvalidRefundFundingConfiguration(
            "proposer subject is invalid".into(),
        ));
    }
    if command.refund_funder_subject.trim().is_empty()
        || command.refund_funder_subject.len() > 256
        || command
            .refund_funder_subject
            .bytes()
            .any(|byte| byte.is_ascii_control())
    {
        return Err(StorageError::InvalidRefundFundingConfiguration(
            "refund funder subject is invalid".into(),
        ));
    }
    for (field, value, max) in [
        ("idempotency key", command.idempotency_key.as_str(), 200),
        ("configuration key", command.configuration_key.as_str(), 128),
    ] {
        if value.trim().is_empty() || value.len() > max || value != value.trim() {
            return Err(StorageError::InvalidRefundFundingConfiguration(format!(
                "{field} is outside its bounded text domain"
            )));
        }
    }
    if command.version <= 0 {
        return Err(StorageError::InvalidRefundFundingConfiguration(
            "configuration version must be positive".into(),
        ));
    }
    for (field, value) in [
        ("merchant account", command.merchant_account.as_str()),
        ("mint", command.mint.as_str()),
        ("tax vault", command.tax_vault.as_str()),
        ("vault authority", command.vault_authority.as_str()),
        ("refund funder", command.refund_funder.as_str()),
        ("net source", command.net_source.as_str()),
        ("token program", command.token_program.as_str()),
    ] {
        if value.trim().len() < 32 || value.len() > 44 || value != value.trim() {
            return Err(StorageError::InvalidRefundFundingConfiguration(format!(
                "{field} is outside the bounded Solana address domain"
            )));
        }
        let parsed = Pubkey::from_str(value).map_err(|_| {
            StorageError::InvalidRefundFundingConfiguration(format!(
                "{field} is not a valid Solana public key"
            ))
        })?;
        if parsed.to_string() != value {
            return Err(StorageError::InvalidRefundFundingConfiguration(format!(
                "{field} is not canonically encoded"
            )));
        }
    }
    if !matches!(
        command.token_program.as_str(),
        SPL_TOKEN_PROGRAM_ID | TOKEN_2022_PROGRAM_ID
    ) {
        return Err(StorageError::InvalidRefundFundingConfiguration(
            "token program is not an explicitly supported program".into(),
        ));
    }
    let addresses = [
        command.tax_vault.as_str(),
        command.net_source.as_str(),
        command.refund_funder.as_str(),
        command.mint.as_str(),
    ];
    for (index, address) in addresses.iter().enumerate() {
        if addresses[..index].contains(address) {
            return Err(StorageError::InvalidRefundFundingConfiguration(
                "funding accounts and mint must be pairwise distinct".into(),
            ));
        }
    }
    if command.effective_from_unix <= 0
        || command
            .effective_until_unix
            .is_some_and(|until| until <= command.effective_from_unix)
    {
        return Err(StorageError::InvalidRefundFundingConfiguration(
            "effective window is invalid".into(),
        ));
    }
    Ok(())
}

fn refund_funding_configuration_digest(
    command: &CreateRefundFundingConfigurationCommand,
) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(b"solana-tax-protocol/refund-funding-configuration/v1");
    hasher.update(command.merchant_id.as_bytes());
    write_projection_text(&mut hasher, &command.configuration_key);
    hasher.update(command.version.to_le_bytes());
    for value in [
        &command.merchant_account,
        &command.mint,
        &command.tax_vault,
        &command.vault_authority,
        &command.refund_funder,
        &command.refund_funder_subject,
        &command.net_source,
        &command.token_program,
    ] {
        write_projection_text(&mut hasher, value);
    }
    hasher.update(command.effective_from_unix.to_le_bytes());
    hasher.update(
        command
            .effective_until_unix
            .unwrap_or_default()
            .to_le_bytes(),
    );
    hasher.finalize().into()
}

fn refund_funding_configuration_request_digest(
    command: &CreateRefundFundingConfigurationCommand,
    configuration_digest: Digest32,
) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(b"solana-tax-protocol/refund-funding-request/v1");
    hasher.update(command.merchant_id.as_bytes());
    write_projection_text(&mut hasher, &command.idempotency_key);
    write_projection_text(&mut hasher, &command.actor_subject);
    hasher.update(configuration_digest);
    hasher.finalize().into()
}

fn validate_refund_funding_configuration_transition(
    command: &RefundFundingConfigurationTransition,
) -> Result<(), StorageError> {
    if command.merchant_id.get_version_num() != 7
        || command.configuration_id.get_version_num() != 7
        || command.actor_subject.trim().is_empty()
        || command.actor_subject.len() > 256
        || command
            .actor_subject
            .bytes()
            .any(|byte| byte.is_ascii_control())
        || !command.evidence.is_object()
    {
        return Err(StorageError::InvalidRefundFundingConfiguration(
            "transition identity, actor, or evidence is invalid".into(),
        ));
    }
    if serde_json::to_vec(&command.evidence)?.len() > MAX_VAULT_RELEASE_EVIDENCE_BYTES {
        return Err(StorageError::InvalidRefundFundingConfiguration(
            "transition evidence exceeds the bounded size".into(),
        ));
    }
    if !matches!(
        command.to_status.as_str(),
        "approved" | "suspended" | "retired"
    ) {
        return Err(StorageError::InvalidRefundFundingConfigurationTransition);
    }
    Ok(())
}

fn parse_u64(value: &str, field: &str) -> Result<u64, StorageError> {
    value
        .parse()
        .map_err(|_| StorageError::InvalidIndexerState(format!("{field} is not a u64")))
}

fn valid_adjustment_authority_role(role: &str) -> bool {
    matches!(
        role,
        "proposer"
            | "approver"
            | "refund_funder"
            | "governance"
            | "revenue_stream_proposer"
            | "revenue_stream_approver"
            | "revenue_stream_operator"
            | "revenue_stream_governance"
    )
}

fn validate_adjustment_authority_challenge_command(
    command: &CreateAdjustmentAuthorityChallenge,
) -> Result<(), StorageError> {
    if command.merchant_id.get_version_num() != 7 {
        return Err(StorageError::InvalidAdjustmentAuthorityChallenge(
            "merchant ID must be UUIDv7".into(),
        ));
    }
    if command.subject.trim().is_empty()
        || command.subject.len() > 256
        || command.subject.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(StorageError::InvalidAdjustmentAuthorityChallenge(
            "authority subject is invalid".into(),
        ));
    }
    if !valid_adjustment_authority_role(&command.role) {
        return Err(StorageError::InvalidAdjustmentAuthorityChallenge(
            "authority role is invalid".into(),
        ));
    }
    if command.now_unix <= 0 || !(60..=300).contains(&command.ttl_seconds) {
        return Err(StorageError::InvalidAdjustmentAuthorityChallenge(
            "challenge clock or TTL is outside the bounded range".into(),
        ));
    }
    Ok(())
}

fn validate_adjustment_authority_binding_command(
    command: &BindAdjustmentAuthority,
) -> Result<(), StorageError> {
    if command.merchant_id.get_version_num() != 7
        || command.challenge_id.get_version_num() != 7
        || command.public_key == [0; 32]
        || command.proof_signature == [0; 64]
        || command.now_unix <= 0
        || !valid_adjustment_authority_role(&command.role)
        || command.subject.trim().is_empty()
        || command.subject.len() > 256
        || command.subject.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(StorageError::InvalidAdjustmentAuthorityChallenge(
            "authority proof input is invalid".into(),
        ));
    }
    Ok(())
}

fn validate_adjustment_evidence_intent_command(
    command: &CreateSettlementAdjustmentEvidenceIntent,
) -> Result<(), StorageError> {
    if command.adjustment_id.get_version_num() != 7
        || command.merchant_id.get_version_num() != 7
        || command.proposer == [0; 32]
        || command.approver == [0; 32]
        || command.proposer == command.approver
        || command.original_receipt == [0; 32]
        || command.merchant == [0; 32]
        || command.mint == [0; 32]
        || command.registry_address == [0; 32]
        || command.registry_slot == 0
        || command.registry_commitment != "finalized"
        || command.registry_account_digest == [0; 32]
        || command.request_digest == [0; 32]
        || command.token_program.len() < 32
        || command.token_program.len() > 44
    {
        return Err(StorageError::InvalidAdjustmentEvidenceIntent(
            "adjustment evidence intent input is invalid".into(),
        ));
    }
    Ok(())
}

fn parse_i128(value: &str, field: &str) -> Result<i128, StorageError> {
    value
        .parse()
        .map_err(|_| StorageError::InvalidIndexerState(format!("{field} is not an i128")))
}

fn write_projection_text(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn parse_checkpoint_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<Checkpoint, StorageError> {
    Ok(Checkpoint {
        next_slot: parse_u64(row.get(0), "next_slot")?,
        finalized_slot: parse_u64(row.get(1), "finalized_slot")?,
        generation: parse_u64(row.get(2), "generation")?,
    })
}

fn parse_receipt_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<IndexedReceipt, StorageError> {
    let to_digest = |index: usize, field: &str| -> Result<[u8; 32], StorageError> {
        let value: Option<Vec<u8>> = row.get(index);
        value
            .ok_or_else(|| {
                StorageError::InvalidIndexerState(format!(
                    "indexed receipt {field} identity is incomplete"
                ))
            })?
            .try_into()
            .map_err(|_| {
                StorageError::InvalidIndexerState(format!(
                    "indexed receipt {field} is not 32 bytes"
                ))
            })
    };
    let evidence_digest = to_digest(5, "evidence digest")?;
    Ok(IndexedReceipt {
        signature: row.get(0),
        slot: parse_u64(row.get(1), "slot")?,
        receipt: to_digest(6, "receipt")?,
        payer: to_digest(7, "payer")?,
        merchant: to_digest(8, "merchant")?,
        mint: to_digest(9, "mint")?,
        nonce: parse_u64(row.get(10), "nonce")?,
        gross_atomic: parse_u64(row.get(2), "gross_atomic")?,
        tax_atomic: parse_u64(row.get(3), "tax_atomic")?,
        net_atomic: parse_u64(row.get(4), "net_atomic")?,
        policy_digest: to_digest(11, "policy digest")?,
        evidence_digest,
    })
}

fn parse_split_settlement_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<IndexedSplitSettlement, StorageError> {
    let to_digest = |index: usize, field: &str| -> Result<[u8; 32], StorageError> {
        let value: Vec<u8> = row.get(index);
        value
            .try_into()
            .map_err(|_| StorageError::InvalidIndexerState(format!("{field} is not 32 bytes")))
    };
    Ok(IndexedSplitSettlement {
        signature: row.get(0),
        slot: parse_u64(row.get(1), "split slot")?,
        receipt: to_digest(2, "receipt")?,
        payer: to_digest(3, "payer")?,
        merchant: to_digest(4, "merchant")?,
        mint: to_digest(5, "mint")?,
        nonce: parse_u64(row.get(6), "split nonce")?,
        gross_atomic: parse_u64(row.get(7), "split gross_atomic")?,
        tax_atomic: parse_u64(row.get(8), "split tax_atomic")?,
        net_atomic: parse_u64(row.get(9), "split net_atomic")?,
        split_plan_version: parse_u64(row.get(10), "split_plan_version")?,
        split_plan_digest: to_digest(11, "split_plan_digest")?,
        policy_digest: to_digest(12, "policy_digest")?,
        evidence_digest: to_digest(13, "split evidence_digest")?,
    })
}

async fn persist_indexed_settlement_adjustments(
    transaction: &deadpool_postgres::tokio_postgres::Transaction<'_>,
    stream_key: &str,
    records: &[IndexedSettlementAdjustment],
) -> Result<(), StorageError> {
    for record in records {
        let existing = transaction
            .query_opt(
                "SELECT signature, slot::text, event_index::text, adjustment,
                        original_receipt, merchant, mint, nonce::text, kind::text,
                        gross_sign::text, gross_magnitude::text, tax_sign::text,
                        tax_magnitude::text, net_sign::text, net_magnitude::text,
                        adjustment_digest, adjustment_evidence_digest,
                        evidence_digest, approver
                 FROM indexed_settlement_adjustments
                 WHERE stream_key = $1 AND signature = $2
                   AND event_index = $3::text::smallint
                 FOR UPDATE",
                &[
                    &stream_key,
                    &record.signature,
                    &record.event_index.to_string(),
                ],
            )
            .await?;
        if let Some(row) = existing {
            let existing_record = parse_indexed_settlement_adjustment_row(row)?;
            merge_settlement_adjustment(Some(existing_record), record.clone())?;
            continue;
        }
        let identity_conflict = transaction
            .query_opt(
                "SELECT 1 FROM indexed_settlement_adjustments
                 WHERE stream_key = $1 AND adjustment = $2
                 LIMIT 1",
                &[&stream_key, &&record.adjustment[..]],
            )
            .await?
            .is_some();
        if identity_conflict {
            return Err(StorageError::Indexer(
                indexer::IndexerError::ConflictingEvidence,
            ));
        }
        transaction
            .execute(
                "INSERT INTO indexed_settlement_adjustments
                    (stream_key, signature, slot, event_index, adjustment,
                     original_receipt, merchant, mint, nonce, kind, gross_sign,
                     gross_magnitude, tax_sign, tax_magnitude, net_sign,
                     net_magnitude, adjustment_digest, adjustment_evidence_digest,
                     evidence_digest, approver)
                 VALUES ($1, $2, $3::text::numeric, $4::text::smallint, $5,
                         $6, $7, $8, $9::text::numeric, $10::text::smallint,
                         $11::text::smallint, $12::text::numeric,
                         $13::text::smallint, $14::text::numeric,
                         $15::text::smallint, $16::text::numeric, $17, $18, $19, $20)",
                &[
                    &stream_key,
                    &record.signature,
                    &record.slot.to_string(),
                    &record.event_index.to_string(),
                    &&record.adjustment[..],
                    &&record.original_receipt[..],
                    &&record.merchant[..],
                    &&record.mint[..],
                    &record.nonce.to_string(),
                    &record.kind.to_string(),
                    &record.gross_sign.to_string(),
                    &record.gross_magnitude.to_string(),
                    &record.tax_sign.to_string(),
                    &record.tax_magnitude.to_string(),
                    &record.net_sign.to_string(),
                    &record.net_magnitude.to_string(),
                    &&record.adjustment_digest[..],
                    &&record.adjustment_evidence_digest[..],
                    &&record.evidence_digest[..],
                    &&record.approver[..],
                ],
            )
            .await?;
    }
    Ok(())
}

fn parse_indexed_settlement_adjustment_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<IndexedSettlementAdjustment, StorageError> {
    let to_digest = |index: usize, field: &str| -> Result<[u8; 32], StorageError> {
        let value: Vec<u8> = row.get(index);
        value
            .try_into()
            .map_err(|_| StorageError::InvalidIndexerState(format!("{field} is not 32 bytes")))
    };
    let to_u8 = |index: usize, field: &str| -> Result<u8, StorageError> {
        parse_u64(row.get(index), field)?
            .try_into()
            .map_err(|_| StorageError::InvalidIndexerState(format!("{field} is not a u8")))
    };
    let event_index = parse_u64(row.get(2), "adjustment event_index")?
        .try_into()
        .map_err(|_| {
            StorageError::InvalidIndexerState("adjustment event_index is not u16".into())
        })?;
    let adjustment_evidence_digest: Option<Vec<u8>> = row.get(16);
    let adjustment_evidence_digest = adjustment_evidence_digest
        .ok_or_else(|| {
            StorageError::InvalidIndexerState(
                "legacy adjustment projection lacks on-chain evidence digest".into(),
            )
        })?
        .try_into()
        .map_err(|_| {
            StorageError::InvalidIndexerState("adjustment evidence digest is not 32 bytes".into())
        })?;
    Ok(IndexedSettlementAdjustment {
        signature: row.get(0),
        slot: parse_u64(row.get(1), "adjustment slot")?,
        event_index,
        adjustment: to_digest(3, "adjustment PDA")?,
        original_receipt: to_digest(4, "adjustment receipt")?,
        merchant: to_digest(5, "adjustment merchant")?,
        mint: to_digest(6, "adjustment mint")?,
        nonce: parse_u64(row.get(7), "adjustment nonce")?,
        kind: to_u8(8, "adjustment kind")?,
        gross_sign: to_u8(9, "gross sign")?,
        gross_magnitude: parse_u64(row.get(10), "gross magnitude")?,
        tax_sign: to_u8(11, "tax sign")?,
        tax_magnitude: parse_u64(row.get(12), "tax magnitude")?,
        net_sign: to_u8(13, "net sign")?,
        net_magnitude: parse_u64(row.get(14), "net magnitude")?,
        adjustment_digest: to_digest(15, "adjustment digest")?,
        adjustment_evidence_digest,
        evidence_digest: to_digest(17, "raw evidence digest")?,
        approver: to_digest(18, "adjustment approver")?,
    })
}

async fn persist_indexed_adjustment_refund_executions(
    transaction: &deadpool_postgres::tokio_postgres::Transaction<'_>,
    stream_key: &str,
    executions: &[IndexedAdjustmentRefundExecution],
) -> Result<(), StorageError> {
    for execution in executions {
        let existing = transaction
            .query_opt(
                "SELECT signature, slot::text, event_index::text, claim,
                        adjustment, original_receipt, merchant, mint,
                        nonce::text, gross_magnitude::text, tax_magnitude::text,
                        net_magnitude::text, refund_funder, refund_destination,
                        approver, reference_digest, evidence_digest
                 FROM indexed_adjustment_refund_executions
                 WHERE stream_key = $1 AND signature = $2 AND event_index = $3::text::smallint
                 FOR UPDATE",
                &[
                    &stream_key,
                    &execution.signature,
                    &execution.event_index.to_string(),
                ],
            )
            .await?;
        if let Some(row) = existing {
            let existing_execution = parse_indexed_adjustment_refund_execution_row(row)?;
            merge_adjustment_refund_execution(Some(existing_execution), execution.clone())?;
            continue;
        }
        let claim_conflict = transaction
            .query_opt(
                "SELECT 1 FROM indexed_adjustment_refund_executions
                 WHERE stream_key = $1 AND claim = $2
                 LIMIT 1",
                &[&stream_key, &&execution.claim[..]],
            )
            .await?
            .is_some();
        if claim_conflict {
            return Err(StorageError::Indexer(
                indexer::IndexerError::ConflictingEvidence,
            ));
        }
        transaction
            .execute(
                "INSERT INTO indexed_adjustment_refund_executions
                    (stream_key, signature, slot, event_index, claim, adjustment,
                     original_receipt, merchant, mint, nonce, gross_magnitude,
                     tax_magnitude, net_magnitude, refund_funder,
                     refund_destination, approver, reference_digest, evidence_digest)
                 VALUES ($1, $2, $3::text::numeric, $4::text::smallint, $5, $6,
                         $7, $8, $9, $10::text::numeric, $11::text::numeric,
                         $12::text::numeric, $13::text::numeric, $14, $15, $16,
                         $17, $18)",
                &[
                    &stream_key,
                    &execution.signature,
                    &execution.slot.to_string(),
                    &execution.event_index.to_string(),
                    &&execution.claim[..],
                    &&execution.adjustment[..],
                    &&execution.original_receipt[..],
                    &&execution.merchant[..],
                    &&execution.mint[..],
                    &execution.nonce.to_string(),
                    &execution.gross_magnitude.to_string(),
                    &execution.tax_magnitude.to_string(),
                    &execution.net_magnitude.to_string(),
                    &&execution.refund_funder[..],
                    &&execution.refund_destination[..],
                    &&execution.approver[..],
                    &&execution.reference_digest[..],
                    &&execution.evidence_digest[..],
                ],
            )
            .await?;
    }
    Ok(())
}

fn parse_indexed_adjustment_refund_execution_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<IndexedAdjustmentRefundExecution, StorageError> {
    let to_digest = |index: usize, field: &str| -> Result<[u8; 32], StorageError> {
        let value: Vec<u8> = row.get(index);
        value
            .try_into()
            .map_err(|_| StorageError::InvalidIndexerState(format!("{field} is not 32 bytes")))
    };
    let event_index = parse_u64(row.get(2), "refund event_index")?
        .try_into()
        .map_err(|_| StorageError::InvalidIndexerState("refund event_index is not a u16".into()))?;
    Ok(IndexedAdjustmentRefundExecution {
        signature: row.get(0),
        slot: parse_u64(row.get(1), "refund execution slot")?,
        event_index,
        claim: to_digest(3, "refund claim")?,
        adjustment: to_digest(4, "refund adjustment")?,
        original_receipt: to_digest(5, "refund original receipt")?,
        merchant: to_digest(6, "refund merchant")?,
        mint: to_digest(7, "refund mint")?,
        nonce: parse_u64(row.get(8), "refund nonce")?,
        gross_magnitude: parse_u64(row.get(9), "refund gross magnitude")?,
        tax_magnitude: parse_u64(row.get(10), "refund tax magnitude")?,
        net_magnitude: parse_u64(row.get(11), "refund net magnitude")?,
        refund_funder: to_digest(12, "refund funder")?,
        refund_destination: to_digest(13, "refund destination")?,
        approver: to_digest(14, "refund approver")?,
        reference_digest: to_digest(15, "refund reference digest")?,
        evidence_digest: to_digest(16, "refund evidence digest")?,
    })
}

async fn finalize_refund_execute_intents_for_proof(
    transaction: &deadpool_postgres::tokio_postgres::Transaction<'_>,
    stream_key: &str,
    execution: &IndexedAdjustmentRefundExecution,
    proof_digest: Digest32,
) -> Result<(), StorageError> {
    let rows = transaction
        .query(
            "SELECT id, merchant_id, status, unsigned_artifact
             FROM refund_execute_intents
             WHERE claim = $1 AND original_receipt = $2 AND merchant = $3
               AND mint = $4 AND signature = $5
             FOR UPDATE",
            &[
                &&execution.claim[..],
                &&execution.original_receipt[..],
                &&execution.merchant[..],
                &&execution.mint[..],
                &execution.signature,
            ],
        )
        .await?;
    if rows.len() > 1 {
        return Err(StorageError::InvalidRefundBalanceProof(
            "finalized refund execution matches multiple durable execute intents".into(),
        ));
    }
    let Some(row) = rows.into_iter().next() else {
        return Ok(());
    };
    let intent_id: Uuid = row.get(0);
    let merchant_id: Uuid = row.get(1);
    let current_status = row.get::<_, String>(2);
    let unsigned_artifact: Value = row.get(3);
    let artifact = serde_json::from_value::<RefundArtifact>(unsigned_artifact).map_err(|_| {
        StorageError::InvalidRefundBalanceProof(
            "durable execute artifact is not valid JSON evidence".into(),
        )
    })?;
    let adjustment_account = artifact
        .instruction
        .accounts
        .get(7)
        .map(|account| account.pubkey.as_str())
        .ok_or_else(|| {
            StorageError::InvalidRefundBalanceProof(
                "durable execute artifact has no adjustment account".into(),
            )
        })?;
    if adjustment_account != Pubkey::new_from_array(execution.adjustment).to_string() {
        return Err(StorageError::InvalidRefundBalanceProof(
            "durable execute artifact adjustment is not bound to the indexed execution".into(),
        ));
    }
    let current = parse_settlement_status(&current_status)?;
    if current == SettlementStatus::Finalized {
        return Ok(());
    }
    if !can_transition(current, SettlementStatus::Finalized) {
        return Err(StorageError::InvalidRefundBalanceProof(format!(
            "execute intent {intent_id} cannot transition from {current_status} to finalized"
        )));
    }
    let slot = execution.slot.to_string();
    let updated = transaction
        .execute(
            "UPDATE refund_execute_intents
             SET status = 'finalized', commitment = 'finalized',
                 submitted_slot = $3::text::numeric, last_error_code = NULL,
                 updated_at = now()
             WHERE id = $1 AND merchant_id = $2 AND status = $4",
            &[&intent_id, &merchant_id, &slot, &current_status],
        )
        .await?;
    if updated != 1 {
        return Err(StorageError::InvalidRefundBalanceProof(
            "execute intent changed while recording finalized refund proof".into(),
        ));
    }
    let evidence = serde_json::json!({
        "source": "finalized_refund_balance_proof",
        "stream_key": stream_key,
        "signature": execution.signature,
        "slot": execution.slot,
        "event_index": execution.event_index,
        "proof_digest": proof_digest,
    });
    transaction
        .execute(
            "INSERT INTO refund_execute_intent_events
                (intent_id, merchant_id, from_status, to_status, signature, slot, evidence)
             VALUES ($1, $2, $3, 'finalized', $4, $5::text::numeric, $6)",
            &[
                &intent_id,
                &merchant_id,
                &current_status,
                &execution.signature,
                &slot,
                &evidence,
            ],
        )
        .await?;
    transaction
        .execute(
            "INSERT INTO outbox_events
                (aggregate_type, aggregate_id, event_type, payload)
             VALUES ($1, $2, $3, $4)",
            &[
                &"refund_execute_intent",
                &intent_id,
                &"tax.refund_execute_intent.finalized",
                &serde_json::json!({
                    "schema_version": OUTBOX_PAYLOAD_SCHEMA_VERSION,
                    "merchant_id": merchant_id,
                    "intent_id": intent_id,
                    "signature": execution.signature,
                    "slot": execution.slot,
                    "proof_digest": proof_digest,
                }),
            ],
        )
        .await?;
    Ok(())
}

fn validate_refund_balance_proof_binding(
    execution: &IndexedAdjustmentRefundExecution,
    topology: &RefundBalanceProofTopology,
    proof: &RefundBalanceProof,
) -> Result<(), StorageError> {
    if topology.claim != execution.claim
        || topology.adjustment != execution.adjustment
        || topology.original_receipt != execution.original_receipt
        || topology.merchant != execution.merchant
        || topology.mint != execution.mint
        || topology.refund_funder != execution.refund_funder
        || topology.payer_destination != execution.refund_destination
        || topology.claim == [0; 32]
        || topology.token_program == [0; 32]
        || topology.tax_vault == [0; 32]
        || topology.refund_source == [0; 32]
        || topology.vault_authority == [0; 32]
        || topology.payer == [0; 32]
        || topology.tax_vault == topology.refund_source
        || topology.tax_vault == topology.payer_destination
        || topology.refund_source == topology.payer_destination
    {
        return Err(StorageError::InvalidRefundBalanceProof(
            "proof topology is not bound to the indexed execution".to_string(),
        ));
    }
    if proof.finalized_slot != execution.slot
        || proof.gross_atomic != execution.gross_magnitude
        || proof.tax_atomic != execution.tax_magnitude
        || proof.net_atomic != execution.net_magnitude
    {
        return Err(StorageError::InvalidRefundBalanceProof(
            "proof slot or amounts do not match the indexed execution".to_string(),
        ));
    }
    Ok(())
}

fn parse_refund_balance_proof_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<
    (
        Uuid,
        RefundBalanceProofTopology,
        u64,
        RefundBalanceProof,
        [u8; 32],
    ),
    StorageError,
> {
    let to_array = |index: usize, field: &str| -> Result<[u8; 32], StorageError> {
        let value: Vec<u8> = row.get(index);
        value
            .try_into()
            .map_err(|_| StorageError::InvalidIndexerState(format!("{field} is not 32 bytes")))
    };
    let proof_digest: Vec<u8> = row.get(25);
    let proof_digest = proof_digest.try_into().map_err(|_| {
        StorageError::InvalidIndexerState("refund proof digest is not 32 bytes".to_string())
    })?;
    let topology = RefundBalanceProofTopology {
        claim: to_array(2, "refund proof claim")?,
        adjustment: to_array(3, "refund proof adjustment")?,
        original_receipt: to_array(4, "refund proof original receipt")?,
        merchant: to_array(5, "refund proof merchant")?,
        mint: to_array(6, "refund proof mint")?,
        token_program: to_array(7, "refund proof token program")?,
        tax_vault: to_array(8, "refund proof tax vault")?,
        refund_source: to_array(9, "refund proof refund source")?,
        payer_destination: to_array(10, "refund proof payer destination")?,
        vault_authority: to_array(11, "refund proof vault authority")?,
        refund_funder: to_array(12, "refund proof refund funder")?,
        payer: to_array(13, "refund proof payer")?,
    };
    let proof = RefundBalanceProof {
        finalized_slot: parse_u64(row.get(1), "refund proof slot")?,
        gross_atomic: parse_u64(row.get(15), "refund proof gross")?,
        tax_atomic: parse_u64(row.get(16), "refund proof tax")?,
        net_atomic: parse_u64(row.get(17), "refund proof net")?,
        tax_vault_before_atomic: parse_u64(row.get(18), "refund proof tax vault before")?,
        tax_vault_after_atomic: parse_u64(row.get(19), "refund proof tax vault after")?,
        refund_source_before_atomic: parse_u64(row.get(20), "refund proof source before")?,
        refund_source_after_atomic: parse_u64(row.get(21), "refund proof source after")?,
        payer_destination_before_atomic: parse_u64(row.get(22), "refund proof destination before")?,
        payer_destination_after_atomic: parse_u64(row.get(23), "refund proof destination after")?,
    };
    Ok((
        row.get(0),
        topology,
        parse_u64(row.get(14), "refund proof nonce")?,
        proof,
        proof_digest,
    ))
}

fn parse_raw_evidence_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<RawChainEvidence, StorageError> {
    let event_discriminator: Vec<u8> = row.get(3);
    let event_discriminator: [u8; 8] = event_discriminator.try_into().map_err(|_| {
        StorageError::InvalidIndexerState("event discriminator is not 8 bytes".into())
    })?;
    let evidence_digest: Vec<u8> = row.get(5);
    let evidence_digest: [u8; 32] = evidence_digest
        .try_into()
        .map_err(|_| StorageError::InvalidIndexerState("evidence digest is not 32 bytes".into()))?;
    let event_index = parse_u64(row.get(2), "event_index")?
        .try_into()
        .map_err(|_| StorageError::InvalidIndexerState("event_index is not a u16".into()))?;
    Ok(RawChainEvidence {
        signature: row.get(0),
        slot: parse_u64(row.get(1), "slot")?,
        event_index,
        event_discriminator,
        event_data: row.get(4),
        evidence_digest,
    })
}

fn parse_raw_transaction_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<RawChainTransaction, StorageError> {
    let payload_digest: Vec<u8> = row.get(4);
    let payload_digest: [u8; 32] = payload_digest.try_into().map_err(|_| {
        StorageError::InvalidIndexerState("transaction payload digest is not 32 bytes".into())
    })?;
    let transaction_index = parse_u64(row.get(2), "transaction_index")?
        .try_into()
        .map_err(|_| StorageError::InvalidIndexerState("transaction index is not a u32".into()))?;
    Ok(RawChainTransaction {
        signature: row.get(0),
        slot: parse_u64(row.get(1), "slot")?,
        transaction_index,
        payload: row.get(3),
        payload_digest,
    })
}

async fn persist_protocol_pause_events(
    transaction: &deadpool_postgres::tokio_postgres::Transaction<'_>,
    stream_key: &str,
    events: &[ProtocolPauseChangedEvent],
    range: indexer::ScanRange,
    observed_finalized_slot: u64,
) -> Result<(), StorageError> {
    for event in events {
        event.validate(range, observed_finalized_slot)?;
        let existing = transaction
            .query_opt(
                "SELECT signature, slot::text, event_index::text, authority,
                        paused, evidence_digest
                 FROM protocol_pause_events
                 WHERE stream_key = $1 AND signature = $2 AND event_index = $3::text::smallint
                 FOR UPDATE",
                &[
                    &stream_key,
                    &event.signature,
                    &event.event_index.to_string(),
                ],
            )
            .await?;
        if let Some(row) = existing {
            if parse_protocol_pause_event_row(row)? != *event {
                return Err(StorageError::Indexer(
                    indexer::IndexerError::ConflictingEvidence,
                ));
            }
            continue;
        }
        transaction
            .execute(
                "INSERT INTO protocol_pause_events
                    (stream_key, signature, slot, event_index, authority, paused, evidence_digest)
                 VALUES ($1, $2, $3::text::numeric, $4::text::smallint, $5, $6, $7)",
                &[
                    &stream_key,
                    &event.signature,
                    &event.slot.to_string(),
                    &event.event_index.to_string(),
                    &&event.authority[..],
                    &event.paused,
                    &&event.evidence_digest[..],
                ],
            )
            .await?;
    }
    Ok(())
}

fn parse_protocol_pause_event_row(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<ProtocolPauseChangedEvent, StorageError> {
    let authority: Vec<u8> = row.get(3);
    let authority: [u8; 32] = authority
        .try_into()
        .map_err(|_| StorageError::InvalidIndexerState("pause authority is not 32 bytes".into()))?;
    let evidence_digest: Vec<u8> = row.get(5);
    let evidence_digest: [u8; 32] = evidence_digest.try_into().map_err(|_| {
        StorageError::InvalidIndexerState("pause evidence digest is not 32 bytes".into())
    })?;
    Ok(ProtocolPauseChangedEvent {
        signature: row.get(0),
        slot: parse_u64(row.get(1), "pause event slot")?,
        event_index: parse_u64(row.get(2), "pause event index")?
            .try_into()
            .map_err(|_| {
                StorageError::InvalidIndexerState("pause event index is not a u16".into())
            })?,
        authority,
        paused: row.get(4),
        evidence_digest,
    })
}

fn parse_replay_gap_row(
    row: deadpool_postgres::tokio_postgres::Row,
    claim_token: Uuid,
) -> Result<ReplayGapRecord, StorageError> {
    let attempts = parse_u64(row.get(5), "attempts")?
        .try_into()
        .map_err(|_| StorageError::InvalidReplayState("attempt count is not a u32".into()))?;
    let work = ReplayWork {
        range: indexer::ScanRange {
            start_slot: parse_u64(row.get(1), "start_slot")?,
            end_slot: parse_u64(row.get(2), "end_slot")?,
        },
        observed_finalized_slot: parse_u64(row.get(3), "observed_finalized_slot")?,
        reason: replay_reason_from_db(row.get(4))?,
        attempt: attempts,
    };
    validate_replay_gap(&ReplayGap {
        range: work.range,
        observed_finalized_slot: work.observed_finalized_slot,
        reason: work.reason,
        attempts: work.attempt,
    })?;
    Ok(ReplayGapRecord {
        id: row.get(0),
        work,
        claim_token,
    })
}

fn parse_replay_gap_view(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<ReplayGapView, StorageError> {
    let attempts = parse_u64(row.get(6), "replay attempts")?
        .try_into()
        .map_err(|_| StorageError::InvalidReplayState("replay attempts exceed u32".into()))?;
    let start_slot = parse_u64(row.get(2), "replay start_slot")?;
    let end_slot = parse_u64(row.get(3), "replay end_slot")?;
    let observed_finalized_slot = parse_u64(row.get(4), "replay finalized slot")?;
    let created_at: String = row.get(9);
    if created_at.is_empty() {
        return Err(StorageError::InvalidReplayState(
            "replay gap created_at is empty".into(),
        ));
    }
    Ok(ReplayGapView {
        id: row.get(0),
        stream_key: row.get(1),
        start_slot,
        end_slot,
        observed_finalized_slot,
        reason: row.get(5),
        attempts,
        status: row.get(7),
        last_error_code: row.get(8),
        created_at,
        dead_lettered_at: row.get(10),
        operator_acknowledged_at: row.get(11),
        operator_acknowledged_by: row.get(12),
        operator_acknowledgement: row.get(13),
    })
}

fn parse_settlement_route_verification_run_view(
    row: deadpool_postgres::tokio_postgres::Row,
) -> Result<SettlementRouteVerificationRunView, StorageError> {
    let view = SettlementRouteVerificationRunView {
        id: row.get(0),
        run_kind: row.get(1),
        status: row.get(2),
        batch_size: parse_route_verification_run_count(row.get::<_, i16>(3) as i32, "batch_size")?,
        examined_count: parse_route_verification_run_count(row.get(4), "examined_count")?,
        activated_count: parse_route_verification_run_count(row.get(5), "activated_count")?,
        already_active_count: parse_route_verification_run_count(
            row.get(6),
            "already_active_count",
        )?,
        failed_count: parse_route_verification_run_count(row.get(7), "failed_count")?,
        failure_code: row.get(8),
        operator_approval_id: row.get(9),
        started_at: row.get(10),
        completed_at: row.get(11),
    };
    validate_settlement_route_verification_run_view(&view)?;
    Ok(view)
}

fn parse_route_verification_run_count(value: i32, field: &str) -> Result<u32, StorageError> {
    if !(0..=64).contains(&value) {
        return Err(StorageError::SettlementRouteVerificationInvalid(format!(
            "{field} is outside the bounded range"
        )));
    }
    Ok(value as u32)
}

fn validate_settlement_route_verification_run_view(
    view: &SettlementRouteVerificationRunView,
) -> Result<(), StorageError> {
    if view.id.get_version_num() != 7 {
        return Err(StorageError::SettlementRouteVerificationInvalid(
            "run ID must be UUIDv7".into(),
        ));
    }
    if !(1..=64).contains(&view.batch_size) {
        return Err(StorageError::SettlementRouteVerificationInvalid(
            "batch size is outside the bounded range".into(),
        ));
    }
    if !matches!(view.run_kind.as_str(), "scheduled" | "backfill")
        || !matches!(
            view.status.as_str(),
            "running" | "completed" | "completed_with_failures" | "failed"
        )
    {
        return Err(StorageError::SettlementRouteVerificationInvalid(
            "run kind or status is invalid".into(),
        ));
    }
    if (view.run_kind == "backfill") != view.operator_approval_id.is_some() {
        return Err(StorageError::SettlementRouteVerificationInvalid(
            "approval binding does not match run kind".into(),
        ));
    }
    if view
        .operator_approval_id
        .is_some_and(|approval_id| approval_id.get_version_num() != 7)
    {
        return Err(StorageError::SettlementRouteVerificationInvalid(
            "operator approval ID must be UUIDv7".into(),
        ));
    }
    if view.started_at.is_empty()
        || view.started_at.len() > 64
        || view.started_at.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(StorageError::SettlementRouteVerificationInvalid(
            "started_at is invalid".into(),
        ));
    }
    if let Some(completed_at) = view.completed_at.as_deref()
        && (completed_at.is_empty()
            || completed_at.len() > 64
            || completed_at.bytes().any(|byte| byte.is_ascii_control()))
    {
        return Err(StorageError::SettlementRouteVerificationInvalid(
            "completed_at is invalid".into(),
        ));
    }
    if let Some(failure_code) = view.failure_code.as_deref()
        && (failure_code.is_empty()
            || failure_code.len() > 128
            || failure_code
                .bytes()
                .any(|byte| !byte.is_ascii_alphanumeric() && !matches!(byte, b'_' | b'-')))
    {
        return Err(StorageError::SettlementRouteVerificationInvalid(
            "failure code is invalid".into(),
        ));
    }
    let terminal = view.status != "running";
    if terminal {
        if view.completed_at.is_none()
            || view.examined_count
                != view.activated_count + view.already_active_count + view.failed_count
        {
            return Err(StorageError::SettlementRouteVerificationInvalid(
                "terminal counters or completion timestamp are invalid".into(),
            ));
        }
    } else if view.completed_at.is_some() {
        return Err(StorageError::SettlementRouteVerificationInvalid(
            "running evidence cannot have a completion timestamp".into(),
        ));
    }
    match view.status.as_str() {
        "completed" if view.failed_count != 0 || view.failure_code.is_some() => {
            Err(StorageError::SettlementRouteVerificationInvalid(
                "completed evidence cannot contain failures".into(),
            ))
        }
        "completed_with_failures" if view.failed_count == 0 || view.failure_code.is_none() => {
            Err(StorageError::SettlementRouteVerificationInvalid(
                "completed-with-failures evidence is incomplete".into(),
            ))
        }
        "failed" if view.failure_code.is_none() => {
            Err(StorageError::SettlementRouteVerificationInvalid(
                "failed evidence requires a failure code".into(),
            ))
        }
        _ => Ok(()),
    }
}

fn validate_settlement_route_verification_run_summary(
    summary: SettlementRouteVerificationRunSummary,
) -> Result<(), StorageError> {
    let counts = [
        summary.examined,
        summary.activated,
        summary.already_active,
        summary.failed,
    ];
    if counts.iter().any(|count| *count > 64)
        || summary.examined != summary.activated + summary.already_active + summary.failed
    {
        return Err(StorageError::InvalidConfiguration(
            "settlement route verification run counters are inconsistent".to_string(),
        ));
    }
    Ok(())
}

fn validate_settlement_route_verification_run_start(
    run_kind: &str,
    operator_approval_id: Option<Uuid>,
) -> Result<(), StorageError> {
    match (run_kind, operator_approval_id) {
        ("scheduled", None) => Ok(()),
        ("backfill", Some(approval_id)) if approval_id.get_version_num() == 7 => Ok(()),
        ("scheduled", Some(_)) => Err(StorageError::InvalidConfiguration(
            "scheduled route verification runs cannot carry an approval ID".to_string(),
        )),
        ("backfill", None) => Err(StorageError::InvalidConfiguration(
            "backfill route verification runs require an approval ID".to_string(),
        )),
        _ => Err(StorageError::InvalidConfiguration(
            "settlement route verification run kind is invalid".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tax_engine::{
        BaseReference, Rate, RateSpec, RoundingMode, RoundingScope, RoundingSpec, TaxBase,
        TaxComponentRule,
    };

    #[test]
    fn route_verification_run_summary_requires_bounded_conserving_counts() {
        assert!(
            validate_settlement_route_verification_run_summary(
                SettlementRouteVerificationRunSummary {
                    examined: 4,
                    activated: 2,
                    already_active: 1,
                    failed: 1,
                }
            )
            .is_ok()
        );
        assert!(matches!(
            validate_settlement_route_verification_run_summary(
                SettlementRouteVerificationRunSummary {
                    examined: 4,
                    activated: 2,
                    already_active: 1,
                    failed: 0,
                }
            ),
            Err(StorageError::InvalidConfiguration(_))
        ));
        assert!(matches!(
            validate_settlement_route_verification_run_summary(
                SettlementRouteVerificationRunSummary {
                    examined: 65,
                    activated: 65,
                    already_active: 0,
                    failed: 0,
                }
            ),
            Err(StorageError::InvalidConfiguration(_))
        ));
    }

    #[test]
    fn route_verification_run_kind_requires_the_matching_approval_reference() {
        let approval_id = Uuid::now_v7();
        assert!(validate_settlement_route_verification_run_start("scheduled", None).is_ok());
        assert!(
            validate_settlement_route_verification_run_start("backfill", Some(approval_id)).is_ok()
        );
        assert!(
            validate_settlement_route_verification_run_start("scheduled", Some(approval_id))
                .is_err()
        );
        assert!(validate_settlement_route_verification_run_start("backfill", None).is_err());
        assert!(
            validate_settlement_route_verification_run_start("backfill", Some(Uuid::from_u128(1)))
                .is_err()
        );
    }

    #[test]
    fn route_verification_run_view_requires_bounded_terminal_evidence() {
        let id = Uuid::now_v7();
        let approval_id = Uuid::now_v7();
        let valid = SettlementRouteVerificationRunView {
            id,
            run_kind: "backfill".to_string(),
            status: "completed".to_string(),
            batch_size: 8,
            examined_count: 2,
            activated_count: 1,
            already_active_count: 1,
            failed_count: 0,
            failure_code: None,
            operator_approval_id: Some(approval_id),
            started_at: "2026-10-05T10:00:00Z".to_string(),
            completed_at: Some("2026-10-05T10:00:01Z".to_string()),
        };
        assert!(validate_settlement_route_verification_run_view(&valid).is_ok());
        let mut invalid = valid.clone();
        invalid.failed_count = 1;
        assert!(validate_settlement_route_verification_run_view(&invalid).is_err());
        let mut invalid = valid.clone();
        invalid.operator_approval_id = None;
        assert!(validate_settlement_route_verification_run_view(&invalid).is_err());
        let mut invalid = valid;
        invalid.status = "running".to_string();
        assert!(validate_settlement_route_verification_run_view(&invalid).is_err());
    }

    fn command() -> QuoteCommand {
        QuoteCommand {
            merchant_id: Uuid::now_v7(),
            idempotency_key: "quote-1".to_string(),
            rule_set_id: "us-ca-sales-tax".to_string(),
            rule_version: 1,
            jurisdiction: "US-CA".to_string(),
            regime: "sales-tax".to_string(),
            facts: TaxFacts {
                gross_atomic: 1_100,
                net_atomic: 1_000,
                atomic_decimals: 2,
                currency: "USD".to_string(),
                jurisdiction: "US-CA".to_string(),
                jurisdiction_resolution: None,
                fx_conversion: None,
                exemption_codes: Vec::new(),
                cumulative_period: None,
                applicability: None,
            },
            applicability_context_id: None,
            jurisdiction_candidates: vec![JurisdictionCandidate {
                jurisdiction_code: "US-CA".to_string(),
                source: jurisdiction::EvidenceSource::ShippingDestination,
                evidence_digest: [7; 32],
            }],
            fx_evidence: None,
        }
    }

    fn applicability_context_command() -> CreateApplicabilityContextCommand {
        let merchant_id = Uuid::now_v7();
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
        let decision = tax_applicability::ApplicabilityDecision {
            schema: tax_applicability::APPLICABILITY_SCHEMA_V1.to_string(),
            policy_id: "policy-us-ca-digital".to_string(),
            policy_version: 3,
            policy_digest: [7; 32],
            facts_digest: facts.canonical_digest().unwrap(),
            treatment: tax_applicability::Treatment::Taxable,
            selected_rule_id: Some("taxable-digital-service".to_string()),
            matched_rule_ids: vec!["taxable-digital-service".to_string()],
            evaluated_at_unix: 1_800_000_100,
        };
        let unsigned = policy::UnsignedApplicabilityAttestation {
            schema_version: policy::APPLICABILITY_ATTESTATION_SCHEMA_VERSION,
            merchant_id: merchant_identity_digest(merchant_id.as_bytes()),
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
        };
        CreateApplicabilityContextCommand {
            merchant_id,
            idempotency_key: "applicability-context-1".to_string(),
            attestation: policy::ApplicabilityAttestation::sign(
                unsigned,
                &ed25519_dalek::SigningKey::from_bytes(&[7; 32]),
            )
            .unwrap(),
            effective_from_unix: 1_800_000_100,
            effective_until_unix: Some(1_800_000_500),
            provenance: serde_json::json!({"owner": "tax-domain"}),
            actor_subject: "tax-author".to_string(),
        }
    }

    #[test]
    fn applicability_context_validation_binds_attestation_and_window() {
        let command = applicability_context_command();
        assert!(validate_create_applicability_context_command(&command).is_ok());
        let digest = applicability_context_request_digest(&command).unwrap();
        assert_eq!(
            digest,
            applicability_context_request_digest(&command).unwrap()
        );

        let mut invalid = command.clone();
        invalid.effective_until_unix = Some(invalid.attestation.unsigned.expires_at_unix + 1);
        assert!(matches!(
            validate_create_applicability_context_command(&invalid),
            Err(StorageError::InvalidApplicabilityContext(message))
                if message.contains("signed attestation validity")
        ));

        let mut changed = command;
        changed.attestation.unsigned.facts.product_category = "physical-good".to_string();
        assert!(matches!(
            validate_create_applicability_context_command(&changed),
            Err(StorageError::InvalidApplicabilityContext(_))
        ));
    }

    #[test]
    fn applicability_context_request_digest_is_tenant_and_evidence_bound() {
        let command = applicability_context_command();
        let digest = applicability_context_request_digest(&command).unwrap();
        let mut tenant_changed = command.clone();
        tenant_changed.merchant_id = Uuid::now_v7();
        assert!(matches!(
            applicability_context_request_digest(&tenant_changed),
            Err(StorageError::InvalidApplicabilityContext(message))
                if message.contains("tenant")
        ));
        let mut evidence_changed = command;
        evidence_changed.provenance = serde_json::json!({"owner": "different-owner"});
        assert_ne!(
            digest,
            applicability_context_request_digest(&evidence_changed).unwrap()
        );
    }

    #[test]
    fn applicability_context_transitions_are_bounded_and_ordered() {
        let valid = ApplicabilityContextTransition {
            context_id: Uuid::now_v7(),
            merchant_id: Uuid::now_v7(),
            expected_status: "draft".to_string(),
            next_status: "review".to_string(),
            actor_subject: "reviewer".to_string(),
            evidence: serde_json::json!({"reason": "ready"}),
        };
        assert!(validate_applicability_context_transition(&valid).is_ok());
        let mut invalid = valid.clone();
        invalid.next_status = "published".to_string();
        assert!(matches!(
            validate_applicability_context_transition(&invalid),
            Err(StorageError::ApplicabilityContextTransitionConflict)
        ));
        invalid = valid;
        invalid.actor_subject = "operator\nforged".to_string();
        assert!(matches!(
            validate_applicability_context_transition(&invalid),
            Err(StorageError::InvalidApplicabilityContext(_))
        ));
    }

    fn revenue_stream_plan_command() -> CreateRevenueStreamPlanCommand {
        let merchant_id = Uuid::now_v7();
        CreateRevenueStreamPlanCommand {
            merchant_id,
            destination_route_id: Uuid::now_v7(),
            actor_subject: "stream-proposer".to_string(),
            idempotency_key: "stream-plan-1".to_string(),
            spec: StreamPlanSpec {
                id: StreamPlanId(Uuid::now_v7()),
                merchant_id,
                settlement_route_id: Uuid::now_v7(),
                asset: revenue_streaming::AssetId([3; 32]),
                recipient: revenue_streaming::RecipientId([4; 32]),
                plan_digest: revenue_streaming::Digest([5; 32]),
                policy_digest: revenue_streaming::Digest([6; 32]),
                operator_subject: Some("stream-operator".to_string()),
                start_slot: 100,
                end_slot: Some(1_000),
                max_total_atomic: 1_000,
                max_claim_atomic: 500,
            },
        }
    }

    #[test]
    fn revenue_stream_plan_validation_binds_tenant_and_exact_v7_identity() {
        let command = revenue_stream_plan_command();
        assert!(validate_create_revenue_stream_plan_command(&command).is_ok());
        let digest = revenue_stream_plan_request_digest(&command).expect("stream digest");
        let mut actor_variant = command.clone();
        actor_variant.actor_subject = "different-authorized-subject".to_string();
        assert_eq!(
            revenue_stream_plan_request_digest(&actor_variant).expect("same request digest"),
            digest
        );

        let mut invalid = command;
        invalid.spec.merchant_id = Uuid::now_v7();
        assert!(matches!(
            validate_create_revenue_stream_plan_command(&invalid),
            Err(StorageError::InvalidRevenueStream(_))
        ));

        let mut same_operator = revenue_stream_plan_command();
        same_operator.spec.operator_subject = Some(same_operator.actor_subject.clone());
        assert!(matches!(
            validate_create_revenue_stream_plan_command(&same_operator),
            Err(StorageError::InvalidRevenueStream(_))
        ));

        let mut missing_operator = revenue_stream_plan_command();
        missing_operator.spec.operator_subject = None;
        assert!(matches!(
            validate_create_revenue_stream_plan_command(&missing_operator),
            Err(StorageError::InvalidRevenueStream(_))
        ));
    }

    #[test]
    fn revenue_stream_plan_transition_requires_four_eyes_and_finalized_completion() {
        let approved = validate_revenue_stream_plan_transition_state(
            "proposed",
            "approved",
            RevenueStreamTransitionState {
                proposer_subject: "proposer",
                current_approver: None,
                actor_subject: "approver",
                accrued_total_atomic: 100,
                released_total_atomic: 100,
                end_slot: Some(500),
                observed_slot: None,
            },
        )
        .expect("distinct approver");
        assert_eq!(approved.as_deref(), Some("approver"));

        assert!(matches!(
            validate_revenue_stream_plan_transition_state(
                "proposed",
                "approved",
                RevenueStreamTransitionState {
                    proposer_subject: "proposer",
                    current_approver: None,
                    actor_subject: "proposer",
                    accrued_total_atomic: 0,
                    released_total_atomic: 0,
                    end_slot: None,
                    observed_slot: None,
                },
            ),
            Err(StorageError::RevenueStreamPlanActorConflict)
        ));

        assert!(matches!(
            validate_revenue_stream_plan_transition_state(
                "active",
                "completed",
                RevenueStreamTransitionState {
                    proposer_subject: "proposer",
                    current_approver: Some("approver"),
                    actor_subject: "approver",
                    accrued_total_atomic: 100,
                    released_total_atomic: 100,
                    end_slot: Some(500),
                    observed_slot: Some(499),
                },
            ),
            Err(StorageError::RevenueStreamInsufficientAvailable)
        ));

        assert_eq!(
            validate_revenue_stream_plan_transition_state(
                "active",
                "completed",
                RevenueStreamTransitionState {
                    proposer_subject: "proposer",
                    current_approver: Some("approver"),
                    actor_subject: "approver",
                    accrued_total_atomic: 100,
                    released_total_atomic: 100,
                    end_slot: Some(500),
                    observed_slot: Some(500),
                },
            )
            .expect("complete after finalized release"),
            Some("approver".to_string())
        );
    }

    #[test]
    fn revenue_stream_evidence_validates_conservation_and_window_binding() {
        let command = revenue_stream_plan_command();
        let plan = RevenueStreamPlanRecord {
            id: command.spec.id,
            merchant_id: command.merchant_id,
            idempotency_key: command.idempotency_key.clone(),
            request_digest: revenue_stream_plan_request_digest(&command).expect("digest"),
            destination_route_id: Some(command.destination_route_id),
            spec: command.spec.clone(),
            status: "active".to_string(),
            proposer_subject: command.actor_subject,
            approver_subject: Some("approver".to_string()),
            accrued_total_atomic: 0,
            released_total_atomic: 0,
            next_nonce: 1,
        };
        let accrual = FinalizedAccrual {
            source_signature: RevenueStreamSignature(vec![7; 64]),
            finalized_slot: 200,
            receipt_digest: RevenueStreamDigest([8; 32]),
            plan_digest: command.spec.plan_digest,
            policy_digest: command.spec.policy_digest,
            asset: command.spec.asset,
            recipient: command.spec.recipient,
            gross_atomic: 100,
            tax_atomic: 20,
            net_atomic: 80,
            recipient_atomic: 75,
        };
        validate_revenue_stream_accrual(&accrual).expect("conserving accrual");
        validate_revenue_stream_accrual_binding(&plan, &accrual).expect("matching active stream");

        let mut invalid = accrual.clone();
        invalid.recipient_atomic = 81;
        assert!(matches!(
            validate_revenue_stream_accrual(&invalid),
            Err(StorageError::InvalidRevenueStream(_))
        ));
        let mut outside_window = accrual;
        outside_window.finalized_slot = 1_001;
        assert!(matches!(
            validate_revenue_stream_accrual_binding(&plan, &outside_window),
            Err(StorageError::InvalidRevenueStream(_))
        ));
    }

    fn vault_release_command() -> CreateVaultReleaseClaimCommand {
        CreateVaultReleaseClaimCommand {
            merchant_id: Uuid::now_v7(),
            actor_subject: "proposer".to_string(),
            idempotency_key: "release-1".to_string(),
            route_id: Uuid::now_v7(),
            destination: "SysvarRent111111111111111111111111111111111".to_string(),
            nonce: 7,
            amount_atomic: 25,
            reference_digest: [9; 32],
            execute_after_unix: 1_790_000_000,
            expires_at_unix: 1_790_003_600,
        }
    }

    #[test]
    fn vault_release_claim_wire_contract_preserves_u64_precision() {
        let claim = VaultReleaseClaim {
            id: Uuid::now_v7(),
            merchant_id: Uuid::now_v7(),
            idempotency_key: "release-precision".to_string(),
            request_digest: [1; 32],
            mint: "So11111111111111111111111111111111111111112".to_string(),
            token_program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
            tax_vault: "11111111111111111111111111111111".to_string(),
            destination: "SysvarRent111111111111111111111111111111111".to_string(),
            claim_pda: None,
            nonce: u64::MAX,
            amount_atomic: u64::MAX,
            reference_digest: [2; 32],
            status: "proposed".to_string(),
            proposer_subject: "proposer".to_string(),
            approver_subject: None,
            execute_after_unix: 1_790_000_000,
            expires_at_unix: 1_790_003_600,
            transaction_signature: None,
            slot: Some(u64::MAX),
            last_error_code: None,
        };
        let wire = serde_json::to_value(claim).expect("vault release claim serializes");
        assert_eq!(
            wire["nonce"],
            serde_json::Value::String(u64::MAX.to_string())
        );
        assert_eq!(
            wire["amount_atomic"],
            serde_json::Value::String(u64::MAX.to_string())
        );
        assert_eq!(
            wire["slot"],
            serde_json::Value::String(u64::MAX.to_string())
        );
    }

    fn refund_funding_configuration_command() -> CreateRefundFundingConfigurationCommand {
        CreateRefundFundingConfigurationCommand {
            merchant_id: Uuid::now_v7(),
            actor_subject: "funding-proposer".to_string(),
            idempotency_key: "funding-1".to_string(),
            configuration_key: "primary-usdc".to_string(),
            version: 1,
            merchant_account: "11111111111111111111111111111111".to_string(),
            mint: "So11111111111111111111111111111111111111112".to_string(),
            tax_vault: "SysvarRent111111111111111111111111111111111".to_string(),
            vault_authority: "Vote111111111111111111111111111111111111111".to_string(),
            refund_funder: "Stake11111111111111111111111111111111111111".to_string(),
            refund_funder_subject: "refund-funder-subject".to_string(),
            net_source: "Config1111111111111111111111111111111111111".to_string(),
            token_program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
            effective_from_unix: 1_790_000_000,
            effective_until_unix: Some(1_800_000_000),
        }
    }

    fn evidence_cipher() -> EvidenceCipher {
        EvidenceCipher::from_base64(
            "test-evidence-key-v1",
            "Y2VydGlmaWVkLXRlc3QtZXZpZGVuY2Uta2V5LTMyeHg=",
        )
        .unwrap()
    }

    fn refund_proof_storage_fixture() -> (
        IndexedAdjustmentRefundExecution,
        RefundBalanceProofTopology,
        RefundBalanceProof,
    ) {
        let execution = IndexedAdjustmentRefundExecution {
            signature: "5Vf6aYzq9hM9Vj3P6e1hL5m7oT8pQ2rS4uW6xY8zA1bC".to_string(),
            slot: 777,
            event_index: 0,
            claim: [1; 32],
            adjustment: [2; 32],
            original_receipt: [3; 32],
            merchant: [4; 32],
            mint: [5; 32],
            nonce: 1,
            gross_magnitude: 100,
            tax_magnitude: 25,
            net_magnitude: 75,
            refund_funder: [6; 32],
            refund_destination: [7; 32],
            approver: [8; 32],
            reference_digest: [9; 32],
            evidence_digest: [10; 32],
        };
        let topology = RefundBalanceProofTopology {
            claim: execution.claim,
            adjustment: execution.adjustment,
            original_receipt: execution.original_receipt,
            merchant: execution.merchant,
            mint: execution.mint,
            token_program: [11; 32],
            tax_vault: [12; 32],
            refund_source: [13; 32],
            payer_destination: execution.refund_destination,
            vault_authority: [14; 32],
            refund_funder: execution.refund_funder,
            payer: [15; 32],
        };
        let proof = RefundBalanceProof {
            finalized_slot: 777,
            gross_atomic: 100,
            tax_atomic: 25,
            net_atomic: 75,
            tax_vault_before_atomic: 125,
            tax_vault_after_atomic: 100,
            refund_source_before_atomic: 175,
            refund_source_after_atomic: 100,
            payer_destination_before_atomic: 0,
            payer_destination_after_atomic: 100,
        };
        (execution, topology, proof)
    }

    #[test]
    fn refund_claim_evidence_expectation_is_derived_from_immutable_artifact() {
        let merchant_id = Uuid::now_v7();
        let adjustment_id = Uuid::now_v7();
        let funding_configuration_id = Uuid::now_v7();
        let proposer = Pubkey::new_unique();
        let refund_funder = Pubkey::new_unique();
        let tax_vault = Pubkey::new_unique();
        let net_source = Pubkey::new_unique();
        let input = RefundClaimServerInputs {
            merchant_id,
            adjustment_id,
            funding_configuration_id,
            funding_configuration_digest: [1; 32],
            receipt_signature: "5Ux123456789012345678901234567890123456789012345678901234567890"
                .into(),
            receipt_slot: 7,
            original_receipt: Pubkey::new_unique().to_bytes(),
            adjustment: Pubkey::new_unique().to_bytes(),
            payer: Pubkey::new_unique().to_bytes(),
            merchant: Pubkey::new_unique().to_bytes(),
            mint: Pubkey::new_unique().to_bytes(),
            token_program: SPL_TOKEN_PROGRAM_ID.into(),
            proposer: proposer.to_string(),
            proposer_subject: "proposer".into(),
            refund_funder: refund_funder.to_string(),
            refund_funder_subject: "funder".into(),
            tax_vault: tax_vault.to_string(),
            net_source: net_source.to_string(),
            gross_atomic: 100,
            tax_atomic: 25,
            net_atomic: 75,
            reference_digest: [2; 32],
            claim_nonce: 9,
            execute_after_unix: 1_790_000_000,
            expires_at_unix: 1_790_003_600,
        };
        let artifact = refund_workflow::build_claim_artifact(&input).unwrap();
        let claim = Pubkey::from_str(&artifact.instruction.claim)
            .unwrap()
            .to_bytes();
        let intent = RefundClaimIntent {
            id: Uuid::now_v7(),
            merchant_id,
            idempotency_key: "claim-1".into(),
            request_digest: [3; 32],
            adjustment_id,
            funding_configuration_id,
            receipt_signature: input.receipt_signature,
            receipt_slot: input.receipt_slot,
            mint: input.mint,
            token_program: input.token_program,
            gross_atomic: input.gross_atomic,
            tax_atomic: input.tax_atomic,
            net_atomic: input.net_atomic,
            claim_nonce: input.claim_nonce,
            claim,
            instruction_digest: artifact.instruction_digest,
            artifact_digest: artifact.artifact_digest,
            status: "submitted".into(),
            signed_transaction_digest: None,
            signature: Some("5Vf6aYzq9hM9Vj3P6e1hL5m7oT8pQ2rS4uW6xY8zA1bC".into()),
            submitted_slot: Some(8),
            commitment: Some("processed".into()),
            last_error_code: None,
            unsigned_artifact: artifact.unsigned_artifact().unwrap(),
        };
        let expectation = refund_claim_evidence_expectation(&intent, &artifact).unwrap();
        assert_eq!(expectation.claim, claim);
        assert_eq!(expectation.adjustment, input.adjustment);
        assert_eq!(expectation.refund_funder, refund_funder.to_bytes());
        assert_eq!(expectation.reference_digest, input.reference_digest);
        assert_eq!(expectation.execute_after_unix, input.execute_after_unix);

        let mut tampered = artifact;
        tampered.instruction.claim_nonce += 1;
        assert!(matches!(
            refund_claim_evidence_expectation(&intent, &tampered),
            Err(StorageError::InvalidRefundClaimIntent(message))
                if message.contains("digest")
        ));
    }

    #[test]
    fn refund_balance_proof_binding_is_exact_and_fail_closed() {
        let (execution, topology, proof) = refund_proof_storage_fixture();
        assert!(validate_refund_balance_proof_binding(&execution, &topology, &proof).is_ok());
        let mut wrong_topology = topology;
        wrong_topology.refund_source = wrong_topology.tax_vault;
        assert!(matches!(
            validate_refund_balance_proof_binding(&execution, &wrong_topology, &proof),
            Err(StorageError::InvalidRefundBalanceProof(message)) if message.contains("topology")
        ));
        let mut wrong_proof = proof;
        wrong_proof.finalized_slot = 778;
        assert!(matches!(
            validate_refund_balance_proof_binding(&execution, &topology, &wrong_proof),
            Err(StorageError::InvalidRefundBalanceProof(message)) if message.contains("slot")
        ));
    }

    #[test]
    fn encrypted_facts_bind_tenant_and_digest_and_reject_tampering() {
        let cipher = evidence_cipher();
        let merchant_id = Uuid::now_v7();
        let facts = command().facts;
        let facts_digest = facts.canonical_digest().unwrap();
        let encrypted = cipher
            .encrypt_facts(merchant_id, &facts_digest, &facts)
            .unwrap();
        assert_eq!(encrypted.nonce.len(), 12);
        assert!(!encrypted.ciphertext.is_empty());
        assert!(
            !encrypted
                .ciphertext
                .windows(5)
                .any(|window| window == b"USD")
        );

        let decrypted = cipher
            .decrypt_facts(
                merchant_id,
                &facts_digest,
                &encrypted.key_id,
                &encrypted.nonce,
                &encrypted.ciphertext,
            )
            .unwrap();
        assert_eq!(decrypted, serde_json::to_value(&facts).unwrap());

        let other_merchant = cipher.decrypt_facts(
            Uuid::now_v7(),
            &facts_digest,
            &encrypted.key_id,
            &encrypted.nonce,
            &encrypted.ciphertext,
        );
        assert!(matches!(
            other_merchant,
            Err(StorageError::EvidenceIntegrity(_))
        ));

        let mut tampered = encrypted.ciphertext.clone();
        tampered[0] ^= 1;
        assert!(matches!(
            cipher.decrypt_facts(
                merchant_id,
                &facts_digest,
                &encrypted.key_id,
                &encrypted.nonce,
                &tampered,
            ),
            Err(StorageError::EvidenceIntegrity(_))
        ));
    }

    #[test]
    fn evidence_key_ring_resolves_previous_keys_without_changing_active_key() {
        let legacy_cipher = EvidenceCipher::from_base64(
            "legacy-evidence-key-v1",
            "bGVnYWN5LWV2aWRlbmNlLWtleS0zMi1ieXRlcyEhISE=",
        )
        .unwrap();
        let rotated_cipher = EvidenceCipher::from_key_ring_json(
            r#"{
                "current_key_id":"active-evidence-key-v2",
                "keys":[
                    {"key_id":"active-evidence-key-v2","key_b64":"YWN0aXZlLWV2aWRlbmNlLWtleS0zMi1ieXRlcyEhISE="},
                    {"key_id":"legacy-evidence-key-v1","key_b64":"bGVnYWN5LWV2aWRlbmNlLWtleS0zMi1ieXRlcyEhISE="}
                ]
            }"#,
        )
        .unwrap();
        let merchant_id = Uuid::now_v7();
        let facts = command().facts;
        let facts_digest = facts.canonical_digest().unwrap();
        let legacy = legacy_cipher
            .encrypt_facts(merchant_id, &facts_digest, &facts)
            .unwrap();
        assert_eq!(rotated_cipher.key_id(), "active-evidence-key-v2");
        assert_eq!(
            rotated_cipher
                .decrypt_facts(
                    merchant_id,
                    &facts_digest,
                    &legacy.key_id,
                    &legacy.nonce,
                    &legacy.ciphertext,
                )
                .unwrap(),
            serde_json::to_value(&facts).unwrap()
        );
        let rotated = rotated_cipher
            .encrypt_facts(merchant_id, &facts_digest, &facts)
            .unwrap();
        assert_eq!(rotated.key_id, "active-evidence-key-v2");
        assert!(matches!(
            EvidenceCipher::from_base64(
                "active-evidence-key-v2",
                "YWN0aXZlLWV2aWRlbmNlLWtleS0zMi1ieXRlcyEhISE="
            )
            .unwrap()
            .decrypt_facts(
                merchant_id,
                &facts_digest,
                &legacy.key_id,
                &legacy.nonce,
                &legacy.ciphertext,
            ),
            Err(StorageError::EvidenceIntegrity(_))
        ));
    }

    #[test]
    fn evidence_key_ring_rejects_ambiguous_or_oversized_configuration() {
        let entry = serde_json::json!({
            "key_id": "active-evidence-key-v2",
            "key_b64": "YWN0aXZlLWV2aWRlbmNlLWtleS0zMi1ieXRlcyEhISE="
        });

        let duplicate_ids = serde_json::json!({
            "current_key_id": "active-evidence-key-v2",
            "keys": [entry.clone(), entry.clone()]
        });
        assert!(matches!(
            LocalEvidenceKeyProvider::from_json(&duplicate_ids.to_string()),
            Err(StorageError::InvalidConfiguration(message))
                if message.contains("duplicate key IDs")
        ));

        let missing_current = serde_json::json!({
            "current_key_id": "missing-evidence-key",
            "keys": [entry.clone()]
        });
        assert!(matches!(
            LocalEvidenceKeyProvider::from_json(&missing_current.to_string()),
            Err(StorageError::InvalidConfiguration(message))
                if message.contains("current key ID is not present")
        ));

        let oversized = serde_json::json!({
            "current_key_id": "active-evidence-key-v2",
            "keys": vec![entry; MAX_EVIDENCE_KEY_RING_SIZE + 1]
        });
        assert!(matches!(
            LocalEvidenceKeyProvider::from_json(&oversized.to_string()),
            Err(StorageError::InvalidConfiguration(message))
                if message.contains("1 to 8 keys")
        ));
    }

    #[test]
    fn settlement_transition_sql_keeps_all_transition_bindings_distinct() {
        assert!(SETTLEMENT_INTENT_TRANSITION_SQL.contains("SET status = $3"));
        assert!(
            SETTLEMENT_INTENT_TRANSITION_SQL
                .contains("signed_transaction_digest = COALESCE($4, signed_transaction_digest)")
        );
        assert!(SETTLEMENT_INTENT_TRANSITION_SQL.contains("last_error_code = $8"));
        assert!(SETTLEMENT_INTENT_TRANSITION_SQL.contains("WHERE id = $1 AND merchant_id = $2"));
        assert!(SETTLEMENT_INTENT_TRANSITION_SQL.ends_with("AND status = $9"));
    }

    #[test]
    fn vault_release_validation_binds_window_and_nonzero_reference() {
        let command = vault_release_command();
        assert!(validate_vault_release_claim_command(&command).is_ok());

        let mut invalid = command.clone();
        invalid.reference_digest = [0; 32];
        assert!(matches!(
            validate_vault_release_claim_command(&invalid),
            Err(StorageError::InvalidVaultReleaseClaim(message)) if message.contains("reference_digest")
        ));

        let mut too_long = command;
        too_long.expires_at_unix =
            too_long.execute_after_unix + MAX_VAULT_RELEASE_WINDOW.as_secs() as i64 + 1;
        assert!(matches!(
            validate_vault_release_claim_command(&too_long),
            Err(StorageError::InvalidVaultReleaseClaim(message)) if message.contains("window")
        ));
    }

    #[test]
    fn vault_release_transition_requires_evidence_for_chain_states() {
        let base = VaultReleaseClaimTransition {
            merchant_id: Uuid::now_v7(),
            claim_id: Uuid::now_v7(),
            actor_subject: "operator".to_string(),
            to_status: "onchain_created".to_string(),
            evidence: serde_json::json!({"signature": "fixture"}),
            claim_pda: None,
            transaction_signature: None,
            slot: None,
            last_error_code: None,
        };
        assert!(matches!(
            validate_vault_release_transition(&base),
            Err(StorageError::InvalidVaultReleaseClaim(message)) if message.contains("on-chain")
        ));

        let mut executable = base.clone();
        executable.to_status = "executed".to_string();
        executable.transaction_signature = Some("signature".to_string());
        executable.slot = Some(42);
        assert!(validate_vault_release_transition(&executable).is_ok());

        let mut unknown = base;
        unknown.to_status = "unknown".to_string();
        unknown.last_error_code = Some("rpc_timeout".to_string());
        assert!(validate_vault_release_transition(&unknown).is_ok());
    }

    #[test]
    fn quote_retries_only_bounded_serialization_and_deadlock_states() {
        assert_eq!(MAX_QUOTE_RETRY_ATTEMPTS, 3);
        assert!(is_retryable_quote_sql_state("40001"));
        assert!(is_retryable_quote_sql_state("40P01"));
        assert!(!is_retryable_quote_sql_state("57014"));
        assert!(!is_retryable_quote_sql_state("23505"));
    }

    #[test]
    fn refund_intent_retries_only_bounded_serialization_and_deadlock_states() {
        assert_eq!(MAX_REFUND_INTENT_RETRY_ATTEMPTS, 3);
        assert!(is_retryable_refund_intent_sql_state("40001"));
        assert!(is_retryable_refund_intent_sql_state("40P01"));
        assert!(!is_retryable_refund_intent_sql_state("57014"));
        assert!(!is_retryable_refund_intent_sql_state("23505"));
    }

    #[test]
    fn evidence_lookup_retries_only_transient_storage_failures() {
        assert_eq!(MAX_EVIDENCE_LOOKUP_ATTEMPTS, 3);
        assert!(is_retryable_evidence_lookup_error(&StorageError::Pool(
            "connection pool exhausted".to_string()
        )));
        assert!(!is_retryable_evidence_lookup_error(
            &StorageError::QuoteNotFound
        ));
    }

    fn rule_set() -> RuleSet {
        RuleSet {
            id: "us-ca-sales-tax".to_string(),
            version: 1,
            components: vec![TaxComponentRule {
                id: "sales-tax".to_string(),
                label: "Sales tax".to_string(),
                base: TaxBase::single(BaseReference::Net),
                rate: RateSpec::Flat(Rate::new(10, 100).unwrap()),
                price_mode: tax_engine::PriceMode::Exclusive,
                credit_atomic: 0,
                floor_atomic: None,
                cap_atomic: None,
                rounding: RoundingSpec {
                    mode: RoundingMode::HalfUp,
                    scope: RoundingScope::Component,
                    quantum_atomic: 1,
                },
                threshold: None,
                exemption_codes: Vec::new(),
            }],
        }
    }

    #[test]
    fn persisted_rule_documents_require_the_new_envelope_for_new_writes_and_replay_legacy_rows() {
        let rule = rule_set();
        let envelope = serde_json::to_value(RuleSetDocument::new(rule.clone())).unwrap();
        let decoded = decode_persisted_rule_document(envelope).unwrap();
        assert_eq!(decoded, rule);

        let legacy = serde_json::to_value(&rule).unwrap();
        assert_eq!(decode_persisted_rule_document(legacy).unwrap(), rule);

        let mut unsupported = serde_json::to_value(RuleSetDocument::new(rule)).unwrap();
        unsupported["schema"] = serde_json::Value::String("solana-tax-protocol/tax-rule/v3".into());
        assert!(matches!(
            decode_persisted_rule_document(unsupported),
            Err(StorageError::Tax(TaxError::InvalidRuleDocumentSchema))
        ));
    }

    #[test]
    fn persisted_rule_simulation_binds_identity_digest_and_jurisdiction() {
        let rule = rule_set();
        let facts = TaxFacts {
            gross_atomic: 1_100,
            net_atomic: 1_000,
            atomic_decimals: 2,
            currency: "USD".to_string(),
            jurisdiction: "US-CA".to_string(),
            jurisdiction_resolution: None,
            fx_conversion: None,
            exemption_codes: Vec::new(),
            cumulative_period: None,
            applicability: None,
        };
        let digest = rule.canonical_digest().unwrap();
        let envelope = serde_json::to_value(RuleSetDocument::new(rule.clone())).unwrap();
        assert_eq!(
            validate_persisted_rule_for_simulation(
                rule.id.clone(),
                rule.version as i32,
                "US-CA".to_string(),
                "draft".to_string(),
                envelope.clone(),
                &digest,
                &facts,
            )
            .unwrap(),
            rule
        );

        let mut wrong_digest = digest;
        wrong_digest[0] ^= 1;
        assert!(matches!(
            validate_persisted_rule_for_simulation(
                rule.id.clone(),
                rule.version as i32,
                "US-CA".to_string(),
                "draft".to_string(),
                envelope.clone(),
                &wrong_digest,
                &facts,
            ),
            Err(StorageError::RuleSimulationIntegrity(message)) if message.contains("digest")
        ));

        let mut wrong_jurisdiction = facts.clone();
        wrong_jurisdiction.jurisdiction = "GB".to_string();
        assert!(matches!(
            validate_persisted_rule_for_simulation(
                rule.id.clone(),
                rule.version as i32,
                "US-CA".to_string(),
                "draft".to_string(),
                envelope.clone(),
                &digest,
                &wrong_jurisdiction,
            ),
            Err(StorageError::RuleSimulationJurisdictionMismatch)
        ));

        assert!(matches!(
            validate_persisted_rule_for_simulation(
                rule.id,
                1,
                "US-CA".to_string(),
                "retired".to_string(),
                envelope,
                &digest,
                &facts,
            ),
            Err(StorageError::RuleSimulationUnavailable)
        ));
    }

    fn draft_command() -> CreateRuleDraftCommand {
        CreateRuleDraftCommand {
            merchant_id: Uuid::now_v7(),
            actor_subject: "author".to_string(),
            idempotency_key: "draft-1".to_string(),
            external_id: "us-ca-sales-tax".to_string(),
            version: 1,
            jurisdiction: "US-CA".to_string(),
            regime: "sales-tax".to_string(),
            effective_from_unix: 1_798_761_600,
            effective_until_unix: None,
            rule_set: rule_set(),
            provenance: serde_json::json!({"source": "operator"}),
        }
    }

    #[test]
    fn rule_draft_validation_binds_identity_and_effective_window() {
        let command = draft_command();
        assert!(validate_rule_draft_command(&command).is_ok());
        let mut mismatch = command.clone();
        mismatch.rule_set.id = "other-rule".to_string();
        assert!(matches!(
            validate_rule_draft_command(&mismatch),
            Err(StorageError::InvalidRuleDraft(message)) if message.contains("identity")
        ));
        let mut invalid_window = command;
        invalid_window.effective_until_unix = Some(invalid_window.effective_from_unix);
        assert!(matches!(
            validate_rule_draft_command(&invalid_window),
            Err(StorageError::InvalidRuleDraft(message)) if message.contains("effective_until")
        ));
    }

    #[test]
    fn rule_draft_request_digest_changes_when_provenance_or_rule_changes() {
        let first = draft_command();
        let first_digest = first.rule_set.canonical_digest().unwrap();
        let first_request = rule_draft_request_digest(&first, first_digest).unwrap();
        let mut second = first.clone();
        second.provenance = serde_json::json!({"source": "amended"});
        assert_ne!(
            first_request,
            rule_draft_request_digest(&second, first_digest).unwrap()
        );
    }

    #[test]
    fn request_digest_is_stable_and_selector_bound() {
        let first = command();
        let mut second = first.clone();
        assert_eq!(
            quote_request_digest(&first).unwrap(),
            quote_request_digest(&second).unwrap()
        );
        second.rule_version = 2;
        assert_ne!(
            quote_request_digest(&first).unwrap(),
            quote_request_digest(&second).unwrap()
        );
    }

    #[test]
    fn fx_request_digest_is_stable_across_server_issued_quote_replacement() {
        let mut first = command();
        let pre_resolution_digest = first.facts.canonical_digest().unwrap();
        let evidence = persisted_fx_record(first.merchant_id, pre_resolution_digest);
        first.facts.fx_conversion = Some(
            evidence
                .envelope
                .verify_integrity(
                    &ed25519_dalek::VerifyingKey::from_bytes(
                        &evidence.envelope.provider_public_key,
                    )
                    .unwrap(),
                    merchant_identity_digest(first.merchant_id.as_bytes()),
                    pre_resolution_digest,
                )
                .unwrap(),
        );
        first.fx_evidence = Some(evidence);
        let first_digest = quote_request_digest(&first).unwrap();

        let mut replacement = first.clone();
        replacement
            .fx_evidence
            .as_mut()
            .unwrap()
            .envelope
            .unsigned
            .quote_id = "quote-replacement".to_string();
        assert_eq!(first_digest, quote_request_digest(&replacement).unwrap());

        replacement
            .fx_evidence
            .as_mut()
            .unwrap()
            .envelope
            .unsigned
            .source_currency = "GBP".to_string();
        assert_ne!(first_digest, quote_request_digest(&replacement).unwrap());
    }

    #[test]
    fn document_quote_digest_is_schema_and_selector_bound() {
        let facts = TaxDocumentFacts {
            id: "invoice-1".to_string(),
            atomic_decimals: 2,
            currency: "USD".to_string(),
            jurisdiction: "US-CA".to_string(),
            jurisdiction_resolution: None,
            fx_conversion: None,
            cumulative_period: None,
            applicability: None,
            lines: vec![tax_engine::TaxLineFacts {
                id: "line-1".to_string(),
                gross_atomic: 1_100,
                net_atomic: 1_000,
                exemption_codes: Vec::new(),
            }],
        };
        let mut first = DocumentQuoteCommand {
            merchant_id: Uuid::now_v7(),
            idempotency_key: "document-quote-1".to_string(),
            schema: TAX_DOCUMENT_SCHEMA_V1.to_string(),
            rule_set_id: "us-ca-sales-tax".to_string(),
            rule_version: 1,
            jurisdiction: "US-CA".to_string(),
            regime: "sales-tax".to_string(),
            facts,
            applicability_context_id: None,
            jurisdiction_candidates: vec![JurisdictionCandidate {
                jurisdiction_code: "US-CA".to_string(),
                source: jurisdiction::EvidenceSource::ShippingDestination,
                evidence_digest: [7; 32],
            }],
            fx_evidence: None,
        };
        validate_document_quote_command(&first).expect("document quote is valid");
        let digest = document_quote_request_digest(&first).expect("digest");
        let mut second = first.clone();
        second.schema = "solana-tax-protocol/tax-document/v0".to_string();
        assert_ne!(digest, document_quote_request_digest(&second).unwrap());
        assert!(matches!(
            validate_document_quote_command(&second),
            Err(StorageError::InvalidDocumentQuote(_))
        ));
        first.rule_version = 2;
        assert_ne!(digest, document_quote_request_digest(&first).unwrap());
    }

    #[test]
    fn quote_request_digest_binds_server_owned_applicability_context_reference() {
        let mut first = command();
        let baseline = quote_request_digest(&first).unwrap();
        first.applicability_context_id = Some(Uuid::now_v7());
        assert_ne!(baseline, quote_request_digest(&first).unwrap());

        let mut document = DocumentQuoteCommand {
            merchant_id: first.merchant_id,
            idempotency_key: "document-context-digest".to_string(),
            schema: TAX_DOCUMENT_SCHEMA_V1.to_string(),
            rule_set_id: "us-ca-sales-tax".to_string(),
            rule_version: 1,
            jurisdiction: "US-CA".to_string(),
            regime: "sales-tax".to_string(),
            facts: TaxDocumentFacts {
                id: "invoice-context".to_string(),
                atomic_decimals: 2,
                currency: "USD".to_string(),
                jurisdiction: "US-CA".to_string(),
                jurisdiction_resolution: None,
                fx_conversion: None,
                cumulative_period: None,
                applicability: None,
                lines: vec![tax_engine::TaxLineFacts {
                    id: "line-1".to_string(),
                    gross_atomic: 100,
                    net_atomic: 100,
                    exemption_codes: Vec::new(),
                }],
            },
            applicability_context_id: None,
            jurisdiction_candidates: vec![JurisdictionCandidate {
                jurisdiction_code: "US-CA".to_string(),
                source: jurisdiction::EvidenceSource::ShippingDestination,
                evidence_digest: [7; 32],
            }],
            fx_evidence: None,
        };
        let document_baseline = document_quote_request_digest(&document).unwrap();
        document.applicability_context_id = Some(first.applicability_context_id.unwrap());
        assert_ne!(
            document_baseline,
            document_quote_request_digest(&document).unwrap()
        );
    }

    fn persisted_split_plan_fixture() -> (String, String, Vec<split_plan::SplitRecipient>) {
        (
            "8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR".to_string(),
            "GgBaCs3NCBuZN12kCJgAW63ydqohFkHEdfdEXBPzLHq".to_string(),
            vec![
                split_plan::SplitRecipient {
                    token_account: "QWmroo4YnnMqYW3cnxWkFdaTxGD3P7vMSzwMHGbUzwF".to_string(),
                    basis_points: 7_000,
                },
                split_plan::SplitRecipient {
                    token_account: "CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8".to_string(),
                    basis_points: 3_000,
                },
            ],
        )
    }

    #[test]
    fn persisted_split_plan_recomputes_the_canonical_digest() {
        let (merchant_account, mint, recipients) = persisted_split_plan_fixture();
        let plan = split_plan::SplitPlan::new(
            merchant_account.clone(),
            mint.clone(),
            1,
            recipients.clone(),
        )
        .expect("valid plan");
        let loaded = validate_persisted_split_plan(
            Uuid::now_v7(),
            merchant_account,
            mint,
            1,
            plan.digest().expect("digest"),
            recipients,
            "primary".to_string(),
        )
        .expect("plan integrity");
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.recipients.len(), 2);
    }

    #[test]
    fn persisted_split_plan_rejects_tampered_digest_and_recipients() {
        let (merchant_account, mint, mut recipients) = persisted_split_plan_fixture();
        let plan = split_plan::SplitPlan::new(
            merchant_account.clone(),
            mint.clone(),
            1,
            recipients.clone(),
        )
        .expect("valid plan");
        let mut digest = plan.digest().expect("digest");
        digest[0] ^= 1;
        assert!(matches!(
            validate_persisted_split_plan(
                Uuid::now_v7(),
                merchant_account.clone(),
                mint.clone(),
                1,
                digest,
                recipients.clone(),
                "primary".to_string(),
            ),
            Err(StorageError::SettlementSplitPlanIntegrity(_))
        ));
        recipients[0].basis_points = 6_999;
        assert!(matches!(
            validate_persisted_split_plan(
                Uuid::now_v7(),
                merchant_account,
                mint,
                1,
                plan.digest().expect("original digest"),
                recipients,
                "primary".to_string(),
            ),
            Err(StorageError::SettlementSplitPlanIntegrity(_))
        ));
    }

    #[test]
    fn governance_subject_assignment_enforces_four_eyes_lifecycle() {
        let (author, reviewer, approver) =
            assign_governance_subjects("draft", "simulated", "author", None, None, None).unwrap();
        let (author, reviewer, approver) = assign_governance_subjects(
            "simulated",
            "reviewed",
            "reviewer",
            author,
            reviewer,
            approver,
        )
        .unwrap();
        assert!(matches!(
            assign_governance_subjects(
                "reviewed",
                "approved",
                "author",
                author.clone(),
                reviewer.clone(),
                approver.clone(),
            ),
            Err(StorageError::RuleGovernanceConflict)
        ));
        let (_, _, approver) = assign_governance_subjects(
            "reviewed", "approved", "approver", author, reviewer, approver,
        )
        .unwrap();
        assert_eq!(approver.as_deref(), Some("approver"));
    }

    #[test]
    fn rule_release_provenance_requires_a_bounded_versioned_envelope() {
        let provenance = serde_json::json!({
            "schema": "tax-rule-provenance/v1",
            "source_reference": "controlled-record://tax/us-ca/2026-01",
            "source_digest_hex": "ab".repeat(32),
            "legal_tax_owner": "tax-owner:us-ca",
            "jurisdiction_scope": "US-CA",
            "reviewed_at_unix": 1_790_000_000_i64,
            "review_reference": "review://tax/us-ca/2026-01"
        });

        assert!(validate_rule_release_provenance(&provenance, "US-CA").is_ok());
    }

    #[test]
    fn rule_release_provenance_rejects_missing_or_ambiguous_fields() {
        let missing_owner = serde_json::json!({
            "schema": "tax-rule-provenance/v1",
            "source_reference": "controlled-record://tax/us-ca/2026-01",
            "source_digest_hex": "ab".repeat(32),
            "jurisdiction_scope": "US-CA",
            "reviewed_at_unix": 1_790_000_000_i64,
            "review_reference": "review://tax/us-ca/2026-01"
        });
        assert!(matches!(
            validate_rule_release_provenance(&missing_owner, "US-CA"),
            Err(StorageError::InvalidRuleReleaseProvenance(_))
        ));

        let unknown_schema = serde_json::json!({
            "schema": "tax-rule-provenance/v2",
            "source_reference": "controlled-record://tax/us-ca/2026-01",
            "source_digest_hex": "ab".repeat(32),
            "legal_tax_owner": "tax-owner:us-ca",
            "jurisdiction_scope": "US-CA",
            "reviewed_at_unix": 1_790_000_000_i64,
            "review_reference": "review://tax/us-ca/2026-01"
        });
        assert!(matches!(
            validate_rule_release_provenance(&unknown_schema, "US-CA"),
            Err(StorageError::InvalidRuleReleaseProvenance(_))
        ));
    }

    #[test]
    fn rule_release_provenance_rejects_digest_time_and_jurisdiction_substitution() {
        let mut provenance = serde_json::json!({
            "schema": "tax-rule-provenance/v1",
            "source_reference": "controlled-record://tax/us-ca/2026-01",
            "source_digest_hex": "ab".repeat(32),
            "legal_tax_owner": "tax-owner:us-ca",
            "jurisdiction_scope": "US-CA",
            "reviewed_at_unix": 1_790_000_000_i64,
            "review_reference": "review://tax/us-ca/2026-01"
        });

        provenance["source_digest_hex"] = serde_json::json!("not-a-digest");
        assert!(matches!(
            validate_rule_release_provenance(&provenance, "US-CA"),
            Err(StorageError::InvalidRuleReleaseProvenance(_))
        ));

        provenance["source_digest_hex"] = serde_json::json!("ab".repeat(32));
        provenance["reviewed_at_unix"] = serde_json::json!(4_102_444_801_i64);
        assert!(matches!(
            validate_rule_release_provenance(&provenance, "US-CA"),
            Err(StorageError::InvalidRuleReleaseProvenance(_))
        ));

        provenance["reviewed_at_unix"] = serde_json::json!(1_790_000_000_i64);
        provenance["jurisdiction_scope"] = serde_json::json!("GB");
        assert!(matches!(
            validate_rule_release_provenance(&provenance, "US-CA"),
            Err(StorageError::InvalidRuleReleaseProvenance(_))
        ));
    }

    #[test]
    fn stored_quote_round_trips_lossless_amounts() {
        let result = tax_engine::calculate(&rule_set(), &command().facts).unwrap();
        let quote = StoredQuote {
            calculation_id: Uuid::now_v7(),
            status: CALCULATION_STATUS.to_string(),
            expires_at_unix: 1_800_000_900,
            applicability_context_id: None,
            applicability_attestation_digest: None,
            result,
        };
        let encoded = serde_json::to_string(&quote).unwrap();
        assert!(encoded.contains("\"total_tax_atomic\":\"100\""));
        assert_eq!(
            serde_json::from_str::<StoredQuote>(&encoded)
                .unwrap()
                .result
                .total_tax_atomic,
            100
        );
    }

    #[test]
    fn persisted_fx_evidence_binds_projection_and_pre_resolution_facts() {
        let command = command();
        let pre_resolution_digest = command.facts.canonical_digest().unwrap();
        let record = persisted_fx_record(command.merchant_id, pre_resolution_digest);
        let mut persisted_facts = command.facts.clone();
        persisted_facts.fx_conversion = Some(tax_engine::FxConversionEvidence {
            source_currency: "EUR".to_string(),
            target_currency: "USD".to_string(),
            numerator: 2,
            denominator: 3,
            rounding_mode: RoundingMode::HalfEven,
            quantum_atomic: 1,
            provider: "provider-a".to_string(),
            quote_id: "quote-1".to_string(),
            observed_at_unix: 1_800_000_000,
            evidence_digest: [9; 32],
        });

        let evidence_ref = build_fx_evidence_ref(
            command.merchant_id,
            &command.facts,
            &persisted_facts,
            Some(&record),
        )
        .expect("persisted FX evidence");

        assert_eq!(
            evidence_ref
                .as_ref()
                .and_then(|value| value.get("envelope"))
                .and_then(|value| value.get("unsigned"))
                .and_then(|value| value.get("facts_digest"))
                .and_then(|value| value.as_array())
                .map(Vec::len),
            Some(32)
        );

        let mut mismatch = persisted_facts;
        mismatch
            .fx_conversion
            .as_mut()
            .expect("FX projection")
            .numerator = 4;
        assert!(matches!(
            build_fx_evidence_ref(
                command.merchant_id,
                &command.facts,
                &mismatch,
                Some(&record),
            ),
            Err(StorageError::FxEvidenceIntegrity(_))
        ));
    }

    #[test]
    fn persisted_fx_evidence_replay_is_integrity_only_and_fails_closed_on_tampering() {
        let command = command();
        let pre_resolution_digest = command.facts.canonical_digest().unwrap();
        let record = persisted_fx_record(command.merchant_id, pre_resolution_digest);
        let conversion = record
            .envelope
            .verify_integrity(
                &ed25519_dalek::VerifyingKey::from_bytes(&record.envelope.provider_public_key)
                    .unwrap(),
                merchant_identity_digest(command.merchant_id.as_bytes()),
                pre_resolution_digest,
            )
            .unwrap();
        let mut persisted_facts = command.facts.clone();
        persisted_facts.fx_conversion = Some(conversion);
        verify_persisted_fx_evidence(
            command.merchant_id,
            &command.facts,
            &persisted_facts,
            &record,
        )
        .expect("historical FX evidence");

        let mut tampered = record;
        tampered.envelope.payload_digest[0] ^= 1;
        assert!(matches!(
            verify_persisted_fx_evidence(
                command.merchant_id,
                &command.facts,
                &persisted_facts,
                &tampered,
            ),
            Err(StorageError::FxEvidenceIntegrity(_))
        ));
    }

    fn persisted_fx_record(
        merchant_id: Uuid,
        pre_resolution_facts_digest: Digest32,
    ) -> PersistedFxEvidence {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[3; 32]);
        let envelope = fx_evidence::FxEvidenceEnvelope::sign(
            fx_evidence::UnsignedFxEvidence {
                schema_version: fx_evidence::FX_EVIDENCE_SCHEMA_VERSION,
                merchant_id: merchant_identity_digest(merchant_id.as_bytes()),
                facts_digest: pre_resolution_facts_digest,
                source_currency: "EUR".to_string(),
                target_currency: "USD".to_string(),
                numerator: 2,
                denominator: 3,
                rounding_mode: RoundingMode::HalfEven,
                quantum_atomic: 1,
                provider: "provider-a".to_string(),
                quote_id: "quote-1".to_string(),
                observed_at_unix: 1_800_000_000,
                expires_at_unix: 1_800_000_300,
                evidence_digest: [9; 32],
                provider_key_id: [4; 32],
            },
            &signing_key,
        )
        .unwrap();
        PersistedFxEvidence {
            pre_resolution_facts_digest,
            envelope,
        }
    }

    #[test]
    fn persisted_normalized_facts_replay_the_complete_evidence_envelope() {
        let facts = command().facts;
        let rules = rule_set();
        let result = tax_engine::calculate(&rules, &facts).unwrap();
        let verified = verify_persisted_calculation_evidence(
            Uuid::now_v7(),
            Uuid::now_v7(),
            CALCULATION_STATUS.to_string(),
            tax_engine::ENGINE_VERSION.to_string(),
            result.facts_digest.to_vec(),
            serde_json::to_value(&result).unwrap(),
            Some(serde_json::to_value(&facts).unwrap()),
            serde_json::json!({}),
            serde_json::to_value(&rules).unwrap(),
            result.rule_digest.to_vec(),
            tax_engine::ENGINE_VERSION,
        )
        .unwrap();
        assert_eq!(verified.status, CALCULATION_STATUS);
        assert_eq!(verified.facts_digest, result.facts_digest);
        assert_eq!(verified.rule_digest, result.rule_digest);
    }

    #[test]
    fn persisted_replay_rejects_fact_tampering_and_legacy_rows() {
        let mut facts = command().facts;
        let rules = rule_set();
        let result = tax_engine::calculate(&rules, &facts).unwrap();
        facts.net_atomic += 1;
        let tampered = verify_persisted_calculation_evidence(
            Uuid::now_v7(),
            Uuid::now_v7(),
            CALCULATION_STATUS.to_string(),
            tax_engine::ENGINE_VERSION.to_string(),
            result.facts_digest.to_vec(),
            serde_json::to_value(&result).unwrap(),
            Some(serde_json::to_value(&facts).unwrap()),
            serde_json::json!({}),
            serde_json::to_value(&rules).unwrap(),
            result.rule_digest.to_vec(),
            tax_engine::ENGINE_VERSION,
        );
        assert!(matches!(
            tampered,
            Err(StorageError::ReplayEvidenceIntegrity(_))
        ));

        let legacy = verify_persisted_calculation_evidence(
            Uuid::now_v7(),
            Uuid::now_v7(),
            CALCULATION_STATUS.to_string(),
            tax_engine::ENGINE_VERSION.to_string(),
            result.facts_digest.to_vec(),
            serde_json::to_value(&result).unwrap(),
            None,
            serde_json::json!({}),
            serde_json::to_value(&rules).unwrap(),
            result.rule_digest.to_vec(),
            tax_engine::ENGINE_VERSION,
        );
        assert_eq!(
            legacy.unwrap_err().to_string(),
            "stored tax calculation replay evidence is unavailable"
        );
    }

    #[test]
    fn compiled_rule_cache_is_digest_keyed_and_bounded() {
        let cache = CompiledRuleCache::new(2).unwrap();
        let rule = |id: &str| RuleSet {
            id: id.to_string(),
            version: 1,
            components: vec![TaxComponentRule {
                id: "sales-tax".to_string(),
                label: "Sales tax".to_string(),
                base: TaxBase::single(BaseReference::Net),
                rate: RateSpec::Flat(Rate::new(10, 100).unwrap()),
                price_mode: tax_engine::PriceMode::Exclusive,
                credit_atomic: 0,
                floor_atomic: None,
                cap_atomic: None,
                rounding: RoundingSpec {
                    mode: RoundingMode::HalfUp,
                    scope: RoundingScope::Component,
                    quantum_atomic: 1,
                },
                threshold: None,
                exemption_codes: Vec::new(),
            }],
        };
        let first = rule("first");
        let second = rule("second");
        let third = rule("third");
        let first_digest = first.canonical_digest().unwrap();

        cache.get_or_compile(first_digest, &first).unwrap();
        cache.get_or_compile(first_digest, &first).unwrap();
        cache
            .get_or_compile(second.canonical_digest().unwrap(), &second)
            .unwrap();
        cache
            .get_or_compile(third.canonical_digest().unwrap(), &third)
            .unwrap();

        assert_eq!(
            cache.stats(),
            RuleCacheStats {
                hits: 1,
                misses: 3,
                evictions: 1,
                entries: 2,
                capacity: 2,
            }
        );
    }

    #[test]
    fn outbox_options_are_bounded() {
        assert!(validate_outbox_options(1, Duration::from_secs(1)).is_ok());
        assert!(validate_outbox_options(0, Duration::from_secs(1)).is_err());
        assert!(
            validate_outbox_options(MAX_OUTBOX_BATCH_SIZE + 1, Duration::from_secs(1)).is_err()
        );
        assert!(validate_outbox_options(1, Duration::from_secs(301)).is_err());
    }

    #[test]
    fn replay_options_and_reason_encoding_are_bounded() {
        assert!(validate_replay_options(1, Duration::from_secs(1)).is_ok());
        assert!(validate_replay_options(0, Duration::from_secs(1)).is_err());
        assert!(
            validate_replay_options(MAX_REPLAY_BATCH_SIZE + 1, Duration::from_secs(1)).is_err()
        );
        assert!(validate_replay_options(1, Duration::from_secs(301)).is_err());
        for reason in [
            ReplayGapReason::IncompleteProviderPage,
            ReplayGapReason::ProviderDisagreement,
            ReplayGapReason::ProjectionCommitFailure,
            ReplayGapReason::OperatorRequested,
        ] {
            assert_eq!(
                replay_reason_from_db(replay_reason_to_db(reason)).unwrap(),
                reason
            );
        }
    }

    #[test]
    fn outbox_payload_is_versioned_and_tenant_scoped() {
        let merchant_id = command().merchant_id;
        let payload = outbox_payload(merchant_id, serde_json::json!({"status": "ok"}));
        assert_eq!(payload["schema_version"], OUTBOX_PAYLOAD_SCHEMA_VERSION);
        assert_eq!(payload["merchant_id"], merchant_id.to_string());
        assert_eq!(payload["quote"]["status"], "ok");
    }

    #[test]
    fn replay_operator_inputs_and_filters_are_bounded() {
        assert!(validate_replay_status_filter(Some("dead_lettered")).is_ok());
        assert!(validate_replay_status_filter(Some("unknown")).is_err());
        assert!(validate_replay_operator_input("operator-1", "incident INC-1").is_ok());
        assert!(validate_replay_operator_input("", "incident INC-1").is_err());
        assert!(validate_replay_operator_input("operator-1", " ").is_err());
        assert!(
            validate_replay_operator_input(
                "operator-1",
                &"x".repeat(MAX_REPLAY_OPERATOR_NOTE_BYTES + 1)
            )
            .is_err()
        );
    }

    #[test]
    fn indexer_stream_keys_are_bounded() {
        assert!(validate_indexer_stream_key("mainnet/program-1").is_ok());
        assert!(validate_indexer_stream_key(" ").is_err());
        assert!(validate_indexer_stream_key(&"x".repeat(201)).is_err());
    }

    #[test]
    fn settlement_status_validation_matches_the_domain_transition_table() {
        assert_eq!(
            parse_settlement_status("submitted").unwrap(),
            SettlementStatus::Submitted
        );
        assert_eq!(
            parse_settlement_status("submitting").unwrap(),
            SettlementStatus::Submitting
        );
        assert!(parse_settlement_status("made_up").is_err());
        assert!(can_transition(
            SettlementStatus::Signed,
            SettlementStatus::Submitting
        ));
        assert!(can_transition(
            SettlementStatus::Submitting,
            SettlementStatus::Unknown
        ));
        assert!(can_transition(
            SettlementStatus::Unknown,
            SettlementStatus::Finalized
        ));
        assert!(!can_transition(
            SettlementStatus::Finalized,
            SettlementStatus::Submitted
        ));
    }

    #[test]
    fn settlement_digests_are_exactly_32_bytes() {
        assert!(parse_digest(vec![1; 32]).is_ok());
        assert_eq!(
            parse_digest(vec![1; 31]).unwrap_err().to_string(),
            "settlement digest must contain exactly 32 bytes"
        );
    }

    #[test]
    fn refund_funding_configuration_is_bounded_and_digest_bound() {
        let command = refund_funding_configuration_command();
        assert!(validate_refund_funding_configuration_command(&command).is_ok());
        let digest = refund_funding_configuration_digest(&command);
        let request_digest = refund_funding_configuration_request_digest(&command, digest);
        assert_ne!(digest, request_digest);

        let mut aliased = command.clone();
        aliased.net_source = aliased.tax_vault.clone();
        assert!(matches!(
            validate_refund_funding_configuration_command(&aliased),
            Err(StorageError::InvalidRefundFundingConfiguration(message))
                if message.contains("pairwise distinct")
        ));

        let mut changed = command;
        changed.net_source = "Address111111111111111111111111111111111111".to_string();
        assert_ne!(digest, refund_funding_configuration_digest(&changed));
    }

    #[test]
    fn refund_funding_configuration_requires_four_eyes_and_bounded_transitions() {
        let command = RefundFundingConfigurationTransition {
            merchant_id: Uuid::now_v7(),
            configuration_id: Uuid::now_v7(),
            actor_subject: "funding-approver".to_string(),
            to_status: "approved".to_string(),
            evidence: serde_json::json!({"ticket": "INC-1"}),
        };
        assert!(validate_refund_funding_configuration_transition(&command).is_ok());

        let mut invalid = command.clone();
        invalid.to_status = "active".to_string();
        assert!(matches!(
            validate_refund_funding_configuration_transition(&invalid),
            Err(StorageError::InvalidRefundFundingConfigurationTransition)
        ));

        let mut unbounded = command;
        unbounded.evidence =
            serde_json::json!({"note": "x".repeat(MAX_VAULT_RELEASE_EVIDENCE_BYTES)});
        assert!(matches!(
            validate_refund_funding_configuration_transition(&unbounded),
            Err(StorageError::InvalidRefundFundingConfiguration(_))
        ));
    }
}
