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
    pub receipt_slot: Option<u64>,
    pub authority_subject: String,
    pub refund_funder_subject: String,
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
            refund_funder_subject: input.refund_funder_subject.clone(),
        },
        refund_destination,
    )
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
            refund_funder_subject: input.refund_funder_subject.clone(),
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
    refund_funder_subject: String,
}

fn finish_artifact(
    instruction: CompiledSettlementRefundInstruction,
    context: ArtifactContext,
    refund_destination: String,
) -> Result<RefundArtifact, RefundWorkflowError> {
    let instruction_bytes = serde_json::to_vec(&instruction)?;
    let instruction_digest =
        Sha256::digest([b"refund-instruction/v1".as_slice(), &instruction_bytes].concat()).into();
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
        refund_funder_subject: context.refund_funder_subject,
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
}
