#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use settlement_client::{
    CompiledSettlementRefundInstruction, SettlementCompileError,
    SettlementRefundClaimCompileRequest, SettlementRefundExecutionCompileRequest,
    compile_settlement_refund_claim_instruction, compile_settlement_refund_execution_instruction,
    derive_source_token_account,
};
use sha2::{Digest, Sha256};
use solana_pubkey::Pubkey;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

const ARTIFACT_SCHEMA: &str = "refund-movement-artifact/v1";
const SPL_TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

#[derive(Debug, Error)]
pub enum RefundWorkflowError {
    #[error("refund workflow identity is not UUIDv7")]
    InvalidIdentity,
    #[error("refund workflow address is invalid: {0}")]
    InvalidAddress(&'static str),
    #[error("refund workflow subject is invalid: {0}")]
    InvalidSubject(&'static str),
    #[error("refund amount conservation is invalid")]
    AmountConservation,
    #[error("refund amounts must be positive and bounded")]
    InvalidAmount,
    #[error("refund movement supports only the canonical legacy SPL token program")]
    UnsupportedTokenProgram,
    #[error("refund workflow digest must not be zero")]
    EmptyDigest,
    #[error("refund claim window is invalid")]
    InvalidWindow,
    #[error("refund compiler failed: {0}")]
    Compiler(#[from] SettlementCompileError),
    #[error("refund artifact serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundClaimServerInputs {
    pub merchant_id: Uuid,
    pub adjustment_id: Uuid,
    pub funding_configuration_id: Uuid,
    pub funding_configuration_digest: [u8; 32],
    pub receipt_signature: String,
    pub receipt_slot: u64,
    pub original_receipt: [u8; 32],
    pub adjustment: [u8; 32],
    pub payer: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub token_program: String,
    pub proposer: String,
    pub proposer_subject: String,
    pub refund_funder: String,
    pub refund_funder_subject: String,
    pub tax_vault: String,
    pub net_source: String,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub reference_digest: [u8; 32],
    pub claim_nonce: u64,
    pub execute_after_unix: i64,
    pub expires_at_unix: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundExecuteServerInputs {
    pub merchant_id: Uuid,
    pub adjustment_id: Uuid,
    pub funding_configuration_id: Uuid,
    pub funding_configuration_digest: [u8; 32],
    pub receipt_signature: String,
    pub receipt_slot: u64,
    pub original_receipt: [u8; 32],
    pub adjustment: [u8; 32],
    pub payer: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub token_program: String,
    pub approver: String,
    pub refund_funder: String,
    pub refund_funder_subject: String,
    pub tax_vault: String,
    pub net_source: String,
    pub claim_nonce: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RefundArtifact {
    pub schema: String,
    pub instruction: CompiledSettlementRefundInstruction,
    pub instruction_digest: [u8; 32],
    pub artifact_digest: [u8; 32],
    pub refund_destination: String,
    pub merchant_id: Uuid,
    pub adjustment_id: Uuid,
    pub funding_configuration_id: Uuid,
    pub funding_configuration_digest: [u8; 32],
    pub receipt_signature: String,
    #[serde(with = "policy::serde_optional_u64_string")]
    pub receipt_slot: Option<u64>,
    pub authority_subject: String,
    pub proposer_subject: String,
    pub refund_funder_subject: String,
    pub proposer: String,
    pub refund_funder: String,
    pub merchant: String,
    pub mint: String,
    pub token_program: String,
    pub tax_vault: String,
    pub net_source: String,
    pub gross_atomic: Option<String>,
    pub tax_atomic: Option<String>,
    pub net_atomic: Option<String>,
    pub execute_after_unix: Option<i64>,
    pub expires_at_unix: Option<i64>,
}

impl RefundArtifact {
    pub fn unsigned_artifact(&self) -> Result<Value, RefundWorkflowError> {
        Ok(serde_json::to_value(self)?)
    }
}

pub fn build_claim_artifact(
    input: &RefundClaimServerInputs,
) -> Result<RefundArtifact, RefundWorkflowError> {
    validate_common(&CommonValidation {
        merchant_id: input.merchant_id,
        adjustment_id: input.adjustment_id,
        funding_configuration_id: input.funding_configuration_id,
        funding_configuration_digest: input.funding_configuration_digest,
        receipt_signature: input.receipt_signature.as_str(),
        receipt_slot: input.receipt_slot,
        original_receipt: input.original_receipt,
        merchant: input.merchant,
        mint: input.mint,
        token_program: input.token_program.as_str(),
        refund_funder: input.refund_funder.as_str(),
        refund_funder_subject: input.refund_funder_subject.as_str(),
        tax_vault: input.tax_vault.as_str(),
        net_source: input.net_source.as_str(),
    })?;
    if input.proposer_subject.trim().is_empty()
        || input.proposer_subject.len() > 256
        || input
            .proposer_subject
            .bytes()
            .any(|byte| byte.is_ascii_control())
    {
        return Err(RefundWorkflowError::InvalidSubject("proposer_subject"));
    }
    if input.gross_atomic == 0 {
        return Err(RefundWorkflowError::InvalidAmount);
    }
    if input.tax_atomic.checked_add(input.net_atomic) != Some(input.gross_atomic) {
        return Err(RefundWorkflowError::AmountConservation);
    }
    if input.claim_nonce == 0
        || input.reference_digest == [0; 32]
        || input.execute_after_unix <= 0
        || input.expires_at_unix <= input.execute_after_unix
    {
        return Err(RefundWorkflowError::InvalidWindow);
    }
    let proposer = canonical_address(input.proposer.as_str(), "proposer")?;
    let refund_funder = canonical_address(input.refund_funder.as_str(), "refund_funder")?;
    if proposer == refund_funder {
        return Err(RefundWorkflowError::InvalidSubject(
            "claim and funder authorities must differ",
        ));
    }
    let refund_destination = derive_source_token_account(
        &canonical_bytes(input.payer, "payer")?,
        &canonical_bytes(input.mint, "mint")?,
        input.token_program.as_str(),
    )?;
    let request = SettlementRefundClaimCompileRequest {
        program_id: settlement_client::SETTLEMENT_PROGRAM_ID.to_string(),
        proposer: proposer.clone(),
        refund_funder,
        merchant: canonical_bytes(input.merchant, "merchant")?,
        mint: canonical_bytes(input.mint, "mint")?,
        token_program: input.token_program.clone(),
        original_receipt: canonical_bytes(input.original_receipt, "original_receipt")?,
        adjustment: canonical_bytes(input.adjustment, "adjustment")?,
        tax_vault: canonical_address(input.tax_vault.as_str(), "tax_vault")?,
        net_source: canonical_address(input.net_source.as_str(), "net_source")?,
        refund_destination: refund_destination.clone(),
        claim_nonce: input.claim_nonce,
        execute_after_unix: input.execute_after_unix,
        expires_at_unix: input.expires_at_unix,
        reference_digest: input.reference_digest,
    };
    let instruction = compile_settlement_refund_claim_instruction(&request)?;
    finish_artifact(
        instruction,
        ArtifactContext {
            merchant_id: input.merchant_id,
            adjustment_id: input.adjustment_id,
            funding_configuration_id: input.funding_configuration_id,
            funding_configuration_digest: input.funding_configuration_digest,
            receipt_signature: input.receipt_signature.clone(),
            receipt_slot: Some(input.receipt_slot),
            authority_subject: proposer,
            proposer_subject: input.proposer_subject.clone(),
            refund_funder_subject: input.refund_funder_subject.clone(),
            proposer: input.proposer.clone(),
            refund_funder: input.refund_funder.clone(),
            merchant: Pubkey::new_from_array(input.merchant).to_string(),
            mint: Pubkey::new_from_array(input.mint).to_string(),
            token_program: input.token_program.clone(),
            tax_vault: input.tax_vault.clone(),
            net_source: input.net_source.clone(),
            gross_atomic: Some(input.gross_atomic.to_string()),
            tax_atomic: Some(input.tax_atomic.to_string()),
            net_atomic: Some(input.net_atomic.to_string()),
            execute_after_unix: Some(input.execute_after_unix),
            expires_at_unix: Some(input.expires_at_unix),
        },
        refund_destination,
    )
}

/// Computes the canonical preparation digest for idempotency and evidence.
/// The claim nonce and preparation window are deliberately excluded because
/// they are server-allocated; all other authoritative inputs are
/// length-delimited to prevent ambiguous concatenations.
pub fn claim_request_digest(input: &RefundClaimServerInputs) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"refund-claim-request/v1");
    hasher.update(input.merchant_id.as_bytes());
    hasher.update(input.adjustment_id.as_bytes());
    hasher.update(input.funding_configuration_id.as_bytes());
    hasher.update(input.funding_configuration_digest);
    update_digest_text(&mut hasher, &input.receipt_signature);
    hasher.update(input.receipt_slot.to_le_bytes());
    hasher.update(input.original_receipt);
    hasher.update(input.adjustment);
    hasher.update(input.payer);
    hasher.update(input.merchant);
    hasher.update(input.mint);
    update_digest_text(&mut hasher, &input.token_program);
    update_digest_text(&mut hasher, &input.proposer);
    update_digest_text(&mut hasher, &input.proposer_subject);
    update_digest_text(&mut hasher, &input.refund_funder);
    update_digest_text(&mut hasher, &input.refund_funder_subject);
    update_digest_text(&mut hasher, &input.tax_vault);
    update_digest_text(&mut hasher, &input.net_source);
    hasher.update(input.gross_atomic.to_le_bytes());
    hasher.update(input.tax_atomic.to_le_bytes());
    hasher.update(input.net_atomic.to_le_bytes());
    hasher.update(input.reference_digest);
    hasher.finalize().into()
}

/// Recompiles and compares a claim artifact at an authority-sensitive storage
/// boundary. This makes the persisted artifact self-validating instead of
/// relying on the caller having used the builder correctly.
pub fn validate_claim_artifact(
    input: &RefundClaimServerInputs,
    artifact: &RefundArtifact,
) -> Result<(), RefundWorkflowError> {
    let expected = build_claim_artifact(input)?;
    if expected.instruction != artifact.instruction
        || expected.instruction_digest != artifact.instruction_digest
        || expected.artifact_digest != artifact.artifact_digest
        || expected.refund_destination != artifact.refund_destination
        || expected.merchant_id != artifact.merchant_id
        || expected.adjustment_id != artifact.adjustment_id
        || expected.funding_configuration_id != artifact.funding_configuration_id
        || expected.funding_configuration_digest != artifact.funding_configuration_digest
        || expected.receipt_signature != artifact.receipt_signature
        || expected.receipt_slot != artifact.receipt_slot
        || expected.authority_subject != artifact.authority_subject
        || expected.proposer_subject != artifact.proposer_subject
        || expected.refund_funder_subject != artifact.refund_funder_subject
        || expected.proposer != artifact.proposer
        || expected.refund_funder != artifact.refund_funder
        || expected.merchant != artifact.merchant
        || expected.mint != artifact.mint
        || expected.token_program != artifact.token_program
        || expected.tax_vault != artifact.tax_vault
        || expected.net_source != artifact.net_source
        || expected.gross_atomic != artifact.gross_atomic
        || expected.tax_atomic != artifact.tax_atomic
        || expected.net_atomic != artifact.net_atomic
        || expected.execute_after_unix != artifact.execute_after_unix
        || expected.expires_at_unix != artifact.expires_at_unix
    {
        return Err(RefundWorkflowError::InvalidIdentity);
    }
    Ok(())
}

pub fn refund_instruction_digest(
    instruction: &CompiledSettlementRefundInstruction,
) -> Result<[u8; 32], RefundWorkflowError> {
    let instruction_bytes = serde_json::to_vec(instruction)?;
    Ok(Sha256::digest([b"refund-instruction/v1".as_slice(), &instruction_bytes].concat()).into())
}

fn update_digest_text(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

pub fn build_execute_artifact(
    input: &RefundExecuteServerInputs,
) -> Result<RefundArtifact, RefundWorkflowError> {
    validate_common(&CommonValidation {
        merchant_id: input.merchant_id,
        adjustment_id: input.adjustment_id,
        funding_configuration_id: input.funding_configuration_id,
        funding_configuration_digest: input.funding_configuration_digest,
        receipt_signature: input.receipt_signature.as_str(),
        receipt_slot: input.receipt_slot,
        original_receipt: input.original_receipt,
        merchant: input.merchant,
        mint: input.mint,
        token_program: input.token_program.as_str(),
        refund_funder: input.refund_funder.as_str(),
        refund_funder_subject: input.refund_funder_subject.as_str(),
        tax_vault: input.tax_vault.as_str(),
        net_source: input.net_source.as_str(),
    })?;
    let approver = canonical_address(input.approver.as_str(), "approver")?;
    let refund_funder = canonical_address(input.refund_funder.as_str(), "refund_funder")?;
    if approver == refund_funder {
        return Err(RefundWorkflowError::InvalidSubject(
            "execute and funder authorities must differ",
        ));
    }
    let refund_destination = derive_source_token_account(
        &canonical_bytes(input.payer, "payer")?,
        &canonical_bytes(input.mint, "mint")?,
        input.token_program.as_str(),
    )?;
    let instruction = compile_settlement_refund_execution_instruction(
        &SettlementRefundExecutionCompileRequest {
            program_id: settlement_client::SETTLEMENT_PROGRAM_ID.to_string(),
            approver: approver.clone(),
            refund_funder,
            merchant: canonical_bytes(input.merchant, "merchant")?,
            mint: canonical_bytes(input.mint, "mint")?,
            token_program: input.token_program.clone(),
            original_receipt: canonical_bytes(input.original_receipt, "original_receipt")?,
            adjustment: canonical_bytes(input.adjustment, "adjustment")?,
            tax_vault: canonical_address(input.tax_vault.as_str(), "tax_vault")?,
            net_source: canonical_address(input.net_source.as_str(), "net_source")?,
            refund_destination: refund_destination.clone(),
            claim_nonce: input.claim_nonce,
        },
    )?;
    finish_artifact(
        instruction,
        ArtifactContext {
            merchant_id: input.merchant_id,
            adjustment_id: input.adjustment_id,
            funding_configuration_id: input.funding_configuration_id,
            funding_configuration_digest: input.funding_configuration_digest,
            receipt_signature: input.receipt_signature.clone(),
            receipt_slot: Some(input.receipt_slot),
            authority_subject: approver,
            proposer_subject: String::new(),
            refund_funder_subject: input.refund_funder_subject.clone(),
            proposer: String::new(),
            refund_funder: input.refund_funder.clone(),
            merchant: Pubkey::new_from_array(input.merchant).to_string(),
            mint: Pubkey::new_from_array(input.mint).to_string(),
            token_program: input.token_program.clone(),
            tax_vault: input.tax_vault.clone(),
            net_source: input.net_source.clone(),
            gross_atomic: None,
            tax_atomic: None,
            net_atomic: None,
            execute_after_unix: None,
            expires_at_unix: None,
        },
        refund_destination,
    )
}

struct CommonValidation<'a> {
    merchant_id: Uuid,
    adjustment_id: Uuid,
    funding_configuration_id: Uuid,
    funding_configuration_digest: [u8; 32],
    receipt_signature: &'a str,
    receipt_slot: u64,
    original_receipt: [u8; 32],
    merchant: [u8; 32],
    mint: [u8; 32],
    token_program: &'a str,
    refund_funder: &'a str,
    refund_funder_subject: &'a str,
    tax_vault: &'a str,
    net_source: &'a str,
}

fn validate_common(input: &CommonValidation<'_>) -> Result<(), RefundWorkflowError> {
    if input.merchant_id.get_version_num() != 7
        || input.adjustment_id.get_version_num() != 7
        || input.funding_configuration_id.get_version_num() != 7
        || input.funding_configuration_digest == [0; 32]
        || input.receipt_signature.trim().is_empty()
        || input.receipt_signature.len() > 128
        || input.receipt_slot == 0
        || input.original_receipt == [0; 32]
        || input.merchant == [0; 32]
        || input.mint == [0; 32]
        || input.token_program != SPL_TOKEN_PROGRAM
    {
        return Err(RefundWorkflowError::InvalidIdentity);
    }
    for (field, value) in [
        ("refund_funder", input.refund_funder),
        ("refund_funder_subject", input.refund_funder_subject),
        ("tax_vault", input.tax_vault),
        ("net_source", input.net_source),
    ] {
        if value.trim().is_empty()
            || value.len() > 256
            || value.bytes().any(|byte| byte.is_ascii_control())
        {
            return Err(RefundWorkflowError::InvalidSubject(field));
        }
    }
    canonical_address(input.tax_vault, "tax_vault")?;
    canonical_address(input.net_source, "net_source")?;
    Ok(())
}

fn canonical_address(value: &str, field: &'static str) -> Result<String, RefundWorkflowError> {
    let key = Pubkey::from_str(value).map_err(|_| RefundWorkflowError::InvalidAddress(field))?;
    if key == Pubkey::default() || key.to_string() != value {
        return Err(RefundWorkflowError::InvalidAddress(field));
    }
    Ok(key.to_string())
}

fn canonical_bytes(value: [u8; 32], field: &'static str) -> Result<String, RefundWorkflowError> {
    if value == [0; 32] {
        return Err(RefundWorkflowError::InvalidAddress(field));
    }
    Ok(Pubkey::new_from_array(value).to_string())
}

struct ArtifactContext {
    merchant_id: Uuid,
    adjustment_id: Uuid,
    funding_configuration_id: Uuid,
    funding_configuration_digest: [u8; 32],
    receipt_signature: String,
    receipt_slot: Option<u64>,
    authority_subject: String,
    proposer_subject: String,
    refund_funder_subject: String,
    proposer: String,
    refund_funder: String,
    merchant: String,
    mint: String,
    token_program: String,
    tax_vault: String,
    net_source: String,
    gross_atomic: Option<String>,
    tax_atomic: Option<String>,
    net_atomic: Option<String>,
    execute_after_unix: Option<i64>,
    expires_at_unix: Option<i64>,
}

fn finish_artifact(
    instruction: CompiledSettlementRefundInstruction,
    context: ArtifactContext,
    refund_destination: String,
) -> Result<RefundArtifact, RefundWorkflowError> {
    let instruction_digest = refund_instruction_digest(&instruction)?;
    let mut artifact_hasher = Sha256::new();
    artifact_hasher.update(b"refund-artifact/v1");
    artifact_hasher.update(context.merchant_id.as_bytes());
    artifact_hasher.update(context.adjustment_id.as_bytes());
    artifact_hasher.update(context.funding_configuration_id.as_bytes());
    artifact_hasher.update(context.funding_configuration_digest);
    artifact_hasher.update(context.receipt_signature.as_bytes());
    artifact_hasher.update(context.receipt_slot.unwrap_or_default().to_le_bytes());
    artifact_hasher.update(instruction_digest);
    artifact_hasher.update(context.authority_subject.as_bytes());
    artifact_hasher.update(context.proposer_subject.as_bytes());
    artifact_hasher.update(context.refund_funder_subject.as_bytes());
    let artifact_digest = artifact_hasher.finalize().into();
    Ok(RefundArtifact {
        schema: ARTIFACT_SCHEMA.to_string(),
        instruction,
        instruction_digest,
        artifact_digest,
        refund_destination,
        merchant_id: context.merchant_id,
        adjustment_id: context.adjustment_id,
        funding_configuration_id: context.funding_configuration_id,
        funding_configuration_digest: context.funding_configuration_digest,
        receipt_signature: context.receipt_signature,
        receipt_slot: context.receipt_slot,
        authority_subject: context.authority_subject,
        proposer_subject: context.proposer_subject,
        refund_funder_subject: context.refund_funder_subject,
        proposer: context.proposer,
        refund_funder: context.refund_funder,
        merchant: context.merchant,
        mint: context.mint,
        token_program: context.token_program,
        tax_vault: context.tax_vault,
        net_source: context.net_source,
        gross_atomic: context.gross_atomic,
        tax_atomic: context.tax_atomic,
        net_atomic: context.net_atomic,
        execute_after_unix: context.execute_after_unix,
        expires_at_unix: context.expires_at_unix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim() -> RefundClaimServerInputs {
        RefundClaimServerInputs {
            merchant_id: Uuid::now_v7(),
            adjustment_id: Uuid::now_v7(),
            funding_configuration_id: Uuid::now_v7(),
            funding_configuration_digest: [1; 32],
            receipt_signature: "5Ux123456789012345678901234567890123456789012345678901234567890"
                .into(),
            receipt_slot: 7,
            original_receipt: [2; 32],
            adjustment: [7; 32],
            payer: [3; 32],
            merchant: [4; 32],
            mint: [5; 32],
            token_program: SPL_TOKEN_PROGRAM.into(),
            proposer: Pubkey::new_unique().to_string(),
            proposer_subject: "proposer".into(),
            refund_funder: Pubkey::new_unique().to_string(),
            refund_funder_subject: "funder".into(),
            tax_vault: Pubkey::new_unique().to_string(),
            net_source: Pubkey::new_unique().to_string(),
            gross_atomic: 100,
            tax_atomic: 25,
            net_atomic: 75,
            reference_digest: [6; 32],
            claim_nonce: 9,
            execute_after_unix: 1_790_000_000,
            expires_at_unix: 1_790_003_600,
        }
    }

    #[test]
    fn derives_destination_and_binds_server_context() {
        let input = claim();
        let artifact = build_claim_artifact(&input).expect("artifact");
        assert_eq!(artifact.schema, ARTIFACT_SCHEMA);
        assert_eq!(artifact.receipt_slot, Some(input.receipt_slot));
        assert_eq!(artifact.proposer_subject, input.proposer_subject);
        assert_eq!(artifact.gross_atomic.as_deref(), Some("100"));
        assert_ne!(artifact.instruction_digest, [0; 32]);
        assert_ne!(artifact.artifact_digest, [0; 32]);
        assert_eq!(artifact.instruction.claim_nonce, input.claim_nonce);
        assert!(artifact.unsigned_artifact().expect("json").is_object());
    }

    #[test]
    fn rejects_non_conserving_or_non_legacy_inputs() {
        let mut input = claim();
        input.net_atomic = 74;
        assert!(matches!(
            build_claim_artifact(&input),
            Err(RefundWorkflowError::AmountConservation)
        ));
        let mut input = claim();
        input.token_program = Pubkey::new_unique().to_string();
        assert!(matches!(
            build_claim_artifact(&input),
            Err(RefundWorkflowError::InvalidIdentity)
        ));
    }

    #[test]
    fn request_digest_excludes_server_allocated_nonce() {
        let first = claim();
        let mut second = first.clone();
        second.claim_nonce += 1;
        assert_eq!(claim_request_digest(&first), claim_request_digest(&second));
        second.execute_after_unix += 1;
        second.expires_at_unix += 1;
        assert_eq!(claim_request_digest(&first), claim_request_digest(&second));
        second.net_atomic += 1;
        assert_ne!(claim_request_digest(&first), claim_request_digest(&second));
    }
}
