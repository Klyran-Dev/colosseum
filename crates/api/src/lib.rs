#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

mod auth;
mod jurisdiction;
pub use auth::JwtAuthenticator;
pub use jurisdiction::JurisdictionProviderRegistry;

use ::jurisdiction::ResolutionPolicy;
use anchor_lang::prelude::Pubkey;
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State, rejection::JsonRejection},
    http::{HeaderMap, HeaderName, HeaderValue, Request, StatusCode, header::CONTENT_TYPE},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use document_payment::{DocumentPaymentError, PaymentScope, authorize_document_payment};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use indexer::{
    FinalizedRefundClaimCancellationEvidence, FinalizedRefundClaimEvidence,
    FinalizedSignaturePageSource, FinalizedTransactionSource, IndexedSettlementAdjustment,
};
use indexer_service::{
    IndexerServiceError, MAX_TARGETED_RECOVERY_PAGES, recover_targeted_signature,
};
use observability::ServiceMetrics;
use policy::{
    AttestationBinding, AttestationError, CalculationAttestation, Digest32,
    DocumentAttestationBinding, DocumentCalculationAttestation, merchant_identity_digest,
};
use refund_workflow::{
    RefundArtifact, RefundCancellationArtifact, RefundCancellationServerInputs,
    RefundClaimServerInputs, RefundExecuteServerInputs, build_cancellation_artifact,
    build_claim_artifact, build_execute_artifact, cancellation_instruction_digest,
    cancellation_request_digest, claim_request_digest, execute_request_digest,
    refund_instruction_digest,
};
use serde::{Deserialize, Serialize};
use settlement_adjustments::{AdjustmentDelta, AdjustmentKind};
use settlement_client::{
    CompiledDocumentSettlementInstruction, CompiledSettlementAdjustmentInstruction,
    CompiledSettlementInstruction, CompiledSplitSettlementInstruction,
    DocumentSettlementCompileRequest as ClientDocumentSettlementCompileRequest,
    SETTLEMENT_PROGRAM_ID,
    SettlementAdjustmentCompileRequest as ClientSettlementAdjustmentCompileRequest,
    SettlementCompileError, SettlementCompileRequest, SplitSettlementCompileRequest,
    compile_document_settlement_instruction, compile_settlement_adjustment_instruction,
    compile_settlement_instruction, compile_split_settlement_instruction,
    compiled_instruction_digest, derive_source_token_account,
};
use settlement_coordinator::{
    AdjustmentRegistryVerificationRequest, CoordinatorError, SettlementCoordinator,
    SettlementProviderHealthSnapshot, SettlementStatus, SignedTransaction, can_transition,
};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use storage::{
    BindAdjustmentAuthority, CreateAdjustmentAuthorityChallenge,
    CreateDocumentPaymentIntentCommand, CreateJurisdictionPolicyCommand,
    CreateRefundCancellationIntentCommand, CreateRefundClaimIntentCommand,
    CreateRefundExecuteIntentCommand, CreateRefundFundingConfigurationCommand,
    CreateSettlementAdjustmentChainIntent, CreateSettlementAdjustmentCommand,
    CreateSettlementAdjustmentEvidenceIntent, CreateVaultReleaseClaimCommand, DatabasePoolStats,
    DocumentQuoteCommand, DocumentQuoteOutcome, EvidenceAccessAuditCommand,
    JurisdictionPolicyGovernanceTransition, PgStore, QuoteCommand, QuoteOutcome,
    RefundCancellationFinalization, RefundCancellationIntentTransition,
    RefundClaimCancellationEvidenceExpectation, RefundClaimIntentTransition,
    RefundExecuteIntentTransition, RefundFundingConfigurationTransition, RuleCacheStats,
    RuleGovernanceTransition, SettlementAdjustmentChainIntent, SettlementAdjustmentChainTransition,
    SettlementAdjustmentEvidenceIntent, SettlementAdjustmentRecord, SettlementAdjustmentTransition,
    SettlementTransition, StorageError, VaultReleaseClaimTransition,
};
use tax_engine::{
    CalculationResult, RuleSet, TAX_DOCUMENT_SCHEMA_V1, TaxDocumentFacts, TaxDocumentResult,
    TaxError, TaxFacts, calculate, calculate_document,
};
use tower_http::{limit::RequestBodyLimitLayer, trace::TraceLayer};
use uuid::Uuid;

const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

#[derive(Clone)]
pub struct AppState {
    pub ready: bool,
    pub simulation_mode: bool,
    pub storage: Option<Arc<PgStore>>,
    pub authenticator: Option<auth::JwtAuthenticator>,
    pub jurisdiction_provider_registry: Option<Arc<JurisdictionProviderRegistry>>,
    pub settlement_coordinator: Option<Arc<SettlementCoordinator>>,
    pub targeted_recovery: Option<Arc<TargetedRecoveryPorts>>,
    pub metrics: Arc<ServiceMetrics>,
}

pub struct TargetedRecoveryPorts {
    pub signature_source: Arc<dyn FinalizedSignaturePageSource>,
    pub transaction_source: Arc<dyn FinalizedTransactionSource>,
    pub stream_key: String,
    pub max_pages: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CalculateRequest {
    pub facts: TaxFacts,
    pub rule_set: RuleSet,
}

#[derive(Debug, Serialize)]
pub struct CalculateResponse {
    pub calculation_id: Uuid,
    pub status: &'static str,
    pub result: CalculationResult,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CalculateDocumentRequest {
    pub schema: String,
    pub facts: TaxDocumentFacts,
    pub rule_set: RuleSet,
}

#[derive(Debug, Serialize)]
pub struct CalculateDocumentResponse {
    pub schema: &'static str,
    pub calculation_id: Uuid,
    pub status: &'static str,
    pub result: TaxDocumentResult,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QuoteRequest {
    pub idempotency_key: String,
    pub rule_set_id: String,
    pub rule_version: u32,
    pub jurisdiction: String,
    pub regime: String,
    pub facts: TaxFacts,
    #[serde(default)]
    pub jurisdiction_evidence: Vec<::jurisdiction::JurisdictionEvidenceEnvelope>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentQuoteRequest {
    pub schema: String,
    pub idempotency_key: String,
    pub rule_set_id: String,
    pub rule_version: u32,
    pub jurisdiction: String,
    pub regime: String,
    pub facts: TaxDocumentFacts,
    #[serde(default)]
    pub jurisdiction_evidence: Vec<::jurisdiction::JurisdictionEvidenceEnvelope>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuleDraftCreateRequest {
    pub idempotency_key: String,
    pub external_id: String,
    pub version: u32,
    pub jurisdiction: String,
    pub regime: String,
    pub effective_from_unix: i64,
    pub effective_until_unix: Option<i64>,
    pub rule_set: RuleSet,
    pub provenance: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuleTransitionRequest {
    pub to_status: String,
    pub evidence: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSimulationRequest {
    pub facts: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JurisdictionPolicyCreateRequest {
    pub idempotency_key: String,
    pub rule_set_id: Uuid,
    pub policy: ResolutionPolicy,
    pub effective_from_unix: i64,
    pub effective_until_unix: Option<i64>,
    pub provenance: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JurisdictionPolicyTransitionRequest {
    pub to_status: String,
    pub evidence: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VaultReleaseClaimCreateRequest {
    pub idempotency_key: String,
    pub mint: String,
    pub token_program: String,
    pub tax_vault: String,
    pub destination: String,
    #[serde(with = "policy::serde_u64_string")]
    pub nonce: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub amount_atomic: u64,
    #[serde(deserialize_with = "deserialize_digest")]
    pub reference_digest: [u8; 32],
    pub execute_after_unix: i64,
    pub expires_at_unix: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VaultReleaseClaimTransitionRequest {
    pub to_status: String,
    pub evidence: serde_json::Value,
    pub claim_pda: Option<String>,
    pub transaction_signature: Option<String>,
    pub slot: Option<u64>,
    pub last_error_code: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementAdjustmentCreateRequest {
    pub idempotency_key: String,
    pub original_settlement_id: Uuid,
    pub asset_id: [u8; 32],
    pub kind: AdjustmentKind,
    pub delta: AdjustmentDelta,
    pub reason_code: String,
    pub evidence_digest: [u8; 32],
    pub source_calculation_id: Uuid,
    pub replacement_calculation_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementAdjustmentTransitionRequest {
    pub to_status: String,
    pub evidence: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefundFundingConfigurationCreateRequest {
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefundFundingConfigurationTransitionRequest {
    pub to_status: String,
    pub evidence: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefundClaimPrepareRequest {
    pub idempotency_key: String,
    pub adjustment_id: Uuid,
    pub confirm: bool,
}

#[derive(Debug, Serialize)]
pub struct RefundClaimPreparationResponse {
    pub status: String,
    pub intent_id: Uuid,
    pub request_digest: Digest32,
    pub instruction_digest: Digest32,
    pub artifact_digest: Digest32,
    pub claim: String,
    pub artifact: RefundArtifact,
    pub signed_transaction_digest: Option<Digest32>,
    pub signature: Option<String>,
    #[serde(with = "policy::serde_optional_u64_string")]
    pub submitted_slot: Option<u64>,
    pub commitment: Option<String>,
    pub last_error_code: Option<String>,
    pub finalized_evidence: Option<FinalizedRefundClaimEvidenceResponse>,
}

#[derive(Debug, Serialize)]
pub struct FinalizedRefundClaimEvidenceResponse {
    pub event_type: &'static str,
    pub event_version: &'static str,
    pub event_discriminator: Vec<u8>,
    pub commitment: &'static str,
    pub signature: String,
    #[serde(with = "policy::serde_u64_string")]
    pub slot: u64,
    pub event_index: u16,
    pub claim: Digest32,
    pub adjustment: Digest32,
    pub original_receipt: Digest32,
    pub merchant: Digest32,
    pub mint: Digest32,
    #[serde(with = "policy::serde_u64_string")]
    pub nonce: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub gross_magnitude: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub tax_magnitude: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub net_magnitude: u64,
    pub refund_funder: Digest32,
    pub refund_destination: Digest32,
    pub proposer: Digest32,
    pub reference_digest: Digest32,
    pub execute_after_unix: i64,
    pub expires_at_unix: i64,
    pub evidence_digest: Digest32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefundClaimSubmitRequest {
    pub intent_id: Uuid,
    pub signed_transaction_base64: String,
}

#[derive(Debug, Serialize)]
pub struct RefundClaimSubmissionResponse {
    pub intent_id: Uuid,
    pub status: String,
    pub signature: Option<String>,
    pub provider_index: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefundCancellationPrepareRequest {
    pub idempotency_key: String,
    pub claim_intent_id: Uuid,
    pub confirm: bool,
}

#[derive(Debug, Serialize)]
pub struct RefundCancellationPreparationResponse {
    pub status: String,
    pub intent_id: Uuid,
    pub request_digest: Digest32,
    pub instruction_digest: Digest32,
    pub artifact_digest: Digest32,
    pub claim: String,
    pub artifact: RefundCancellationArtifact,
    pub signed_transaction_digest: Option<Digest32>,
    pub signature: Option<String>,
    #[serde(with = "policy::serde_optional_u64_string")]
    pub submitted_slot: Option<u64>,
    pub commitment: Option<String>,
    pub last_error_code: Option<String>,
    pub finalized_evidence: Option<FinalizedRefundCancellationEvidenceResponse>,
}

#[derive(Debug, Serialize)]
pub struct FinalizedRefundCancellationEvidenceResponse {
    pub event_type: &'static str,
    pub event_version: &'static str,
    pub event_discriminator: Vec<u8>,
    pub commitment: &'static str,
    pub signature: String,
    #[serde(with = "policy::serde_u64_string")]
    pub slot: u64,
    pub event_index: u16,
    pub claim: Digest32,
    pub adjustment: Digest32,
    pub merchant: Digest32,
    pub mint: Digest32,
    #[serde(with = "policy::serde_u64_string")]
    pub nonce: u64,
    pub governance_authority: Digest32,
    pub reference_digest: Digest32,
    pub evidence_digest: Digest32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefundCancellationSubmitRequest {
    pub intent_id: Uuid,
    pub signed_transaction_base64: String,
}

#[derive(Debug, Serialize)]
pub struct RefundCancellationSubmissionResponse {
    pub intent_id: Uuid,
    pub status: String,
    pub signature: Option<String>,
    pub provider_index: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefundExecutePrepareRequest {
    pub idempotency_key: String,
    pub claim_intent_id: Uuid,
    pub confirm: bool,
}

#[derive(Debug, Serialize)]
pub struct RefundExecutePreparationResponse {
    pub status: String,
    pub intent_id: Uuid,
    pub request_digest: Digest32,
    pub instruction_digest: Digest32,
    pub artifact_digest: Digest32,
    pub claim: String,
    pub artifact: RefundArtifact,
    pub signed_transaction_digest: Option<Digest32>,
    pub signature: Option<String>,
    #[serde(with = "policy::serde_optional_u64_string")]
    pub submitted_slot: Option<u64>,
    pub commitment: Option<String>,
    pub last_error_code: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefundExecuteSubmitRequest {
    pub intent_id: Uuid,
    pub signed_transaction_base64: String,
}

#[derive(Debug, Serialize)]
pub struct RefundExecuteSubmissionResponse {
    pub intent_id: Uuid,
    pub status: String,
    pub signature: Option<String>,
    pub provider_index: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdjustmentAuthorityRoleRequest {
    pub role: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdjustmentAuthorityBindRequest {
    pub role: String,
    pub challenge_id: Uuid,
    pub public_key: String,
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct AdjustmentAuthorityChallengeResponse {
    pub challenge_id: Uuid,
    pub role: String,
    pub message: String,
    pub message_digest: Digest32,
    pub expires_at_unix: i64,
}

#[derive(Debug, Serialize)]
pub struct AdjustmentAuthorityBindingResponse {
    pub binding_id: Uuid,
    pub role: String,
    pub subject: String,
    pub public_key: String,
    pub challenge_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementAdjustmentEvidenceCompileRequest {
    pub confirm: bool,
}

#[derive(Debug, Serialize)]
pub struct CompiledSettlementAdjustmentResponse {
    pub status: &'static str,
    pub intent_id: Uuid,
    pub chain_intent_id: Uuid,
    pub request_digest: Digest32,
    pub instruction_digest: Digest32,
    pub registry: String,
    #[serde(with = "policy::serde_u64_string")]
    pub registry_slot: u64,
    pub registry_commitment: &'static str,
    pub registry_account_digest: Digest32,
    pub instruction: CompiledSettlementAdjustmentInstruction,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubmitSettlementAdjustmentRequest {
    pub chain_intent_id: Uuid,
    pub signed_transaction_base64: String,
    pub instruction: CompiledSettlementAdjustmentInstruction,
}

#[derive(Debug, Serialize)]
pub struct SubmitSettlementAdjustmentResponse {
    pub chain_intent_id: Uuid,
    pub status: String,
    pub signature: Option<String>,
    pub provider_index: Option<usize>,
    pub finalized_evidence: Option<FinalizedSettlementAdjustmentEvidenceResponse>,
}

#[derive(Debug, Serialize)]
pub struct FinalizedSettlementAdjustmentEvidenceResponse {
    pub event_type: &'static str,
    pub event_version: &'static str,
    pub event_discriminator: Vec<u8>,
    pub commitment: &'static str,
    pub signature: String,
    #[serde(with = "policy::serde_u64_string")]
    pub slot: u64,
    pub event_index: u16,
    pub adjustment: Digest32,
    pub original_receipt: Digest32,
    pub merchant: Digest32,
    pub mint: Digest32,
    #[serde(with = "policy::serde_u64_string")]
    pub nonce: u64,
    pub kind: u8,
    pub gross_sign: u8,
    #[serde(with = "policy::serde_u64_string")]
    pub gross_magnitude: u64,
    pub tax_sign: u8,
    #[serde(with = "policy::serde_u64_string")]
    pub tax_magnitude: u64,
    pub net_sign: u8,
    #[serde(with = "policy::serde_u64_string")]
    pub net_magnitude: u64,
    pub adjustment_digest: Digest32,
    pub adjustment_evidence_digest: Digest32,
    pub event_evidence_digest: Digest32,
    pub approver: Digest32,
}

fn finalized_adjustment_evidence_response(
    record: IndexedSettlementAdjustment,
) -> FinalizedSettlementAdjustmentEvidenceResponse {
    FinalizedSettlementAdjustmentEvidenceResponse {
        event_type: "SettlementAdjustmentRecorded",
        event_version: "v1",
        event_discriminator: indexer::SETTLEMENT_ADJUSTMENT_RECORDED_EVENT_DISCRIMINATOR.to_vec(),
        commitment: "finalized",
        signature: record.signature,
        slot: record.slot,
        event_index: record.event_index,
        adjustment: record.adjustment,
        original_receipt: record.original_receipt,
        merchant: record.merchant,
        mint: record.mint,
        nonce: record.nonce,
        kind: record.kind,
        gross_sign: record.gross_sign,
        gross_magnitude: record.gross_magnitude,
        tax_sign: record.tax_sign,
        tax_magnitude: record.tax_magnitude,
        net_sign: record.net_sign,
        net_magnitude: record.net_magnitude,
        adjustment_digest: record.adjustment_digest,
        adjustment_evidence_digest: record.adjustment_evidence_digest,
        event_evidence_digest: record.evidence_digest,
        approver: record.approver,
    }
}

fn expected_finalized_adjustment_record(
    chain_intent: &SettlementAdjustmentChainIntent,
    evidence_intent: &SettlementAdjustmentEvidenceIntent,
    adjustment: &SettlementAdjustmentRecord,
    signature: String,
    slot: u64,
) -> Result<IndexedSettlementAdjustment, ApiError> {
    let adjustment_pda = Pubkey::find_program_address(
        &[
            b"adjustment",
            &evidence_intent.original_receipt,
            &evidence_intent.nonce.to_le_bytes(),
        ],
        &Pubkey::from_str(SETTLEMENT_PROGRAM_ID).map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "SETTLEMENT_PROGRAM_CONFIGURATION_INVALID",
            message: "configured settlement program id is invalid".into(),
        })?,
    )
    .0;
    let (gross_sign, gross_magnitude) = signed_magnitude_u64(adjustment.delta.gross_atomic)?;
    let (tax_sign, tax_magnitude) = signed_magnitude_u64(adjustment.delta.tax_atomic)?;
    let (net_sign, net_magnitude) = signed_magnitude_u64(adjustment.delta.net_atomic)?;
    Ok(IndexedSettlementAdjustment {
        signature,
        slot,
        event_index: chain_intent.finalized_event_index.unwrap_or(0),
        adjustment: adjustment_pda.to_bytes(),
        original_receipt: evidence_intent.original_receipt,
        merchant: evidence_intent.merchant,
        mint: evidence_intent.mint,
        nonce: evidence_intent.nonce,
        kind: adjustment_kind_tag(adjustment.kind),
        gross_sign,
        gross_magnitude,
        tax_sign,
        tax_magnitude,
        net_sign,
        net_magnitude,
        adjustment_digest: adjustment.adjustment_digest,
        adjustment_evidence_digest: adjustment.evidence_digest,
        evidence_digest: chain_intent
            .finalized_event_evidence_digest
            .unwrap_or([1; 32]),
        approver: evidence_intent.approver,
    })
}

async fn load_finalized_adjustment_evidence_response(
    store: &PgStore,
    intent: &SettlementAdjustmentChainIntent,
    merchant_id: Uuid,
) -> Result<Option<FinalizedSettlementAdjustmentEvidenceResponse>, ApiError> {
    if intent.status != SettlementStatus::Finalized.as_str() {
        return Ok(None);
    }
    let signature = intent.signature.clone().ok_or_else(|| ApiError {
        status: StatusCode::CONFLICT,
        code: "ADJUSTMENT_FINALIZED_PROOF_INCOMPLETE",
        message: "finalized adjustment intent is missing its server-bound signature".into(),
    })?;
    let slot = intent.slot.ok_or_else(|| ApiError {
        status: StatusCode::CONFLICT,
        code: "ADJUSTMENT_FINALIZED_PROOF_INCOMPLETE",
        message: "finalized adjustment intent is missing its finalized slot".into(),
    })?;
    if intent.commitment.as_deref() != Some("finalized")
        || intent.finalized_event_index.is_none()
        || intent.finalized_event_evidence_digest.is_none()
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "ADJUSTMENT_FINALIZED_PROOF_INCOMPLETE",
            message: "finalized adjustment intent is missing its typed-event proof binding".into(),
        });
    }
    let evidence_intent = store
        .load_settlement_adjustment_evidence_intent(intent.evidence_intent_id, merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError {
            status: StatusCode::CONFLICT,
            code: "ADJUSTMENT_EVIDENCE_INTENT_NOT_FOUND",
            message: "immutable adjustment evidence is unavailable for the finalized proof".into(),
        })?;
    let adjustment = store
        .load_settlement_adjustment(intent.adjustment_id, merchant_id)
        .await
        .map_err(ApiError::from)?;
    let expected = expected_finalized_adjustment_record(
        intent,
        &evidence_intent,
        &adjustment,
        signature,
        slot,
    )?;
    let record = store
        .load_finalized_settlement_adjustment_record(intent.id, merchant_id, &expected)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError {
            status: StatusCode::CONFLICT,
            code: "ADJUSTMENT_FINALIZED_PROOF_NOT_FOUND",
            message: "finalized status has no matching typed event evidence".into(),
        })?;
    Ok(Some(finalized_adjustment_evidence_response(record)))
}

#[derive(Serialize)]
struct AdjustmentCompilerDigestInput<'a> {
    schema: &'static str,
    adjustment_id: Uuid,
    adjustment_digest: Digest32,
    evidence_digest: Digest32,
    source_calculation_id: Uuid,
    replacement_calculation_id: Option<Uuid>,
    kind: AdjustmentKind,
    delta: AdjustmentDelta,
    asset_id: [u8; 32],
    settlement_id: Uuid,
    signature: &'a str,
    slot: u64,
    receipt: [u8; 32],
    merchant: [u8; 32],
    mint: [u8; 32],
    proposer: &'a [u8; 32],
    approver: &'a [u8; 32],
    token_program: &'a str,
    registry: &'a str,
    registry_slot: u64,
    registry_commitment: &'a str,
    registry_account_digest: Digest32,
}

#[derive(Debug, Serialize)]
pub struct SettlementAdjustmentTransitionResponse {
    pub status: &'static str,
    pub adjustment_id: Uuid,
    pub from_status: String,
    pub to_status: String,
    pub approver_subject: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VaultReleaseClaimTransitionResponse {
    pub status: &'static str,
    pub claim_id: Uuid,
    pub from_status: String,
    pub to_status: String,
    pub approver_subject: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct JurisdictionPolicyTransitionResponse {
    pub status: &'static str,
    pub policy_id: Uuid,
    pub from_status: String,
    pub to_status: String,
    pub author_subject: Option<String>,
    pub reviewer_subject: Option<String>,
    pub approver_subject: Option<String>,
    pub governance_revision: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayGapListQuery {
    pub stream_key: String,
    pub status: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultReconciliationListQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayGapOperatorRequest {
    pub note: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetedRecoveryRequest {
    pub address: String,
    pub signature: String,
    pub note: String,
    pub max_pages: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct TargetedRecoveryResponse {
    pub status: &'static str,
    pub audit_id: Uuid,
    pub signature: String,
    pub pages_scanned: usize,
    pub slot: u64,
    pub raw_transaction_count: usize,
    pub raw_evidence_count: usize,
    pub receipt_count: usize,
}

#[derive(Debug, Serialize)]
pub struct RuleTransitionResponse {
    pub status: &'static str,
    pub rule_set_id: Uuid,
    pub from_status: String,
    pub to_status: String,
    pub author_subject: Option<String>,
    pub reviewer_subject: Option<String>,
    pub approver_subject: Option<String>,
    pub governance_revision: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VerifyAttestationRequest {
    pub attestation: CalculationAttestation,
    pub binding: AttestationBindingRequest,
    pub now_unix: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AttestationBindingRequest {
    pub merchant_id: Digest32,
    pub rule_set_id: String,
    pub rule_version: u32,
    pub jurisdiction: String,
    pub currency: String,
    pub asset_mint: Digest32,
    pub facts_digest: Digest32,
    pub rule_digest: Digest32,
    #[serde(with = "policy::serde_u64_string")]
    pub gross_atomic: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub tax_atomic: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub net_atomic: u64,
}

#[derive(Debug, Serialize)]
pub struct VerifyAttestationResponse {
    pub status: &'static str,
    pub payload_digest: Digest32,
    pub issuer_public_key: [u8; 32],
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuoteAttestationVerifyRequest {
    pub route_id: Uuid,
    pub attestation: CalculationAttestation,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentQuoteAttestationVerifyRequest {
    pub route_id: Uuid,
    pub attestation: DocumentCalculationAttestation,
}

#[derive(Debug, Serialize)]
pub struct QuoteAttestationVerifyResponse {
    pub status: &'static str,
    pub calculation_id: Uuid,
    pub payload_digest: Digest32,
    pub issuer_public_key: [u8; 32],
    pub expires_at_unix: i64,
}

#[derive(Debug, Serialize)]
pub struct DocumentQuoteAttestationVerifyResponse {
    pub status: &'static str,
    pub calculation_id: Uuid,
    pub payload_digest: Digest32,
    pub issuer_public_key: [u8; 32],
    pub expires_at_unix: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SubmitSettlementRequest {
    pub intent_id: Uuid,
    pub signed_transaction_base64: String,
    pub instruction: SubmittedSettlementInstruction,
}

/// Document and split variants are listed before the legacy fallback because
/// their provenance fields are part of the signed-message binding contract.
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SubmittedSettlementInstruction {
    Document(CompiledDocumentSettlementInstruction),
    Split(CompiledSplitSettlementInstruction),
    Legacy(CompiledSettlementInstruction),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementCompileFromQuoteRequest {
    pub calculation_id: Uuid,
    pub route_id: Uuid,
    pub payer: String,
    pub nonce: u64,
    pub policy_sequence: u64,
    #[serde(deserialize_with = "deserialize_digest")]
    pub policy_digest: [u8; 32],
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentSettlementCompileFromQuoteRequest {
    pub calculation_id: Uuid,
    pub route_id: Uuid,
    pub idempotency_key: String,
    pub payer: String,
    pub nonce: u64,
    pub policy_sequence: u64,
    #[serde(deserialize_with = "deserialize_digest")]
    pub policy_digest: [u8; 32],
    pub scope: PaymentScope,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SplitSettlementCompileFromQuoteRequest {
    pub calculation_id: Uuid,
    pub route_id: Uuid,
    pub split_plan_id: Uuid,
    pub payer: String,
    pub nonce: u64,
    pub policy_sequence: u64,
    #[serde(deserialize_with = "deserialize_digest")]
    pub policy_digest: [u8; 32],
}

#[derive(Debug, Serialize)]
pub struct SubmitSettlementResponse {
    pub intent_id: Uuid,
    pub status: String,
    pub signature: Option<String>,
    pub provider_index: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct CompiledSettlementResponse {
    #[serde(flatten)]
    pub instruction: CompiledSettlementInstruction,
    pub intent_id: Option<Uuid>,
    pub request_digest: Digest32,
    pub instruction_digest: Digest32,
    pub route_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct CompiledDocumentSettlementResponse {
    #[serde(flatten)]
    pub instruction: CompiledDocumentSettlementInstruction,
    pub intent_id: Option<Uuid>,
    pub request_digest: Digest32,
    pub instruction_digest: Digest32,
    pub route_id: Uuid,
    pub authorization: storage::DocumentPaymentAuthorizationRecord,
}

#[derive(Debug, Serialize)]
pub struct CompiledSplitSettlementResponse {
    #[serde(flatten)]
    pub instruction: CompiledSplitSettlementInstruction,
    pub intent_id: Option<Uuid>,
    pub request_digest: Digest32,
    pub instruction_digest: Digest32,
    pub route_id: Uuid,
    pub split_plan_id: Uuid,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

async fn assign_request_id(mut request: Request<Body>, next: Next) -> Response {
    let request_id = Uuid::now_v7().to_string();
    let request_id_header = match HeaderValue::try_from(request_id) {
        Ok(value) => value,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "request correlation unavailable",
            )
                .into_response();
        }
    };
    request
        .headers_mut()
        .insert(REQUEST_ID_HEADER.clone(), request_id_header.clone());
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(REQUEST_ID_HEADER.clone(), request_id_header);
    response
}

impl From<TaxError> for ApiError {
    fn from(error: TaxError) -> Self {
        let code = match error {
            TaxError::InvalidRule(_) => "INVALID_RULE",
            TaxError::InvalidRuleDocumentSchema => "INVALID_RULE_DOCUMENT_SCHEMA",
            TaxError::InvalidFacts(_) => "INVALID_FACTS",
            TaxError::MissingComponent(_) => "MISSING_COMPONENT",
            TaxError::DependencyCycle(_) => "DEPENDENCY_CYCLE",
            TaxError::ArithmeticOverflow => "ARITHMETIC_OVERFLOW",
            TaxError::UnsupportedInclusive(_) => "UNSUPPORTED_INCLUSIVE_RULE",
            TaxError::MissingCumulativeSnapshot => "MISSING_CUMULATIVE_SNAPSHOT",
            TaxError::UnsupportedThreshold(_) => "UNSUPPORTED_THRESHOLD_RULE",
            TaxError::UnsupportedDocumentScope(_) => "UNSUPPORTED_DOCUMENT_SCOPE",
            TaxError::EngineVersionMismatch { .. } => "ENGINE_VERSION_MISMATCH",
            TaxError::ReplayMismatch => "CALCULATION_REPLAY_MISMATCH",
            TaxError::JurisdictionResolution(_) => "JURISDICTION_RESOLUTION_FAILED",
        };
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code,
            message: error.to_string(),
        }
    }
}

impl From<AttestationError> for ApiError {
    fn from(error: AttestationError) -> Self {
        let code = match error {
            AttestationError::UnsupportedSchemaVersion => "UNSUPPORTED_ATTESTATION_SCHEMA",
            AttestationError::InvalidTextField => "INVALID_ATTESTATION_TEXT",
            AttestationError::InvalidCurrency => "INVALID_ATTESTATION_CURRENCY",
            AttestationError::EmptyIdentityOrDigest => "EMPTY_ATTESTATION_IDENTITY_OR_DIGEST",
            AttestationError::InvalidRuleVersion => "INVALID_ATTESTATION_RULE_VERSION",
            AttestationError::AmountConservationViolation => {
                "ATTESTATION_AMOUNT_CONSERVATION_VIOLATION"
            }
            AttestationError::InvalidValidityWindow => "INVALID_ATTESTATION_VALIDITY_WINDOW",
            AttestationError::InvalidSequence => "INVALID_ATTESTATION_SEQUENCE",
            AttestationError::EncodingFailure => "ATTESTATION_ENCODING_FAILURE",
            AttestationError::InvalidIssuerKey => "INVALID_ATTESTATION_ISSUER_KEY",
            AttestationError::InvalidSignature => "INVALID_ATTESTATION_SIGNATURE",
            AttestationError::PayloadDigestMismatch => "ATTESTATION_PAYLOAD_DIGEST_MISMATCH",
            AttestationError::OutsideValidityWindow => "ATTESTATION_OUTSIDE_VALIDITY_WINDOW",
            AttestationError::BindingMismatch => "ATTESTATION_BINDING_MISMATCH",
        };
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code,
            message: error.to_string(),
        }
    }
}

impl From<DocumentPaymentError> for ApiError {
    fn from(error: DocumentPaymentError) -> Self {
        let status = match error {
            DocumentPaymentError::UseFullDocumentScope => StatusCode::BAD_REQUEST,
            DocumentPaymentError::EmptySourceAttestationDigest
            | DocumentPaymentError::EmptyLineSubset
            | DocumentPaymentError::LineSubsetTooLarge
            | DocumentPaymentError::InvalidLineId
            | DocumentPaymentError::DuplicateLineId
            | DocumentPaymentError::UnknownLineId
            | DocumentPaymentError::AmountOutOfRange
            | DocumentPaymentError::AmountConservationViolation
            | DocumentPaymentError::ArithmeticOverflow
            | DocumentPaymentError::InvalidDocumentResult => StatusCode::UNPROCESSABLE_ENTITY,
        };
        Self {
            status,
            code: "INVALID_DOCUMENT_PAYMENT_SCOPE",
            message: error.to_string(),
        }
    }
}

impl From<StorageError> for ApiError {
    fn from(error: StorageError) -> Self {
        let (status, code, message) = match error {
            StorageError::InvalidConfiguration(message) => {
                (StatusCode::BAD_REQUEST, "INVALID_QUOTE_REQUEST", message)
            }
            StorageError::Pool(_) | StorageError::Database(_) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "QUOTE_STORAGE_UNAVAILABLE",
                "quote storage is temporarily unavailable".to_string(),
            ),
            StorageError::Serialization(_) | StorageError::InvalidStoredResponse => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "QUOTE_EVIDENCE_CORRUPT",
                "stored quote evidence is invalid".to_string(),
            ),
            StorageError::ReplayEvidenceUnavailable => (
                StatusCode::CONFLICT,
                "QUOTE_REPLAY_UNAVAILABLE",
                "this legacy quote does not contain sufficient normalized evidence for replay"
                    .to_string(),
            ),
            StorageError::ReplayEvidenceIntegrity(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "QUOTE_REPLAY_EVIDENCE_CORRUPT",
                "stored quote replay evidence failed integrity validation".to_string(),
            ),
            StorageError::DocumentReplayEvidenceUnavailable => (
                StatusCode::CONFLICT,
                "DOCUMENT_QUOTE_REPLAY_UNAVAILABLE",
                "the document quote does not contain sufficient evidence for replay".to_string(),
            ),
            StorageError::DocumentReplayEvidenceIntegrity(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "DOCUMENT_QUOTE_REPLAY_EVIDENCE_CORRUPT",
                "stored document quote replay evidence failed integrity validation".to_string(),
            ),
            StorageError::DocumentQuoteNotFound => (
                StatusCode::NOT_FOUND,
                "DOCUMENT_QUOTE_NOT_FOUND",
                "the document quote was not found for this merchant".to_string(),
            ),
            StorageError::InvalidDocumentQuote(message) => (
                StatusCode::BAD_REQUEST,
                "INVALID_DOCUMENT_QUOTE",
                message,
            ),
            StorageError::DocumentCalculationScopeMismatch
            | StorageError::AggregateCalculationScopeMismatch => (
                StatusCode::CONFLICT,
                "CALCULATION_SCOPE_MISMATCH",
                "the calculation belongs to a different authority boundary".to_string(),
            ),
            StorageError::EvidenceCipherUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "QUOTE_EVIDENCE_KEY_UNAVAILABLE",
                "quote evidence encryption is not configured".to_string(),
            ),
            StorageError::EvidenceEncryption(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "QUOTE_EVIDENCE_ENCRYPTION_FAILED",
                "quote evidence encryption failed".to_string(),
            ),
            StorageError::EvidenceIntegrity(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "QUOTE_EVIDENCE_REPLAY_INTEGRITY_FAILED",
                "stored encrypted quote evidence failed integrity validation".to_string(),
            ),
            StorageError::EvidenceTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "QUOTE_EVIDENCE_TOO_LARGE",
                "normalized quote evidence exceeds the bounded size".to_string(),
            ),
            StorageError::QuoteNotFound => (
                StatusCode::NOT_FOUND,
                "QUOTE_NOT_FOUND",
                "the quote was not found for this merchant".to_string(),
            ),
            StorageError::QuoteExpired => (
                StatusCode::CONFLICT,
                "QUOTE_EXPIRED",
                "the quote has expired; create a new quote before settlement".to_string(),
            ),
            StorageError::QuoteNotSettleable => (
                StatusCode::CONFLICT,
                "QUOTE_NOT_SETTLEABLE",
                "the quote is not in a settleable state".to_string(),
            ),
            StorageError::AttestationNotFound => (
                StatusCode::NOT_FOUND,
                "CALCULATION_ATTESTATION_NOT_FOUND",
                "no calculation attestation was found for this merchant and calculation"
                    .to_string(),
            ),
            StorageError::AttestationConflict => (
                StatusCode::CONFLICT,
                "CALCULATION_ATTESTATION_CONFLICT",
                "the calculation already has different immutable attestation evidence".to_string(),
            ),
            StorageError::InvalidAttestation(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_CALCULATION_ATTESTATION",
                message,
            ),
            StorageError::AttestationEvidenceTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "ATTESTATION_EVIDENCE_TOO_LARGE",
                "calculation attestation evidence exceeds the bounded size".to_string(),
            ),
            StorageError::SettlementRouteNotFound => (
                StatusCode::NOT_FOUND,
                "SETTLEMENT_ROUTE_NOT_FOUND",
                "no settlement route was found for this merchant".to_string(),
            ),
            StorageError::SettlementRouteUnavailable => (
                StatusCode::CONFLICT,
                "SETTLEMENT_ROUTE_UNAVAILABLE",
                "the settlement route is not active or is outside its effective window".to_string(),
            ),
            StorageError::SettlementRouteVerificationInvalid(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "SETTLEMENT_ROUTE_VERIFICATION_INVALID",
                message,
            ),
            StorageError::SettlementRouteIntegrity(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SETTLEMENT_ROUTE_INTEGRITY_FAILURE",
                message,
            ),
            StorageError::SettlementRouteActivationConflict => (
                StatusCode::CONFLICT,
                "SETTLEMENT_ROUTE_ACTIVATION_CONFLICT",
                "the settlement route cannot be activated in its current lifecycle state"
                    .to_string(),
            ),
            StorageError::RefundFundingConfigurationNotFound => (
                StatusCode::NOT_FOUND,
                "REFUND_FUNDING_CONFIGURATION_NOT_FOUND",
                "no refund funding configuration was found for this merchant".to_string(),
            ),
            StorageError::RefundFundingConfigurationIdempotencyConflict => (
                StatusCode::CONFLICT,
                "REFUND_FUNDING_CONFIGURATION_IDEMPOTENCY_CONFLICT",
                "the idempotency key was already used for different refund funding input"
                    .to_string(),
            ),
            StorageError::InvalidRefundFundingConfiguration(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_REFUND_FUNDING_CONFIGURATION",
                message,
            ),
            StorageError::InvalidRefundFundingConfigurationTransition => (
                StatusCode::CONFLICT,
                "INVALID_REFUND_FUNDING_CONFIGURATION_TRANSITION",
                "the refund funding configuration cannot take that lifecycle transition"
                    .to_string(),
            ),
            StorageError::RefundFundingConfigurationGovernanceConflict => (
                StatusCode::FORBIDDEN,
                "REFUND_FUNDING_CONFIGURATION_GOVERNANCE_CONFLICT",
                "the authenticated subject cannot perform that funding configuration transition"
                    .to_string(),
            ),
            StorageError::RefundFundingConfigurationVerificationInvalid(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "REFUND_FUNDING_CONFIGURATION_VERIFICATION_INVALID",
                message,
            ),
            StorageError::RefundFundingConfigurationActivationConflict => (
                StatusCode::CONFLICT,
                "REFUND_FUNDING_CONFIGURATION_ACTIVATION_CONFLICT",
                "the refund funding configuration cannot be activated in its current lifecycle state"
                    .to_string(),
            ),
            StorageError::RefundClaimIntentNotFound => (
                StatusCode::NOT_FOUND,
                "REFUND_CLAIM_INTENT_NOT_FOUND",
                "the refund claim intent was not found for this merchant".to_string(),
            ),
            StorageError::RefundClaimIntentIdempotencyConflict => (
                StatusCode::CONFLICT,
                "REFUND_CLAIM_INTENT_IDEMPOTENCY_CONFLICT",
                "the refund claim idempotency key was reused for different input".to_string(),
            ),
            StorageError::InvalidRefundClaimIntent(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_REFUND_CLAIM_INTENT",
                message,
            ),
            StorageError::RefundClaimNonceAllocationConflict => (
                StatusCode::CONFLICT,
                "REFUND_CLAIM_NONCE_ALLOCATION_CONFLICT",
                "a refund claim nonce could not be allocated safely; retry the request".to_string(),
            ),
            StorageError::RefundExecuteIntentNotFound => (
                StatusCode::NOT_FOUND,
                "REFUND_EXECUTE_INTENT_NOT_FOUND",
                "the refund execute intent was not found for this merchant".to_string(),
            ),
            StorageError::RefundExecuteIntentIdempotencyConflict => (
                StatusCode::CONFLICT,
                "REFUND_EXECUTE_INTENT_IDEMPOTENCY_CONFLICT",
                "the refund execute idempotency key was reused for different input".to_string(),
            ),
            StorageError::InvalidRefundExecuteIntent(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_REFUND_EXECUTE_INTENT",
                message,
            ),
            StorageError::RefundCancellationIntentNotFound => (
                StatusCode::NOT_FOUND,
                "REFUND_CANCELLATION_INTENT_NOT_FOUND",
                "the refund cancellation intent was not found for this merchant".to_string(),
            ),
            StorageError::RefundCancellationIntentIdempotencyConflict => (
                StatusCode::CONFLICT,
                "REFUND_CANCELLATION_INTENT_IDEMPOTENCY_CONFLICT",
                "the refund cancellation idempotency key was reused for different input"
                    .to_string(),
            ),
            StorageError::InvalidRefundCancellationIntent(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_REFUND_CANCELLATION_INTENT",
                message,
            ),
            StorageError::SettlementSplitPlanNotFound => (
                StatusCode::NOT_FOUND,
                "SETTLEMENT_SPLIT_PLAN_NOT_FOUND",
                "no settlement split plan was found for this merchant".to_string(),
            ),
            StorageError::SettlementSplitPlanUnavailable => (
                StatusCode::CONFLICT,
                "SETTLEMENT_SPLIT_PLAN_UNAVAILABLE",
                "the settlement split plan is not active or is outside its effective window"
                    .to_string(),
            ),
            StorageError::SettlementSplitPlanIntegrity(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SETTLEMENT_SPLIT_PLAN_EVIDENCE_CORRUPT",
                "stored settlement split plan evidence failed integrity validation".to_string(),
            ),
            StorageError::SettlementReceiptBindingNotFound => (
                StatusCode::NOT_FOUND,
                "SETTLEMENT_RECEIPT_BINDING_NOT_FOUND",
                "no finalized receipt evidence was found for this settlement".to_string(),
            ),
            StorageError::InvalidSettlementReceiptBinding(_) => (
                StatusCode::CONFLICT,
                "SETTLEMENT_RECEIPT_BINDING_UNAVAILABLE",
                "finalized receipt evidence is incomplete or conflicting".to_string(),
            ),
            StorageError::InvalidAdjustmentAuthorityChallenge(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_ADJUSTMENT_AUTHORITY_CHALLENGE",
                "adjustment authority challenge evidence is invalid".to_string(),
            ),
            StorageError::AdjustmentAuthorityChallengeNotFound => (
                StatusCode::CONFLICT,
                "ADJUSTMENT_AUTHORITY_CHALLENGE_UNAVAILABLE",
                "the wallet proof challenge is expired, consumed, or unavailable".to_string(),
            ),
            StorageError::AdjustmentAuthorityBindingNotFound => (
                StatusCode::NOT_FOUND,
                "ADJUSTMENT_AUTHORITY_BINDING_NOT_FOUND",
                "no active wallet authority binding was found".to_string(),
            ),
            StorageError::AdjustmentAuthorityConflict => (
                StatusCode::CONFLICT,
                "ADJUSTMENT_AUTHORITY_CONFLICT",
                "the wallet key is already actively bound or the binding is rotating".to_string(),
            ),
            StorageError::AdjustmentEvidenceIntentNotFound => (
                StatusCode::NOT_FOUND,
                "ADJUSTMENT_EVIDENCE_INTENT_NOT_FOUND",
                "the adjustment evidence intent was not found".to_string(),
            ),
            StorageError::AdjustmentEvidenceIntentConflict => (
                StatusCode::CONFLICT,
                "ADJUSTMENT_EVIDENCE_INTENT_CONFLICT",
                "the adjustment evidence intent is bound to different immutable input".to_string(),
            ),
            StorageError::InvalidAdjustmentEvidenceIntent(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INVALID_ADJUSTMENT_EVIDENCE_INTENT",
                "stored adjustment evidence intent failed integrity validation".to_string(),
            ),
            StorageError::AdjustmentChainIntentNotFound => (
                StatusCode::NOT_FOUND,
                "ADJUSTMENT_CHAIN_INTENT_NOT_FOUND",
                "the adjustment submission intent was not found".to_string(),
            ),
            StorageError::AdjustmentChainIntentConflict => (
                StatusCode::CONFLICT,
                "ADJUSTMENT_CHAIN_INTENT_CONFLICT",
                "the adjustment submission intent conflicts with immutable evidence".to_string(),
            ),
            StorageError::InvalidAdjustmentChainTransition => (
                StatusCode::CONFLICT,
                "INVALID_ADJUSTMENT_CHAIN_TRANSITION",
                "the adjustment submission cannot advance through this lifecycle transition"
                    .to_string(),
            ),
            StorageError::Tax(tax_error) => {
                return ApiError::from(tax_error);
            }
            StorageError::RuleNotFound => (
                StatusCode::NOT_FOUND,
                "PUBLISHED_RULE_NOT_FOUND",
                "no effective published rule matches the selector".to_string(),
            ),
            StorageError::RuleDigestMismatch | StorageError::RuleIdentityMismatch => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "PUBLISHED_RULE_EVIDENCE_CORRUPT",
                "published rule evidence failed integrity validation".to_string(),
            ),
            StorageError::JurisdictionPolicyNotFound => (
                StatusCode::SERVICE_UNAVAILABLE,
                "JURISDICTION_POLICY_UNAVAILABLE",
                "no effective jurisdiction policy is configured for the published rule".to_string(),
            ),
            StorageError::JurisdictionPolicyDigestMismatch => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "JURISDICTION_POLICY_EVIDENCE_CORRUPT",
                "jurisdiction policy evidence failed integrity validation".to_string(),
            ),
            StorageError::JurisdictionMismatch => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "JURISDICTION_SELECTOR_MISMATCH",
                "resolved jurisdiction does not match the quote selector".to_string(),
            ),
            StorageError::RuleGovernanceNotFound => (
                StatusCode::NOT_FOUND,
                "RULE_GOVERNANCE_RULE_NOT_FOUND",
                "the tax rule was not found".to_string(),
            ),
            StorageError::RuleSimulationNotFound => (
                StatusCode::NOT_FOUND,
                "RULE_SIMULATION_RULE_NOT_FOUND",
                "the tax rule was not found for this merchant".to_string(),
            ),
            StorageError::RuleSimulationJurisdictionMismatch => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "RULE_SIMULATION_JURISDICTION_MISMATCH",
                "simulation facts jurisdiction must match the persisted rule".to_string(),
            ),
            StorageError::RuleSimulationUnavailable => (
                StatusCode::CONFLICT,
                "RULE_SIMULATION_UNAVAILABLE",
                "retired tax rules cannot be simulated".to_string(),
            ),
            StorageError::RuleSimulationIntegrity(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "RULE_SIMULATION_EVIDENCE_CORRUPT",
                "persisted tax-rule simulation evidence failed integrity validation".to_string(),
            ),
            StorageError::InvalidRuleGovernanceEvidence => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_RULE_GOVERNANCE_EVIDENCE",
                "rule governance evidence must be a JSON object".to_string(),
            ),
            StorageError::IdempotencyConflict => (
                StatusCode::CONFLICT,
                "IDEMPOTENCY_KEY_REUSED",
                "idempotency key was already used for different input".to_string(),
            ),
            StorageError::IdempotencyInProgress => (
                StatusCode::CONFLICT,
                "IDEMPOTENCY_REQUEST_IN_PROGRESS",
                "idempotent request is still being completed".to_string(),
            ),
            StorageError::DocumentPaymentAuthorizationNotFound => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "DOCUMENT_PAYMENT_AUTHORIZATION_MISSING",
                "the durable document payment authorization is missing".to_string(),
            ),
            StorageError::DocumentPaymentAuthorizationConflict => (
                StatusCode::CONFLICT,
                "DOCUMENT_PAYMENT_AUTHORIZATION_CONFLICT",
                "the settlement intent is not a document payment intent".to_string(),
            ),
            StorageError::DocumentPaymentAuthorizationIntegrity(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "DOCUMENT_PAYMENT_AUTHORIZATION_CORRUPT",
                message,
            ),
            StorageError::AmountOutOfRange => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "AMOUNT_OUT_OF_RANGE",
                "financial amount is outside the supported range".to_string(),
            ),
            StorageError::InvalidIndexerStreamKey => (
                StatusCode::BAD_REQUEST,
                "INVALID_INDEXER_STREAM",
                "indexer stream key is invalid".to_string(),
            ),
            StorageError::InvalidIndexerState(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INDEXER_EVIDENCE_CORRUPT",
                "indexed chain evidence failed integrity validation".to_string(),
            ),
            StorageError::InvalidReplayReason(_) | StorageError::InvalidReplayState(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INDEXER_REPLAY_STATE_CORRUPT",
                "indexed replay state failed integrity validation".to_string(),
            ),
            StorageError::ReplayGapNotFound => (
                StatusCode::NOT_FOUND,
                "INDEXER_REPLAY_GAP_NOT_FOUND",
                "the replay gap was not found".to_string(),
            ),
            StorageError::ReplayGapNotDeadLettered
            | StorageError::ReplayGapNotAcknowledged
            | StorageError::ReplayGapAlreadyAcknowledged => (
                StatusCode::CONFLICT,
                "INDEXER_REPLAY_GAP_STATE_CONFLICT",
                error.to_string(),
            ),
            StorageError::InvalidReplayOperatorInput(message) => (
                StatusCode::BAD_REQUEST,
                "INVALID_INDEXER_REPLAY_OPERATOR_INPUT",
                message,
            ),
            StorageError::IndexerCheckpointConflict => (
                StatusCode::CONFLICT,
                "INDEXER_CHECKPOINT_CONFLICT",
                "indexed chain state changed; retry from the current checkpoint".to_string(),
            ),
            StorageError::Indexer(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_INDEXER_PAGE",
                "indexed chain page failed validation".to_string(),
            ),
            StorageError::InvalidSettlementDigest
            | StorageError::InvalidSettlementStatus(_)
            | StorageError::InvalidSettlementTransition
            | StorageError::SettlementEvidenceTooLarge => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_SETTLEMENT_LIFECYCLE",
                error.to_string(),
            ),
            StorageError::InvalidRuleGovernanceTransition
            | StorageError::RuleGovernanceConflict => (
                StatusCode::CONFLICT,
                "INVALID_RULE_GOVERNANCE_TRANSITION",
                "the tax rule cannot advance through this governance transition".to_string(),
            ),
            StorageError::RuleGovernanceEvidenceTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "RULE_GOVERNANCE_EVIDENCE_TOO_LARGE",
                "rule governance evidence exceeds the bounded size".to_string(),
            ),
            StorageError::InvalidRuleDraft(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_RULE_DRAFT",
                message,
            ),
            StorageError::RuleDraftConflict => (
                StatusCode::CONFLICT,
                "RULE_DRAFT_CONFLICT",
                "a rule draft with this tenant, external ID, and version already exists"
                    .to_string(),
            ),
            StorageError::InvalidJurisdictionPolicyTransition => (
                StatusCode::CONFLICT,
                "INVALID_JURISDICTION_POLICY_TRANSITION",
                "the jurisdiction policy cannot advance through this governance transition"
                    .to_string(),
            ),
            StorageError::JurisdictionPolicyGovernanceConflict => (
                StatusCode::CONFLICT,
                "JURISDICTION_POLICY_GOVERNANCE_CONFLICT",
                "the jurisdiction policy actors or effective window do not satisfy governance requirements"
                    .to_string(),
            ),
            StorageError::JurisdictionPolicyGovernanceNotFound => (
                StatusCode::NOT_FOUND,
                "JURISDICTION_POLICY_GOVERNANCE_NOT_FOUND",
                "the jurisdiction policy was not found for this merchant".to_string(),
            ),
            StorageError::InvalidJurisdictionPolicyEvidence => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_JURISDICTION_POLICY_EVIDENCE",
                "jurisdiction policy governance evidence must be a JSON object".to_string(),
            ),
            StorageError::JurisdictionPolicyEvidenceTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "JURISDICTION_POLICY_EVIDENCE_TOO_LARGE",
                "jurisdiction policy governance evidence exceeds the bounded size".to_string(),
            ),
            StorageError::InvalidJurisdictionPolicyDraft(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_JURISDICTION_POLICY_DRAFT",
                message,
            ),
            StorageError::JurisdictionPolicyDraftConflict => (
                StatusCode::CONFLICT,
                "JURISDICTION_POLICY_DRAFT_CONFLICT",
                "a jurisdiction policy already exists for this tenant and rule release".to_string(),
            ),
            StorageError::VaultReleaseClaimNotFound => (
                StatusCode::NOT_FOUND,
                "VAULT_RELEASE_CLAIM_NOT_FOUND",
                "the vault release claim was not found for this merchant".to_string(),
            ),
            StorageError::VaultReconciliationTargetNotFound => (
                StatusCode::NOT_FOUND,
                "VAULT_RECONCILIATION_TARGET_NOT_FOUND",
                "the vault reconciliation target was not found for this merchant".to_string(),
            ),
            StorageError::InvalidVaultReleaseClaimTransition
            | StorageError::VaultReleaseClaimActorConflict => (
                StatusCode::CONFLICT,
                "VAULT_RELEASE_CLAIM_STATE_CONFLICT",
                "the vault release claim cannot advance through this transition".to_string(),
            ),
            StorageError::InvalidVaultReleaseEvidence => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_VAULT_RELEASE_EVIDENCE",
                "vault release evidence must be a JSON object".to_string(),
            ),
            StorageError::VaultReleaseEvidenceTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "VAULT_RELEASE_EVIDENCE_TOO_LARGE",
                "vault release evidence exceeds the bounded size".to_string(),
            ),
            StorageError::InvalidVaultReleaseClaim(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_VAULT_RELEASE_CLAIM",
                message,
            ),
            StorageError::VaultReleaseClaimIdempotencyConflict => (
                StatusCode::CONFLICT,
                "VAULT_RELEASE_CLAIM_IDEMPOTENCY_CONFLICT",
                "the vault release idempotency key was reused for different input".to_string(),
            ),
            StorageError::SettlementAdjustmentNotFound => (
                StatusCode::NOT_FOUND,
                "SETTLEMENT_ADJUSTMENT_NOT_FOUND",
                "the settlement adjustment was not found for this merchant".to_string(),
            ),
            StorageError::InvalidSettlementAdjustment(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_SETTLEMENT_ADJUSTMENT",
                message,
            ),
            StorageError::InvalidSettlementAdjustmentTransition
            | StorageError::SettlementAdjustmentActorConflict => (
                StatusCode::CONFLICT,
                "SETTLEMENT_ADJUSTMENT_STATE_CONFLICT",
                "the settlement adjustment cannot advance through this transition".to_string(),
            ),
            StorageError::SettlementAdjustmentIdempotencyConflict => (
                StatusCode::CONFLICT,
                "SETTLEMENT_ADJUSTMENT_IDEMPOTENCY_CONFLICT",
                "the settlement adjustment idempotency key was reused for different input"
                    .to_string(),
            ),
            StorageError::RefundBalanceProofExecutionNotFound => (
                StatusCode::NOT_FOUND,
                "REFUND_BALANCE_PROOF_EXECUTION_NOT_FOUND",
                "the finalized refund execution evidence was not found".to_string(),
            ),
            StorageError::InvalidRefundBalanceProof(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_REFUND_BALANCE_PROOF",
                message,
            ),
            StorageError::RefundBalanceProofConflict => (
                StatusCode::CONFLICT,
                "REFUND_BALANCE_PROOF_CONFLICT",
                "immutable refund balance proof evidence conflicts with the stored record"
                    .to_string(),
            ),
            StorageError::RefundBalanceProofLeaseLost => (
                StatusCode::CONFLICT,
                "REFUND_BALANCE_PROOF_LEASE_LOST",
                "the refund proof worker no longer owns this evidence candidate".to_string(),
            ),
        };
        Self {
            status,
            code,
            message,
        }
    }
}

fn log_adjustment_compile_storage_error(stage: &'static str, error: StorageError) -> ApiError {
    tracing::error!(stage, error = %error, "settlement adjustment compilation storage operation failed");
    ApiError::from(error)
}

fn log_adjustment_compile_coordinator_error(
    stage: &'static str,
    error: CoordinatorError,
) -> ApiError {
    tracing::error!(stage, error = %error, "settlement adjustment compilation coordinator operation failed");
    ApiError::from(error)
}

impl From<jurisdiction::ProviderRegistryError> for ApiError {
    fn from(error: jurisdiction::ProviderRegistryError) -> Self {
        let (status, code) = match error {
            jurisdiction::ProviderRegistryError::UnknownKey => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "JURISDICTION_PROVIDER_KEY_UNKNOWN",
            ),
            jurisdiction::ProviderRegistryError::Empty
            | jurisdiction::ProviderRegistryError::Invalid(_) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "JURISDICTION_PROVIDER_UNAVAILABLE",
            ),
            jurisdiction::ProviderRegistryError::Evidence(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "JURISDICTION_EVIDENCE_INVALID",
            ),
        };
        Self {
            status,
            code,
            message: error.to_string(),
        }
    }
}

impl From<CoordinatorError> for ApiError {
    fn from(error: CoordinatorError) -> Self {
        let status = match &error {
            CoordinatorError::InvalidSignedTransaction(_) | CoordinatorError::InvalidSignature => {
                StatusCode::BAD_REQUEST
            }
            CoordinatorError::AddressLookupTable(_) => StatusCode::SERVICE_UNAVAILABLE,
            CoordinatorError::InvalidProviderResponse(_)
            | CoordinatorError::InvalidAdjustmentRegistry(_)
            | CoordinatorError::InvalidConfiguration(_)
            | CoordinatorError::ProviderAttemptsExhausted(_) => StatusCode::SERVICE_UNAVAILABLE,
        };
        Self {
            status,
            code: "SETTLEMENT_COORDINATOR_ERROR",
            message: error.to_string(),
        }
    }
}

impl From<SettlementCompileError> for ApiError {
    fn from(error: SettlementCompileError) -> Self {
        let (status, code) = match &error {
            SettlementCompileError::InvalidPublicKey { .. }
            | SettlementCompileError::InvalidAmount { .. } => {
                (StatusCode::BAD_REQUEST, "INVALID_SETTLEMENT_INPUT")
            }
            SettlementCompileError::UnsupportedProgramId => {
                (StatusCode::UNPROCESSABLE_ENTITY, "UNSUPPORTED_PROGRAM_ID")
            }
            SettlementCompileError::InvalidGrossAmount => {
                (StatusCode::UNPROCESSABLE_ENTITY, "INVALID_GROSS_AMOUNT")
            }
            SettlementCompileError::AmountConservationViolation => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "SETTLEMENT_AMOUNT_CONSERVATION_VIOLATION",
            ),
            SettlementCompileError::EmptyPolicyDigest => {
                (StatusCode::UNPROCESSABLE_ENTITY, "EMPTY_POLICY_DIGEST")
            }
            SettlementCompileError::EmptyDocumentAttestationDigest
            | SettlementCompileError::EmptyPaymentScopeDigest => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "EMPTY_DOCUMENT_PROVENANCE_DIGEST",
            ),
            SettlementCompileError::InvalidNonce => {
                (StatusCode::UNPROCESSABLE_ENTITY, "INVALID_SETTLEMENT_NONCE")
            }
            SettlementCompileError::InvalidPolicySequence => {
                (StatusCode::UNPROCESSABLE_ENTITY, "INVALID_POLICY_SEQUENCE")
            }
            SettlementCompileError::UnsupportedTokenProgram => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "UNSUPPORTED_TOKEN_PROGRAM",
            ),
            SettlementCompileError::AliasedTokenAccounts => {
                (StatusCode::UNPROCESSABLE_ENTITY, "ALIASED_TOKEN_ACCOUNTS")
            }
            SettlementCompileError::DuplicateAccountMeta => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "DUPLICATE_SETTLEMENT_ACCOUNT_META",
            ),
            SettlementCompileError::InvalidSplitPlan { .. } => {
                (StatusCode::UNPROCESSABLE_ENTITY, "INVALID_SPLIT_PLAN")
            }
            SettlementCompileError::InvalidWireBudget => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "SETTLEMENT_WIRE_BUDGET_EXCEEDED",
            ),
            SettlementCompileError::InvalidComputeBudgetPolicy => (
                StatusCode::SERVICE_UNAVAILABLE,
                "COMPUTE_BUDGET_POLICY_UNAVAILABLE",
            ),
            SettlementCompileError::InvalidAdjustmentMagnitude { .. } => (
                StatusCode::BAD_REQUEST,
                "INVALID_SETTLEMENT_ADJUSTMENT_AMOUNT",
            ),
            SettlementCompileError::InvalidAdjustmentNonce
            | SettlementCompileError::InvalidAdjustmentKind
            | SettlementCompileError::AdjustmentAuthorityConflict
            | SettlementCompileError::InvalidAdjustmentSign
            | SettlementCompileError::EmptyAdjustmentDigest
            | SettlementCompileError::EmptyAdjustmentEvidenceDigest
            | SettlementCompileError::InvalidAdjustmentReceipt => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_SETTLEMENT_ADJUSTMENT_INPUT",
            ),
            SettlementCompileError::AdjustmentAmountConservationViolation => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "SETTLEMENT_ADJUSTMENT_CONSERVATION_VIOLATION",
            ),
            SettlementCompileError::InvalidRefundClaimNonce
            | SettlementCompileError::InvalidRefundClaimWindow
            | SettlementCompileError::EmptyRefundReferenceDigest
            | SettlementCompileError::RefundClaimAuthorityConflict
            | SettlementCompileError::AliasedRefundTokenAccounts => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_SETTLEMENT_REFUND_CLAIM",
            ),
            SettlementCompileError::InvalidRefundCancellationNonce
            | SettlementCompileError::InvalidRefundCancellationKey { .. }
            | SettlementCompileError::AliasedRefundCancellationAccounts => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_SETTLEMENT_REFUND_CANCELLATION",
            ),
        };
        Self {
            status,
            code,
            message: error.to_string(),
        }
    }
}

impl From<auth::AuthError> for ApiError {
    fn from(error: auth::AuthError) -> Self {
        let status = match error {
            auth::AuthError::NotConfigured | auth::AuthError::ClockUnavailable => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            auth::AuthError::InsufficientScope | auth::AuthError::InsufficientRole => {
                StatusCode::FORBIDDEN
            }
            _ => StatusCode::UNAUTHORIZED,
        };
        Self {
            status,
            code: error.code(),
            message: error.public_message().to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorBody {
                error: ErrorDetail {
                    code: self.code,
                    message: self.message,
                },
            }),
        )
            .into_response()
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics))
        .route("/v1/tax/calculate", post(calculate_tax))
        .route("/v1/tax/calculate-document", post(calculate_document_tax))
        .route("/v1/tax/quote", post(quote_tax))
        .route("/v1/tax/document-quote", post(document_quote_tax))
        .route("/v1/tax/rules", post(create_rule_draft))
        .route(
            "/v1/tax/rules/{rule_set_id}/transition",
            post(transition_rule_set),
        )
        .route(
            "/v1/tax/rules/{rule_set_id}/governance",
            get(load_rule_governance),
        )
        .route(
            "/v1/tax/rules/{rule_set_id}/simulate",
            post(simulate_rule_set),
        )
        .route(
            "/v1/tax/jurisdiction-policies",
            post(create_jurisdiction_policy),
        )
        .route(
            "/v1/tax/jurisdiction-policies/{policy_id}/transition",
            post(transition_jurisdiction_policy),
        )
        .route(
            "/v1/tax/jurisdiction-policies/{policy_id}/governance",
            get(load_jurisdiction_policy_governance),
        )
        .route("/v1/vault/release-claims", post(create_vault_release_claim))
        .route(
            "/v1/vault/release-claims/{claim_id}",
            get(load_vault_release_claim),
        )
        .route(
            "/v1/vault/release-claims/{claim_id}/transition",
            post(transition_vault_release_claim),
        )
        .route(
            "/v1/settlement/adjustments",
            post(create_settlement_adjustment),
        )
        .route(
            "/v1/settlement/adjustments/{adjustment_id}",
            get(load_settlement_adjustment),
        )
        .route(
            "/v1/settlement/adjustments/{adjustment_id}/transition",
            post(transition_settlement_adjustment),
        )
        .route(
            "/v1/settlement/adjustment-authority/challenges",
            post(create_adjustment_authority_challenge),
        )
        .route(
            "/v1/settlement/adjustment-authority/bindings",
            post(bind_adjustment_authority),
        )
        .route(
            "/v1/settlement/adjustments/{adjustment_id}/compile",
            post(compile_settlement_adjustment),
        )
        .route(
            "/v1/settlement/adjustment-chain-intents/submit",
            post(submit_settlement_adjustment),
        )
        .route(
            "/v1/settlement/adjustment-chain-intents/{intent_id}",
            get(get_settlement_adjustment_chain_intent),
        )
        .route(
            "/v1/settlement/adjustment-chain-intents/{intent_id}/reconcile",
            post(reconcile_settlement_adjustment_chain_intent),
        )
        .route(
            "/v1/settlement/refund-funding-configurations",
            post(create_refund_funding_configuration).get(list_refund_funding_configurations),
        )
        .route(
            "/v1/settlement/refund-funding-configurations/{configuration_id}",
            get(load_refund_funding_configuration),
        )
        .route(
            "/v1/settlement/refund-funding-configurations/{configuration_id}/transition",
            post(transition_refund_funding_configuration),
        )
        .route("/v1/settlement/refund-claims", post(prepare_refund_claim))
        .route(
            "/v1/settlement/refund-claims/submit",
            post(submit_refund_claim),
        )
        .route(
            "/v1/settlement/refund-claims/{intent_id}",
            get(load_refund_claim),
        )
        .route(
            "/v1/settlement/refund-claims/{intent_id}/reconcile",
            post(reconcile_refund_claim),
        )
        .route(
            "/v1/settlement/refund-claim-cancellations",
            post(prepare_refund_cancellation),
        )
        .route(
            "/v1/settlement/refund-claim-cancellations/submit",
            post(submit_refund_cancellation),
        )
        .route(
            "/v1/settlement/refund-claim-cancellations/{intent_id}",
            get(load_refund_cancellation),
        )
        .route(
            "/v1/settlement/refund-claim-cancellations/{intent_id}/reconcile",
            post(reconcile_refund_cancellation),
        )
        .route(
            "/v1/settlement/refund-executions",
            post(prepare_refund_execute),
        )
        .route(
            "/v1/settlement/refund-executions/submit",
            post(submit_refund_execute),
        )
        .route(
            "/v1/settlement/refund-executions/{intent_id}",
            get(load_refund_execute),
        )
        .route(
            "/v1/vault/reconciliation/targets",
            get(list_vault_reconciliation_targets),
        )
        .route(
            "/v1/vault/reconciliation/targets/{target_id}/runs",
            get(list_vault_reconciliation_runs),
        )
        .route("/v1/tax/quotes/{calculation_id}/replay", get(replay_quote))
        .route(
            "/v1/tax/document-quotes/{calculation_id}/replay",
            get(replay_document_quote),
        )
        .route(
            "/v1/tax/quotes/{calculation_id}/attestation/verify",
            post(verify_quote_attestation),
        )
        .route(
            "/v1/tax/document-quotes/{calculation_id}/attestation/verify",
            post(verify_document_quote_attestation),
        )
        .route("/v1/indexer/replay-gaps", get(list_replay_gaps))
        .route(
            "/v1/indexer/replay-gaps/{gap_id}/acknowledge",
            post(acknowledge_replay_gap),
        )
        .route(
            "/v1/indexer/replay-gaps/{gap_id}/requeue",
            post(requeue_replay_gap),
        )
        .route("/v1/indexer/targeted-recovery", post(targeted_recovery))
        .route("/v1/tax/attestation/verify", post(verify_attestation))
        .route("/v1/settlement/compile", post(compile_settlement))
        .route(
            "/v1/settlement/compile-from-quote",
            post(compile_settlement_from_quote),
        )
        .route(
            "/v1/settlement/document/compile-from-quote",
            post(compile_document_settlement_from_quote),
        )
        .route(
            "/v1/settlement/split/compile-from-quote",
            post(compile_split_settlement_from_quote),
        )
        .route("/v1/settlement/routes", get(list_settlement_routes))
        .route(
            "/v1/settlement/split-plans",
            get(list_settlement_split_plans),
        )
        .route("/v1/settlement/submit", post(submit_settlement))
        .route(
            "/v1/settlement/intents/{intent_id}",
            get(get_settlement_intent),
        )
        .route(
            "/v1/settlement/intents/{intent_id}/reconcile",
            post(reconcile_settlement_intent),
        )
        .layer(RequestBodyLimitLayer::new(64 * 1024))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                let request_id = request
                    .headers()
                    .get(&REQUEST_ID_HEADER)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("missing");
                tracing::info_span!(
                    "http.request",
                    request_id,
                    method = %request.method(),
                    uri = %request.uri()
                )
            }),
        )
        .layer(middleware::from_fn(assign_request_id))
}

async fn healthz() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn readyz(State(state): State<AppState>) -> StatusCode {
    if state.ready {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

async fn metrics(State(state): State<AppState>) -> Response {
    let mut metrics_body = state.metrics.render_prometheus();
    if let Some(store) = state.storage.as_ref() {
        metrics_body.push_str(&render_rule_cache_metrics(store.rule_cache_stats()));
        metrics_body.push_str(&render_database_pool_metrics(store.database_pool_stats()));
    }
    if let Some(coordinator) = state.settlement_coordinator.as_ref() {
        metrics_body.push_str(&render_settlement_provider_metrics(
            &coordinator.health_snapshot(),
        ));
    }
    (
        [(CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        metrics_body,
    )
        .into_response()
}

fn render_database_pool_metrics(stats: DatabasePoolStats) -> String {
    format!(
        "# HELP stp_database_pool_max_size Configured PostgreSQL pool maximum size.\n\\
# TYPE stp_database_pool_max_size gauge\n\\
stp_database_pool_max_size{{service=\"api\"}} {}\n\\
# HELP stp_database_pool_size Current PostgreSQL pool size.\n\\
# TYPE stp_database_pool_size gauge\n\\
stp_database_pool_size{{service=\"api\"}} {}\n\\
# HELP stp_database_pool_available Available PostgreSQL connections.\n\\
# TYPE stp_database_pool_available gauge\n\\
stp_database_pool_available{{service=\"api\"}} {}\n\\
# HELP stp_database_pool_waiting Futures waiting for a PostgreSQL connection.\n\\
# TYPE stp_database_pool_waiting gauge\n\\
stp_database_pool_waiting{{service=\"api\"}} {}\n",
        stats.max_size, stats.size, stats.available, stats.waiting
    )
}

fn render_rule_cache_metrics(stats: RuleCacheStats) -> String {
    format!(
        "# HELP stp_tax_rule_cache_hits_total Compiled tax-rule cache hits.\n\
# TYPE stp_tax_rule_cache_hits_total counter\n\
stp_tax_rule_cache_hits_total{{service=\"api\"}} {}\n\
# HELP stp_tax_rule_cache_misses_total Compiled tax-rule cache misses.\n\
# TYPE stp_tax_rule_cache_misses_total counter\n\
stp_tax_rule_cache_misses_total{{service=\"api\"}} {}\n\
# HELP stp_tax_rule_cache_evictions_total Compiled tax-rule cache evictions.\n\
# TYPE stp_tax_rule_cache_evictions_total counter\n\
stp_tax_rule_cache_evictions_total{{service=\"api\"}} {}\n\
# HELP stp_tax_rule_cache_entries Current compiled tax-rule cache entries.\n\
# TYPE stp_tax_rule_cache_entries gauge\n\
stp_tax_rule_cache_entries{{service=\"api\"}} {}\n\
# HELP stp_tax_rule_cache_capacity Configured compiled tax-rule cache capacity.\n\
# TYPE stp_tax_rule_cache_capacity gauge\n\
stp_tax_rule_cache_capacity{{service=\"api\"}} {}\n",
        stats.hits, stats.misses, stats.evictions, stats.entries, stats.capacity
    )
}

fn render_settlement_provider_metrics(snapshots: &[SettlementProviderHealthSnapshot]) -> String {
    let mut output = String::new();
    output.push_str("# TYPE stp_settlement_provider_health_score gauge\n");
    output.push_str("# TYPE stp_settlement_provider_consecutive_failures gauge\n");
    output.push_str("# TYPE stp_settlement_provider_cooldown_ms gauge\n");
    for snapshot in snapshots.iter().take(3) {
        let labels = format!(
            "service=\"api\",provider_index=\"{}\"",
            snapshot.provider_index
        );
        output.push_str(&format!(
            "stp_settlement_provider_health_score{{{labels}}} {}\n",
            snapshot.score
        ));
        output.push_str(&format!(
            "stp_settlement_provider_consecutive_failures{{{labels}}} {}\n",
            snapshot.consecutive_failures
        ));
        output.push_str(&format!(
            "stp_settlement_provider_cooldown_ms{{{labels}}} {}\n",
            snapshot.cooldown_remaining.as_millis()
        ));
    }
    output
}

async fn calculate_tax(
    State(state): State<AppState>,
    Json(request): Json<CalculateRequest>,
) -> Result<Json<CalculateResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = calculate_tax_inner(state, request).await;
    metrics.observe_or_log("tax_calculate", started.elapsed(), result.is_err());
    result
}

async fn calculate_tax_inner(
    state: AppState,
    request: CalculateRequest,
) -> Result<Json<CalculateResponse>, ApiError> {
    if !state.simulation_mode {
        return Err(ApiError {
            status: StatusCode::NOT_IMPLEMENTED,
            code: "PUBLISHED_RULE_REGISTRY_REQUIRED",
            message:
                "simulation endpoint is disabled until a published rule registry is configured"
                    .into(),
        });
    }
    let result = calculate(&request.rule_set, &request.facts).map_err(ApiError::from)?;
    Ok(Json(CalculateResponse {
        calculation_id: Uuid::now_v7(),
        status: "simulated_unattested",
        result,
    }))
}

async fn calculate_document_tax(
    State(state): State<AppState>,
    Json(request): Json<CalculateDocumentRequest>,
) -> Result<Json<CalculateDocumentResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = calculate_document_tax_inner(state, request).await;
    metrics.observe_or_log("tax_document_calculate", started.elapsed(), result.is_err());
    result
}

async fn calculate_document_tax_inner(
    state: AppState,
    request: CalculateDocumentRequest,
) -> Result<Json<CalculateDocumentResponse>, ApiError> {
    if !state.simulation_mode {
        return Err(ApiError {
            status: StatusCode::NOT_IMPLEMENTED,
            code: "PUBLISHED_RULE_REGISTRY_REQUIRED",
            message:
                "simulation endpoint is disabled until a published rule registry is configured"
                    .into(),
        });
    }
    if request.schema != TAX_DOCUMENT_SCHEMA_V1 {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "UNSUPPORTED_DOCUMENT_SCHEMA",
            message: "document schema is not supported".into(),
        });
    }
    let result = calculate_document(&request.rule_set, &request.facts).map_err(ApiError::from)?;
    Ok(Json(CalculateDocumentResponse {
        schema: TAX_DOCUMENT_SCHEMA_V1,
        calculation_id: Uuid::now_v7(),
        status: "simulated_unattested",
        result,
    }))
}

async fn quote_tax(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<QuoteRequest>, JsonRejection>,
) -> Result<Json<storage::StoredQuote>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = quote_tax_inner(state, headers, payload).await;
    metrics.observe_or_log("tax_quote", started.elapsed(), result.is_err());
    result
}

async fn quote_tax_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<QuoteRequest>, JsonRejection>,
) -> Result<Json<storage::StoredQuote>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:quote")?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let Some(store) = state.storage else {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "QUOTE_STORAGE_UNAVAILABLE",
            message: "published-rule quote storage is not configured".to_string(),
        });
    };
    if request.facts.jurisdiction_resolution.is_some() {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "CLIENT_JURISDICTION_RESOLUTION_FORBIDDEN",
            message: "jurisdiction resolution evidence is server-owned".to_string(),
        });
    }
    let provider_registry = state
        .jurisdiction_provider_registry
        .as_ref()
        .ok_or(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "JURISDICTION_PROVIDER_UNAVAILABLE",
            message: "jurisdiction provider verification is not configured".to_string(),
        })?;
    let facts_digest = request.facts.canonical_digest().map_err(ApiError::from)?;
    let candidates = provider_registry.verify(
        &request.jurisdiction_evidence,
        principal.merchant_id,
        facts_digest,
        server_unix_now()?,
    )?;
    let outcome = store
        .quote(QuoteCommand {
            merchant_id: principal.merchant_id,
            idempotency_key: request.idempotency_key,
            rule_set_id: request.rule_set_id,
            rule_version: request.rule_version,
            jurisdiction: request.jurisdiction,
            regime: request.regime,
            facts: request.facts,
            jurisdiction_candidates: candidates,
        })
        .await
        .map_err(ApiError::from)?;
    let response = match outcome {
        QuoteOutcome::Created(response) | QuoteOutcome::Replay(response) => response,
    };
    Ok(Json(response))
}

async fn document_quote_tax(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<DocumentQuoteRequest>, JsonRejection>,
) -> Result<Json<storage::StoredDocumentQuote>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = document_quote_tax_inner(state, headers, payload).await;
    metrics.observe_or_log("tax_document_quote", started.elapsed(), result.is_err());
    result
}

async fn document_quote_tax_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<DocumentQuoteRequest>, JsonRejection>,
) -> Result<Json<storage::StoredDocumentQuote>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:quote")?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    if request.schema != TAX_DOCUMENT_SCHEMA_V1 {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "UNSUPPORTED_DOCUMENT_SCHEMA",
            message: "document schema is not supported".into(),
        });
    }
    let Some(store) = state.storage else {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "QUOTE_STORAGE_UNAVAILABLE",
            message: "published-rule document quote storage is not configured".into(),
        });
    };
    if request.facts.jurisdiction_resolution.is_some() {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "CLIENT_JURISDICTION_RESOLUTION_FORBIDDEN",
            message: "jurisdiction resolution evidence is server-owned".to_string(),
        });
    }
    let provider_registry = state
        .jurisdiction_provider_registry
        .as_ref()
        .ok_or(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "JURISDICTION_PROVIDER_UNAVAILABLE",
            message: "jurisdiction provider verification is not configured".to_string(),
        })?;
    let facts_digest = request.facts.canonical_digest().map_err(ApiError::from)?;
    let candidates = provider_registry.verify(
        &request.jurisdiction_evidence,
        principal.merchant_id,
        facts_digest,
        server_unix_now()?,
    )?;
    let outcome = store
        .document_quote(DocumentQuoteCommand {
            merchant_id: principal.merchant_id,
            idempotency_key: request.idempotency_key,
            schema: request.schema,
            rule_set_id: request.rule_set_id,
            rule_version: request.rule_version,
            jurisdiction: request.jurisdiction,
            regime: request.regime,
            facts: request.facts,
            jurisdiction_candidates: candidates,
        })
        .await
        .map_err(ApiError::from)?;
    let response = match outcome {
        DocumentQuoteOutcome::Created(response) | DocumentQuoteOutcome::Replay(response) => {
            response
        }
    };
    Ok(Json(response))
}

async fn create_rule_draft(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<RuleDraftCreateRequest>, JsonRejection>,
) -> Result<Json<storage::CreatedRuleDraft>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = create_rule_draft_inner(state, headers, payload).await;
    metrics.observe_or_log("tax_rule_draft_create", started.elapsed(), result.is_err());
    result
}

async fn create_rule_draft_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<RuleDraftCreateRequest>, JsonRejection>,
) -> Result<Json<storage::CreatedRuleDraft>, ApiError> {
    let principal =
        authenticate_scope_role(&state, &headers, "tax:rule:author", "tax_rule_author")?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let Some(store) = state.storage else {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "RULE_GOVERNANCE_STORAGE_UNAVAILABLE",
            message: "tax-rule governance storage is not configured".to_string(),
        });
    };
    let outcome = store
        .create_rule_draft(storage::CreateRuleDraftCommand {
            merchant_id: principal.merchant_id,
            actor_subject: principal.subject,
            idempotency_key: request.idempotency_key,
            external_id: request.external_id,
            version: request.version,
            jurisdiction: request.jurisdiction,
            regime: request.regime,
            effective_from_unix: request.effective_from_unix,
            effective_until_unix: request.effective_until_unix,
            rule_set: request.rule_set,
            provenance: request.provenance,
        })
        .await
        .map_err(|error| {
            tracing::error!(error = %error, "rule draft persistence failed");
            ApiError::from(error)
        })?;
    let response = match outcome {
        storage::RuleDraftOutcome::Created(response)
        | storage::RuleDraftOutcome::Replay(response) => response,
    };
    Ok(Json(response))
}

async fn transition_rule_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(rule_set_id): Path<Uuid>,
    payload: Result<Json<RuleTransitionRequest>, JsonRejection>,
) -> Result<Json<RuleTransitionResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = transition_rule_set_inner(state, headers, rule_set_id, payload).await;
    metrics.observe_or_log("tax_rule_transition", started.elapsed(), result.is_err());
    result
}

async fn transition_rule_set_inner(
    state: AppState,
    headers: HeaderMap,
    rule_set_id: Uuid,
    payload: Result<Json<RuleTransitionRequest>, JsonRejection>,
) -> Result<Json<RuleTransitionResponse>, ApiError> {
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let (required_scope, required_role) = match request.to_status.as_str() {
        "simulated" => ("tax:rule:author", "tax_rule_author"),
        "reviewed" => ("tax:rule:review", "tax_rule_reviewer"),
        "approved" => ("tax:rule:approve", "tax_rule_approver"),
        "published" | "retired" => ("tax:rule:publish", "tax_rule_approver"),
        _ => {
            return Err(ApiError {
                status: StatusCode::BAD_REQUEST,
                code: "INVALID_RULE_GOVERNANCE_STATUS",
                message: "unsupported tax-rule governance target status".to_string(),
            });
        }
    };
    let principal = authenticate_scope_role(&state, &headers, required_scope, required_role)?;
    let Some(store) = state.storage else {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "RULE_GOVERNANCE_STORAGE_UNAVAILABLE",
            message: "tax-rule governance storage is not configured".to_string(),
        });
    };
    let result = store
        .transition_rule_set(RuleGovernanceTransition {
            merchant_id: principal.merchant_id,
            rule_set_id,
            actor_subject: principal.subject,
            to_status: request.to_status,
            evidence: request.evidence,
        })
        .await
        .map_err(ApiError::from)?;
    Ok(Json(RuleTransitionResponse {
        status: "transitioned",
        rule_set_id: result.rule_set_id,
        from_status: result.from_status,
        to_status: result.to_status,
        author_subject: result.author_subject,
        reviewer_subject: result.reviewer_subject,
        approver_subject: result.approver_subject,
        governance_revision: result.governance_revision,
    }))
}

async fn load_rule_governance(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(rule_set_id): Path<Uuid>,
) -> Result<Json<storage::RuleGovernanceState>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = load_rule_governance_inner(state, headers, rule_set_id).await;
    metrics.observe_or_log(
        "tax_rule_governance_read",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn load_rule_governance_inner(
    state: AppState,
    headers: HeaderMap,
    rule_set_id: Uuid,
) -> Result<Json<storage::RuleGovernanceState>, ApiError> {
    let principal = authenticate_scope_any_role(
        &state,
        &headers,
        "tax:rule:read",
        &[
            "tax_rule_author",
            "tax_rule_reviewer",
            "tax_rule_approver",
            "tax_rule_auditor",
        ],
    )?;
    let Some(store) = state.storage else {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "RULE_GOVERNANCE_STORAGE_UNAVAILABLE",
            message: "tax-rule governance storage is not configured".to_string(),
        });
    };
    Ok(Json(
        store
            .load_rule_governance(rule_set_id, principal.merchant_id)
            .await
            .map_err(ApiError::from)?,
    ))
}

async fn simulate_rule_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(rule_set_id): Path<Uuid>,
    payload: Result<Json<RuleSimulationRequest>, JsonRejection>,
) -> Result<Json<CalculateResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = simulate_rule_set_inner(state, headers, rule_set_id, payload).await;
    metrics.observe_or_log("tax_rule_simulation", started.elapsed(), result.is_err());
    result
}

async fn simulate_rule_set_inner(
    state: AppState,
    headers: HeaderMap,
    rule_set_id: Uuid,
    payload: Result<Json<RuleSimulationRequest>, JsonRejection>,
) -> Result<Json<CalculateResponse>, ApiError> {
    let principal = authenticate_scope_any_role(
        &state,
        &headers,
        "tax:rule:read",
        &[
            "tax_rule_author",
            "tax_rule_reviewer",
            "tax_rule_approver",
            "tax_rule_auditor",
        ],
    )?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let facts = parse_strict_simulation_facts(request.facts)?;
    if facts.jurisdiction_resolution.is_some() {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "CLIENT_JURISDICTION_RESOLUTION_FORBIDDEN",
            message: "jurisdiction resolution evidence is server-owned".to_string(),
        });
    }
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "RULE_GOVERNANCE_STORAGE_UNAVAILABLE",
        message: "tax-rule governance storage is not configured".to_string(),
    })?;
    let result = store
        .simulate_persisted_rule(rule_set_id, principal.merchant_id, &facts)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(CalculateResponse {
        calculation_id: Uuid::now_v7(),
        status: "simulated_unattested",
        result,
    }))
}

fn parse_strict_simulation_facts(value: serde_json::Value) -> Result<TaxFacts, ApiError> {
    let object = value.as_object().ok_or_else(|| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_SIMULATION_FACTS",
        message: "simulation facts must be a JSON object".to_string(),
    })?;
    const ALLOWED_FIELDS: &[&str] = &[
        "gross_atomic",
        "net_atomic",
        "currency",
        "jurisdiction",
        "jurisdiction_resolution",
        "exemption_codes",
        "cumulative_period",
    ];
    if object
        .keys()
        .any(|key| !ALLOWED_FIELDS.contains(&key.as_str()))
    {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "INVALID_SIMULATION_FACTS",
            message: "simulation facts contain an unsupported field".to_string(),
        });
    }
    serde_json::from_value(value).map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_SIMULATION_FACTS",
        message: "simulation facts are malformed".to_string(),
    })
}

async fn create_jurisdiction_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<JurisdictionPolicyCreateRequest>, JsonRejection>,
) -> Result<Json<storage::CreatedJurisdictionPolicy>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = create_jurisdiction_policy_inner(state, headers, payload).await;
    metrics.observe_or_log(
        "jurisdiction_policy_draft_create",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn create_jurisdiction_policy_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<JurisdictionPolicyCreateRequest>, JsonRejection>,
) -> Result<Json<storage::CreatedJurisdictionPolicy>, ApiError> {
    let principal = authenticate_scope_role(
        &state,
        &headers,
        "tax:jurisdiction:author",
        "tax_jurisdiction_author",
    )?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let Some(store) = state.storage else {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "JURISDICTION_POLICY_GOVERNANCE_STORAGE_UNAVAILABLE",
            message: "jurisdiction policy governance storage is not configured".to_string(),
        });
    };
    let outcome = store
        .create_jurisdiction_policy(CreateJurisdictionPolicyCommand {
            merchant_id: principal.merchant_id,
            actor_subject: principal.subject,
            idempotency_key: request.idempotency_key,
            rule_set_id: request.rule_set_id,
            policy: request.policy,
            effective_from_unix: request.effective_from_unix,
            effective_until_unix: request.effective_until_unix,
            provenance: request.provenance,
        })
        .await
        .map_err(ApiError::from)?;
    let response = match outcome {
        storage::JurisdictionPolicyDraftOutcome::Created(response)
        | storage::JurisdictionPolicyDraftOutcome::Replay(response) => response,
    };
    Ok(Json(response))
}

async fn transition_jurisdiction_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(policy_id): Path<Uuid>,
    payload: Result<Json<JurisdictionPolicyTransitionRequest>, JsonRejection>,
) -> Result<Json<JurisdictionPolicyTransitionResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = transition_jurisdiction_policy_inner(state, headers, policy_id, payload).await;
    metrics.observe_or_log(
        "jurisdiction_policy_transition",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn transition_jurisdiction_policy_inner(
    state: AppState,
    headers: HeaderMap,
    policy_id: Uuid,
    payload: Result<Json<JurisdictionPolicyTransitionRequest>, JsonRejection>,
) -> Result<Json<JurisdictionPolicyTransitionResponse>, ApiError> {
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let (required_scope, required_role) = match request.to_status.as_str() {
        "reviewed" => ("tax:jurisdiction:review", "tax_jurisdiction_reviewer"),
        "approved" => ("tax:jurisdiction:approve", "tax_jurisdiction_approver"),
        "published" | "retired" => ("tax:jurisdiction:publish", "tax_jurisdiction_approver"),
        "blocked" => ("tax:jurisdiction:approve", "tax_jurisdiction_approver"),
        _ => {
            return Err(ApiError {
                status: StatusCode::BAD_REQUEST,
                code: "INVALID_JURISDICTION_POLICY_STATUS",
                message: "unsupported jurisdiction policy governance target status".to_string(),
            });
        }
    };
    let principal = authenticate_scope_role(&state, &headers, required_scope, required_role)?;
    let Some(store) = state.storage else {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "JURISDICTION_POLICY_GOVERNANCE_STORAGE_UNAVAILABLE",
            message: "jurisdiction policy governance storage is not configured".to_string(),
        });
    };
    let result = store
        .transition_jurisdiction_policy(JurisdictionPolicyGovernanceTransition {
            merchant_id: principal.merchant_id,
            policy_id,
            actor_subject: principal.subject,
            to_status: request.to_status,
            evidence: request.evidence,
        })
        .await
        .map_err(ApiError::from)?;
    Ok(Json(JurisdictionPolicyTransitionResponse {
        status: "transitioned",
        policy_id: result.policy_id,
        from_status: result.from_status,
        to_status: result.to_status,
        author_subject: result.author_subject,
        reviewer_subject: result.reviewer_subject,
        approver_subject: result.approver_subject,
        governance_revision: result.governance_revision,
    }))
}

async fn load_jurisdiction_policy_governance(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(policy_id): Path<Uuid>,
) -> Result<Json<storage::JurisdictionPolicyGovernanceState>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = load_jurisdiction_policy_governance_inner(state, headers, policy_id).await;
    metrics.observe_or_log(
        "jurisdiction_policy_governance_read",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn load_jurisdiction_policy_governance_inner(
    state: AppState,
    headers: HeaderMap,
    policy_id: Uuid,
) -> Result<Json<storage::JurisdictionPolicyGovernanceState>, ApiError> {
    let principal = authenticate_scope_any_role(
        &state,
        &headers,
        "tax:jurisdiction:read",
        &[
            "tax_jurisdiction_author",
            "tax_jurisdiction_reviewer",
            "tax_jurisdiction_approver",
            "tax_jurisdiction_auditor",
        ],
    )?;
    let Some(store) = state.storage else {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "JURISDICTION_POLICY_GOVERNANCE_STORAGE_UNAVAILABLE",
            message: "jurisdiction policy governance storage is not configured".to_string(),
        });
    };
    Ok(Json(
        store
            .load_jurisdiction_policy_governance(policy_id, principal.merchant_id)
            .await
            .map_err(ApiError::from)?,
    ))
}

async fn create_settlement_adjustment(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<SettlementAdjustmentCreateRequest>, JsonRejection>,
) -> Result<Json<storage::SettlementAdjustmentRecord>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = create_settlement_adjustment_inner(state, headers, payload).await;
    metrics.observe_or_log(
        "settlement_adjustment_create",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn create_settlement_adjustment_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<SettlementAdjustmentCreateRequest>, JsonRejection>,
) -> Result<Json<storage::SettlementAdjustmentRecord>, ApiError> {
    let principal = authenticate_scope_role(
        &state,
        &headers,
        "tax:adjustment:propose",
        "settlement_adjustment_proposer",
    )?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_ADJUSTMENT_STORAGE_UNAVAILABLE",
        message: "settlement adjustment storage is not configured".to_string(),
    })?;
    let outcome = store
        .create_settlement_adjustment(CreateSettlementAdjustmentCommand {
            merchant_id: principal.merchant_id,
            actor_subject: principal.subject.clone(),
            idempotency_key: request.idempotency_key,
            proposal: settlement_adjustments::AdjustmentProposal {
                id: Uuid::now_v7(),
                merchant_id: principal.merchant_id,
                original_settlement_id: request.original_settlement_id,
                asset_id: request.asset_id,
                kind: request.kind,
                delta: request.delta,
                reason_code: request.reason_code,
                evidence_digest: request.evidence_digest,
                source_calculation_id: request.source_calculation_id,
                replacement_calculation_id: request.replacement_calculation_id,
                proposed_by: principal.subject,
            },
        })
        .await
        .map_err(|error| {
            tracing::error!(error = ?error, "settlement adjustment proposal persistence failed");
            ApiError::from(error)
        })?;
    let record = match outcome {
        storage::SettlementAdjustmentOutcome::Created(record)
        | storage::SettlementAdjustmentOutcome::Replay(record) => record,
    };
    Ok(Json(record))
}

async fn load_settlement_adjustment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(adjustment_id): Path<Uuid>,
) -> Result<Json<storage::SettlementAdjustmentRecord>, ApiError> {
    let principal = authenticate_scope_any_role(
        &state,
        &headers,
        "tax:adjustment:read",
        &[
            "settlement_adjustment_proposer",
            "settlement_adjustment_reviewer",
            "settlement_adjustment_approver",
            "settlement_adjustment_auditor",
        ],
    )?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_ADJUSTMENT_STORAGE_UNAVAILABLE",
        message: "settlement adjustment storage is not configured".to_string(),
    })?;
    Ok(Json(
        store
            .load_settlement_adjustment(adjustment_id, principal.merchant_id)
            .await
            .map_err(ApiError::from)?,
    ))
}

async fn transition_settlement_adjustment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(adjustment_id): Path<Uuid>,
    payload: Result<Json<SettlementAdjustmentTransitionRequest>, JsonRejection>,
) -> Result<Json<SettlementAdjustmentTransitionResponse>, ApiError> {
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let (scope, role) = match request.to_status.as_str() {
        "approved" => ("tax:adjustment:approve", "settlement_adjustment_approver"),
        "rejected" => ("tax:adjustment:reject", "settlement_adjustment_reviewer"),
        "voided" => ("tax:adjustment:reject", "settlement_adjustment_approver"),
        _ => {
            return Err(ApiError {
                status: StatusCode::BAD_REQUEST,
                code: "INVALID_SETTLEMENT_ADJUSTMENT_STATUS",
                message: "only approved, rejected, or voided transitions are supported".to_string(),
            });
        }
    };
    let principal = authenticate_scope_role(&state, &headers, scope, role)?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_ADJUSTMENT_STORAGE_UNAVAILABLE",
        message: "settlement adjustment storage is not configured".to_string(),
    })?;
    let result = store
        .transition_settlement_adjustment(SettlementAdjustmentTransition {
            merchant_id: principal.merchant_id,
            adjustment_id,
            actor_subject: principal.subject,
            to_status: request.to_status,
            evidence: request.evidence,
        })
        .await
        .map_err(ApiError::from)?;
    Ok(Json(SettlementAdjustmentTransitionResponse {
        status: "transitioned",
        adjustment_id: result.adjustment_id,
        from_status: result.from_status,
        to_status: result.to_status,
        approver_subject: result.approver_subject,
    }))
}

async fn create_adjustment_authority_challenge(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<AdjustmentAuthorityRoleRequest>, JsonRejection>,
) -> Result<Json<AdjustmentAuthorityChallengeResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = create_adjustment_authority_challenge_inner(state, headers, payload).await;
    metrics.observe_or_log(
        "adjustment_authority_challenge",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn create_adjustment_authority_challenge_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<AdjustmentAuthorityRoleRequest>, JsonRejection>,
) -> Result<Json<AdjustmentAuthorityChallengeResponse>, ApiError> {
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let (scope, role) = adjustment_authority_requirements(&request.role)?;
    let principal = authenticate_scope_role(&state, &headers, scope, role)?;
    let store = state.storage.clone().ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "ADJUSTMENT_AUTHORITY_STORAGE_UNAVAILABLE",
        message: "adjustment authority storage is not configured".to_string(),
    })?;
    let challenge = store
        .create_adjustment_authority_challenge(CreateAdjustmentAuthorityChallenge {
            merchant_id: principal.merchant_id,
            subject: principal.subject,
            role: request.role,
            now_unix: server_unix_now()?,
            ttl_seconds: 300,
        })
        .await
        .map_err(ApiError::from)?;
    let message = String::from_utf8(challenge.message).map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "ADJUSTMENT_AUTHORITY_CHALLENGE_CORRUPT",
        message: "stored authority challenge is not valid UTF-8".to_string(),
    })?;
    Ok(Json(AdjustmentAuthorityChallengeResponse {
        challenge_id: challenge.id,
        role: challenge.role,
        message,
        message_digest: challenge.message_digest,
        expires_at_unix: challenge.expires_at_unix,
    }))
}

async fn bind_adjustment_authority(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<AdjustmentAuthorityBindRequest>, JsonRejection>,
) -> Result<Json<AdjustmentAuthorityBindingResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = bind_adjustment_authority_inner(state, headers, payload).await;
    metrics.observe_or_log(
        "adjustment_authority_bind",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn bind_adjustment_authority_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<AdjustmentAuthorityBindRequest>, JsonRejection>,
) -> Result<Json<AdjustmentAuthorityBindingResponse>, ApiError> {
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let (scope, role) = adjustment_authority_requirements(&request.role)?;
    let principal = authenticate_scope_role(&state, &headers, scope, role)?;
    let store = state.storage.clone().ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "ADJUSTMENT_AUTHORITY_STORAGE_UNAVAILABLE",
        message: "adjustment authority storage is not configured".to_string(),
    })?;
    let challenge = store
        .load_adjustment_authority_challenge(principal.merchant_id, request.challenge_id)
        .await
        .map_err(ApiError::from)?;
    if challenge.subject != principal.subject || challenge.role != request.role {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "ADJUSTMENT_AUTHORITY_CHALLENGE_MISMATCH",
            message: "the challenge is not bound to this subject and role".to_string(),
        });
    }
    let public_key = Pubkey::from_str(&request.public_key).map_err(|_| ApiError {
        status: StatusCode::UNPROCESSABLE_ENTITY,
        code: "INVALID_SOLANA_PUBLIC_KEY",
        message: "public key is not a valid Solana Ed25519 key".to_string(),
    })?;
    if public_key == Pubkey::default() {
        return Err(ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "INVALID_SOLANA_PUBLIC_KEY",
            message: "public key must not be the default key".to_string(),
        });
    }
    let signature_bytes = URL_SAFE_NO_PAD
        .decode(request.signature.as_bytes())
        .map_err(|_| ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "INVALID_WALLET_PROOF",
            message: "wallet proof signature is not base64url".to_string(),
        })?;
    let signature = verify_wallet_proof(&public_key, &challenge.message, &signature_bytes)?;
    let binding = store
        .bind_adjustment_authority(BindAdjustmentAuthority {
            merchant_id: principal.merchant_id,
            subject: principal.subject,
            role: request.role,
            challenge_id: request.challenge_id,
            public_key: public_key.to_bytes(),
            proof_signature: signature.to_bytes(),
            now_unix: server_unix_now()?,
        })
        .await
        .map_err(ApiError::from)?;
    Ok(Json(AdjustmentAuthorityBindingResponse {
        binding_id: binding.id,
        role: binding.role,
        subject: binding.subject,
        public_key: public_key.to_string(),
        challenge_id: binding.challenge_id,
    }))
}

async fn compile_settlement_adjustment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(adjustment_id): Path<Uuid>,
    payload: Result<Json<SettlementAdjustmentEvidenceCompileRequest>, JsonRejection>,
) -> Result<Json<CompiledSettlementAdjustmentResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = compile_settlement_adjustment_inner(state, headers, adjustment_id, payload).await;
    metrics.observe_or_log(
        "settlement_adjustment_compile",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn compile_settlement_adjustment_inner(
    state: AppState,
    headers: HeaderMap,
    adjustment_id: Uuid,
    payload: Result<Json<SettlementAdjustmentEvidenceCompileRequest>, JsonRejection>,
) -> Result<Json<CompiledSettlementAdjustmentResponse>, ApiError> {
    let principal = authenticate_scope_role(
        &state,
        &headers,
        "tax:adjustment:compile",
        "settlement_adjustment_approver",
    )?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    if !request.confirm {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "ADJUSTMENT_COMPILE_CONFIRMATION_REQUIRED",
            message: "explicit evidence-compilation confirmation is required".to_string(),
        });
    }
    let store = state.storage.clone().ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_ADJUSTMENT_STORAGE_UNAVAILABLE",
        message: "settlement adjustment storage is not configured".to_string(),
    })?;
    let adjustment = store
        .load_settlement_adjustment(adjustment_id, principal.merchant_id)
        .await
        .map_err(|error| log_adjustment_compile_storage_error("load_adjustment", error))?;
    if adjustment.status != "approved"
        || adjustment.approver_subject.as_deref() != Some(principal.subject.as_str())
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "SETTLEMENT_ADJUSTMENT_NOT_APPROVED",
            message: "only the recorded approver may compile an approved adjustment".to_string(),
        });
    }
    store
        .ensure_calculation_evidence_exists(principal.merchant_id, adjustment.source_calculation_id)
        .await
        .map_err(|error| log_adjustment_compile_storage_error("source_calculation", error))?;
    if let Some(replacement) = adjustment.replacement_calculation_id {
        store
            .ensure_calculation_evidence_exists(principal.merchant_id, replacement)
            .await
            .map_err(|error| {
                log_adjustment_compile_storage_error("replacement_calculation", error)
            })?;
    }
    let (receipt, proposer_binding, approver_binding) = tokio::try_join!(
        store.load_finalized_settlement_receipt_binding(
            principal.merchant_id,
            adjustment.original_settlement_id,
            adjustment.asset_id,
        ),
        store.load_active_adjustment_authority(
            principal.merchant_id,
            &adjustment.proposer_subject,
            "proposer",
        ),
        store.load_active_adjustment_authority(
            principal.merchant_id,
            principal.subject.as_str(),
            "approver",
        ),
    )
    .map_err(|error| log_adjustment_compile_storage_error("receipt_and_authorities", error))?;
    if approver_binding.subject != principal.subject
        || approver_binding.role != "approver"
        || proposer_binding.subject != adjustment.proposer_subject
        || proposer_binding.role != "proposer"
        || proposer_binding.public_key == approver_binding.public_key
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "ADJUSTMENT_AUTHORITY_BINDING_CONFLICT",
            message: "active proposer and approver wallet bindings are invalid".to_string(),
        });
    }
    let merchant = Pubkey::new_from_array(receipt.merchant).to_string();
    let mint = Pubkey::new_from_array(receipt.mint).to_string();
    let route = store
        .load_active_settlement_route_for_chain_identity(principal.merchant_id, &merchant, &mint)
        .await
        .map_err(|error| log_adjustment_compile_storage_error("settlement_route", error))?;
    if route.mint != mint || route.merchant_account != merchant {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "SETTLEMENT_ROUTE_IDENTITY_CONFLICT",
            message: "active route identity does not match the finalized receipt".to_string(),
        });
    }
    let coordinator = state.settlement_coordinator.clone().ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "ADJUSTMENT_REGISTRY_RPC_NOT_CONFIGURED",
        message: "a settlement RPC coordinator is required to verify the finalized adjustment registry".to_string(),
    })?;
    let registry = coordinator
        .verify_adjustment_registry(&AdjustmentRegistryVerificationRequest {
            merchant: merchant.clone(),
            mint: mint.clone(),
            proposer: Pubkey::new_from_array(proposer_binding.public_key).to_string(),
            approver: Pubkey::new_from_array(approver_binding.public_key).to_string(),
            governance: None,
        })
        .await
        .map_err(|error| log_adjustment_compile_coordinator_error("verify_registry", error))?;
    let registry_address = Pubkey::from_str(&registry.registry).map_err(|_| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "ADJUSTMENT_REGISTRY_EVIDENCE_INVALID",
        message: "verified registry address is not a valid Solana public key".to_string(),
    })?;
    let request_digest = adjustment_compiler_request_digest(
        &adjustment,
        &receipt,
        &proposer_binding.public_key,
        &approver_binding.public_key,
        &route.token_program,
        &registry,
    )?;
    let intent = store
        .reserve_settlement_adjustment_evidence_intent(CreateSettlementAdjustmentEvidenceIntent {
            adjustment_id,
            merchant_id: principal.merchant_id,
            proposer: proposer_binding.public_key,
            approver: approver_binding.public_key,
            original_receipt: receipt.receipt,
            merchant: receipt.merchant,
            mint: receipt.mint,
            token_program: route.token_program.clone(),
            registry_address: registry_address.to_bytes(),
            registry_slot: registry.finalized_slot,
            registry_commitment: registry.commitment.to_string(),
            registry_account_digest: registry.account_digest,
            request_digest,
        })
        .await
        .map_err(|error| log_adjustment_compile_storage_error("reserve_evidence_intent", error))?;
    let (gross_sign, gross_magnitude) = signed_magnitude(adjustment.delta.gross_atomic)?;
    let (tax_sign, tax_magnitude) = signed_magnitude(adjustment.delta.tax_atomic)?;
    let (net_sign, net_magnitude) = signed_magnitude(adjustment.delta.net_atomic)?;
    let compiled =
        compile_settlement_adjustment_instruction(&ClientSettlementAdjustmentCompileRequest {
            program_id: SETTLEMENT_PROGRAM_ID.to_string(),
            proposer: Pubkey::new_from_array(intent.proposer).to_string(),
            approver: Pubkey::new_from_array(intent.approver).to_string(),
            merchant: Pubkey::new_from_array(intent.merchant).to_string(),
            mint: Pubkey::new_from_array(intent.mint).to_string(),
            token_program: intent.token_program.clone(),
            original_receipt: Pubkey::new_from_array(intent.original_receipt).to_string(),
            adjustment_nonce: intent.nonce,
            kind: adjustment_kind_tag(adjustment.kind),
            gross_sign,
            gross_magnitude,
            tax_sign,
            tax_magnitude,
            net_sign,
            net_magnitude,
            adjustment_digest: adjustment.adjustment_digest,
            evidence_digest: adjustment.evidence_digest,
            source_calculation_digest: calculation_reference_digest(
                adjustment.source_calculation_id,
            ),
            replacement_calculation_digest: adjustment
                .replacement_calculation_id
                .map(calculation_reference_digest)
                .unwrap_or([0; 32]),
        })
        .map_err(ApiError::from)?;
    let instruction_digest =
        sha256_digest(&serde_json::to_vec(&compiled).map_err(|error| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "SETTLEMENT_DIGEST_FAILURE",
            message: error.to_string(),
        })?);
    let intent = store
        .bind_settlement_adjustment_evidence_intent_digest(
            principal.merchant_id,
            intent.id,
            instruction_digest,
        )
        .await
        .map_err(|error| log_adjustment_compile_storage_error("bind_instruction_digest", error))?;
    let chain_intent = store
        .reserve_settlement_adjustment_chain_intent(CreateSettlementAdjustmentChainIntent {
            evidence_intent_id: intent.id,
            adjustment_id: intent.adjustment_id,
            merchant_id: intent.merchant_id,
            instruction_digest,
        })
        .await
        .map_err(|error| log_adjustment_compile_storage_error("reserve_chain_intent", error))?;
    Ok(Json(CompiledSettlementAdjustmentResponse {
        status: "prepared_unsubmitted",
        intent_id: intent.id,
        chain_intent_id: chain_intent.id,
        request_digest: intent.request_digest,
        instruction_digest,
        registry: registry.registry,
        registry_slot: registry.finalized_slot,
        registry_commitment: registry.commitment,
        registry_account_digest: registry.account_digest,
        instruction: compiled,
    }))
}

async fn submit_settlement_adjustment(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<SubmitSettlementAdjustmentRequest>, JsonRejection>,
) -> Result<Json<SubmitSettlementAdjustmentResponse>, ApiError> {
    let principal = authenticate_scope_role(
        &state,
        &headers,
        "tax:adjustment:submit",
        "settlement_adjustment_approver",
    )?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_ADJUSTMENT_STORAGE_UNAVAILABLE",
        message: "settlement adjustment storage is not configured".to_string(),
    })?;
    let intent = store
        .load_settlement_adjustment_chain_intent(request.chain_intent_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError {
            status: StatusCode::NOT_FOUND,
            code: "ADJUSTMENT_CHAIN_INTENT_NOT_FOUND",
            message: "the adjustment submission intent was not found".to_string(),
        })?;
    if intent.status != SettlementStatus::PreparedUnsubmitted.as_str()
        && intent.status != SettlementStatus::Signed.as_str()
    {
        return Ok(Json(SubmitSettlementAdjustmentResponse {
            chain_intent_id: intent.id,
            status: intent.status,
            signature: intent.signature,
            provider_index: None,
            finalized_evidence: None,
        }));
    }
    let coordinator = state.settlement_coordinator.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_RPC_NOT_CONFIGURED",
        message: "settlement RPC providers are not configured".to_string(),
    })?;
    let transaction =
        SignedTransaction::parse(&request.signed_transaction_base64).map_err(ApiError::from)?;
    let signed_digest = sha256_digest(transaction.bytes());
    let instruction_digest = sha256_digest(&serde_json::to_vec(&request.instruction).map_err(
        |error| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "SETTLEMENT_DIGEST_FAILURE",
            message: error.to_string(),
        },
    )?);
    if instruction_digest != intent.instruction_digest {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "ADJUSTMENT_INSTRUCTION_CONFLICT",
            message: "compiled adjustment instruction does not match the durable intent".into(),
        });
    }
    transaction
        .verify_adjustment_instruction(&request.instruction)
        .map_err(ApiError::from)?;
    let transaction_signature = transaction.signature().map_err(ApiError::from)?;
    coordinator
        .validate_blockhash(&transaction)
        .await
        .map_err(ApiError::from)?;
    if let Some(existing_digest) = intent.signed_transaction_digest
        && existing_digest != signed_digest
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "ADJUSTMENT_SIGNED_BYTES_CONFLICT",
            message: "intent is already bound to different signed bytes".into(),
        });
    }
    if intent.status == SettlementStatus::PreparedUnsubmitted.as_str()
        && !store
            .transition_settlement_adjustment_chain_intent(SettlementAdjustmentChainTransition {
                intent_id: intent.id,
                merchant_id: principal.merchant_id,
                expected_status: SettlementStatus::PreparedUnsubmitted.as_str().into(),
                next_status: SettlementStatus::Signed.as_str().into(),
                signed_transaction_digest: Some(signed_digest),
                signature: None,
                slot: None,
                last_error_code: None,
                finalized_event_index: None,
                finalized_event_evidence_digest: None,
                evidence: serde_json::json!({"source": "wallet_handoff"}),
            })
            .await
            .map_err(ApiError::from)?
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "ADJUSTMENT_CHAIN_INTENT_STATE_CHANGED",
            message: "adjustment submission changed concurrently; reconcile before retry".into(),
        });
    }
    if !store
        .transition_settlement_adjustment_chain_intent(SettlementAdjustmentChainTransition {
            intent_id: intent.id,
            merchant_id: principal.merchant_id,
            expected_status: SettlementStatus::Signed.as_str().into(),
            next_status: SettlementStatus::Submitting.as_str().into(),
            signed_transaction_digest: Some(signed_digest),
            signature: Some(transaction_signature.clone()),
            slot: None,
            last_error_code: None,
            finalized_event_index: None,
            finalized_event_evidence_digest: None,
            evidence: serde_json::json!({"source": "submission_fence"}),
        })
        .await
        .map_err(ApiError::from)?
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "ADJUSTMENT_SUBMISSION_IN_PROGRESS",
            message: "adjustment submission is already owned by another request; reconcile the intent before retry".into(),
        });
    }
    let submitted = match coordinator.submit(&transaction).await {
        Ok(receipt) => receipt,
        Err(error) => {
            let error_code = settlement_submission_error_code(&error);
            let _ = store
                .transition_settlement_adjustment_chain_intent(
                    SettlementAdjustmentChainTransition {
                        intent_id: intent.id,
                        merchant_id: principal.merchant_id,
                        expected_status: SettlementStatus::Submitting.as_str().into(),
                        next_status: SettlementStatus::Unknown.as_str().into(),
                        signed_transaction_digest: Some(signed_digest),
                        signature: Some(transaction_signature),
                        slot: None,
                        last_error_code: Some(error_code.to_string()),
                        finalized_event_index: None,
                        finalized_event_evidence_digest: None,
                        evidence: serde_json::json!({"source": "rpc", "error_code": error_code}),
                    },
                )
                .await;
            return Err(ApiError {
                status: StatusCode::SERVICE_UNAVAILABLE,
                code: "ADJUSTMENT_SUBMISSION_UNKNOWN",
                message:
                    "provider outcome is unknown; reconcile the adjustment intent before retry"
                        .into(),
            });
        }
    };
    if !store
        .transition_settlement_adjustment_chain_intent(SettlementAdjustmentChainTransition {
            intent_id: intent.id,
            merchant_id: principal.merchant_id,
            expected_status: SettlementStatus::Submitting.as_str().into(),
            next_status: submitted.status.as_str().into(),
            signed_transaction_digest: None,
            signature: Some(submitted.signature.clone()),
            slot: None,
            last_error_code: None,
            finalized_event_index: None,
            finalized_event_evidence_digest: None,
            evidence: serde_json::json!({"source": "rpc", "provider_index": submitted.provider_index}),
        })
        .await
        .map_err(ApiError::from)?
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "ADJUSTMENT_CHAIN_INTENT_STATE_CHANGED",
            message: "adjustment submission changed concurrently; reconcile by signature".into(),
        });
    }
    Ok(Json(SubmitSettlementAdjustmentResponse {
        chain_intent_id: intent.id,
        status: submitted.status.as_str().to_string(),
        signature: Some(submitted.signature),
        provider_index: Some(submitted.provider_index),
        finalized_evidence: None,
    }))
}

async fn get_settlement_adjustment_chain_intent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(intent_id): Path<Uuid>,
) -> Result<Json<SubmitSettlementAdjustmentResponse>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:adjustment:read")?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_ADJUSTMENT_STORAGE_UNAVAILABLE",
        message: "settlement adjustment storage is not configured".to_string(),
    })?;
    let intent = store
        .load_settlement_adjustment_chain_intent(intent_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError {
            status: StatusCode::NOT_FOUND,
            code: "ADJUSTMENT_CHAIN_INTENT_NOT_FOUND",
            message: "the adjustment submission intent was not found".to_string(),
        })?;
    let finalized_evidence =
        load_finalized_adjustment_evidence_response(&store, &intent, principal.merchant_id).await?;
    Ok(Json(SubmitSettlementAdjustmentResponse {
        chain_intent_id: intent.id,
        status: intent.status,
        signature: intent.signature,
        provider_index: None,
        finalized_evidence,
    }))
}

async fn reconcile_settlement_adjustment_chain_intent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(intent_id): Path<Uuid>,
) -> Result<Json<SubmitSettlementAdjustmentResponse>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:adjustment:reconcile")?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_ADJUSTMENT_STORAGE_UNAVAILABLE",
        message: "settlement adjustment storage is not configured".to_string(),
    })?;
    let coordinator = state.settlement_coordinator.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_RPC_NOT_CONFIGURED",
        message: "settlement RPC providers are not configured".to_string(),
    })?;
    let intent = store
        .load_settlement_adjustment_chain_intent(intent_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError {
            status: StatusCode::NOT_FOUND,
            code: "ADJUSTMENT_CHAIN_INTENT_NOT_FOUND",
            message: "the adjustment submission intent was not found".to_string(),
        })?;
    let Some(signature) = intent.signature.clone() else {
        return Ok(Json(SubmitSettlementAdjustmentResponse {
            chain_intent_id: intent.id,
            status: intent.status,
            signature: None,
            provider_index: None,
            finalized_evidence: None,
        }));
    };
    let observed = coordinator
        .observe(&signature)
        .await
        .map_err(ApiError::from)?;
    let current = parse_settlement_status_for_api(&intent.status)?;
    let target = if observed.status == SettlementStatus::Finalized {
        SettlementStatus::Confirmed
    } else {
        observed.status
    };
    let mut status = current;
    if target != current
        && can_transition(current, target)
        && store
            .transition_settlement_adjustment_chain_intent(SettlementAdjustmentChainTransition {
                intent_id: intent.id,
                merchant_id: principal.merchant_id,
                expected_status: current.as_str().into(),
                next_status: target.as_str().into(),
                signed_transaction_digest: None,
                signature: Some(signature.clone()),
                slot: observed.slot,
                last_error_code: (target == SettlementStatus::Unknown)
                    .then_some("rpc_status_unknown".to_string()),
                finalized_event_index: None,
                finalized_event_evidence_digest: None,
                evidence: serde_json::json!({
                    "source": "rpc_status",
                    "provider_index": observed.provider_index,
                    "finalized_event_required": observed.status == SettlementStatus::Finalized
                }),
            })
            .await
            .map_err(ApiError::from)?
    {
        status = target;
    }
    if observed.status == SettlementStatus::Finalized {
        let evidence_intent = store
            .load_settlement_adjustment_evidence_intent(
                intent.evidence_intent_id,
                principal.merchant_id,
            )
            .await
            .map_err(ApiError::from)?
            .ok_or_else(|| ApiError {
                status: StatusCode::CONFLICT,
                code: "ADJUSTMENT_EVIDENCE_INTENT_NOT_FOUND",
                message: "immutable adjustment evidence is unavailable for reconciliation".into(),
            })?;
        let adjustment = store
            .load_settlement_adjustment(intent.adjustment_id, principal.merchant_id)
            .await
            .map_err(ApiError::from)?;
        let Some(observed_slot) = observed.slot else {
            return Ok(Json(SubmitSettlementAdjustmentResponse {
                chain_intent_id: intent.id,
                status: status.as_str().to_string(),
                signature: Some(signature),
                provider_index: Some(observed.provider_index),
                finalized_evidence: None,
            }));
        };
        let expected = expected_finalized_adjustment_record(
            &intent,
            &evidence_intent,
            &adjustment,
            signature.clone(),
            observed_slot,
        )?;
        if let Some(record) = store
            .load_finalized_settlement_adjustment_record(
                intent.id,
                principal.merchant_id,
                &expected,
            )
            .await
            .map_err(ApiError::from)?
            && status != SettlementStatus::Finalized
            && can_transition(status, SettlementStatus::Finalized)
            && store
                .transition_settlement_adjustment_chain_intent(
                    SettlementAdjustmentChainTransition {
                        intent_id: intent.id,
                        merchant_id: principal.merchant_id,
                        expected_status: status.as_str().into(),
                        next_status: SettlementStatus::Finalized.as_str().into(),
                        signed_transaction_digest: None,
                        signature: Some(record.signature.clone()),
                        slot: Some(record.slot),
                        last_error_code: None,
                        finalized_event_index: Some(record.event_index),
                        finalized_event_evidence_digest: Some(record.evidence_digest),
                        evidence: serde_json::json!({
                            "source": "finalized_indexer_event",
                            "event_index": record.event_index,
                            "evidence_digest": record.evidence_digest
                        }),
                    },
                )
                .await
                .map_err(ApiError::from)?
        {}
    }
    let response_intent = store
        .load_settlement_adjustment_chain_intent(intent.id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError {
            status: StatusCode::CONFLICT,
            code: "ADJUSTMENT_CHAIN_INTENT_NOT_FOUND",
            message: "adjustment intent disappeared during reconciliation".into(),
        })?;
    let finalized_evidence = load_finalized_adjustment_evidence_response(
        &store,
        &response_intent,
        principal.merchant_id,
    )
    .await?;
    Ok(Json(SubmitSettlementAdjustmentResponse {
        chain_intent_id: response_intent.id,
        status: response_intent.status,
        signature: response_intent.signature,
        provider_index: Some(observed.provider_index),
        finalized_evidence,
    }))
}

fn parse_settlement_status_for_api(value: &str) -> Result<SettlementStatus, ApiError> {
    match value {
        "prepared_unsubmitted" => Ok(SettlementStatus::PreparedUnsubmitted),
        "signed" => Ok(SettlementStatus::Signed),
        "submitting" => Ok(SettlementStatus::Submitting),
        "submitted" => Ok(SettlementStatus::Submitted),
        "processed" => Ok(SettlementStatus::Processed),
        "confirmed" => Ok(SettlementStatus::Confirmed),
        "finalized" => Ok(SettlementStatus::Finalized),
        "rejected" => Ok(SettlementStatus::Rejected),
        "unknown" => Ok(SettlementStatus::Unknown),
        _ => Err(ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "INVALID_ADJUSTMENT_CHAIN_LIFECYCLE",
            message: "stored adjustment chain status is invalid".to_string(),
        }),
    }
}

fn adjustment_authority_requirements(role: &str) -> Result<(&'static str, &'static str), ApiError> {
    match role {
        "proposer" => Ok((
            "tax:adjustment:authority:bind",
            "settlement_adjustment_proposer",
        )),
        "approver" => Ok((
            "tax:adjustment:authority:bind",
            "settlement_adjustment_approver",
        )),
        "refund_funder" => Ok(("tax:refund:authority:bind", "refund_funding_funder")),
        "governance" => Ok((
            "tax:adjustment:authority:bind",
            "settlement_adjustment_governance",
        )),
        _ => Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "INVALID_ADJUSTMENT_AUTHORITY_ROLE",
            message: "role must be proposer, approver, refund_funder, or governance".to_string(),
        }),
    }
}

fn verify_wallet_proof(
    public_key: &Pubkey,
    message: &[u8],
    signature_bytes: &[u8],
) -> Result<Signature, ApiError> {
    let signature = Signature::from_slice(signature_bytes).map_err(|_| ApiError {
        status: StatusCode::UNPROCESSABLE_ENTITY,
        code: "INVALID_WALLET_PROOF",
        message: "wallet proof signature is not Ed25519".to_string(),
    })?;
    let verifying_key = VerifyingKey::from_bytes(&public_key.to_bytes()).map_err(|_| ApiError {
        status: StatusCode::UNPROCESSABLE_ENTITY,
        code: "INVALID_SOLANA_PUBLIC_KEY",
        message: "public key is not a valid Ed25519 verifying key".to_string(),
    })?;
    verifying_key
        .verify(message, &signature)
        .map_err(|_| ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "INVALID_WALLET_PROOF",
            message: "wallet proof does not match the server challenge".to_string(),
        })?;
    Ok(signature)
}

fn adjustment_kind_tag(kind: AdjustmentKind) -> u8 {
    match kind {
        AdjustmentKind::Refund => 1,
        AdjustmentKind::Reversal => 2,
        AdjustmentKind::Correction => 3,
        AdjustmentKind::Chargeback => 4,
        AdjustmentKind::RemittanceAdjustment => 5,
    }
}

fn signed_magnitude(value: i128) -> Result<(u8, String), ApiError> {
    let magnitude = value.unsigned_abs();
    let magnitude = u64::try_from(magnitude).map_err(|_| ApiError {
        status: StatusCode::UNPROCESSABLE_ENTITY,
        code: "SETTLEMENT_ADJUSTMENT_AMOUNT_OUT_OF_RANGE",
        message: "adjustment amount exceeds the unsigned atomic range".to_string(),
    })?;
    Ok((u8::from(value < 0), magnitude.to_string()))
}

fn signed_magnitude_u64(value: i128) -> Result<(u8, u64), ApiError> {
    let magnitude = u64::try_from(value.unsigned_abs()).map_err(|_| ApiError {
        status: StatusCode::UNPROCESSABLE_ENTITY,
        code: "SETTLEMENT_ADJUSTMENT_AMOUNT_OUT_OF_RANGE",
        message: "adjustment amount exceeds the unsigned atomic range".to_string(),
    })?;
    Ok((u8::from(value < 0), magnitude))
}

fn calculation_reference_digest(calculation_id: Uuid) -> Digest32 {
    let mut input = b"solana-tax-protocol/calculation-id/v1".to_vec();
    input.extend_from_slice(calculation_id.as_bytes());
    sha256_digest(&input)
}

fn adjustment_compiler_request_digest(
    adjustment: &storage::SettlementAdjustmentRecord,
    receipt: &storage::SettlementReceiptBinding,
    proposer: &[u8; 32],
    approver: &[u8; 32],
    token_program: &str,
    registry: &settlement_coordinator::AdjustmentRegistryVerification,
) -> Result<Digest32, ApiError> {
    let payload = serde_json::to_vec(&AdjustmentCompilerDigestInput {
        schema: "solana-tax-protocol/adjustment-compile-request/v1",
        adjustment_id: adjustment.id,
        adjustment_digest: adjustment.adjustment_digest,
        evidence_digest: adjustment.evidence_digest,
        source_calculation_id: adjustment.source_calculation_id,
        replacement_calculation_id: adjustment.replacement_calculation_id,
        kind: adjustment.kind,
        delta: adjustment.delta,
        asset_id: adjustment.asset_id,
        settlement_id: receipt.settlement_id,
        signature: receipt.signature.as_str(),
        slot: receipt.slot,
        receipt: receipt.receipt,
        merchant: receipt.merchant,
        mint: receipt.mint,
        proposer,
        approver,
        token_program,
        registry: &registry.registry,
        registry_slot: registry.finalized_slot,
        registry_commitment: registry.commitment,
        registry_account_digest: registry.account_digest,
    })
    .map_err(|error| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "SETTLEMENT_DIGEST_FAILURE",
        message: error.to_string(),
    })?;
    Ok(sha256_digest(&payload))
}

async fn create_vault_release_claim(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<VaultReleaseClaimCreateRequest>, JsonRejection>,
) -> Result<Json<storage::VaultReleaseClaim>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = create_vault_release_claim_inner(state, headers, payload).await;
    metrics.observe_or_log(
        "vault_release_claim_create",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn create_vault_release_claim_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<VaultReleaseClaimCreateRequest>, JsonRejection>,
) -> Result<Json<storage::VaultReleaseClaim>, ApiError> {
    let principal = authenticate_scope_role(
        &state,
        &headers,
        "tax:vault:propose",
        "vault_release_proposer",
    )?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    validate_release_public_keys(&request)?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "VAULT_RELEASE_STORAGE_UNAVAILABLE",
        message: "vault release storage is not configured".to_string(),
    })?;
    let outcome = store
        .create_vault_release_claim(CreateVaultReleaseClaimCommand {
            merchant_id: principal.merchant_id,
            actor_subject: principal.subject,
            idempotency_key: request.idempotency_key,
            mint: request.mint,
            token_program: request.token_program,
            tax_vault: request.tax_vault,
            destination: request.destination,
            nonce: request.nonce,
            amount_atomic: request.amount_atomic,
            reference_digest: request.reference_digest,
            execute_after_unix: request.execute_after_unix,
            expires_at_unix: request.expires_at_unix,
        })
        .await
        .map_err(|error| {
            tracing::error!(error = ?error, "vault release claim persistence failed");
            ApiError::from(error)
        })?;
    let claim = match outcome {
        storage::VaultReleaseClaimOutcome::Created(claim)
        | storage::VaultReleaseClaimOutcome::Replay(claim) => claim,
    };
    Ok(Json(claim))
}

async fn load_vault_release_claim(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(claim_id): Path<Uuid>,
) -> Result<Json<storage::VaultReleaseClaim>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = load_vault_release_claim_inner(state, headers, claim_id).await;
    metrics.observe_or_log(
        "vault_release_claim_read",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn load_vault_release_claim_inner(
    state: AppState,
    headers: HeaderMap,
    claim_id: Uuid,
) -> Result<Json<storage::VaultReleaseClaim>, ApiError> {
    let principal = authenticate_scope_any_role(
        &state,
        &headers,
        "tax:vault:read",
        &[
            "vault_release_proposer",
            "vault_release_approver",
            "vault_release_governance",
            "vault_release_auditor",
            "vault_release_operator",
        ],
    )?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "VAULT_RELEASE_STORAGE_UNAVAILABLE",
        message: "vault release storage is not configured".to_string(),
    })?;
    Ok(Json(
        store
            .load_vault_release_claim(claim_id, principal.merchant_id)
            .await
            .map_err(|error| {
                tracing::error!(error = ?error, "vault release claim read failed");
                ApiError::from(error)
            })?,
    ))
}

async fn list_vault_reconciliation_targets(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<VaultReconciliationListQuery>,
) -> Result<Json<Vec<storage::VaultReconciliationTargetView>>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = list_vault_reconciliation_targets_inner(state, headers, query).await;
    metrics.observe_or_log(
        "vault_reconciliation_read",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn list_vault_reconciliation_targets_inner(
    state: AppState,
    headers: HeaderMap,
    query: VaultReconciliationListQuery,
) -> Result<Json<Vec<storage::VaultReconciliationTargetView>>, ApiError> {
    let principal = authenticate_scope_any_role(
        &state,
        &headers,
        "tax:vault:read",
        &[
            "vault_release_proposer",
            "vault_release_approver",
            "vault_release_governance",
            "vault_release_auditor",
            "vault_release_operator",
        ],
    )?;
    let limit = query.limit.unwrap_or(50);
    if !(1..=64).contains(&limit) {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "INVALID_VAULT_RECONCILIATION_LIMIT",
            message: "reconciliation target limit must be between 1 and 64".to_string(),
        });
    }
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "VAULT_RECONCILIATION_STORAGE_UNAVAILABLE",
        message: "vault reconciliation storage is not configured".to_string(),
    })?;
    Ok(Json(
        store
            .list_vault_reconciliation_target_views(principal.merchant_id, limit)
            .await
            .map_err(|error| {
                tracing::error!(error = ?error, "vault reconciliation target read failed");
                ApiError::from(error)
            })?,
    ))
}

async fn list_vault_reconciliation_runs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(target_id): Path<Uuid>,
    Query(query): Query<VaultReconciliationListQuery>,
) -> Result<Json<Vec<storage::VaultReconciliationRunView>>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = list_vault_reconciliation_runs_inner(state, headers, target_id, query).await;
    metrics.observe_or_log(
        "vault_reconciliation_read",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn list_vault_reconciliation_runs_inner(
    state: AppState,
    headers: HeaderMap,
    target_id: Uuid,
    query: VaultReconciliationListQuery,
) -> Result<Json<Vec<storage::VaultReconciliationRunView>>, ApiError> {
    let principal = authenticate_scope_any_role(
        &state,
        &headers,
        "tax:vault:read",
        &[
            "vault_release_proposer",
            "vault_release_approver",
            "vault_release_governance",
            "vault_release_auditor",
            "vault_release_operator",
        ],
    )?;
    let limit = query.limit.unwrap_or(50);
    if !(1..=64).contains(&limit) {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "INVALID_VAULT_RECONCILIATION_LIMIT",
            message: "reconciliation run limit must be between 1 and 64".to_string(),
        });
    }
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "VAULT_RECONCILIATION_STORAGE_UNAVAILABLE",
        message: "vault reconciliation storage is not configured".to_string(),
    })?;
    Ok(Json(
        store
            .list_vault_reconciliation_runs(principal.merchant_id, target_id, limit)
            .await
            .map_err(|error| {
                tracing::error!(error = ?error, "vault reconciliation run read failed");
                ApiError::from(error)
            })?,
    ))
}

async fn transition_vault_release_claim(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(claim_id): Path<Uuid>,
    payload: Result<Json<VaultReleaseClaimTransitionRequest>, JsonRejection>,
) -> Result<Json<VaultReleaseClaimTransitionResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = transition_vault_release_claim_inner(state, headers, claim_id, payload).await;
    metrics.observe_or_log(
        "vault_release_claim_transition",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn transition_vault_release_claim_inner(
    state: AppState,
    headers: HeaderMap,
    claim_id: Uuid,
    payload: Result<Json<VaultReleaseClaimTransitionRequest>, JsonRejection>,
) -> Result<Json<VaultReleaseClaimTransitionResponse>, ApiError> {
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    validate_release_transition_evidence(&request)?;
    let (scope, role) = match request.to_status.as_str() {
        "approved" => ("tax:vault:approve", "vault_release_approver"),
        "onchain_created" => ("tax:vault:record", "vault_release_operator"),
        "executed" => ("tax:vault:execute", "vault_release_approver"),
        "cancelled" => ("tax:vault:cancel", "vault_release_governance"),
        "unknown" => ("tax:vault:record", "vault_release_operator"),
        _ => {
            return Err(ApiError {
                status: StatusCode::BAD_REQUEST,
                code: "INVALID_VAULT_RELEASE_STATUS",
                message: "unsupported vault release claim status".to_string(),
            });
        }
    };
    let principal = authenticate_scope_role(&state, &headers, scope, role)?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "VAULT_RELEASE_STORAGE_UNAVAILABLE",
        message: "vault release storage is not configured".to_string(),
    })?;
    let result = store
        .transition_vault_release_claim(VaultReleaseClaimTransition {
            merchant_id: principal.merchant_id,
            claim_id,
            actor_subject: principal.subject,
            to_status: request.to_status,
            evidence: request.evidence,
            claim_pda: request.claim_pda,
            transaction_signature: request.transaction_signature,
            slot: request.slot,
            last_error_code: request.last_error_code,
        })
        .await
        .map_err(|error| {
            tracing::error!(error = ?error, "vault release claim transition failed");
            ApiError::from(error)
        })?;
    Ok(Json(VaultReleaseClaimTransitionResponse {
        status: "transitioned",
        claim_id: result.claim_id,
        from_status: result.from_status,
        to_status: result.to_status,
        approver_subject: result.approver_subject,
    }))
}

fn validate_release_public_keys(request: &VaultReleaseClaimCreateRequest) -> Result<(), ApiError> {
    for (field, value) in [
        ("mint", request.mint.as_str()),
        ("token_program", request.token_program.as_str()),
        ("tax_vault", request.tax_vault.as_str()),
        ("destination", request.destination.as_str()),
    ] {
        Pubkey::from_str(value).map_err(|_| ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "INVALID_VAULT_RELEASE_ACCOUNT",
            message: format!("{field} is not a valid Solana public key"),
        })?;
    }
    if request.tax_vault == request.destination {
        return Err(ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "INVALID_VAULT_RELEASE_ACCOUNT",
            message: "tax vault and destination must be distinct".to_string(),
        });
    }
    Ok(())
}

fn validate_release_transition_evidence(
    request: &VaultReleaseClaimTransitionRequest,
) -> Result<(), ApiError> {
    if let Some(claim_pda) = request.claim_pda.as_deref() {
        Pubkey::from_str(claim_pda).map_err(|_| ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "INVALID_VAULT_RELEASE_CLAIM_PDA",
            message: "claim_pda is not a valid Solana public key".to_string(),
        })?;
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct QuoteReplayResponse {
    pub status: &'static str,
    pub calculation_id: Uuid,
    pub engine_version: String,
    pub facts_digest: Digest32,
    pub rule_digest: Digest32,
}

async fn replay_quote(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(calculation_id): Path<Uuid>,
) -> Result<Json<QuoteReplayResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = replay_quote_inner(state, headers, calculation_id).await;
    metrics.observe_or_log("tax_replay", started.elapsed(), result.is_err());
    result
}

async fn replay_quote_inner(
    state: AppState,
    headers: HeaderMap,
    calculation_id: Uuid,
) -> Result<Json<QuoteReplayResponse>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:quote")?;
    let Some(store) = state.storage else {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "QUOTE_STORAGE_UNAVAILABLE",
            message: "published-rule quote storage is not configured".to_string(),
        });
    };
    let request_id = request_id_from_headers(&headers);
    let verification = store
        .verify_quote_replay(principal.merchant_id, calculation_id)
        .await;
    let audit_result = match &verification {
        Ok(evidence) => {
            store
                .record_evidence_access_audit(EvidenceAccessAuditCommand {
                    merchant_id: principal.merchant_id,
                    calculation_id,
                    actor_subject: principal.subject.clone(),
                    request_id,
                    purpose: "tax_quote_replay".into(),
                    outcome: "allowed".into(),
                    facts_digest: Some(evidence.facts_digest),
                })
                .await
        }
        Err(_) => {
            store
                .record_evidence_access_audit(EvidenceAccessAuditCommand {
                    merchant_id: principal.merchant_id,
                    calculation_id,
                    actor_subject: principal.subject.clone(),
                    request_id,
                    purpose: "tax_quote_replay".into(),
                    outcome: "denied".into(),
                    facts_digest: None,
                })
                .await
        }
    };
    audit_result.map_err(ApiError::from)?;
    let verification = verification.map_err(ApiError::from)?;
    Ok(Json(QuoteReplayResponse {
        status: "replay_verified",
        calculation_id: verification.calculation_id,
        engine_version: verification.engine_version,
        facts_digest: verification.facts_digest,
        rule_digest: verification.rule_digest,
    }))
}

#[derive(Debug, Serialize)]
pub struct DocumentQuoteReplayResponse {
    pub status: &'static str,
    pub schema: String,
    pub calculation_id: Uuid,
    pub engine_version: String,
    pub facts_digest: Digest32,
    pub rule_digest: Digest32,
}

async fn replay_document_quote(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(calculation_id): Path<Uuid>,
) -> Result<Json<DocumentQuoteReplayResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = replay_document_quote_inner(state, headers, calculation_id).await;
    metrics.observe_or_log("tax_document_replay", started.elapsed(), result.is_err());
    result
}

async fn replay_document_quote_inner(
    state: AppState,
    headers: HeaderMap,
    calculation_id: Uuid,
) -> Result<Json<DocumentQuoteReplayResponse>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:quote")?;
    let Some(store) = state.storage else {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "QUOTE_STORAGE_UNAVAILABLE",
            message: "published-rule document quote storage is not configured".to_string(),
        });
    };
    let request_id = request_id_from_headers(&headers);
    let verification = store
        .verify_document_quote_replay(principal.merchant_id, calculation_id)
        .await;
    let audit_result = match &verification {
        Ok(evidence) => {
            store
                .record_evidence_access_audit(EvidenceAccessAuditCommand {
                    merchant_id: principal.merchant_id,
                    calculation_id,
                    actor_subject: principal.subject.clone(),
                    request_id,
                    purpose: "tax_document_quote_replay".into(),
                    outcome: "allowed".into(),
                    facts_digest: Some(evidence.facts_digest),
                })
                .await
        }
        Err(_) => {
            store
                .record_evidence_access_audit(EvidenceAccessAuditCommand {
                    merchant_id: principal.merchant_id,
                    calculation_id,
                    actor_subject: principal.subject.clone(),
                    request_id,
                    purpose: "tax_document_quote_replay".into(),
                    outcome: "denied".into(),
                    facts_digest: None,
                })
                .await
        }
    };
    audit_result.map_err(ApiError::from)?;
    let verification = verification.map_err(ApiError::from)?;
    Ok(Json(DocumentQuoteReplayResponse {
        status: "replay_verified",
        schema: verification.schema,
        calculation_id: verification.calculation_id,
        engine_version: verification.engine_version,
        facts_digest: verification.facts_digest,
        rule_digest: verification.rule_digest,
    }))
}

fn request_id_from_headers(headers: &HeaderMap) -> Uuid {
    headers
        .get(&REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok())
        .unwrap_or_else(Uuid::now_v7)
}

async fn list_replay_gaps(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ReplayGapListQuery>,
) -> Result<Json<Vec<storage::ReplayGapView>>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = list_replay_gaps_inner(state, headers, query).await;
    metrics.observe_or_log(
        "indexer_replay_operator",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn list_replay_gaps_inner(
    state: AppState,
    headers: HeaderMap,
    query: ReplayGapListQuery,
) -> Result<Json<Vec<storage::ReplayGapView>>, ApiError> {
    authenticate_scope_role(&state, &headers, "tax:indexer:read", "indexer_operator")?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "INDEXER_STORAGE_UNAVAILABLE",
        message: "indexer replay storage is not configured".to_string(),
    })?;
    Ok(Json(
        store
            .list_replay_gaps(
                &query.stream_key,
                query.status.as_deref(),
                query.limit.unwrap_or(50),
            )
            .await
            .map_err(ApiError::from)?,
    ))
}

async fn acknowledge_replay_gap(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(gap_id): Path<Uuid>,
    payload: Result<Json<ReplayGapOperatorRequest>, JsonRejection>,
) -> Result<Json<storage::ReplayGapView>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = acknowledge_replay_gap_inner(state, headers, gap_id, payload).await;
    metrics.observe_or_log(
        "indexer_replay_operator",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn acknowledge_replay_gap_inner(
    state: AppState,
    headers: HeaderMap,
    gap_id: Uuid,
    payload: Result<Json<ReplayGapOperatorRequest>, JsonRejection>,
) -> Result<Json<storage::ReplayGapView>, ApiError> {
    let principal =
        authenticate_scope_role(&state, &headers, "tax:indexer:operate", "indexer_operator")?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "INDEXER_STORAGE_UNAVAILABLE",
        message: "indexer replay storage is not configured".to_string(),
    })?;
    Ok(Json(
        store
            .acknowledge_replay_gap_for_operator(gap_id, &principal.subject, &request.note)
            .await
            .map_err(ApiError::from)?,
    ))
}

async fn requeue_replay_gap(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(gap_id): Path<Uuid>,
    payload: Result<Json<ReplayGapOperatorRequest>, JsonRejection>,
) -> Result<Json<storage::ReplayGapView>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = requeue_replay_gap_inner(state, headers, gap_id, payload).await;
    metrics.observe_or_log(
        "indexer_replay_operator",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn requeue_replay_gap_inner(
    state: AppState,
    headers: HeaderMap,
    gap_id: Uuid,
    payload: Result<Json<ReplayGapOperatorRequest>, JsonRejection>,
) -> Result<Json<storage::ReplayGapView>, ApiError> {
    let principal =
        authenticate_scope_role(&state, &headers, "tax:indexer:operate", "indexer_operator")?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "INDEXER_STORAGE_UNAVAILABLE",
        message: "indexer replay storage is not configured".to_string(),
    })?;
    Ok(Json(
        store
            .requeue_replay_gap_for_operator(gap_id, &principal.subject, &request.note)
            .await
            .map_err(ApiError::from)?,
    ))
}

async fn targeted_recovery(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<TargetedRecoveryRequest>, JsonRejection>,
) -> Result<Json<TargetedRecoveryResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = targeted_recovery_inner(state, headers, payload).await;
    metrics.observe_or_log(
        "indexer_targeted_recovery",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn targeted_recovery_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<TargetedRecoveryRequest>, JsonRejection>,
) -> Result<Json<TargetedRecoveryResponse>, ApiError> {
    let principal =
        authenticate_scope_role(&state, &headers, "tax:indexer:operate", "indexer_operator")?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let ports = state.targeted_recovery.as_ref().ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "INDEXER_TARGETED_RECOVERY_UNAVAILABLE",
        message: "targeted recovery is not configured for this API instance".to_string(),
    })?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "INDEXER_STORAGE_UNAVAILABLE",
        message: "indexer recovery storage is not configured".to_string(),
    })?;
    let max_pages = request.max_pages.unwrap_or(ports.max_pages);
    if max_pages == 0 || max_pages > ports.max_pages || max_pages > MAX_TARGETED_RECOVERY_PAGES {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "INVALID_INDEXER_TARGETED_RECOVERY_BUDGET",
            message: "targeted recovery page budget exceeds the server policy".to_string(),
        });
    }
    let request_id = request_id_from_headers(&headers);
    let audit_id = store
        .record_targeted_recovery_audit(storage::TargetedRecoveryAuditCommand {
            request_id,
            stream_key: ports.stream_key.clone(),
            target_address: request.address.clone(),
            target_signature: request.signature.clone(),
            operator_subject: principal.subject.clone(),
            note: request.note.clone(),
            outcome: "requested".to_string(),
            pages_scanned: None,
            slot: None,
            error_code: None,
        })
        .await
        .map_err(ApiError::from)?;

    let recovery = recover_targeted_signature(
        &store,
        ports.signature_source.as_ref(),
        ports.transaction_source.as_ref(),
        &ports.stream_key,
        &request.address,
        &request.signature,
        max_pages,
    )
    .await;
    match recovery {
        Ok(recovery) => {
            store
                .record_targeted_recovery_audit(storage::TargetedRecoveryAuditCommand {
                    request_id,
                    stream_key: ports.stream_key.clone(),
                    target_address: request.address,
                    target_signature: recovery.signature.clone(),
                    operator_subject: principal.subject,
                    note: request.note,
                    outcome: "recovered".to_string(),
                    pages_scanned: Some(recovery.pages_scanned as u32),
                    slot: Some(recovery.slot),
                    error_code: None,
                })
                .await
                .map_err(ApiError::from)?;
            Ok(Json(TargetedRecoveryResponse {
                status: "recovered",
                audit_id,
                signature: recovery.signature,
                pages_scanned: recovery.pages_scanned,
                slot: recovery.slot,
                raw_transaction_count: recovery.raw_transaction_count,
                raw_evidence_count: recovery.raw_evidence_count,
                receipt_count: recovery.receipt_count,
            }))
        }
        Err(error) => {
            let error_code = targeted_recovery_error_code(&error);
            let audit_result = store
                .record_targeted_recovery_audit(storage::TargetedRecoveryAuditCommand {
                    request_id,
                    stream_key: ports.stream_key.clone(),
                    target_address: request.address,
                    target_signature: request.signature,
                    operator_subject: principal.subject,
                    note: request.note,
                    outcome: "failed".to_string(),
                    pages_scanned: None,
                    slot: None,
                    error_code: Some(error_code.to_string()),
                })
                .await;
            audit_result.map_err(ApiError::from)?;
            Err(targeted_recovery_api_error(error))
        }
    }
}

fn targeted_recovery_error_code(error: &IndexerServiceError) -> &'static str {
    match error {
        IndexerServiceError::TargetNotFoundWithinBudget { .. } => "target_not_found_within_budget",
        IndexerServiceError::TargetedRecoveryInvalid(_) => "targeted_recovery_invalid",
        IndexerServiceError::Provider(_) => "provider_failure",
        IndexerServiceError::Storage(_) => "storage_failure",
        IndexerServiceError::Configuration(_) => "recovery_configuration_invalid",
        IndexerServiceError::RefundClaimDomain(_) => "refund_claim_domain_invalid",
        _ => "targeted_recovery_failed",
    }
}

fn targeted_recovery_api_error(error: IndexerServiceError) -> ApiError {
    match error {
        IndexerServiceError::TargetNotFoundWithinBudget { .. } => ApiError {
            status: StatusCode::NOT_FOUND,
            code: "INDEXER_TARGET_NOT_FOUND_WITHIN_BUDGET",
            message: "the target signature was not found within the bounded recovery budget"
                .to_string(),
        },
        IndexerServiceError::Configuration(message)
        | IndexerServiceError::TargetedRecoveryInvalid(message) => ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "INVALID_INDEXER_TARGETED_RECOVERY",
            message,
        },
        IndexerServiceError::Provider(message) => ApiError {
            status: StatusCode::BAD_GATEWAY,
            code: "INDEXER_TARGETED_RECOVERY_PROVIDER_FAILED",
            message: message.to_string(),
        },
        IndexerServiceError::Storage(error) => ApiError::from(error),
        IndexerServiceError::Domain(error) => ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "INVALID_INDEXER_PAGE",
            message: error.to_string(),
        },
        IndexerServiceError::RefundProofDomain(message) => ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "INVALID_REFUND_BALANCE_PROOF",
            message,
        },
        IndexerServiceError::RefundClaimDomain(message) => ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "INVALID_REFUND_CLAIM_RECONCILIATION",
            message,
        },
        IndexerServiceError::GapRecordFailed { primary, gap } => ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "INDEXER_TARGETED_RECOVERY_AUDIT_FAILED",
            message: format!("{primary}; audit persistence failed: {gap}"),
        },
        IndexerServiceError::ReplayLeaseLost => ApiError {
            status: StatusCode::CONFLICT,
            code: "INDEXER_REPLAY_LEASE_LOST",
            message: "recovery lease ownership was lost".to_string(),
        },
    }
}

async fn compile_settlement(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<SettlementCompileRequest>, JsonRejection>,
) -> Result<Json<CompiledSettlementResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = compile_settlement_inner(state, headers, payload).await;
    metrics.observe_or_log("settlement_compile", started.elapsed(), result.is_err());
    result
}

async fn compile_settlement_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<SettlementCompileRequest>, JsonRejection>,
) -> Result<Json<CompiledSettlementResponse>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:settlement:compile")?;
    if !state.simulation_mode {
        return Err(ApiError {
            status: StatusCode::NOT_IMPLEMENTED,
            code: "INTERNAL_SETTLEMENT_COMPILE_DISABLED",
            message:
                "amount-bearing settlement compilation is disabled outside simulation mode; use the authenticated quote-bound route"
                    .to_string(),
        });
    }
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    compile_settlement_request(state, principal.merchant_id, request, None).await
}

async fn list_settlement_routes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<storage::SettlementRoute>>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:settlement:compile")?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_STORAGE_UNAVAILABLE",
        message: "settlement route storage is not configured".to_string(),
    })?;
    Ok(Json(
        store
            .list_active_settlement_routes(principal.merchant_id)
            .await
            .map_err(ApiError::from)?,
    ))
}

async fn create_refund_funding_configuration(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<RefundFundingConfigurationCreateRequest>, JsonRejection>,
) -> Result<Json<storage::RefundFundingConfiguration>, ApiError> {
    let started = std::time::Instant::now();
    let metrics = Arc::clone(&state.metrics);
    let result = create_refund_funding_configuration_inner(state, headers, payload).await;
    metrics.observe_or_log(
        "refund_funding_configuration_create",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn create_refund_funding_configuration_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<RefundFundingConfigurationCreateRequest>, JsonRejection>,
) -> Result<Json<storage::RefundFundingConfiguration>, ApiError> {
    let principal = authenticate_scope_role(
        &state,
        &headers,
        "tax:refund:configure",
        "refund_funding_proposer",
    )?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let keys = canonicalize_refund_funding_public_keys(&request)?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_FUNDING_STORAGE_UNAVAILABLE",
        message: "refund funding configuration storage is not configured".to_string(),
    })?;
    let result = store
        .create_refund_funding_configuration(CreateRefundFundingConfigurationCommand {
            merchant_id: principal.merchant_id,
            actor_subject: principal.subject,
            idempotency_key: request.idempotency_key,
            configuration_key: request.configuration_key,
            version: request.version,
            merchant_account: keys.merchant_account,
            mint: keys.mint,
            tax_vault: keys.tax_vault,
            vault_authority: keys.vault_authority,
            refund_funder: keys.refund_funder,
            refund_funder_subject: request.refund_funder_subject,
            net_source: keys.net_source,
            token_program: keys.token_program,
            effective_from_unix: request.effective_from_unix,
            effective_until_unix: request.effective_until_unix,
        })
        .await
        .map_err(ApiError::from)?;
    let configuration = match result {
        storage::RefundFundingConfigurationOutcome::Created(configuration)
        | storage::RefundFundingConfigurationOutcome::Replay(configuration) => configuration,
    };
    Ok(Json(configuration))
}

async fn list_refund_funding_configurations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<storage::RefundFundingConfiguration>>, ApiError> {
    let started = std::time::Instant::now();
    let metrics = Arc::clone(&state.metrics);
    let result = list_refund_funding_configurations_inner(state, headers).await;
    metrics.observe_or_log(
        "refund_funding_configuration_read",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn list_refund_funding_configurations_inner(
    state: AppState,
    headers: HeaderMap,
) -> Result<Json<Vec<storage::RefundFundingConfiguration>>, ApiError> {
    let principal = authenticate_scope_any_role(
        &state,
        &headers,
        "tax:refund:configure",
        &[
            "refund_funding_proposer",
            "refund_funding_approver",
            "settlement_adjustment_auditor",
        ],
    )?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_FUNDING_STORAGE_UNAVAILABLE",
        message: "refund funding configuration storage is not configured".to_string(),
    })?;
    Ok(Json(
        store
            .list_refund_funding_configurations(principal.merchant_id)
            .await
            .map_err(ApiError::from)?,
    ))
}

async fn load_refund_funding_configuration(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(configuration_id): Path<Uuid>,
) -> Result<Json<storage::RefundFundingConfiguration>, ApiError> {
    let started = std::time::Instant::now();
    let metrics = Arc::clone(&state.metrics);
    let result = load_refund_funding_configuration_inner(state, headers, configuration_id).await;
    metrics.observe_or_log(
        "refund_funding_configuration_read",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn load_refund_funding_configuration_inner(
    state: AppState,
    headers: HeaderMap,
    configuration_id: Uuid,
) -> Result<Json<storage::RefundFundingConfiguration>, ApiError> {
    let principal = authenticate_scope_any_role(
        &state,
        &headers,
        "tax:refund:configure",
        &[
            "refund_funding_proposer",
            "refund_funding_approver",
            "settlement_adjustment_auditor",
        ],
    )?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_FUNDING_STORAGE_UNAVAILABLE",
        message: "refund funding configuration storage is not configured".to_string(),
    })?;
    Ok(Json(
        store
            .load_refund_funding_configuration(configuration_id, principal.merchant_id)
            .await
            .map_err(ApiError::from)?,
    ))
}

async fn transition_refund_funding_configuration(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(configuration_id): Path<Uuid>,
    payload: Result<Json<RefundFundingConfigurationTransitionRequest>, JsonRejection>,
) -> Result<Json<storage::RefundFundingConfigurationTransitionResult>, ApiError> {
    let started = std::time::Instant::now();
    let metrics = Arc::clone(&state.metrics);
    let result =
        transition_refund_funding_configuration_inner(state, headers, configuration_id, payload)
            .await;
    metrics.observe_or_log(
        "refund_funding_configuration_transition",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn transition_refund_funding_configuration_inner(
    state: AppState,
    headers: HeaderMap,
    configuration_id: Uuid,
    payload: Result<Json<RefundFundingConfigurationTransitionRequest>, JsonRejection>,
) -> Result<Json<storage::RefundFundingConfigurationTransitionResult>, ApiError> {
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let to_status = request.to_status;
    let role: &'static str = match to_status.as_str() {
        "approved" | "suspended" | "retired" => "refund_funding_approver",
        _ => {
            return Err(ApiError {
                status: StatusCode::BAD_REQUEST,
                code: "INVALID_REFUND_FUNDING_TRANSITION",
                message: "unsupported refund funding configuration transition".to_string(),
            });
        }
    };
    let principal = authenticate_scope_role(&state, &headers, "tax:refund:configure", role)?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_FUNDING_STORAGE_UNAVAILABLE",
        message: "refund funding configuration storage is not configured".to_string(),
    })?;
    Ok(Json(
        store
            .transition_refund_funding_configuration(RefundFundingConfigurationTransition {
                merchant_id: principal.merchant_id,
                configuration_id,
                actor_subject: principal.subject,
                to_status,
                evidence: request.evidence,
            })
            .await
            .map_err(ApiError::from)?,
    ))
}

async fn prepare_refund_cancellation(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<RefundCancellationPrepareRequest>, JsonRejection>,
) -> Result<Json<RefundCancellationPreparationResponse>, ApiError> {
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    if !request.confirm {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "REFUND_CANCELLATION_PREPARE_CONFIRMATION_REQUIRED",
            message: "explicit cancellation-preparation confirmation is required".into(),
        });
    }
    if request.idempotency_key.trim().is_empty()
        || request.idempotency_key.len() > 200
        || request.idempotency_key != request.idempotency_key.trim()
    {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "INVALID_IDEMPOTENCY_KEY",
            message: "idempotency key must be a bounded, non-empty trimmed string".into(),
        });
    }
    if request.claim_intent_id.get_version_num() != 7 {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "INVALID_CLAIM_INTENT_ID",
            message: "claim_intent_id must be a UUIDv7".into(),
        });
    }
    let principal = authenticate_scope_role(
        &state,
        &headers,
        "tax:refund:claim:cancel:prepare",
        "settlement_adjustment_governance",
    )?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_CANCELLATION_STORAGE_UNAVAILABLE",
        message: "refund cancellation storage is not configured".into(),
    })?;
    if let Some(intent) = store
        .load_refund_cancellation_intent_by_idempotency(
            principal.merchant_id,
            request.idempotency_key.as_str(),
        )
        .await
        .map_err(ApiError::from)?
    {
        let artifact =
            serde_json::from_value::<RefundCancellationArtifact>(intent.unsigned_artifact.clone())
                .map_err(|_| ApiError {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    code: "REFUND_CANCELLATION_ARTIFACT_CORRUPT",
                    message: "persisted refund cancellation artifact failed integrity validation"
                        .into(),
                })?;
        if artifact.governance_subject != principal.subject {
            return Err(ApiError::from(
                StorageError::RefundCancellationIntentIdempotencyConflict,
            ));
        }
        let finalized_evidence = load_finalized_refund_cancellation_evidence_response(
            &store,
            state.targeted_recovery.as_deref(),
            &intent,
        )
        .await?;
        return Ok(Json(refund_cancellation_response(
            intent,
            artifact,
            finalized_evidence,
        )));
    }

    let claim = store
        .load_refund_claim_intent(request.claim_intent_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::from(StorageError::RefundClaimIntentNotFound))?;
    if claim.status != SettlementStatus::Finalized.as_str() {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CANCELLATION_CLAIM_NOT_FINALIZED",
            message: "cancellation requires finalized refund-claim creation evidence".into(),
        });
    }
    let claim_artifact = serde_json::from_value::<RefundArtifact>(claim.unsigned_artifact.clone())
        .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CLAIM_ARTIFACT_CORRUPT",
            message: "persisted refund claim artifact failed integrity validation".into(),
        })?;
    let adjustment = store
        .load_settlement_adjustment(claim.adjustment_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?;
    let approver_subject = adjustment
        .approver_subject
        .clone()
        .ok_or_else(|| ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CANCELLATION_APPROVER_UNAVAILABLE",
            message: "the approved adjustment has no bound approver authority".into(),
        })?;
    let (governance_binding, proposer_binding, approver_binding) = tokio::try_join!(
        store.load_active_adjustment_authority(
            principal.merchant_id,
            principal.subject.as_str(),
            "governance",
        ),
        store.load_active_adjustment_authority(
            principal.merchant_id,
            claim_artifact.proposer_subject.as_str(),
            "proposer",
        ),
        store.load_active_adjustment_authority(
            principal.merchant_id,
            approver_subject.as_str(),
            "approver",
        ),
    )
    .map_err(ApiError::from)?;
    if governance_binding.subject != principal.subject
        || governance_binding.role != "governance"
        || proposer_binding.subject != claim_artifact.proposer_subject
        || proposer_binding.role != "proposer"
        || approver_binding.subject != approver_subject
        || approver_binding.role != "approver"
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CANCELLATION_AUTHORITY_BINDING_CONFLICT",
            message: "governance, proposer, or approver wallet binding is invalid".into(),
        });
    }
    let merchant = Pubkey::from_str(&claim_artifact.merchant).map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "REFUND_CLAIM_ARTIFACT_CORRUPT",
        message: "refund claim merchant identity is invalid".into(),
    })?;
    let mint = Pubkey::from_str(&claim_artifact.mint).map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "REFUND_CLAIM_ARTIFACT_CORRUPT",
        message: "refund claim mint identity is invalid".into(),
    })?;
    let adjustment_pda =
        Pubkey::from_str(&claim_artifact.instruction.adjustment).map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CLAIM_ARTIFACT_CORRUPT",
            message: "refund claim adjustment identity is invalid".into(),
        })?;
    let coordinator = state
        .settlement_coordinator
        .clone()
        .ok_or_else(|| ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "ADJUSTMENT_REGISTRY_RPC_NOT_CONFIGURED",
            message: "finalized adjustment-registry verification is required".into(),
        })?;
    coordinator
        .verify_adjustment_registry(&AdjustmentRegistryVerificationRequest {
            merchant: merchant.to_string(),
            mint: mint.to_string(),
            proposer: Pubkey::new_from_array(proposer_binding.public_key).to_string(),
            approver: Pubkey::new_from_array(approver_binding.public_key).to_string(),
            governance: Some(Pubkey::new_from_array(governance_binding.public_key).to_string()),
        })
        .await
        .map_err(ApiError::from)?;
    let inputs = RefundCancellationServerInputs {
        merchant_id: principal.merchant_id,
        claim_intent_id: claim.id,
        adjustment_id: claim.adjustment_id,
        merchant: merchant.to_bytes(),
        mint: mint.to_bytes(),
        adjustment: adjustment_pda.to_bytes(),
        claim_nonce: claim.claim_nonce,
        governance_authority: Pubkey::new_from_array(governance_binding.public_key).to_string(),
        governance_subject: principal.subject.clone(),
    };
    let request_digest = cancellation_request_digest(&inputs);
    let artifact = build_cancellation_artifact(&inputs).map_err(|error| ApiError {
        status: StatusCode::UNPROCESSABLE_ENTITY,
        code: "INVALID_REFUND_CANCELLATION_ARTIFACT",
        message: error.to_string(),
    })?;
    let intent = store
        .create_refund_cancellation_intent(CreateRefundCancellationIntentCommand {
            merchant_id: principal.merchant_id,
            idempotency_key: request.idempotency_key,
            request_digest,
            inputs,
            artifact: artifact.clone(),
        })
        .await
        .map_err(ApiError::from)?;
    Ok(Json(refund_cancellation_response(intent, artifact, None)))
}

fn refund_cancellation_response(
    intent: storage::RefundCancellationIntent,
    artifact: RefundCancellationArtifact,
    finalized_evidence: Option<FinalizedRefundCancellationEvidenceResponse>,
) -> RefundCancellationPreparationResponse {
    RefundCancellationPreparationResponse {
        status: intent.status,
        intent_id: intent.id,
        request_digest: intent.request_digest,
        instruction_digest: intent.instruction_digest,
        artifact_digest: intent.artifact_digest,
        claim: Pubkey::new_from_array(intent.claim).to_string(),
        artifact,
        signed_transaction_digest: intent.signed_transaction_digest,
        signature: intent.signature,
        submitted_slot: intent.submitted_slot,
        commitment: intent.commitment,
        last_error_code: intent.last_error_code,
        finalized_evidence,
    }
}

fn finalized_refund_cancellation_evidence_response(
    evidence: FinalizedRefundClaimCancellationEvidence,
) -> FinalizedRefundCancellationEvidenceResponse {
    FinalizedRefundCancellationEvidenceResponse {
        event_type: "AdjustmentRefundClaimCancelled",
        event_version: "v1",
        event_discriminator: indexer::ADJUSTMENT_REFUND_CLAIM_CANCELLED_EVENT_DISCRIMINATOR
            .to_vec(),
        commitment: "finalized",
        signature: evidence.signature,
        slot: evidence.slot,
        event_index: evidence.event_index,
        claim: evidence.claim,
        adjustment: evidence.adjustment,
        merchant: evidence.merchant,
        mint: evidence.mint,
        nonce: evidence.nonce,
        governance_authority: evidence.governance_authority,
        reference_digest: evidence.reference_digest,
        evidence_digest: evidence.evidence_digest,
    }
}

async fn load_finalized_refund_cancellation_evidence_response(
    store: &PgStore,
    ports: Option<&TargetedRecoveryPorts>,
    intent: &storage::RefundCancellationIntent,
) -> Result<Option<FinalizedRefundCancellationEvidenceResponse>, ApiError> {
    if intent.status != SettlementStatus::Finalized.as_str() {
        return Ok(None);
    }
    let ports = ports.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_CANCELLATION_RECONCILIATION_UNAVAILABLE",
        message: "finalized refund cancellation evidence is not configured".into(),
    })?;
    let signature = intent.signature.clone().ok_or_else(|| ApiError {
        status: StatusCode::CONFLICT,
        code: "REFUND_CANCELLATION_FINALIZED_PROOF_INCOMPLETE",
        message: "finalized refund cancellation is missing its server-bound signature".into(),
    })?;
    let expected = refund_cancellation_evidence_expectation(store, intent).await?;
    let evidence = store
        .load_finalized_refund_claim_cancellation_evidence(&ports.stream_key, &signature, &expected)
        .await
        .map_err(ApiError::from)?;
    if evidence.signature != signature || intent.submitted_slot != Some(evidence.slot) {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CANCELLATION_FINALIZED_PROOF_CONFLICT",
            message: "finalized refund cancellation evidence is not bound to the durable intent"
                .into(),
        });
    }
    Ok(Some(finalized_refund_cancellation_evidence_response(
        evidence,
    )))
}

async fn refund_cancellation_evidence_expectation(
    store: &PgStore,
    intent: &storage::RefundCancellationIntent,
) -> Result<RefundClaimCancellationEvidenceExpectation, ApiError> {
    let claim = store
        .load_refund_claim_intent(intent.claim_intent_id, intent.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CLAIM_INTENT_NOT_FOUND",
            message: "the cancellation's bound refund claim is missing".into(),
        })?;
    let claim_artifact = serde_json::from_value::<RefundArtifact>(claim.unsigned_artifact.clone())
        .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CLAIM_ARTIFACT_CORRUPT",
            message: "the cancellation's bound refund claim artifact is invalid".into(),
        })?;
    let reference_digest = storage::refund_claim_evidence_expectation(&claim, &claim_artifact)
        .map_err(|error| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CLAIM_ARTIFACT_CORRUPT",
            message: error.to_string(),
        })?
        .reference_digest;
    let governance_authority =
        Pubkey::from_str(&intent.governance_authority).map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CANCELLATION_ARTIFACT_CORRUPT",
            message: "persisted governance authority is invalid".into(),
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

async fn load_refund_cancellation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(intent_id): Path<Uuid>,
) -> Result<Json<RefundCancellationPreparationResponse>, ApiError> {
    let principal = authenticate_scope_any_role(
        &state,
        &headers,
        "tax:refund:claim:cancel:read",
        &[
            "settlement_adjustment_governance",
            "settlement_adjustment_approver",
            "settlement_adjustment_auditor",
        ],
    )?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_CANCELLATION_STORAGE_UNAVAILABLE",
        message: "refund cancellation storage is not configured".into(),
    })?;
    let intent = store
        .load_refund_cancellation_intent(intent_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::from(StorageError::RefundCancellationIntentNotFound))?;
    let artifact =
        serde_json::from_value::<RefundCancellationArtifact>(intent.unsigned_artifact.clone())
            .map_err(|_| ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "REFUND_CANCELLATION_ARTIFACT_CORRUPT",
                message: "persisted refund cancellation artifact failed integrity validation"
                    .into(),
            })?;
    let finalized_evidence = load_finalized_refund_cancellation_evidence_response(
        &store,
        state.targeted_recovery.as_deref(),
        &intent,
    )
    .await?;
    Ok(Json(refund_cancellation_response(
        intent,
        artifact,
        finalized_evidence,
    )))
}

async fn reconcile_refund_cancellation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(intent_id): Path<Uuid>,
) -> Result<Json<RefundCancellationPreparationResponse>, ApiError> {
    let principal = authenticate_scope_any_role(
        &state,
        &headers,
        "tax:refund:claim:cancel:reconcile",
        &[
            "settlement_adjustment_governance",
            "settlement_adjustment_approver",
            "settlement_adjustment_auditor",
        ],
    )?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_CANCELLATION_STORAGE_UNAVAILABLE",
        message: "refund cancellation storage is not configured".into(),
    })?;
    let intent = store
        .load_refund_cancellation_intent(intent_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::from(StorageError::RefundCancellationIntentNotFound))?;
    let artifact =
        serde_json::from_value::<RefundCancellationArtifact>(intent.unsigned_artifact.clone())
            .map_err(|_| ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "REFUND_CANCELLATION_ARTIFACT_CORRUPT",
                message: "persisted refund cancellation artifact failed integrity validation"
                    .into(),
            })?;
    if intent.status == SettlementStatus::Finalized.as_str() {
        let evidence = load_finalized_refund_cancellation_evidence_response(
            &store,
            state.targeted_recovery.as_deref(),
            &intent,
        )
        .await?;
        return Ok(Json(refund_cancellation_response(
            intent, artifact, evidence,
        )));
    }
    if matches!(
        intent.status.as_str(),
        "prepared_unsubmitted" | "signed" | "rejected"
    ) {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CANCELLATION_NOT_SUBMITTED",
            message: "cancellation reconciliation requires a persisted submitted signature".into(),
        });
    }
    let signature = intent.signature.clone().ok_or_else(|| ApiError {
        status: StatusCode::CONFLICT,
        code: "REFUND_CANCELLATION_SIGNATURE_MISSING",
        message: "cancellation reconciliation requires the signature fenced at submission".into(),
    })?;
    let ports = state.targeted_recovery.as_ref().ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_CANCELLATION_RECONCILIATION_UNAVAILABLE",
        message: "finalized transaction recovery is not configured".into(),
    })?;
    let recovery = recover_targeted_signature(
        store.as_ref(),
        ports.signature_source.as_ref(),
        ports.transaction_source.as_ref(),
        &ports.stream_key,
        &intent.governance_authority,
        &signature,
        ports.max_pages,
    )
    .await
    .map_err(targeted_recovery_api_error)?;
    let expected = refund_cancellation_evidence_expectation(&store, &intent).await?;
    let evidence = store
        .load_finalized_refund_claim_cancellation_evidence(&ports.stream_key, &signature, &expected)
        .await
        .map_err(ApiError::from)?;
    if evidence.signature != signature || evidence.slot != recovery.slot {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CANCELLATION_FINALIZED_EVIDENCE_CONFLICT",
            message: "finalized cancellation event and recovered transaction disagree".into(),
        });
    }
    if !store
        .finalize_refund_claim_cancellation(RefundCancellationFinalization {
            cancellation_intent_id: intent.id,
            claim_intent_id: intent.claim_intent_id,
            merchant_id: principal.merchant_id,
            expected_cancellation_status: intent.status.clone(),
            signature: evidence.signature.clone(),
            slot: evidence.slot,
            evidence: serde_json::json!({
                "source": "finalized_refund_claim_cancellation_event",
                "stream_key": ports.stream_key,
                "event_index": evidence.event_index,
                "event_digest": evidence.evidence_digest,
                "recovery_pages_scanned": recovery.pages_scanned,
            }),
        })
        .await
        .map_err(ApiError::from)?
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CANCELLATION_STATE_CHANGED",
            message: "cancellation changed concurrently; reload before retry".into(),
        });
    }
    let finalized = store
        .load_refund_cancellation_intent(intent.id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::from(StorageError::RefundCancellationIntentNotFound))?;
    let finalized_artifact =
        serde_json::from_value::<RefundCancellationArtifact>(finalized.unsigned_artifact.clone())
            .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CANCELLATION_ARTIFACT_CORRUPT",
            message: "persisted refund cancellation artifact failed integrity validation".into(),
        })?;
    let finalized_evidence = load_finalized_refund_cancellation_evidence_response(
        &store,
        state.targeted_recovery.as_deref(),
        &finalized,
    )
    .await?;
    Ok(Json(refund_cancellation_response(
        finalized,
        finalized_artifact,
        finalized_evidence,
    )))
}

async fn submit_refund_cancellation(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<RefundCancellationSubmitRequest>, JsonRejection>,
) -> Result<Json<RefundCancellationSubmissionResponse>, ApiError> {
    let principal = authenticate_scope_role(
        &state,
        &headers,
        "tax:refund:claim:cancel:submit",
        "settlement_adjustment_governance",
    )?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".into(),
    })?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_CANCELLATION_STORAGE_UNAVAILABLE",
        message: "refund cancellation storage is not configured".into(),
    })?;
    let intent = store
        .load_refund_cancellation_intent(request.intent_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::from(StorageError::RefundCancellationIntentNotFound))?;
    let artifact =
        serde_json::from_value::<RefundCancellationArtifact>(intent.unsigned_artifact.clone())
            .map_err(|_| ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "REFUND_CANCELLATION_ARTIFACT_CORRUPT",
                message: "persisted refund cancellation artifact failed integrity validation"
                    .into(),
            })?;
    if artifact.governance_subject != principal.subject {
        return Err(ApiError::from(
            StorageError::RefundCancellationIntentIdempotencyConflict,
        ));
    }
    if intent.status != SettlementStatus::PreparedUnsubmitted.as_str()
        && intent.status != SettlementStatus::Signed.as_str()
    {
        return Ok(Json(RefundCancellationSubmissionResponse {
            intent_id: intent.id,
            status: intent.status,
            signature: intent.signature,
            provider_index: None,
        }));
    }
    if cancellation_instruction_digest(&artifact.instruction).map_err(|error| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "REFUND_CANCELLATION_ARTIFACT_CORRUPT",
        message: error.to_string(),
    })? != intent.instruction_digest
    {
        return Err(ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CANCELLATION_INSTRUCTION_CORRUPT",
            message: "persisted cancellation instruction does not match its durable digest".into(),
        });
    }
    let coordinator = state.settlement_coordinator.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_RPC_NOT_CONFIGURED",
        message: "settlement RPC providers are not configured".into(),
    })?;
    let transaction =
        SignedTransaction::parse(&request.signed_transaction_base64).map_err(ApiError::from)?;
    let signed_digest = sha256_digest(transaction.bytes());
    transaction
        .verify_refund_cancellation_instruction(&artifact.instruction)
        .map_err(ApiError::from)?;
    let transaction_signature = transaction.signature().map_err(ApiError::from)?;
    coordinator
        .validate_blockhash(&transaction)
        .await
        .map_err(ApiError::from)?;
    if let Some(existing_digest) = intent.signed_transaction_digest
        && existing_digest != signed_digest
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CANCELLATION_SIGNED_BYTES_CONFLICT",
            message: "cancellation intent is already bound to different signed bytes".into(),
        });
    }
    if intent.status == SettlementStatus::PreparedUnsubmitted.as_str()
        && !store
            .transition_refund_cancellation_intent(RefundCancellationIntentTransition {
                intent_id: intent.id,
                merchant_id: principal.merchant_id,
                expected_status: SettlementStatus::PreparedUnsubmitted.as_str().into(),
                next_status: SettlementStatus::Signed.as_str().into(),
                signed_transaction_digest: Some(signed_digest),
                signature: None,
                slot: None,
                last_error_code: None,
                evidence: serde_json::json!({"source": "wallet_handoff"}),
            })
            .await
            .map_err(ApiError::from)?
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CANCELLATION_STATE_CHANGED",
            message: "refund cancellation changed concurrently; reconcile before retry".into(),
        });
    }
    if !store
        .transition_refund_cancellation_intent(RefundCancellationIntentTransition {
            intent_id: intent.id,
            merchant_id: principal.merchant_id,
            expected_status: SettlementStatus::Signed.as_str().into(),
            next_status: SettlementStatus::Submitting.as_str().into(),
            signed_transaction_digest: Some(signed_digest),
            signature: Some(transaction_signature.clone()),
            slot: None,
            last_error_code: None,
            evidence: serde_json::json!({"source": "submission_fence"}),
        })
        .await
        .map_err(ApiError::from)?
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CANCELLATION_SUBMISSION_IN_PROGRESS",
            message: "refund cancellation submission is already owned by another request; reconcile the intent before retry".into(),
        });
    }
    let submitted = match coordinator.submit(&transaction).await {
        Ok(receipt) => receipt,
        Err(error) => {
            let error_code = settlement_submission_error_code(&error);
            let _ = store
                .transition_refund_cancellation_intent(RefundCancellationIntentTransition {
                    intent_id: intent.id,
                    merchant_id: principal.merchant_id,
                    expected_status: SettlementStatus::Submitting.as_str().into(),
                    next_status: SettlementStatus::Unknown.as_str().into(),
                    signed_transaction_digest: Some(signed_digest),
                    signature: Some(transaction_signature),
                    slot: None,
                    last_error_code: Some(error_code.to_string()),
                    evidence: serde_json::json!({"source": "rpc", "error_code": error_code}),
                })
                .await;
            return Err(ApiError {
                status: StatusCode::SERVICE_UNAVAILABLE,
                code: "REFUND_CANCELLATION_SUBMISSION_UNKNOWN",
                message:
                    "provider outcome is unknown; reconcile the cancellation intent before retry"
                        .into(),
            });
        }
    };
    if !store
        .transition_refund_cancellation_intent(RefundCancellationIntentTransition {
            intent_id: intent.id,
            merchant_id: principal.merchant_id,
            expected_status: SettlementStatus::Submitting.as_str().into(),
            next_status: submitted.status.as_str().into(),
            signed_transaction_digest: None,
            signature: Some(submitted.signature.clone()),
            slot: None,
            last_error_code: None,
            evidence: serde_json::json!({"source": "rpc", "provider_index": submitted.provider_index}),
        })
        .await
        .map_err(ApiError::from)?
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CANCELLATION_STATE_CHANGED",
            message: "refund cancellation changed while recording provider acknowledgement".into(),
        });
    }
    Ok(Json(RefundCancellationSubmissionResponse {
        intent_id: intent.id,
        status: submitted.status.as_str().to_string(),
        signature: Some(submitted.signature),
        provider_index: Some(submitted.provider_index),
    }))
}

async fn prepare_refund_claim(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<RefundClaimPrepareRequest>, JsonRejection>,
) -> Result<Json<RefundClaimPreparationResponse>, ApiError> {
    let started = Instant::now();
    let metrics = Arc::clone(&state.metrics);
    let result = prepare_refund_claim_inner(state, headers, payload).await;
    metrics.observe_or_log("refund_claim_prepare", started.elapsed(), result.is_err());
    result
}

async fn prepare_refund_claim_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<RefundClaimPrepareRequest>, JsonRejection>,
) -> Result<Json<RefundClaimPreparationResponse>, ApiError> {
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    if !request.confirm {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "REFUND_CLAIM_PREPARE_CONFIRMATION_REQUIRED",
            message: "explicit claim-preparation confirmation is required".to_string(),
        });
    }
    if request.idempotency_key.trim().is_empty()
        || request.idempotency_key.len() > 200
        || request.idempotency_key != request.idempotency_key.trim()
    {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "INVALID_IDEMPOTENCY_KEY",
            message: "idempotency key must be a bounded, non-empty trimmed string".to_string(),
        });
    }
    if request.adjustment_id.get_version_num() != 7 {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "INVALID_ADJUSTMENT_ID",
            message: "adjustment_id must be a UUIDv7".to_string(),
        });
    }
    let principal = authenticate_scope_role(
        &state,
        &headers,
        "tax:refund:claim:prepare",
        "settlement_adjustment_proposer",
    )?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_CLAIM_STORAGE_UNAVAILABLE",
        message: "refund claim storage is not configured".to_string(),
    })?;

    if let Some(intent) = store
        .load_refund_claim_intent_by_idempotency(
            principal.merchant_id,
            request.idempotency_key.as_str(),
        )
        .await
        .map_err(ApiError::from)?
    {
        let artifact = serde_json::from_value::<RefundArtifact>(intent.unsigned_artifact.clone())
            .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CLAIM_ARTIFACT_CORRUPT",
            message: "persisted refund claim artifact failed integrity validation".into(),
        })?;
        if artifact.proposer_subject != principal.subject {
            return Err(ApiError::from(
                StorageError::RefundClaimIntentIdempotencyConflict,
            ));
        }
        let finalized_evidence = load_finalized_refund_claim_evidence_response(
            &store,
            state.targeted_recovery.as_deref(),
            &intent,
            &artifact,
        )
        .await?;
        return Ok(Json(refund_claim_response(
            intent,
            artifact,
            finalized_evidence,
        )));
    }

    let adjustment = store
        .load_settlement_adjustment(request.adjustment_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?;
    if adjustment.status != "approved"
        || adjustment.kind != AdjustmentKind::Refund
        || adjustment.proposer_subject != principal.subject
        || adjustment.approver_subject.is_none()
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CLAIM_ADJUSTMENT_NOT_READY",
            message: "the authenticated proposer has no approved refund adjustment ready for claim preparation".into(),
        });
    }
    let receipt = store
        .load_finalized_settlement_receipt_binding(
            principal.merchant_id,
            adjustment.original_settlement_id,
            adjustment.asset_id,
        )
        .await
        .map_err(ApiError::from)?;
    let token_program = SPL_TOKEN_PROGRAM_ID.to_string();
    let receipt_mint = Pubkey::new_from_array(receipt.mint).to_string();
    let (funding, proposer_binding, adjustment_binding) = tokio::try_join!(
        store.load_active_refund_funding_configuration(
            principal.merchant_id,
            &receipt_mint,
            token_program.as_str(),
        ),
        store.load_active_adjustment_authority(
            principal.merchant_id,
            &adjustment.proposer_subject,
            "proposer",
        ),
        store.load_finalized_settlement_adjustment_binding(principal.merchant_id, adjustment.id,),
    )
    .map_err(ApiError::from)?;
    let configured_merchant =
        Pubkey::from_str(&funding.merchant_account).map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_FUNDING_CONFIGURATION_CORRUPT",
            message: "stored refund funding merchant account is invalid".into(),
        })?;
    if configured_merchant.to_bytes() != receipt.merchant
        || Pubkey::from_str(&funding.mint)
            .map(|key| key.to_bytes() != receipt.mint)
            .unwrap_or(true)
        || funding.token_program != token_program
        || adjustment_binding.original_receipt != receipt.receipt
        || adjustment_binding.adjustment_digest != adjustment.adjustment_digest
        || adjustment_binding.evidence_digest != adjustment.evidence_digest
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CLAIM_TOPOLOGY_CONFLICT",
            message:
                "finalized receipt, adjustment evidence, and active funding topology do not agree"
                    .into(),
        });
    }
    let funder_subject = funding
        .refund_funder_subject
        .clone()
        .ok_or_else(|| ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_FUNDING_AUTHORITY_UNAVAILABLE",
            message: "active refund funding has no configured funder subject".into(),
        })?;
    let funder_binding = store
        .load_active_adjustment_authority(
            principal.merchant_id,
            funder_subject.as_str(),
            "refund_funder",
        )
        .await
        .map_err(ApiError::from)?;
    let proposer_key = Pubkey::new_from_array(proposer_binding.public_key);
    let funder_key = Pubkey::from_str(&funding.refund_funder).map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "REFUND_FUNDING_CONFIGURATION_CORRUPT",
        message: "stored refund funder account is invalid".into(),
    })?;
    if proposer_binding.subject != adjustment.proposer_subject
        || proposer_binding.role != "proposer"
        || funder_binding.subject != funder_subject
        || funder_binding.role != "refund_funder"
        || funder_binding.public_key != funder_key.to_bytes()
        || proposer_key == funder_key
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CLAIM_AUTHORITY_CONFLICT",
            message: "active proposer and refund-funder wallet bindings are invalid or aliased"
                .into(),
        });
    }
    let (gross_sign, gross_atomic) = signed_magnitude_u64(adjustment.delta.gross_atomic)?;
    let (tax_sign, tax_atomic) = signed_magnitude_u64(adjustment.delta.tax_atomic)?;
    let (net_sign, net_atomic) = signed_magnitude_u64(adjustment.delta.net_atomic)?;
    if gross_sign != 1 || tax_sign != 1 || net_sign != 1 {
        return Err(ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "REFUND_CLAIM_DELTA_NOT_NEGATIVE",
            message: "refund claim preparation requires a negative conserving adjustment delta"
                .into(),
        });
    }
    let now = server_unix_now()?;
    let expires_at_unix = now.checked_add(3_600).ok_or_else(|| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "REFUND_CLAIM_WINDOW_OVERFLOW",
        message: "refund claim expiry window overflowed".into(),
    })?;
    let claim_nonce = store
        .reserve_refund_claim_nonce(principal.merchant_id, adjustment.id)
        .await
        .map_err(ApiError::from)?;
    let inputs = RefundClaimServerInputs {
        merchant_id: principal.merchant_id,
        adjustment_id: adjustment.id,
        funding_configuration_id: funding.id,
        funding_configuration_digest: funding.configuration_digest,
        receipt_signature: receipt.signature,
        receipt_slot: receipt.slot,
        original_receipt: receipt.receipt,
        adjustment: adjustment_binding.adjustment,
        payer: receipt.payer,
        merchant: receipt.merchant,
        mint: receipt.mint,
        token_program,
        proposer: proposer_key.to_string(),
        proposer_subject: principal.subject,
        refund_funder: funder_key.to_string(),
        refund_funder_subject: funder_subject,
        tax_vault: funding.tax_vault,
        net_source: funding.net_source,
        gross_atomic,
        tax_atomic,
        net_atomic,
        reference_digest: adjustment.evidence_digest,
        claim_nonce,
        execute_after_unix: now,
        expires_at_unix,
    };
    let request_digest = claim_request_digest(&inputs);
    let artifact = build_claim_artifact(&inputs).map_err(|error| ApiError {
        status: StatusCode::UNPROCESSABLE_ENTITY,
        code: "INVALID_REFUND_CLAIM_ARTIFACT",
        message: error.to_string(),
    })?;
    let intent = store
        .create_refund_claim_intent(CreateRefundClaimIntentCommand {
            merchant_id: principal.merchant_id,
            idempotency_key: request.idempotency_key,
            request_digest,
            inputs,
            artifact: artifact.clone(),
        })
        .await
        .map_err(ApiError::from)?;
    let finalized_evidence = load_finalized_refund_claim_evidence_response(
        &store,
        state.targeted_recovery.as_deref(),
        &intent,
        &artifact,
    )
    .await?;
    Ok(Json(refund_claim_response(
        intent,
        artifact,
        finalized_evidence,
    )))
}

fn refund_claim_response(
    intent: storage::RefundClaimIntent,
    artifact: RefundArtifact,
    finalized_evidence: Option<FinalizedRefundClaimEvidenceResponse>,
) -> RefundClaimPreparationResponse {
    RefundClaimPreparationResponse {
        status: intent.status.clone(),
        intent_id: intent.id,
        request_digest: intent.request_digest,
        instruction_digest: intent.instruction_digest,
        artifact_digest: intent.artifact_digest,
        claim: Pubkey::new_from_array(intent.claim).to_string(),
        artifact,
        signed_transaction_digest: intent.signed_transaction_digest,
        signature: intent.signature,
        submitted_slot: intent.submitted_slot,
        commitment: intent.commitment,
        last_error_code: intent.last_error_code,
        finalized_evidence,
    }
}

fn finalized_refund_claim_evidence_response(
    evidence: FinalizedRefundClaimEvidence,
) -> FinalizedRefundClaimEvidenceResponse {
    FinalizedRefundClaimEvidenceResponse {
        event_type: "AdjustmentRefundClaimCreated",
        event_version: "v1",
        event_discriminator: indexer::ADJUSTMENT_REFUND_CLAIM_CREATED_EVENT_DISCRIMINATOR.to_vec(),
        commitment: "finalized",
        signature: evidence.signature,
        slot: evidence.slot,
        event_index: evidence.event_index,
        claim: evidence.claim,
        adjustment: evidence.adjustment,
        original_receipt: evidence.original_receipt,
        merchant: evidence.merchant,
        mint: evidence.mint,
        nonce: evidence.nonce,
        gross_magnitude: evidence.gross_magnitude,
        tax_magnitude: evidence.tax_magnitude,
        net_magnitude: evidence.net_magnitude,
        refund_funder: evidence.refund_funder,
        refund_destination: evidence.refund_destination,
        proposer: evidence.proposer,
        reference_digest: evidence.reference_digest,
        execute_after_unix: evidence.execute_after_unix,
        expires_at_unix: evidence.expires_at_unix,
        evidence_digest: evidence.evidence_digest,
    }
}

async fn load_finalized_refund_claim_evidence_response(
    store: &PgStore,
    ports: Option<&TargetedRecoveryPorts>,
    intent: &storage::RefundClaimIntent,
    artifact: &RefundArtifact,
) -> Result<Option<FinalizedRefundClaimEvidenceResponse>, ApiError> {
    if intent.status != SettlementStatus::Finalized.as_str() {
        return Ok(None);
    }
    let ports = ports.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_CLAIM_RECONCILIATION_UNAVAILABLE",
        message: "finalized refund claim evidence is not configured".to_string(),
    })?;
    let signature = intent.signature.clone().ok_or_else(|| ApiError {
        status: StatusCode::CONFLICT,
        code: "REFUND_CLAIM_FINALIZED_PROOF_INCOMPLETE",
        message: "finalized refund claim is missing its server-bound signature".to_string(),
    })?;
    if intent.commitment.as_deref() != Some("finalized") || intent.submitted_slot.is_none() {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CLAIM_FINALIZED_PROOF_INCOMPLETE",
            message: "finalized refund claim is missing its finality binding".to_string(),
        });
    }
    let expected =
        storage::refund_claim_evidence_expectation(intent, artifact).map_err(|error| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CLAIM_ARTIFACT_CORRUPT",
            message: error.to_string(),
        })?;
    let evidence = store
        .load_finalized_refund_claim_evidence(&ports.stream_key, &signature, &expected)
        .await
        .map_err(ApiError::from)?;
    if evidence.signature != signature || Some(evidence.slot) != intent.submitted_slot {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CLAIM_FINALIZED_PROOF_CONFLICT",
            message: "finalized refund claim evidence is not bound to the durable intent".into(),
        });
    }
    Ok(Some(finalized_refund_claim_evidence_response(evidence)))
}

async fn load_refund_claim(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(intent_id): Path<Uuid>,
) -> Result<Json<RefundClaimPreparationResponse>, ApiError> {
    let principal = authenticate_scope_role(
        &state,
        &headers,
        "tax:refund:claim:read",
        "settlement_adjustment_proposer",
    )?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_CLAIM_STORAGE_UNAVAILABLE",
        message: "refund claim storage is not configured".to_string(),
    })?;
    let intent = store
        .load_refund_claim_intent(intent_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::from(StorageError::RefundClaimIntentNotFound))?;
    let artifact = serde_json::from_value::<RefundArtifact>(intent.unsigned_artifact.clone())
        .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CLAIM_ARTIFACT_CORRUPT",
            message: "persisted refund claim artifact failed integrity validation".into(),
        })?;
    if artifact.proposer_subject != principal.subject {
        return Err(ApiError::from(
            StorageError::RefundClaimIntentIdempotencyConflict,
        ));
    }
    let finalized_evidence = load_finalized_refund_claim_evidence_response(
        &store,
        state.targeted_recovery.as_deref(),
        &intent,
        &artifact,
    )
    .await?;
    Ok(Json(refund_claim_response(
        intent,
        artifact,
        finalized_evidence,
    )))
}

async fn reconcile_refund_claim(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(intent_id): Path<Uuid>,
) -> Result<Json<RefundClaimPreparationResponse>, ApiError> {
    let principal = authenticate_scope_role(
        &state,
        &headers,
        "tax:refund:claim:reconcile",
        "settlement_adjustment_proposer",
    )?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_CLAIM_STORAGE_UNAVAILABLE",
        message: "refund claim storage is not configured".to_string(),
    })?;
    let intent = store
        .load_refund_claim_intent(intent_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::from(StorageError::RefundClaimIntentNotFound))?;
    let artifact = serde_json::from_value::<RefundArtifact>(intent.unsigned_artifact.clone())
        .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CLAIM_ARTIFACT_CORRUPT",
            message: "persisted refund claim artifact failed integrity validation".into(),
        })?;
    if artifact.proposer_subject != principal.subject {
        return Err(ApiError::from(
            StorageError::RefundClaimIntentIdempotencyConflict,
        ));
    }
    if intent.status == SettlementStatus::Finalized.as_str() {
        let finalized_evidence = load_finalized_refund_claim_evidence_response(
            &store,
            state.targeted_recovery.as_deref(),
            &intent,
            &artifact,
        )
        .await?;
        return Ok(Json(refund_claim_response(
            intent,
            artifact,
            finalized_evidence,
        )));
    }
    if matches!(
        intent.status.as_str(),
        "prepared_unsubmitted" | "signed" | "rejected" | "cancelled"
    ) {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CLAIM_NOT_SUBMITTED",
            message: "claim reconciliation requires a persisted submitted signature".into(),
        });
    }
    let signature = intent.signature.clone().ok_or_else(|| ApiError {
        status: StatusCode::CONFLICT,
        code: "REFUND_CLAIM_SIGNATURE_MISSING",
        message: "claim reconciliation requires the signature fenced at submission".into(),
    })?;
    let ports = state.targeted_recovery.as_ref().ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_CLAIM_RECONCILIATION_UNAVAILABLE",
        message: "finalized transaction recovery is not configured".to_string(),
    })?;
    let proposer = artifact
        .instruction
        .accounts
        .first()
        .map(|account| account.pubkey.clone())
        .ok_or_else(|| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CLAIM_ARTIFACT_CORRUPT",
            message: "refund claim artifact has no proposer account".into(),
        })?;
    let recovery = recover_targeted_signature(
        store.as_ref(),
        ports.signature_source.as_ref(),
        ports.transaction_source.as_ref(),
        &ports.stream_key,
        &proposer,
        &signature,
        ports.max_pages,
    )
    .await
    .map_err(targeted_recovery_api_error)?;

    let expected =
        storage::refund_claim_evidence_expectation(&intent, &artifact).map_err(|error| {
            ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "REFUND_CLAIM_ARTIFACT_CORRUPT",
                message: error.to_string(),
            }
        })?;
    let evidence = store
        .load_finalized_refund_claim_evidence(&ports.stream_key, &signature, &expected)
        .await
        .map_err(ApiError::from)?;
    if evidence.signature != signature || evidence.slot != recovery.slot {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CLAIM_FINALIZED_EVIDENCE_CONFLICT",
            message: "finalized claim event and recovered transaction disagree".into(),
        });
    }
    if !store
        .transition_refund_claim_intent(RefundClaimIntentTransition {
            intent_id: intent.id,
            merchant_id: principal.merchant_id,
            expected_status: intent.status.clone(),
            next_status: SettlementStatus::Finalized.as_str().into(),
            signed_transaction_digest: intent.signed_transaction_digest,
            signature: Some(evidence.signature.clone()),
            slot: Some(evidence.slot),
            last_error_code: None,
            evidence: serde_json::json!({
                "source": "finalized_refund_claim_event",
                "stream_key": ports.stream_key,
                "event_index": evidence.event_index,
                "event_digest": evidence.evidence_digest,
                "recovery_pages_scanned": recovery.pages_scanned,
            }),
            reconciliation_claim_token: None,
        })
        .await
        .map_err(ApiError::from)?
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CLAIM_STATE_CHANGED",
            message: "claim changed concurrently; reload before retry".into(),
        });
    }
    let finalized = store
        .load_refund_claim_intent(intent.id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::from(StorageError::RefundClaimIntentNotFound))?;
    let finalized_evidence = load_finalized_refund_claim_evidence_response(
        &store,
        state.targeted_recovery.as_deref(),
        &finalized,
        &artifact,
    )
    .await?;
    Ok(Json(refund_claim_response(
        finalized,
        artifact,
        finalized_evidence,
    )))
}

async fn submit_refund_claim(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<RefundClaimSubmitRequest>, JsonRejection>,
) -> Result<Json<RefundClaimSubmissionResponse>, ApiError> {
    let principal = authenticate_scope_role(
        &state,
        &headers,
        "tax:refund:claim:submit",
        "settlement_adjustment_proposer",
    )?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_CLAIM_STORAGE_UNAVAILABLE",
        message: "refund claim storage is not configured".to_string(),
    })?;
    let intent = store
        .load_refund_claim_intent(request.intent_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::from(StorageError::RefundClaimIntentNotFound))?;
    let artifact = serde_json::from_value::<RefundArtifact>(intent.unsigned_artifact.clone())
        .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CLAIM_ARTIFACT_CORRUPT",
            message: "persisted refund claim artifact failed integrity validation".into(),
        })?;
    if artifact.proposer_subject != principal.subject {
        return Err(ApiError::from(
            StorageError::RefundClaimIntentIdempotencyConflict,
        ));
    }
    if intent.status != SettlementStatus::PreparedUnsubmitted.as_str()
        && intent.status != SettlementStatus::Signed.as_str()
    {
        return Ok(Json(RefundClaimSubmissionResponse {
            intent_id: intent.id,
            status: intent.status,
            signature: intent.signature,
            provider_index: None,
        }));
    }
    if artifact.instruction.status != SettlementStatus::PreparedUnsubmitted.as_str()
        || refund_instruction_digest(&artifact.instruction).map_err(|error| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CLAIM_ARTIFACT_CORRUPT",
            message: error.to_string(),
        })? != intent.instruction_digest
    {
        return Err(ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CLAIM_INSTRUCTION_CORRUPT",
            message: "persisted refund claim instruction does not match its durable digest".into(),
        });
    }
    let coordinator = state.settlement_coordinator.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_RPC_NOT_CONFIGURED",
        message: "settlement RPC providers are not configured".into(),
    })?;
    let transaction =
        SignedTransaction::parse(&request.signed_transaction_base64).map_err(ApiError::from)?;
    let signed_digest = sha256_digest(transaction.bytes());
    transaction
        .verify_refund_instruction(&artifact.instruction)
        .map_err(ApiError::from)?;
    let transaction_signature = transaction.signature().map_err(ApiError::from)?;
    coordinator
        .validate_blockhash(&transaction)
        .await
        .map_err(ApiError::from)?;
    if let Some(existing_digest) = intent.signed_transaction_digest
        && existing_digest != signed_digest
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CLAIM_SIGNED_BYTES_CONFLICT",
            message: "claim intent is already bound to different signed bytes".into(),
        });
    }
    if intent.status == SettlementStatus::PreparedUnsubmitted.as_str()
        && !store
            .transition_refund_claim_intent(RefundClaimIntentTransition {
                intent_id: intent.id,
                merchant_id: principal.merchant_id,
                expected_status: SettlementStatus::PreparedUnsubmitted.as_str().into(),
                next_status: SettlementStatus::Signed.as_str().into(),
                signed_transaction_digest: Some(signed_digest),
                signature: None,
                slot: None,
                last_error_code: None,
                evidence: serde_json::json!({"source": "wallet_handoff"}),
                reconciliation_claim_token: None,
            })
            .await
            .map_err(ApiError::from)?
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CLAIM_STATE_CHANGED",
            message: "refund claim changed concurrently; reconcile before retry".into(),
        });
    }
    if !store
        .transition_refund_claim_intent(RefundClaimIntentTransition {
            intent_id: intent.id,
            merchant_id: principal.merchant_id,
            expected_status: SettlementStatus::Signed.as_str().into(),
            next_status: SettlementStatus::Submitting.as_str().into(),
            signed_transaction_digest: Some(signed_digest),
            signature: Some(transaction_signature.clone()),
            slot: None,
            last_error_code: None,
            evidence: serde_json::json!({"source": "submission_fence"}),
            reconciliation_claim_token: None,
        })
        .await
        .map_err(ApiError::from)?
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CLAIM_SUBMISSION_IN_PROGRESS",
            message: "refund claim submission is already owned by another request; reconcile the intent before retry".into(),
        });
    }
    let submitted = match coordinator.submit(&transaction).await {
        Ok(receipt) => receipt,
        Err(error) => {
            let error_code = settlement_submission_error_code(&error);
            let _ = store
                .transition_refund_claim_intent(RefundClaimIntentTransition {
                    intent_id: intent.id,
                    merchant_id: principal.merchant_id,
                    expected_status: SettlementStatus::Submitting.as_str().into(),
                    next_status: SettlementStatus::Unknown.as_str().into(),
                    signed_transaction_digest: Some(signed_digest),
                    signature: Some(transaction_signature),
                    slot: None,
                    last_error_code: Some(error_code.to_string()),
                    evidence: serde_json::json!({"source": "rpc", "error_code": error_code}),
                    reconciliation_claim_token: None,
                })
                .await;
            return Err(ApiError {
                status: StatusCode::SERVICE_UNAVAILABLE,
                code: "REFUND_CLAIM_SUBMISSION_UNKNOWN",
                message:
                    "provider outcome is unknown; reconcile the refund claim intent before retry"
                        .into(),
            });
        }
    };
    if !store
        .transition_refund_claim_intent(RefundClaimIntentTransition {
            intent_id: intent.id,
            merchant_id: principal.merchant_id,
            expected_status: SettlementStatus::Submitting.as_str().into(),
            next_status: submitted.status.as_str().into(),
            signed_transaction_digest: None,
            signature: Some(submitted.signature.clone()),
            slot: None,
            last_error_code: None,
            evidence: serde_json::json!({
                "source": "rpc",
                "provider_index": submitted.provider_index,
            }),
            reconciliation_claim_token: None,
        })
        .await
        .map_err(ApiError::from)?
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_CLAIM_STATE_CHANGED",
            message: "refund claim changed while recording provider acknowledgement".into(),
        });
    }
    Ok(Json(RefundClaimSubmissionResponse {
        intent_id: intent.id,
        status: submitted.status.as_str().to_string(),
        signature: Some(submitted.signature),
        provider_index: Some(submitted.provider_index),
    }))
}

async fn prepare_refund_execute(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<RefundExecutePrepareRequest>, JsonRejection>,
) -> Result<Json<RefundExecutePreparationResponse>, ApiError> {
    prepare_refund_execute_inner(state, headers, payload).await
}

async fn prepare_refund_execute_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<RefundExecutePrepareRequest>, JsonRejection>,
) -> Result<Json<RefundExecutePreparationResponse>, ApiError> {
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    if !request.confirm {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "REFUND_EXECUTE_PREPARE_CONFIRMATION_REQUIRED",
            message: "explicit execute-preparation confirmation is required".to_string(),
        });
    }
    if request.idempotency_key.trim().is_empty()
        || request.idempotency_key.len() > 200
        || request.idempotency_key != request.idempotency_key.trim()
    {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "INVALID_IDEMPOTENCY_KEY",
            message: "idempotency key must be a bounded, non-empty trimmed string".to_string(),
        });
    }
    if request.claim_intent_id.get_version_num() != 7 {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "INVALID_CLAIM_INTENT_ID",
            message: "claim_intent_id must be a UUIDv7".to_string(),
        });
    }
    let principal = authenticate_scope_role(
        &state,
        &headers,
        "tax:refund:execute:prepare",
        "settlement_adjustment_approver",
    )?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_EXECUTE_STORAGE_UNAVAILABLE",
        message: "refund execute storage is not configured".to_string(),
    })?;
    if let Some(intent) = store
        .load_refund_execute_intent_by_idempotency(
            principal.merchant_id,
            request.idempotency_key.as_str(),
        )
        .await
        .map_err(ApiError::from)?
    {
        let artifact = serde_json::from_value::<RefundArtifact>(intent.unsigned_artifact.clone())
            .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_EXECUTE_ARTIFACT_CORRUPT",
            message: "persisted refund execute artifact failed integrity validation".into(),
        })?;
        validate_refund_execute_replay_authority(store.as_ref(), &intent, &artifact, &principal)
            .await?;
        return Ok(Json(refund_execute_response(intent, artifact)));
    }

    let claim = store
        .load_refund_claim_intent(request.claim_intent_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::from(StorageError::RefundClaimIntentNotFound))?;
    if claim.status != SettlementStatus::Finalized.as_str()
        || claim.signature.is_none()
        || claim.submitted_slot.is_none()
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_EXECUTE_CLAIM_NOT_FINALIZED",
            message: "execute preparation requires finalized claim transaction evidence".into(),
        });
    }
    let claim_artifact = serde_json::from_value::<RefundArtifact>(claim.unsigned_artifact.clone())
        .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CLAIM_ARTIFACT_CORRUPT",
            message: "persisted refund claim artifact failed integrity validation".into(),
        })?;
    if claim_artifact.instruction.claim != Pubkey::new_from_array(claim.claim).to_string()
        || claim_artifact.instruction.claim_nonce != claim.claim_nonce
    {
        return Err(ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_CLAIM_ARTIFACT_CORRUPT",
            message: "finalized claim artifact does not match durable claim identity".into(),
        });
    }
    let adjustment = store
        .load_settlement_adjustment(claim.adjustment_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?;
    if adjustment.status != "approved"
        || adjustment.kind != AdjustmentKind::Refund
        || adjustment.approver_subject.as_deref() != Some(principal.subject.as_str())
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_EXECUTE_ADJUSTMENT_NOT_READY",
            message:
                "the authenticated approver has no approved refund adjustment ready for execution"
                    .into(),
        });
    }
    let receipt = store
        .load_finalized_settlement_receipt_binding(
            principal.merchant_id,
            adjustment.original_settlement_id,
            adjustment.asset_id,
        )
        .await
        .map_err(ApiError::from)?;
    let token_program = SPL_TOKEN_PROGRAM_ID.to_string();
    let receipt_mint = Pubkey::new_from_array(receipt.mint).to_string();
    let (funding, approver_binding, adjustment_binding) = tokio::try_join!(
        store.load_active_refund_funding_configuration(
            principal.merchant_id,
            &receipt_mint,
            token_program.as_str(),
        ),
        store.load_active_adjustment_authority(
            principal.merchant_id,
            principal.subject.as_str(),
            "approver",
        ),
        store.load_finalized_settlement_adjustment_binding(principal.merchant_id, adjustment.id,),
    )
    .map_err(ApiError::from)?;
    let configured_merchant =
        Pubkey::from_str(&funding.merchant_account).map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_FUNDING_CONFIGURATION_CORRUPT",
            message: "stored refund funding merchant account is invalid".into(),
        })?;
    if configured_merchant.to_bytes() != receipt.merchant
        || Pubkey::from_str(&funding.mint)
            .map(|key| key.to_bytes() != receipt.mint)
            .unwrap_or(true)
        || funding.token_program != token_program
        || adjustment_binding.original_receipt != receipt.receipt
        || adjustment_binding.adjustment_digest != adjustment.adjustment_digest
        || adjustment_binding.evidence_digest != adjustment.evidence_digest
        || approver_binding.subject != principal.subject
        || approver_binding.role != "approver"
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_EXECUTE_TOPOLOGY_CONFLICT",
            message: "finalized receipt, adjustment evidence, authority binding, and active funding topology do not agree".into(),
        });
    }
    let funder_subject = funding
        .refund_funder_subject
        .clone()
        .ok_or_else(|| ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_FUNDING_AUTHORITY_UNAVAILABLE",
            message: "active refund funding has no configured funder subject".into(),
        })?;
    let funder_binding = store
        .load_active_adjustment_authority(
            principal.merchant_id,
            funder_subject.as_str(),
            "refund_funder",
        )
        .await
        .map_err(ApiError::from)?;
    let approver_key = Pubkey::new_from_array(approver_binding.public_key);
    let funder_key = Pubkey::from_str(&funding.refund_funder).map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "REFUND_FUNDING_CONFIGURATION_CORRUPT",
        message: "stored refund funder account is invalid".into(),
    })?;
    if approver_key == funder_key
        || funder_binding.subject != funder_subject
        || funder_binding.role != "refund_funder"
        || funder_binding.public_key != funder_key.to_bytes()
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_EXECUTE_AUTHORITY_CONFLICT",
            message: "active approver and refund-funder wallet bindings are invalid or aliased"
                .into(),
        });
    }
    let claim_signature = claim.signature.ok_or_else(|| ApiError {
        status: StatusCode::CONFLICT,
        code: "REFUND_EXECUTE_CLAIM_NOT_FINALIZED",
        message: "finalized claim signature evidence is missing".into(),
    })?;
    let claim_slot = claim.submitted_slot.ok_or_else(|| ApiError {
        status: StatusCode::CONFLICT,
        code: "REFUND_EXECUTE_CLAIM_NOT_FINALIZED",
        message: "finalized claim slot evidence is missing".into(),
    })?;
    let inputs = RefundExecuteServerInputs {
        merchant_id: principal.merchant_id,
        claim_intent_id: claim.id,
        adjustment_id: adjustment.id,
        funding_configuration_id: funding.id,
        funding_configuration_digest: funding.configuration_digest,
        receipt_signature: receipt.signature,
        receipt_slot: receipt.slot,
        claim_signature,
        claim_slot,
        original_receipt: receipt.receipt,
        adjustment: adjustment_binding.adjustment,
        payer: receipt.payer,
        merchant: receipt.merchant,
        mint: receipt.mint,
        token_program,
        approver: approver_key.to_string(),
        refund_funder: funder_key.to_string(),
        refund_funder_subject: funder_subject,
        tax_vault: funding.tax_vault,
        net_source: funding.net_source,
        claim_nonce: claim.claim_nonce,
    };
    let request_digest = execute_request_digest(&inputs);
    let artifact = build_execute_artifact(&inputs).map_err(|error| ApiError {
        status: StatusCode::UNPROCESSABLE_ENTITY,
        code: "INVALID_REFUND_EXECUTE_ARTIFACT",
        message: error.to_string(),
    })?;
    if artifact.instruction.claim != Pubkey::new_from_array(claim.claim).to_string() {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_EXECUTE_CLAIM_TOPOLOGY_CONFLICT",
            message: "compiled execute claim PDA does not match finalized claim evidence".into(),
        });
    }
    let intent = store
        .create_refund_execute_intent(CreateRefundExecuteIntentCommand {
            merchant_id: principal.merchant_id,
            idempotency_key: request.idempotency_key,
            request_digest,
            claim_intent_id: claim.id,
            inputs,
            artifact: artifact.clone(),
        })
        .await
        .map_err(ApiError::from)?;
    Ok(Json(refund_execute_response(intent, artifact)))
}

fn refund_execute_response(
    intent: storage::RefundExecuteIntent,
    artifact: RefundArtifact,
) -> RefundExecutePreparationResponse {
    RefundExecutePreparationResponse {
        status: intent.status.clone(),
        intent_id: intent.id,
        request_digest: intent.request_digest,
        instruction_digest: intent.instruction_digest,
        artifact_digest: intent.artifact_digest,
        claim: Pubkey::new_from_array(intent.claim).to_string(),
        artifact,
        signed_transaction_digest: intent.signed_transaction_digest,
        signature: intent.signature,
        submitted_slot: intent.submitted_slot,
        commitment: intent.commitment,
        last_error_code: intent.last_error_code,
    }
}

async fn validate_refund_execute_replay_authority(
    store: &PgStore,
    intent: &storage::RefundExecuteIntent,
    artifact: &RefundArtifact,
    principal: &auth::AuthenticatedPrincipal,
) -> Result<(), ApiError> {
    if artifact.authority_subject != intent.approver {
        return Err(ApiError::from(
            StorageError::RefundExecuteIntentIdempotencyConflict,
        ));
    }
    let adjustment = store
        .load_settlement_adjustment(intent.adjustment_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?;
    if adjustment.approver_subject.as_deref() != Some(principal.subject.as_str()) {
        return Err(ApiError::from(
            StorageError::RefundExecuteIntentIdempotencyConflict,
        ));
    }
    Ok(())
}

async fn load_refund_execute(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(intent_id): Path<Uuid>,
) -> Result<Json<RefundExecutePreparationResponse>, ApiError> {
    let principal = authenticate_scope_role(
        &state,
        &headers,
        "tax:refund:execute:read",
        "settlement_adjustment_approver",
    )?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_EXECUTE_STORAGE_UNAVAILABLE",
        message: "refund execute storage is not configured".to_string(),
    })?;
    let intent = store
        .load_refund_execute_intent(intent_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::from(StorageError::RefundExecuteIntentNotFound))?;
    let artifact = serde_json::from_value::<RefundArtifact>(intent.unsigned_artifact.clone())
        .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_EXECUTE_ARTIFACT_CORRUPT",
            message: "persisted refund execute artifact failed integrity validation".into(),
        })?;
    validate_refund_execute_replay_authority(store.as_ref(), &intent, &artifact, &principal)
        .await?;
    Ok(Json(refund_execute_response(intent, artifact)))
}

async fn submit_refund_execute(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<RefundExecuteSubmitRequest>, JsonRejection>,
) -> Result<Json<RefundExecuteSubmissionResponse>, ApiError> {
    let principal = authenticate_scope_role(
        &state,
        &headers,
        "tax:refund:execute:submit",
        "settlement_adjustment_approver",
    )?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "REFUND_EXECUTE_STORAGE_UNAVAILABLE",
        message: "refund execute storage is not configured".to_string(),
    })?;
    let intent = store
        .load_refund_execute_intent(request.intent_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::from(StorageError::RefundExecuteIntentNotFound))?;
    let artifact = serde_json::from_value::<RefundArtifact>(intent.unsigned_artifact.clone())
        .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_EXECUTE_ARTIFACT_CORRUPT",
            message: "persisted refund execute artifact failed integrity validation".into(),
        })?;
    validate_refund_execute_replay_authority(store.as_ref(), &intent, &artifact, &principal)
        .await?;
    if intent.status != SettlementStatus::PreparedUnsubmitted.as_str()
        && intent.status != SettlementStatus::Signed.as_str()
    {
        return Ok(Json(RefundExecuteSubmissionResponse {
            intent_id: intent.id,
            status: intent.status,
            signature: intent.signature,
            provider_index: None,
        }));
    }
    if artifact.instruction.status != SettlementStatus::PreparedUnsubmitted.as_str()
        || refund_instruction_digest(&artifact.instruction).map_err(|error| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_EXECUTE_ARTIFACT_CORRUPT",
            message: error.to_string(),
        })? != intent.instruction_digest
    {
        return Err(ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "REFUND_EXECUTE_INSTRUCTION_CORRUPT",
            message: "persisted refund execute instruction does not match its durable digest"
                .into(),
        });
    }
    let coordinator = state.settlement_coordinator.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_RPC_NOT_CONFIGURED",
        message: "settlement RPC providers are not configured".into(),
    })?;
    let transaction =
        SignedTransaction::parse(&request.signed_transaction_base64).map_err(ApiError::from)?;
    let signed_digest = sha256_digest(transaction.bytes());
    transaction
        .verify_refund_instruction(&artifact.instruction)
        .map_err(ApiError::from)?;
    let transaction_signature = transaction.signature().map_err(ApiError::from)?;
    coordinator
        .validate_blockhash(&transaction)
        .await
        .map_err(ApiError::from)?;
    if let Some(existing_digest) = intent.signed_transaction_digest
        && existing_digest != signed_digest
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_EXECUTE_SIGNED_BYTES_CONFLICT",
            message: "execute intent is already bound to different signed bytes".into(),
        });
    }
    if intent.status == SettlementStatus::PreparedUnsubmitted.as_str()
        && !store
            .transition_refund_execute_intent(RefundExecuteIntentTransition {
                intent_id: intent.id,
                merchant_id: principal.merchant_id,
                expected_status: SettlementStatus::PreparedUnsubmitted.as_str().into(),
                next_status: SettlementStatus::Signed.as_str().into(),
                signed_transaction_digest: Some(signed_digest),
                signature: None,
                slot: None,
                last_error_code: None,
                evidence: serde_json::json!({"source": "wallet_handoff"}),
            })
            .await
            .map_err(ApiError::from)?
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_EXECUTE_STATE_CHANGED",
            message: "refund execute changed concurrently; reconcile before retry".into(),
        });
    }
    if !store
        .transition_refund_execute_intent(RefundExecuteIntentTransition {
            intent_id: intent.id,
            merchant_id: principal.merchant_id,
            expected_status: SettlementStatus::Signed.as_str().into(),
            next_status: SettlementStatus::Submitting.as_str().into(),
            signed_transaction_digest: Some(signed_digest),
            signature: Some(transaction_signature.clone()),
            slot: None,
            last_error_code: None,
            evidence: serde_json::json!({"source": "submission_fence"}),
        })
        .await
        .map_err(ApiError::from)?
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_EXECUTE_SUBMISSION_IN_PROGRESS",
            message: "refund execute submission is already owned by another request; reconcile the intent before retry".into(),
        });
    }
    let submitted = match coordinator.submit(&transaction).await {
        Ok(receipt) => receipt,
        Err(error) => {
            let error_code = settlement_submission_error_code(&error);
            let _ = store
                .transition_refund_execute_intent(RefundExecuteIntentTransition {
                    intent_id: intent.id,
                    merchant_id: principal.merchant_id,
                    expected_status: SettlementStatus::Submitting.as_str().into(),
                    next_status: SettlementStatus::Unknown.as_str().into(),
                    signed_transaction_digest: Some(signed_digest),
                    signature: Some(transaction_signature),
                    slot: None,
                    last_error_code: Some(error_code.to_string()),
                    evidence: serde_json::json!({"source": "rpc", "error_code": error_code}),
                })
                .await;
            return Err(ApiError {
                status: StatusCode::SERVICE_UNAVAILABLE,
                code: "REFUND_EXECUTE_SUBMISSION_UNKNOWN",
                message: "provider outcome is unknown; reconcile the execute intent before retry"
                    .into(),
            });
        }
    };
    if !store
        .transition_refund_execute_intent(RefundExecuteIntentTransition {
            intent_id: intent.id,
            merchant_id: principal.merchant_id,
            expected_status: SettlementStatus::Submitting.as_str().into(),
            next_status: submitted.status.as_str().into(),
            signed_transaction_digest: None,
            signature: Some(submitted.signature.clone()),
            slot: None,
            last_error_code: None,
            evidence: serde_json::json!({
                "source": "rpc",
                "provider_index": submitted.provider_index,
            }),
        })
        .await
        .map_err(ApiError::from)?
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "REFUND_EXECUTE_STATE_CHANGED",
            message: "refund execute changed while recording provider acknowledgement".into(),
        });
    }
    Ok(Json(RefundExecuteSubmissionResponse {
        intent_id: intent.id,
        status: submitted.status.as_str().to_string(),
        signature: Some(submitted.signature),
        provider_index: Some(submitted.provider_index),
    }))
}

struct CanonicalRefundFundingPublicKeys {
    merchant_account: String,
    mint: String,
    tax_vault: String,
    vault_authority: String,
    refund_funder: String,
    net_source: String,
    token_program: String,
}

fn canonicalize_refund_funding_public_keys(
    request: &RefundFundingConfigurationCreateRequest,
) -> Result<CanonicalRefundFundingPublicKeys, ApiError> {
    let parse = |field: &'static str, value: &str| {
        Pubkey::from_str(value)
            .map(|key| key.to_string())
            .map_err(|_| ApiError {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "INVALID_REFUND_FUNDING_ACCOUNT",
                message: format!("{field} is not a valid Solana public key"),
            })
    };
    let result = CanonicalRefundFundingPublicKeys {
        merchant_account: parse("merchant_account", &request.merchant_account)?,
        mint: parse("mint", &request.mint)?,
        tax_vault: parse("tax_vault", &request.tax_vault)?,
        vault_authority: parse("vault_authority", &request.vault_authority)?,
        refund_funder: parse("refund_funder", &request.refund_funder)?,
        net_source: parse("net_source", &request.net_source)?,
        token_program: parse("token_program", &request.token_program)?,
    };
    let accounts = [
        result.mint.as_str(),
        result.tax_vault.as_str(),
        result.vault_authority.as_str(),
        result.refund_funder.as_str(),
        result.net_source.as_str(),
    ];
    for (index, account) in accounts.iter().enumerate() {
        if accounts[..index].contains(account) {
            return Err(ApiError {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "INVALID_REFUND_FUNDING_ACCOUNT",
                message: "funding accounts and mint must be pairwise distinct".to_string(),
            });
        }
    }
    Ok(result)
}

async fn list_settlement_split_plans(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<storage::SettlementSplitPlan>>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:settlement:compile")?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_STORAGE_UNAVAILABLE",
        message: "settlement Split Plan storage is not configured".to_string(),
    })?;
    Ok(Json(
        store
            .list_active_settlement_split_plans(principal.merchant_id)
            .await
            .map_err(ApiError::from)?,
    ))
}

async fn compile_settlement_from_quote(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<SettlementCompileFromQuoteRequest>, JsonRejection>,
) -> Result<Json<CompiledSettlementResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = compile_settlement_from_quote_inner(state, headers, payload).await;
    metrics.observe_or_log("settlement_compile", started.elapsed(), result.is_err());
    result
}

async fn compile_settlement_from_quote_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<SettlementCompileFromQuoteRequest>, JsonRejection>,
) -> Result<Json<CompiledSettlementResponse>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:settlement:compile")?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let store = state.storage.clone().ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "QUOTE_STORAGE_UNAVAILABLE",
        message: "quote storage is not configured".to_string(),
    })?;
    let quote = store
        .load_quote_for_settlement(principal.merchant_id, request.calculation_id)
        .await
        .map_err(ApiError::from)?;
    if !state.simulation_mode {
        require_quote_attestation(
            &store,
            principal.merchant_id,
            request.calculation_id,
            request.policy_digest,
        )
        .await?;
    }
    let route = store
        .load_active_settlement_route(principal.merchant_id, request.route_id)
        .await
        .map_err(ApiError::from)?;
    let source = derive_source_token_account(&request.payer, &route.mint, &route.token_program)
        .map_err(ApiError::from)?;
    compile_settlement_request(
        state,
        principal.merchant_id,
        SettlementCompileRequest {
            program_id: SETTLEMENT_PROGRAM_ID.to_string(),
            payer: request.payer,
            merchant: route.merchant_account,
            source,
            mint: route.mint,
            tax_vault: route.tax_vault,
            net_destination: route.net_destination,
            token_program: route.token_program,
            nonce: request.nonce,
            policy_sequence: request.policy_sequence,
            gross_atomic: quote.result.gross_atomic.to_string(),
            tax_atomic: quote.result.total_tax_atomic.to_string(),
            net_atomic: quote.result.total_net_atomic.to_string(),
            policy_digest: request.policy_digest,
        },
        Some(request.route_id),
    )
    .await
}

async fn compile_document_settlement_from_quote(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<DocumentSettlementCompileFromQuoteRequest>, JsonRejection>,
) -> Result<Json<CompiledDocumentSettlementResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = compile_document_settlement_from_quote_inner(state, headers, payload).await;
    metrics.observe_or_log(
        "settlement_document_compile",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn compile_document_settlement_from_quote_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<DocumentSettlementCompileFromQuoteRequest>, JsonRejection>,
) -> Result<Json<CompiledDocumentSettlementResponse>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:settlement:compile")?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let store = state.storage.clone().ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "QUOTE_STORAGE_UNAVAILABLE",
        message: "quote storage is not configured".to_string(),
    })?;
    let (document, attestation, route) = tokio::try_join!(
        store.load_document_quote_for_attestation(principal.merchant_id, request.calculation_id),
        store.load_document_calculation_attestation(principal.merchant_id, request.calculation_id),
        store.load_active_settlement_route(principal.merchant_id, request.route_id),
    )
    .map_err(ApiError::from)?;
    if document.schema != TAX_DOCUMENT_SCHEMA_V1 {
        return Err(ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "DOCUMENT_QUOTE_SCHEMA_CORRUPT",
            message: "stored document quote schema is not supported".to_string(),
        });
    }
    let asset_mint = Pubkey::from_str(&route.mint).map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "SETTLEMENT_ROUTE_EVIDENCE_CORRUPT",
        message: "stored route mint is not a valid Solana public key".to_string(),
    })?;
    let binding = DocumentAttestationBinding {
        document_schema: &document.schema,
        merchant_id: merchant_identity_digest(principal.merchant_id.as_bytes()),
        document_id: &document.result.document_id,
        engine_version: &document.result.engine_version,
        rule_set_id: &document.result.rule_set_id,
        rule_version: document.result.rule_version,
        jurisdiction: &document.result.jurisdiction,
        currency: &document.result.currency,
        asset_mint: asset_mint.to_bytes(),
        facts_digest: document.result.facts_digest,
        rule_digest: document.result.rule_digest,
        gross_atomic: quote_amount_as_u64(document.result.gross_atomic, "gross_atomic")?,
        tax_atomic: quote_amount_as_u64(document.result.total_tax_atomic, "total_tax_atomic")?,
        net_atomic: quote_amount_as_u64(document.result.total_net_atomic, "total_net_atomic")?,
    };
    attestation
        .attestation
        .verify_at(server_unix_now()?)
        .map_err(ApiError::from)?;
    attestation
        .attestation
        .verify_binding(&binding)
        .map_err(ApiError::from)?;
    let serialized_request = serde_json::to_vec(&request).map_err(|error| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "SETTLEMENT_DIGEST_FAILURE",
        message: error.to_string(),
    })?;
    let authorization = authorize_document_payment(
        &document.result,
        attestation.payload_digest,
        request.scope.clone(),
    )
    .map_err(ApiError::from)?;
    let source = derive_source_token_account(&request.payer, &route.mint, &route.token_program)
        .map_err(ApiError::from)?;
    let client_request = ClientDocumentSettlementCompileRequest {
        program_id: SETTLEMENT_PROGRAM_ID.to_string(),
        payer: request.payer,
        merchant: route.merchant_account,
        source,
        mint: route.mint.clone(),
        tax_vault: route.tax_vault,
        net_destination: route.net_destination,
        token_program: route.token_program,
        nonce: request.nonce,
        policy_sequence: request.policy_sequence,
        gross_atomic: authorization.gross_atomic.to_string(),
        tax_atomic: authorization.tax_atomic.to_string(),
        net_atomic: authorization.net_atomic.to_string(),
        policy_digest: request.policy_digest,
        document_attestation_digest: authorization.source_attestation_digest,
        payment_scope_digest: authorization.payment_scope_digest,
    };
    let mut digest_input = b"solana-tax-protocol/document-settlement-request/v1".to_vec();
    digest_input.extend_from_slice(request.route_id.as_bytes());
    digest_input.extend_from_slice(&serialized_request);
    let request_digest = sha256_digest(&digest_input);
    let instruction =
        compile_document_settlement_instruction(&client_request).map_err(ApiError::from)?;
    let instruction_digest =
        sha256_digest(&serde_json::to_vec(&instruction).map_err(|error| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "SETTLEMENT_DIGEST_FAILURE",
            message: error.to_string(),
        })?);
    let outcome = store
        .create_document_payment_intent(CreateDocumentPaymentIntentCommand {
            merchant_id: principal.merchant_id,
            idempotency_key: request.idempotency_key,
            request_digest,
            instruction_digest,
            calculation_id: request.calculation_id,
            route_id: request.route_id,
            mint: client_request.mint,
            authorization,
        })
        .await
        .map_err(ApiError::from)?;
    let document_intent = match outcome {
        storage::DocumentPaymentIntentOutcome::Created(value)
        | storage::DocumentPaymentIntentOutcome::Replay(value) => value,
    };
    Ok(Json(CompiledDocumentSettlementResponse {
        instruction,
        intent_id: Some(document_intent.intent.id),
        request_digest,
        instruction_digest,
        route_id: request.route_id,
        authorization: document_intent.authorization,
    }))
}

async fn compile_split_settlement_from_quote(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<SplitSettlementCompileFromQuoteRequest>, JsonRejection>,
) -> Result<Json<CompiledSplitSettlementResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = compile_split_settlement_from_quote_inner(state, headers, payload).await;
    metrics.observe_or_log(
        "settlement_split_compile",
        started.elapsed(),
        result.is_err(),
    );
    result
}

async fn compile_split_settlement_from_quote_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<SplitSettlementCompileFromQuoteRequest>, JsonRejection>,
) -> Result<Json<CompiledSplitSettlementResponse>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:settlement:compile")?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let store = state.storage.clone().ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "QUOTE_STORAGE_UNAVAILABLE",
        message: "quote storage is not configured".to_string(),
    })?;
    let (quote, route, plan) = tokio::try_join!(
        store.load_quote_for_settlement(principal.merchant_id, request.calculation_id),
        store.load_active_settlement_route(principal.merchant_id, request.route_id),
        store.load_active_settlement_split_plan(principal.merchant_id, request.split_plan_id),
    )
    .map_err(ApiError::from)?;
    if !state.simulation_mode {
        require_quote_attestation(
            &store,
            principal.merchant_id,
            request.calculation_id,
            request.policy_digest,
        )
        .await?;
    }
    if plan.merchant_account != route.merchant_account || plan.mint != route.mint {
        return Err(ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "SETTLEMENT_SPLIT_PLAN_ROUTE_MISMATCH",
            message: "the split plan is not scoped to the selected settlement route".to_string(),
        });
    }
    let source = derive_source_token_account(&request.payer, &route.mint, &route.token_program)
        .map_err(ApiError::from)?;
    compile_split_settlement_request(
        state,
        principal.merchant_id,
        request.route_id,
        request.split_plan_id,
        SplitSettlementCompileRequest {
            program_id: SETTLEMENT_PROGRAM_ID.to_string(),
            payer: request.payer,
            merchant: plan.merchant_account,
            source,
            mint: plan.mint,
            tax_vault: route.tax_vault,
            token_program: route.token_program,
            nonce: request.nonce,
            policy_sequence: request.policy_sequence,
            split_plan_version: u64::try_from(plan.version).map_err(|_| ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "SETTLEMENT_SPLIT_PLAN_EVIDENCE_CORRUPT",
                message: "stored split plan version is invalid".to_string(),
            })?,
            recipients: plan.recipients,
            gross_atomic: quote.result.gross_atomic.to_string(),
            tax_atomic: quote.result.total_tax_atomic.to_string(),
            net_atomic: quote.result.total_net_atomic.to_string(),
            policy_digest: request.policy_digest,
        },
    )
    .await
}

async fn compile_settlement_request(
    state: AppState,
    merchant_id: Uuid,
    request: SettlementCompileRequest,
    route_id: Option<Uuid>,
) -> Result<Json<CompiledSettlementResponse>, ApiError> {
    let serialized_request = serde_json::to_vec(&request).map_err(|error| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "SETTLEMENT_DIGEST_FAILURE",
        message: error.to_string(),
    })?;
    let request_digest = if let Some(route_id) = route_id {
        let mut digest_input = b"solana-tax-protocol/settlement-request/v2".to_vec();
        digest_input.extend_from_slice(route_id.as_bytes());
        digest_input.extend_from_slice(&serialized_request);
        sha256_digest(&digest_input)
    } else {
        sha256_digest(&serialized_request)
    };
    let instruction = compile_settlement_instruction(&request).map_err(ApiError::from)?;
    let instruction_digest =
        compiled_instruction_digest(&instruction).map_err(|error| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "SETTLEMENT_DIGEST_FAILURE",
            message: error.to_string(),
        })?;
    let intent_id = if let Some(store) = state.storage {
        let outcome = store
            .create_settlement_intent(
                merchant_id,
                &request.nonce.to_string(),
                request_digest,
                instruction_digest,
            )
            .await
            .map_err(ApiError::from)?;
        Some(match outcome {
            storage::SettlementIntentOutcome::Created(intent)
            | storage::SettlementIntentOutcome::Replay(intent) => intent.id,
        })
    } else {
        None
    };
    Ok(Json(CompiledSettlementResponse {
        instruction,
        intent_id,
        request_digest,
        instruction_digest,
        route_id,
    }))
}

async fn require_quote_attestation(
    store: &PgStore,
    merchant_id: Uuid,
    calculation_id: Uuid,
    expected_policy_digest: Digest32,
) -> Result<(), ApiError> {
    let stored = store
        .load_calculation_attestation(merchant_id, calculation_id)
        .await
        .map_err(ApiError::from)?;
    if stored.payload_digest != expected_policy_digest {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "SETTLEMENT_ATTESTATION_DIGEST_MISMATCH",
            message:
                "the settlement policy digest does not match the persisted calculation attestation"
                    .to_string(),
        });
    }
    stored
        .attestation
        .verify_at(server_unix_now()?)
        .map_err(ApiError::from)
}

async fn compile_split_settlement_request(
    state: AppState,
    merchant_id: Uuid,
    route_id: Uuid,
    split_plan_id: Uuid,
    request: SplitSettlementCompileRequest,
) -> Result<Json<CompiledSplitSettlementResponse>, ApiError> {
    let serialized_request = serde_json::to_vec(&request).map_err(|error| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "SETTLEMENT_DIGEST_FAILURE",
        message: error.to_string(),
    })?;
    let mut digest_input = b"solana-tax-protocol/split-settlement-request/v1".to_vec();
    digest_input.extend_from_slice(route_id.as_bytes());
    digest_input.extend_from_slice(split_plan_id.as_bytes());
    digest_input.extend_from_slice(&serialized_request);
    let request_digest = sha256_digest(&digest_input);
    let instruction = compile_split_settlement_instruction(&request).map_err(ApiError::from)?;
    let instruction_digest =
        sha256_digest(&serde_json::to_vec(&instruction).map_err(|error| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "SETTLEMENT_DIGEST_FAILURE",
            message: error.to_string(),
        })?);
    let intent_id = if let Some(store) = state.storage {
        let outcome = store
            .create_settlement_intent(
                merchant_id,
                &request.nonce.to_string(),
                request_digest,
                instruction_digest,
            )
            .await
            .map_err(ApiError::from)?;
        Some(match outcome {
            storage::SettlementIntentOutcome::Created(intent)
            | storage::SettlementIntentOutcome::Replay(intent) => intent.id,
        })
    } else {
        None
    };
    Ok(Json(CompiledSplitSettlementResponse {
        instruction,
        intent_id,
        request_digest,
        instruction_digest,
        route_id,
        split_plan_id,
    }))
}

fn sha256_digest(value: &[u8]) -> Digest32 {
    use sha2::{Digest as _, Sha256};
    Sha256::digest(value).into()
}

fn deserialize_digest<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Vec::<u8>::deserialize(deserializer)?;
    value.try_into().map_err(|value: Vec<u8>| {
        serde::de::Error::custom(format!(
            "policy digest must contain 32 bytes, got {}",
            value.len()
        ))
    })
}

async fn submit_settlement(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<SubmitSettlementRequest>, JsonRejection>,
) -> Result<Json<SubmitSettlementResponse>, ApiError> {
    let metrics = Arc::clone(&state.metrics);
    let started = Instant::now();
    let result = submit_settlement_inner(state, headers, payload).await;
    metrics.observe_or_log("settlement_submit", started.elapsed(), result.is_err());
    result
}

async fn submit_settlement_inner(
    state: AppState,
    headers: HeaderMap,
    payload: Result<Json<SubmitSettlementRequest>, JsonRejection>,
) -> Result<Json<SubmitSettlementResponse>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:settlement:submit")?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_STORAGE_UNAVAILABLE",
        message: "durable settlement intent storage is not configured".into(),
    })?;
    let intent = store
        .load_settlement_intent(request.intent_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError {
            status: StatusCode::NOT_FOUND,
            code: "SETTLEMENT_INTENT_NOT_FOUND",
            message: "settlement intent was not found".into(),
        })?;
    if intent.status != SettlementStatus::PreparedUnsubmitted.as_str()
        && intent.status != SettlementStatus::Signed.as_str()
    {
        return Ok(Json(SubmitSettlementResponse {
            intent_id: intent.id,
            status: intent.status,
            signature: intent.signature,
            provider_index: None,
        }));
    }
    let coordinator = state.settlement_coordinator.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_RPC_NOT_CONFIGURED",
        message: "settlement RPC providers are not configured".into(),
    })?;
    let transaction =
        SignedTransaction::parse(&request.signed_transaction_base64).map_err(ApiError::from)?;
    let signed_digest = sha256_digest(transaction.bytes());
    let (instruction_digest, expected_instruction) = match &request.instruction {
        SubmittedSettlementInstruction::Document(instruction) => (
            sha256_digest(&serde_json::to_vec(instruction).map_err(|error| ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "SETTLEMENT_DIGEST_FAILURE",
                message: error.to_string(),
            })?),
            CompiledSettlementInstruction {
                status: instruction.status.clone(),
                program_id: instruction.program_id.clone(),
                instruction_data_base64: instruction.instruction_data_base64.clone(),
                accounts: instruction.accounts.clone(),
                protocol_config: instruction.protocol_config.clone(),
                vault_authority: instruction.vault_authority.clone(),
                policy_attestation: instruction.policy_attestation.clone(),
                receipt: instruction.receipt.clone(),
                compute_budget_policy: instruction.compute_budget_policy.clone(),
            },
        ),
        SubmittedSettlementInstruction::Legacy(instruction) => (
            compiled_instruction_digest(instruction).map_err(|error| ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "SETTLEMENT_DIGEST_FAILURE",
                message: error.to_string(),
            })?,
            instruction.clone(),
        ),
        SubmittedSettlementInstruction::Split(instruction) => (
            sha256_digest(&serde_json::to_vec(instruction).map_err(|error| ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "SETTLEMENT_DIGEST_FAILURE",
                message: error.to_string(),
            })?),
            CompiledSettlementInstruction {
                status: instruction.status.clone(),
                program_id: instruction.program_id.clone(),
                instruction_data_base64: instruction.instruction_data_base64.clone(),
                accounts: instruction.accounts.clone(),
                protocol_config: instruction.protocol_config.clone(),
                vault_authority: instruction.vault_authority.clone(),
                policy_attestation: instruction.policy_attestation.clone(),
                receipt: instruction.receipt.clone(),
                compute_budget_policy: instruction.compute_budget_policy.clone(),
            },
        ),
    };
    if instruction_digest != intent.instruction_digest {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "SETTLEMENT_INSTRUCTION_CONFLICT",
            message: "compiled settlement instruction does not match the durable intent".into(),
        });
    }
    transaction
        .verify_settlement_instruction_with_resolver(&expected_instruction, coordinator.as_ref())
        .await
        .map_err(ApiError::from)?;
    let transaction_signature = transaction.signature().map_err(ApiError::from)?;
    coordinator
        .validate_blockhash(&transaction)
        .await
        .map_err(ApiError::from)?;
    if let Some(existing_digest) = intent.signed_transaction_digest
        && existing_digest != signed_digest
    {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "SETTLEMENT_SIGNED_BYTES_CONFLICT",
            message: "intent is already bound to different signed bytes".into(),
        });
    }

    if intent.status == SettlementStatus::PreparedUnsubmitted.as_str() {
        let advanced = store
            .transition_settlement_intent(SettlementTransition {
                intent_id: intent.id,
                merchant_id: principal.merchant_id,
                expected_status: SettlementStatus::PreparedUnsubmitted.as_str().into(),
                next_status: SettlementStatus::Signed.as_str().into(),
                signed_transaction_digest: Some(signed_digest),
                signature: None,
                slot: None,
                last_error_code: None,
                evidence: serde_json::json!({"source": "wallet_handoff"}),
            })
            .await
            .map_err(ApiError::from)?;
        if !advanced {
            return Err(ApiError {
                status: StatusCode::CONFLICT,
                code: "SETTLEMENT_INTENT_STATE_CHANGED",
                message: "settlement intent changed concurrently; reconcile before retry".into(),
            });
        }
    }

    let claimed = store
        .transition_settlement_intent(SettlementTransition {
            intent_id: intent.id,
            merchant_id: principal.merchant_id,
            expected_status: SettlementStatus::Signed.as_str().into(),
            next_status: SettlementStatus::Submitting.as_str().into(),
            signed_transaction_digest: Some(signed_digest),
            signature: Some(transaction_signature.clone()),
            slot: None,
            last_error_code: None,
            evidence: serde_json::json!({"source": "submission_fence"}),
        })
        .await
        .map_err(ApiError::from)?;
    if !claimed {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "SETTLEMENT_SUBMISSION_IN_PROGRESS",
            message: "settlement submission is already owned by another request; reconcile the intent before retry".into(),
        });
    }

    let submitted = match coordinator.submit(&transaction).await {
        Ok(receipt) => receipt,
        Err(error) => {
            let error_code = settlement_submission_error_code(&error);
            let _ = store
                .transition_settlement_intent(SettlementTransition {
                    intent_id: intent.id,
                    merchant_id: principal.merchant_id,
                    expected_status: SettlementStatus::Submitting.as_str().into(),
                    next_status: SettlementStatus::Unknown.as_str().into(),
                    signed_transaction_digest: Some(signed_digest),
                    signature: Some(transaction_signature.clone()),
                    slot: None,
                    last_error_code: Some(error_code.to_string()),
                    evidence: serde_json::json!({"source": "rpc", "error_code": error_code}),
                })
                .await;
            return Err(ApiError {
                status: StatusCode::SERVICE_UNAVAILABLE,
                code: "SETTLEMENT_SUBMISSION_UNKNOWN",
                message: "provider outcome is unknown; reconcile by intent before retry".into(),
            });
        }
    };
    let advanced = store
        .transition_settlement_intent(SettlementTransition {
            intent_id: intent.id,
            merchant_id: principal.merchant_id,
            expected_status: SettlementStatus::Submitting.as_str().into(),
            next_status: submitted.status.as_str().into(),
            signed_transaction_digest: None,
            signature: Some(submitted.signature.clone()),
            slot: None,
            last_error_code: None,
            evidence: serde_json::json!({"source": "rpc", "provider_index": submitted.provider_index}),
        })
        .await
        .map_err(ApiError::from)?;
    if !advanced {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "SETTLEMENT_INTENT_STATE_CHANGED",
            message: "settlement intent changed concurrently; reconcile by signature".into(),
        });
    }
    Ok(Json(SubmitSettlementResponse {
        intent_id: intent.id,
        status: submitted.status.as_str().to_string(),
        signature: Some(submitted.signature),
        provider_index: Some(submitted.provider_index),
    }))
}

fn settlement_submission_error_code(error: &CoordinatorError) -> &'static str {
    match error {
        CoordinatorError::InvalidSignedTransaction(_) => "invalid_signed_transaction",
        CoordinatorError::AddressLookupTable(_) => "address_lookup_table_resolution_failed",
        CoordinatorError::InvalidConfiguration(_) => "invalid_coordinator_configuration",
        CoordinatorError::ProviderAttemptsExhausted(_) => "provider_attempts_exhausted",
        CoordinatorError::InvalidProviderResponse(_) => "invalid_provider_response",
        CoordinatorError::InvalidAdjustmentRegistry(_) => "invalid_adjustment_registry",
        CoordinatorError::InvalidSignature => "invalid_signature",
    }
}

async fn get_settlement_intent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(intent_id): Path<Uuid>,
) -> Result<Json<SubmitSettlementResponse>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:settlement:read")?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_STORAGE_UNAVAILABLE",
        message: "durable settlement intent storage is not configured".into(),
    })?;
    let intent = store
        .load_settlement_intent(intent_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError {
            status: StatusCode::NOT_FOUND,
            code: "SETTLEMENT_INTENT_NOT_FOUND",
            message: "settlement intent was not found".into(),
        })?;
    Ok(Json(SubmitSettlementResponse {
        intent_id: intent.id,
        status: intent.status,
        signature: intent.signature,
        provider_index: None,
    }))
}

async fn reconcile_settlement_intent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(intent_id): Path<Uuid>,
) -> Result<Json<SubmitSettlementResponse>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:settlement:reconcile")?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_STORAGE_UNAVAILABLE",
        message: "durable settlement intent storage is not configured".into(),
    })?;
    let coordinator = state.settlement_coordinator.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "SETTLEMENT_RPC_NOT_CONFIGURED",
        message: "settlement RPC providers are not configured".into(),
    })?;
    let intent = store
        .load_settlement_intent(intent_id, principal.merchant_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError {
            status: StatusCode::NOT_FOUND,
            code: "SETTLEMENT_INTENT_NOT_FOUND",
            message: "settlement intent was not found".into(),
        })?;
    let Some(signature) = intent.signature.clone() else {
        return Ok(Json(SubmitSettlementResponse {
            intent_id: intent.id,
            status: intent.status,
            signature: None,
            provider_index: None,
        }));
    };
    let observed = coordinator
        .observe(&signature)
        .await
        .map_err(ApiError::from)?;
    let current = match intent.status.as_str() {
        "prepared_unsubmitted" => SettlementStatus::PreparedUnsubmitted,
        "signed" => SettlementStatus::Signed,
        "submitting" => SettlementStatus::Submitting,
        "submitted" => SettlementStatus::Submitted,
        "processed" => SettlementStatus::Processed,
        "confirmed" => SettlementStatus::Confirmed,
        "finalized" => SettlementStatus::Finalized,
        "rejected" => SettlementStatus::Rejected,
        "unknown" => SettlementStatus::Unknown,
        _ => {
            return Err(ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "INVALID_SETTLEMENT_LIFECYCLE",
                message: "stored settlement status is invalid".into(),
            });
        }
    };
    if observed.status != current && can_transition(current, observed.status) {
        let _ = store
            .transition_settlement_intent(SettlementTransition {
                intent_id: intent.id,
                merchant_id: principal.merchant_id,
                expected_status: current.as_str().into(),
                next_status: observed.status.as_str().into(),
                signed_transaction_digest: None,
                signature: Some(signature.clone()),
                slot: observed.slot,
                last_error_code: (observed.status == SettlementStatus::Unknown)
                    .then_some("rpc_status_unknown".to_string()),
                evidence: serde_json::json!({
                    "source": "rpc_status",
                    "provider_index": observed.provider_index
                }),
            })
            .await
            .map_err(ApiError::from)?;
    }
    Ok(Json(SubmitSettlementResponse {
        intent_id: intent.id,
        status: observed.status.as_str().to_string(),
        signature: Some(signature),
        provider_index: Some(observed.provider_index),
    }))
}

fn authenticate_scope(
    state: &AppState,
    headers: &HeaderMap,
    required_scope: &str,
) -> Result<auth::AuthenticatedPrincipal, ApiError> {
    let authenticator = state
        .authenticator
        .as_ref()
        .ok_or_else(|| ApiError::from(auth::AuthError::NotConfigured))?;
    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApiError::from(auth::AuthError::ClockUnavailable))?
        .as_secs();
    let principal = authenticator
        .authenticate(
            headers.get("authorization"),
            i64::try_from(now_unix).unwrap_or(i64::MAX),
        )
        .map_err(ApiError::from)?;
    principal
        .require_scope(required_scope)
        .map_err(ApiError::from)?;
    Ok(principal)
}

fn authenticate_scope_role(
    state: &AppState,
    headers: &HeaderMap,
    required_scope: &str,
    required_role: &str,
) -> Result<auth::AuthenticatedPrincipal, ApiError> {
    let principal = authenticate_scope(state, headers, required_scope)?;
    principal
        .require_role(required_role)
        .map_err(ApiError::from)?;
    Ok(principal)
}

fn authenticate_scope_any_role(
    state: &AppState,
    headers: &HeaderMap,
    required_scope: &str,
    required_roles: &[&str],
) -> Result<auth::AuthenticatedPrincipal, ApiError> {
    let principal = authenticate_scope(state, headers, required_scope)?;
    principal
        .require_any_role(required_roles)
        .map_err(ApiError::from)?;
    Ok(principal)
}

async fn verify_attestation(
    Json(request): Json<VerifyAttestationRequest>,
) -> Result<Json<VerifyAttestationResponse>, ApiError> {
    let binding = AttestationBinding {
        merchant_id: request.binding.merchant_id,
        rule_set_id: &request.binding.rule_set_id,
        rule_version: request.binding.rule_version,
        jurisdiction: &request.binding.jurisdiction,
        currency: &request.binding.currency,
        asset_mint: request.binding.asset_mint,
        facts_digest: request.binding.facts_digest,
        rule_digest: request.binding.rule_digest,
        gross_atomic: request.binding.gross_atomic,
        tax_atomic: request.binding.tax_atomic,
        net_atomic: request.binding.net_atomic,
    };
    request
        .attestation
        .verify_at(request.now_unix)
        .map_err(ApiError::from)?;
    request
        .attestation
        .verify_binding(&binding)
        .map_err(ApiError::from)?;
    Ok(Json(VerifyAttestationResponse {
        status: "verified_unsettled",
        payload_digest: request.attestation.payload_digest,
        issuer_public_key: request.attestation.issuer_public_key,
    }))
}

async fn verify_quote_attestation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(calculation_id): Path<Uuid>,
    payload: Result<Json<QuoteAttestationVerifyRequest>, JsonRejection>,
) -> Result<Json<QuoteAttestationVerifyResponse>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:attestation:verify")?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "ATTESTATION_STORAGE_UNAVAILABLE",
        message: "calculation attestation storage is not configured".to_string(),
    })?;
    let quote = store
        .load_quote_for_settlement(principal.merchant_id, calculation_id)
        .await
        .map_err(ApiError::from)?;
    let route = store
        .load_active_settlement_route(principal.merchant_id, request.route_id)
        .await
        .map_err(ApiError::from)?;
    let asset_mint = Pubkey::from_str(&route.mint).map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "SETTLEMENT_ROUTE_EVIDENCE_CORRUPT",
        message: "stored route mint is not a valid Solana public key".to_string(),
    })?;
    let now_unix = server_unix_now()?;
    let gross_atomic = quote_amount_as_u64(quote.result.gross_atomic, "gross_atomic")?;
    let tax_atomic = quote_amount_as_u64(quote.result.total_tax_atomic, "total_tax_atomic")?;
    let net_atomic = quote_amount_as_u64(quote.result.total_net_atomic, "total_net_atomic")?;
    let binding = AttestationBinding {
        merchant_id: merchant_identity_digest(principal.merchant_id.as_bytes()),
        rule_set_id: &quote.result.rule_set_id,
        rule_version: quote.result.rule_version,
        jurisdiction: &quote.result.jurisdiction,
        currency: &quote.result.currency,
        asset_mint: asset_mint.to_bytes(),
        facts_digest: quote.result.facts_digest,
        rule_digest: quote.result.rule_digest,
        gross_atomic,
        tax_atomic,
        net_atomic,
    };
    request
        .attestation
        .verify_at(now_unix)
        .map_err(ApiError::from)?;
    request
        .attestation
        .verify_binding(&binding)
        .map_err(ApiError::from)?;
    let stored = store
        .record_calculation_attestation(principal.merchant_id, calculation_id, &request.attestation)
        .await
        .map_err(|error| {
            tracing::error!(
                merchant_id = %principal.merchant_id,
                calculation_id = %calculation_id,
                error = ?error,
                "calculation attestation persistence failed"
            );
            ApiError::from(error)
        })?;
    Ok(Json(QuoteAttestationVerifyResponse {
        status: "attested_unsettled",
        calculation_id: stored.calculation_id,
        payload_digest: stored.payload_digest,
        issuer_public_key: stored.issuer_public_key,
        expires_at_unix: stored.expires_at_unix,
    }))
}

async fn verify_document_quote_attestation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(calculation_id): Path<Uuid>,
    payload: Result<Json<DocumentQuoteAttestationVerifyRequest>, JsonRejection>,
) -> Result<Json<DocumentQuoteAttestationVerifyResponse>, ApiError> {
    let principal = authenticate_scope(&state, &headers, "tax:attestation:verify")?;
    let Json(request) = payload.map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_JSON",
        message: "request body is invalid".to_string(),
    })?;
    let store = state.storage.ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "ATTESTATION_STORAGE_UNAVAILABLE",
        message: "document attestation storage is not configured".to_string(),
    })?;
    let document = store
        .load_document_quote_for_attestation(principal.merchant_id, calculation_id)
        .await
        .map_err(ApiError::from)?;
    if document.schema != TAX_DOCUMENT_SCHEMA_V1 {
        return Err(ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "DOCUMENT_QUOTE_SCHEMA_CORRUPT",
            message: "stored document quote schema is not supported".to_string(),
        });
    }
    let route = store
        .load_active_settlement_route(principal.merchant_id, request.route_id)
        .await
        .map_err(ApiError::from)?;
    let asset_mint = Pubkey::from_str(&route.mint).map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "SETTLEMENT_ROUTE_EVIDENCE_CORRUPT",
        message: "stored route mint is not a valid Solana public key".to_string(),
    })?;
    let binding = DocumentAttestationBinding {
        document_schema: &document.schema,
        merchant_id: merchant_identity_digest(principal.merchant_id.as_bytes()),
        document_id: &document.result.document_id,
        engine_version: &document.result.engine_version,
        rule_set_id: &document.result.rule_set_id,
        rule_version: document.result.rule_version,
        jurisdiction: &document.result.jurisdiction,
        currency: &document.result.currency,
        asset_mint: asset_mint.to_bytes(),
        facts_digest: document.result.facts_digest,
        rule_digest: document.result.rule_digest,
        gross_atomic: quote_amount_as_u64(document.result.gross_atomic, "gross_atomic")?,
        tax_atomic: quote_amount_as_u64(document.result.total_tax_atomic, "total_tax_atomic")?,
        net_atomic: quote_amount_as_u64(document.result.total_net_atomic, "total_net_atomic")?,
    };
    request
        .attestation
        .verify_at(server_unix_now()?)
        .map_err(ApiError::from)?;
    request
        .attestation
        .verify_binding(&binding)
        .map_err(ApiError::from)?;
    let stored = store
        .record_document_calculation_attestation(
            principal.merchant_id,
            calculation_id,
            asset_mint.to_bytes(),
            &request.attestation,
        )
        .await
        .map_err(|error| {
            tracing::error!(
                merchant_id = %principal.merchant_id,
                calculation_id = %calculation_id,
                error = ?error,
                "document calculation attestation persistence failed"
            );
            ApiError::from(error)
        })?;
    Ok(Json(DocumentQuoteAttestationVerifyResponse {
        status: "document_attested_unsettled",
        calculation_id: stored.calculation_id,
        payload_digest: stored.payload_digest,
        issuer_public_key: stored.issuer_public_key,
        expires_at_unix: stored.expires_at_unix,
    }))
}

fn quote_amount_as_u64(value: i128, field: &str) -> Result<u64, ApiError> {
    u64::try_from(value).map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "QUOTE_AMOUNT_OUT_OF_ATTESTATION_RANGE",
        message: format!("stored quote {field} is outside the attestation amount range"),
    })
}

fn server_unix_now() -> Result<i64, ApiError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "API_CLOCK_UNAVAILABLE",
            message: "server clock is unavailable".to_string(),
        })?
        .as_secs();
    i64::try_from(seconds).map_err(|_| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "API_CLOCK_UNAVAILABLE",
        message: "server clock is outside the supported range".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{HeaderValue, Request},
    };
    use ed25519_dalek::{Signer, SigningKey};
    use policy::{ATTESTATION_SCHEMA_VERSION, CalculationAttestation, UnsignedAttestation};
    use settlement_client::{SETTLEMENT_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID};
    use std::time::Duration;
    use tower::ServiceExt;

    fn test_metrics() -> Arc<ServiceMetrics> {
        Arc::new(
            ServiceMetrics::new(
                "api",
                &[
                    "tax_calculate",
                    "tax_document_calculate",
                    "tax_quote",
                    "tax_document_quote",
                    "tax_replay",
                    "tax_document_replay",
                    "tax_rule_draft_create",
                    "tax_rule_transition",
                    "tax_rule_governance_read",
                    "jurisdiction_policy_draft_create",
                    "jurisdiction_policy_transition",
                    "jurisdiction_policy_governance_read",
                    "vault_release_claim_create",
                    "vault_release_claim_read",
                    "vault_release_claim_transition",
                    "settlement_adjustment_create",
                    "settlement_adjustment_read",
                    "settlement_adjustment_transition",
                    "adjustment_authority_challenge",
                    "adjustment_authority_bind",
                    "settlement_adjustment_compile",
                    "refund_funding_configuration_create",
                    "refund_funding_configuration_read",
                    "refund_funding_configuration_transition",
                    "vault_reconciliation_read",
                    "indexer_replay_operator",
                    "indexer_targeted_recovery",
                    "settlement_compile",
                    "settlement_submit",
                ],
            )
            .unwrap(),
        )
    }

    #[tokio::test]
    async fn refund_claim_prepare_requires_explicit_confirmation_before_auth_or_storage() {
        let body = serde_json::to_vec(&RefundClaimPrepareRequest {
            idempotency_key: "claim-test-01".to_string(),
            adjustment_id: Uuid::now_v7(),
            confirm: false,
        })
        .expect("request JSON");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: true,
                storage: None,
                authenticator: None,
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/refund-claims")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn refund_claim_submit_requires_authentication_before_storage_or_rpc() {
        let body = serde_json::to_vec(&RefundClaimSubmitRequest {
            intent_id: Uuid::now_v7(),
            signed_transaction_base64: "not-a-transaction".to_string(),
        })
        .expect("request JSON");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/refund-claims/submit")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn refund_cancellation_prepare_requires_explicit_confirmation_before_auth_or_storage() {
        let body = serde_json::to_vec(&RefundCancellationPrepareRequest {
            idempotency_key: "cancel-test-01".to_string(),
            claim_intent_id: Uuid::now_v7(),
            confirm: false,
        })
        .expect("request JSON");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: true,
                storage: None,
                authenticator: None,
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/refund-claim-cancellations")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn refund_cancellation_submit_requires_authentication_before_storage_or_rpc() {
        let body = serde_json::to_vec(&RefundCancellationSubmitRequest {
            intent_id: Uuid::now_v7(),
            signed_transaction_base64: "not-a-transaction".to_string(),
        })
        .expect("request JSON");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/refund-claim-cancellations/submit")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn refund_claim_reconcile_requires_authentication_before_storage_or_recovery() {
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/v1/settlement/refund-claims/{}/reconcile",
                        Uuid::now_v7()
                    ))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn refund_execute_prepare_requires_explicit_confirmation_before_auth_or_storage() {
        let body = serde_json::to_vec(&RefundExecutePrepareRequest {
            idempotency_key: "execute-test-01".to_string(),
            claim_intent_id: Uuid::now_v7(),
            confirm: false,
        })
        .expect("request JSON");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: true,
                storage: None,
                authenticator: None,
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/refund-executions")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn refund_execute_submit_requires_authentication_before_storage_or_rpc() {
        let body = serde_json::to_vec(&RefundExecuteSubmitRequest {
            intent_id: Uuid::now_v7(),
            signed_transaction_base64: "not-a-transaction".to_string(),
        })
        .expect("request JSON");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/refund-executions/submit")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn health_endpoint_is_live_without_claiming_readiness() {
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: true,
                storage: None,
                authenticator: None,
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn api_overwrites_caller_request_id_with_uuidv7_on_every_response() {
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: true,
                storage: None,
                authenticator: None,
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .header("x-request-id", "caller-controlled-value")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let request_id = response
            .headers()
            .get("x-request-id")
            .expect("every response must expose an API-owned request ID")
            .to_str()
            .unwrap();
        let parsed = Uuid::parse_str(request_id).unwrap();
        assert_eq!(parsed.get_version_num(), 7);
        assert_ne!(request_id, "caller-controlled-value");
    }

    #[tokio::test]
    async fn api_correlates_router_errors_before_authentication_or_handler_execution() {
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: true,
                storage: None,
                authenticator: None,
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .uri("/not-a-real-route")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| Uuid::parse_str(value).ok())
            .expect("router errors must carry a valid request ID");
        assert_eq!(request_id.get_version_num(), 7);
    }

    #[tokio::test]
    async fn metrics_endpoint_exposes_only_registered_series() {
        let metrics = test_metrics();
        metrics
            .observe("tax_quote", std::time::Duration::from_millis(25), false)
            .unwrap();
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: true,
                storage: None,
                authenticator: None,
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics,
            })
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "text/plain; version=0.0.4; charset=utf-8"
        );
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("operation=\"tax_quote\""));
        assert!(body.contains("operation=\"tax_calculate\""));
        assert!(!body.contains("operation=\"user-controlled\""));
    }

    #[test]
    fn rule_cache_metrics_use_fixed_labels_and_bounded_names() {
        let rendered = render_rule_cache_metrics(RuleCacheStats {
            hits: 11,
            misses: 3,
            evictions: 1,
            entries: 2,
            capacity: 2,
        });
        assert!(rendered.contains("stp_tax_rule_cache_hits_total{service=\"api\"} 11"));
        assert!(rendered.contains("stp_tax_rule_cache_misses_total{service=\"api\"} 3"));
        assert!(rendered.contains("stp_tax_rule_cache_evictions_total{service=\"api\"} 1"));
        assert!(rendered.contains("stp_tax_rule_cache_entries{service=\"api\"} 2"));
        assert!(rendered.contains("stp_tax_rule_cache_capacity{service=\"api\"} 2"));
        assert!(!rendered.contains("merchant_id"));
    }

    #[test]
    fn database_pool_metrics_use_fixed_labels_and_bounded_names() {
        let rendered = render_database_pool_metrics(DatabasePoolStats {
            max_size: 4,
            size: 4,
            available: 0,
            waiting: 7,
        });
        assert!(rendered.contains("stp_database_pool_max_size{service=\"api\"} 4"));
        assert!(rendered.contains("stp_database_pool_size{service=\"api\"} 4"));
        assert!(rendered.contains("stp_database_pool_available{service=\"api\"} 0"));
        assert!(rendered.contains("stp_database_pool_waiting{service=\"api\"} 7"));
        assert!(!rendered.contains("merchant_id"));
        assert!(!rendered.contains("request_id"));
    }

    #[test]
    fn settlement_provider_metrics_use_fixed_labels_and_redact_identity() {
        let rendered = render_settlement_provider_metrics(&[
            SettlementProviderHealthSnapshot {
                provider_index: 0,
                score: -1,
                consecutive_failures: 1,
                cooldown_remaining: Duration::from_millis(25),
            },
            SettlementProviderHealthSnapshot {
                provider_index: 1,
                score: 2,
                consecutive_failures: 0,
                cooldown_remaining: Duration::ZERO,
            },
        ]);
        assert!(rendered.contains(
            "stp_settlement_provider_health_score{service=\"api\",provider_index=\"0\"} -1"
        ));
        assert!(rendered.contains(
            "stp_settlement_provider_consecutive_failures{service=\"api\",provider_index=\"0\"} 1"
        ));
        assert!(rendered.contains(
            "stp_settlement_provider_cooldown_ms{service=\"api\",provider_index=\"0\"} 25"
        ));
        assert!(!rendered.contains("endpoint"));
        assert!(!rendered.contains("merchant_id"));
        assert!(!rendered.contains("request_id"));
    }

    #[test]
    fn wallet_proof_verification_binds_the_exact_server_message() {
        let signing_key = SigningKey::from_bytes(&[41; 32]);
        let public_key = Pubkey::new_from_array(signing_key.verifying_key().to_bytes());
        let message = b"solana-tax-protocol/adjustment-authority/v1/challenge";
        let signature = signing_key.sign(message);

        assert!(verify_wallet_proof(&public_key, message, &signature.to_bytes()).is_ok());

        let mut tampered_message = message.to_vec();
        tampered_message[0] ^= 1;
        let error = verify_wallet_proof(&public_key, &tampered_message, &signature.to_bytes())
            .expect_err("a proof must not verify against a changed challenge");
        assert_eq!(error.code, "INVALID_WALLET_PROOF");
    }

    #[test]
    fn wallet_proof_verification_rejects_a_different_wallet() {
        let signer = SigningKey::from_bytes(&[42; 32]);
        let other_wallet =
            Pubkey::new_from_array(SigningKey::from_bytes(&[43; 32]).verifying_key().to_bytes());
        let message = b"solana-tax-protocol/adjustment-authority/v1/challenge";
        let signature = signer.sign(message);

        let error = verify_wallet_proof(&other_wallet, message, &signature.to_bytes())
            .expect_err("a proof must be bound to the claimed wallet");
        assert_eq!(error.code, "INVALID_WALLET_PROOF");
    }

    #[tokio::test]
    async fn verifies_a_bound_attestation_before_it_can_be_settled() {
        let unsigned = UnsignedAttestation {
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
        };
        let attestation =
            CalculationAttestation::sign(unsigned, &SigningKey::from_bytes(&[7; 32])).unwrap();
        let body = serde_json::to_vec(&VerifyAttestationRequest {
            attestation,
            binding: AttestationBindingRequest {
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
            },
            now_unix: 1_800_000_100,
        })
        .unwrap();

        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: true,
                storage: None,
                authenticator: None,
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/attestation/verify")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let response_body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        let response: serde_json::Value = serde_json::from_slice(&response_body).unwrap();
        assert_eq!(response["status"], "verified_unsettled");
    }

    #[tokio::test]
    async fn calculation_endpoint_preserves_financial_values_as_strings() {
        let request = CalculateRequest {
            facts: TaxFacts {
                gross_atomic: 1_100,
                net_atomic: 1_000,
                currency: "USD".to_string(),
                jurisdiction: "US-CA".to_string(),
                jurisdiction_resolution: None,
                exemption_codes: Vec::new(),
                cumulative_period: None,
            },
            rule_set: RuleSet {
                id: "api-test".to_string(),
                version: 1,
                components: vec![tax_engine::TaxComponentRule {
                    id: "sales-tax".to_string(),
                    label: "Sales tax".to_string(),
                    base: tax_engine::TaxBase::single(tax_engine::BaseReference::Net),
                    rate: tax_engine::RateSpec::Flat(tax_engine::Rate::new(10, 100).unwrap()),
                    price_mode: tax_engine::PriceMode::Exclusive,
                    credit_atomic: 0,
                    floor_atomic: None,
                    cap_atomic: None,
                    rounding: tax_engine::RoundingSpec {
                        mode: tax_engine::RoundingMode::HalfUp,
                        scope: tax_engine::RoundingScope::Component,
                        quantum_atomic: 1,
                    },
                    threshold: None,
                    exemption_codes: Vec::new(),
                }],
            },
        };
        let body = serde_json::to_vec(&request).unwrap();
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: true,
                storage: None,
                authenticator: None,
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/calculate")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response_body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        let response: serde_json::Value = serde_json::from_slice(&response_body).unwrap();
        assert_eq!(response["result"]["total_tax_atomic"], "100");
        assert_eq!(response["result"]["total_net_atomic"], "1000");
        assert_eq!(
            response["result"]["facts_digest"].as_array().unwrap().len(),
            32
        );
    }

    #[tokio::test]
    async fn document_calculation_endpoint_returns_canonical_line_results() {
        let body = serde_json::json!({
            "schema": "solana-tax-protocol/tax-document/v1",
            "facts": {
                "id": "invoice-api-1",
                "currency": "USD",
                "jurisdiction": "US-CA",
                "jurisdiction_resolution": null,
                "cumulative_period": null,
                "lines": [
                    { "id": "line-b", "gross_atomic": "2", "net_atomic": "2", "exemption_codes": [] },
                    { "id": "line-a", "gross_atomic": "1", "net_atomic": "1", "exemption_codes": [] }
                ]
            },
            "rule_set": {
                "id": "api-document-test",
                "version": 1,
                "components": [{
                    "id": "sales-tax",
                    "label": "Sales tax",
                    "base": { "sources": ["Net"] },
                    "rate": { "Flat": { "numerator": "1", "denominator": "3" } },
                    "price_mode": "Exclusive",
                    "credit_atomic": "0",
                    "floor_atomic": null,
                    "cap_atomic": null,
                    "rounding": { "mode": "HalfUp", "scope": "Line" },
                    "threshold": null,
                    "exemption_codes": []
                }]
            }
        });
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: true,
                storage: None,
                authenticator: None,
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/calculate-document")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response_body = axum::body::to_bytes(response.into_body(), 128 * 1024)
            .await
            .unwrap();
        let response: serde_json::Value = serde_json::from_slice(&response_body).unwrap();
        assert_eq!(response["status"], "simulated_unattested");
        assert_eq!(response["schema"], "solana-tax-protocol/tax-document/v1");
        assert_eq!(response["result"]["total_tax_atomic"], "1");
        assert_eq!(response["result"]["lines"][0]["id"], "line-a");
        assert_eq!(response["result"]["lines"][1]["id"], "line-b");
        assert_eq!(response["result"]["lines"][1]["total_tax_atomic"], "1");
    }

    #[tokio::test]
    async fn document_calculation_endpoint_fails_closed_outside_simulation() {
        let body = serde_json::json!({
            "schema": "solana-tax-protocol/tax-document/v1",
            "facts": {
                "id": "invoice-api-production-boundary",
                "currency": "USD",
                "jurisdiction": "US-CA",
                "jurisdiction_resolution": null,
                "cumulative_period": null,
                "lines": [{ "id": "line-a", "gross_atomic": "1", "net_atomic": "1", "exemption_codes": [] }]
            },
            "rule_set": {
                "id": "caller-rule-must-not-be-authoritative",
                "version": 1,
                "components": []
            }
        });
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: None,
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/calculate-document")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
        let response_body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        let response: serde_json::Value = serde_json::from_slice(&response_body).unwrap();
        assert_eq!(
            response["error"]["code"],
            "PUBLISHED_RULE_REGISTRY_REQUIRED"
        );
    }

    #[tokio::test]
    async fn document_calculation_endpoint_rejects_unknown_schema() {
        let body = serde_json::json!({
            "schema": "solana-tax-protocol/tax-document/v0",
            "facts": {
                "id": "invoice-schema-boundary",
                "currency": "USD",
                "jurisdiction": "US-CA",
                "lines": [{ "id": "line-a", "gross_atomic": "1", "net_atomic": "1", "exemption_codes": [] }]
            },
            "rule_set": { "id": "rule", "version": 1, "components": [] }
        });
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: true,
                storage: None,
                authenticator: None,
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/calculate-document")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let response_body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        let response: serde_json::Value = serde_json::from_slice(&response_body).unwrap();
        assert_eq!(response["error"]["code"], "UNSUPPORTED_DOCUMENT_SCHEMA");
    }

    #[tokio::test]
    async fn persisted_quote_fails_closed_when_storage_is_not_configured() {
        let body = serde_json::to_vec(&QuoteRequest {
            idempotency_key: "local-test-quote".to_string(),
            rule_set_id: "published-rule".to_string(),
            rule_version: 1,
            jurisdiction: "US-CA".to_string(),
            regime: "sales-tax".to_string(),
            facts: TaxFacts {
                gross_atomic: 1_100,
                net_atomic: 1_000,
                currency: "USD".to_string(),
                jurisdiction: "US-CA".to_string(),
                jurisdiction_resolution: None,
                exemption_codes: Vec::new(),
                cumulative_period: None,
            },
            jurisdiction_evidence: Vec::new(),
        })
        .unwrap();
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: true,
                storage: None,
                authenticator: None,
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/quote")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn quote_rejects_missing_bearer_token_before_storage_access() {
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/quote")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&QuoteRequest {
                            idempotency_key: "local-test-quote".to_string(),
                            rule_set_id: "published-rule".to_string(),
                            rule_version: 1,
                            jurisdiction: "US-CA".to_string(),
                            regime: "sales-tax".to_string(),
                            facts: TaxFacts {
                                gross_atomic: 1_100,
                                net_atomic: 1_000,
                                currency: "USD".to_string(),
                                jurisdiction: "US-CA".to_string(),
                                jurisdiction_resolution: None,
                                exemption_codes: Vec::new(),
                                cumulative_period: None,
                            },
                            jurisdiction_evidence: Vec::new(),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn document_attestation_rejects_missing_bearer_token_before_storage_access() {
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/v1/tax/document-quotes/{}/attestation/verify",
                        Uuid::now_v7()
                    ))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn document_attestation_wire_boundary_rejects_aggregate_attestation() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "tax:attestation:verify");
        let aggregate = CalculationAttestation::sign(
            UnsignedAttestation {
                schema_version: ATTESTATION_SCHEMA_VERSION,
                merchant_id: [1; 32],
                rule_set_id: "rule".to_string(),
                rule_version: 1,
                jurisdiction: "US-CA".to_string(),
                currency: "USD".to_string(),
                asset_mint: [2; 32],
                facts_digest: [3; 32],
                rule_digest: [4; 32],
                gross_atomic: 10,
                tax_atomic: 1,
                net_atomic: 9,
                issued_at_unix: now,
                expires_at_unix: now + 300,
                sequence: 1,
                issuer_key_id: [5; 32],
            },
            &SigningKey::from_bytes(&[7; 32]),
        )
        .unwrap();
        let body = serde_json::json!({
            "route_id": Uuid::now_v7(),
            "attestation": aggregate,
        });
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/v1/tax/document-quotes/{}/attestation/verify",
                        Uuid::now_v7()
                    ))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn document_quote_rejects_missing_bearer_token_before_storage_access() {
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/document-quote")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"schema":"solana-tax-protocol/tax-document/v1"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn document_quote_rejects_unknown_schema_before_persistence() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token(now);
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/document-quote")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"schema":"solana-tax-protocol/tax-document/v0","idempotency_key":"doc-1","rule_set_id":"rule","rule_version":1,"jurisdiction":"US-CA","regime":"sales-tax","facts":{"id":"invoice-1","currency":"USD","jurisdiction":"US-CA","lines":[{"id":"line-1","gross_atomic":"100","net_atomic":"100"}]}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), 16 * 1024)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"]["code"], "UNSUPPORTED_DOCUMENT_SCHEMA");
    }

    #[tokio::test]
    async fn document_quote_requires_durable_storage_after_authentication() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token(now);
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/document-quote")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"schema":"solana-tax-protocol/tax-document/v1","idempotency_key":"doc-1","rule_set_id":"rule","rule_version":1,"jurisdiction":"US-CA","regime":"sales-tax","facts":{"id":"invoice-1","currency":"USD","jurisdiction":"US-CA","lines":[{"id":"line-1","gross_atomic":"100","net_atomic":"100"}]}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn quote_rejects_authenticated_principal_without_quote_scope() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "audit:read");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/quote")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn rule_governance_rejects_missing_exact_role_before_storage_access() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "tax:rule:author");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tax/rules/{}/transition", Uuid::now_v7()))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"to_status":"simulated","evidence":{}}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn adjustment_proposal_requires_the_exact_proposer_role() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "tax:adjustment:propose");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/adjustments")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn refund_funding_configuration_requires_the_exact_proposer_role() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "tax:refund:configure");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/refund-funding-configurations")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn refund_funding_configuration_rejects_activation_before_authentication() {
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: None,
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/v1/settlement/refund-funding-configurations/{}/transition",
                        Uuid::now_v7()
                    ))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"to_status":"active","evidence":{}}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn adjustment_transition_rejects_unbounded_or_posting_statuses_before_auth() {
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: None,
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/v1/settlement/adjustments/{}/transition",
                        Uuid::now_v7()
                    ))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"to_status":"posted","evidence":{}}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn adjustment_read_requires_a_governed_read_role_before_storage_access() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "tax:adjustment:read");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/settlement/adjustments/{}", Uuid::now_v7()))
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn rule_draft_creation_requires_the_exact_author_role() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "tax:rule:author");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/rules")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn rule_governance_read_requires_a_governance_role() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "tax:rule:read");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/tax/rules/{}/governance", Uuid::now_v7()))
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn persisted_rule_simulation_requires_a_governance_role_and_storage() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope_and_roles(
            now,
            "tax:rule:read",
            "rule-author",
            &["tax_rule_author"],
        );
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/v1/tax/rules/{}/simulate",
                        Uuid::now_v7()
                    ))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"facts":{"gross_atomic":"1100","net_atomic":"1000","currency":"USD","jurisdiction":"US-CA"}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn persisted_rule_simulation_rejects_client_jurisdiction_evidence() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope_and_roles(
            now,
            "tax:rule:read",
            "rule-author",
            &["tax_rule_author"],
        );
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/v1/tax/rules/{}/simulate",
                        Uuid::now_v7()
                    ))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"facts":{"gross_atomic":"1100","net_atomic":"1000","currency":"USD","jurisdiction":"US-CA","jurisdiction_resolution":{}}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn jurisdiction_policy_authoring_requires_the_exact_author_role() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "tax:jurisdiction:author");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/jurisdiction-policies")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn jurisdiction_policy_transition_requires_the_exact_reviewer_role() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "tax:jurisdiction:review");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/v1/tax/jurisdiction-policies/{}/transition",
                        Uuid::now_v7()
                    ))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"to_status":"reviewed","evidence":{}}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn jurisdiction_policy_governance_read_requires_a_governance_role() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "tax:jurisdiction:read");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/v1/tax/jurisdiction-policies/{}/governance",
                        Uuid::now_v7()
                    ))
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn vault_release_proposal_requires_the_exact_proposer_role() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "tax:vault:propose");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/vault/release-claims")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn vault_reconciliation_read_requires_a_governed_vault_role() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "tax:vault:read");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .uri("/v1/vault/reconciliation/targets")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn vault_release_approval_requires_the_exact_approver_role() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope_and_roles(
            now,
            "tax:vault:approve",
            "test-user",
            &["vault_release_operator"],
        );
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/v1/vault/release-claims/{}/transition",
                        Uuid::now_v7()
                    ))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"to_status":"approved","evidence":{}}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn vault_release_read_requires_a_release_role() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "tax:vault:read");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/vault/release-claims/{}", Uuid::now_v7()))
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn replay_gap_operations_require_the_exact_system_operator_role() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "tax:indexer:read");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .uri("/v1/indexer/replay-gaps?stream_key=mainnet%2Fsettlement")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn settlement_compile_fails_closed_without_authentication() {
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: None,
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/compile")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn settlement_compile_returns_prepared_instruction_for_scoped_principal() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "tax:settlement:compile");
        let body = serde_json::json!({
            "program_id": SETTLEMENT_PROGRAM_ID,
            "payer": "cGfHiC6Kgg3FpFZvgwGcswsCRtp4aBP2fzuXRQPizuN",
            "merchant": "11111111111111111111111111111114",
            "source": "SysvarRent111111111111111111111111111111111",
            "mint": "SysvarC1ock11111111111111111111111111111111",
            "tax_vault": "Sysvar1nstructions1111111111111111111111111",
            "net_destination": "SysvarRecentB1ockHashes11111111111111111111",
            "token_program": SPL_TOKEN_PROGRAM_ID,
            "nonce": 9,
            "policy_sequence": 3,
            "gross_atomic": "100",
            "tax_atomic": "25",
            "net_atomic": "75",
            "policy_digest": [7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7]
        });
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: true,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/compile")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let response_body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        let response: serde_json::Value = serde_json::from_slice(&response_body).unwrap();
        assert_eq!(response["status"], "prepared_unsubmitted");
        assert_eq!(response["accounts"].as_array().unwrap().len(), 12);
    }

    #[tokio::test]
    async fn amount_bearing_settlement_compile_is_disabled_outside_simulation() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let (token, _) = auth::test_token_with_scope(now, "tax:settlement:compile");
        let response = router()
            .with_state(AppState {
                ready: false,
                simulation_mode: false,
                storage: None,
                authenticator: Some(auth::test_authenticator()),
                jurisdiction_provider_registry: None,
                settlement_coordinator: None,
                targeted_recovery: None,
                metrics: test_metrics(),
            })
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/compile")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[test]
    fn finalized_refund_claim_evidence_response_preserves_typed_chain_binding() {
        let evidence = FinalizedRefundClaimEvidence {
            signature: "5VfY7WmYcR7h9o9E9nP7pQ8rS".to_string(),
            slot: 77,
            event_index: 2,
            claim: [1; 32],
            adjustment: [2; 32],
            original_receipt: [3; 32],
            merchant: [4; 32],
            mint: [5; 32],
            nonce: 3,
            gross_magnitude: 100,
            tax_magnitude: 25,
            net_magnitude: 75,
            refund_funder: [6; 32],
            refund_destination: [7; 32],
            proposer: [8; 32],
            reference_digest: [9; 32],
            execute_after_unix: 1_800_000_000,
            expires_at_unix: 1_800_000_600,
            evidence_digest: [10; 32],
        };

        let response = finalized_refund_claim_evidence_response(evidence);
        assert_eq!(response.event_type, "AdjustmentRefundClaimCreated");
        assert_eq!(response.event_version, "v1");
        assert_eq!(response.commitment, "finalized");
        assert_eq!(response.signature, "5VfY7WmYcR7h9o9E9nP7pQ8rS");
        assert_eq!(response.slot, 77);
        assert_eq!(response.event_index, 2);
        assert_eq!(response.gross_magnitude, 100);
        assert_eq!(
            response.tax_magnitude + response.net_magnitude,
            response.gross_magnitude
        );

        let json = serde_json::to_value(response).unwrap();
        assert_eq!(json["slot"], "77");
        assert_eq!(json["nonce"], "3");
        assert_eq!(json["gross_magnitude"], "100");
        assert_eq!(json["tax_magnitude"], "25");
        assert_eq!(json["net_magnitude"], "75");
    }

    #[test]
    fn authenticator_accepts_a_scoped_signed_token() {
        let authenticator = auth::test_authenticator();
        let (token, merchant_id) = auth::test_token(1_800_000_000);
        let principal = authenticator
            .authenticate(
                Some(&HeaderValue::from_str(&format!("Bearer {token}")).unwrap()),
                1_800_000_100,
            )
            .unwrap();
        assert_eq!(principal.merchant_id, merchant_id);
    }
}
