#![deny(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use anchor_lang::prelude::Pubkey;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use split_plan::{MAX_RECIPIENTS, SplitPlan, SplitRecipient};
use std::{collections::BTreeSet, str::FromStr};
use thiserror::Error;

pub const SETTLEMENT_PROGRAM_ID: &str = "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi";
pub const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";
pub const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
pub const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
pub const COMPUTE_BUDGET_PROGRAM_ID: &str = "ComputeBudget111111111111111111111111111111";
pub const COMPUTE_BUDGET_POLICY_VERSION: u32 = 1;
pub const DEFAULT_COMPUTE_UNIT_LIMIT: u32 = 200_000;
pub const DEFAULT_MICRO_LAMPORTS_PER_COMPUTE_UNIT: u64 = 1_000;
pub const MAX_COMPUTE_UNIT_LIMIT: u32 = 1_400_000;
pub const MAX_MICRO_LAMPORTS_PER_COMPUTE_UNIT: u64 = 10_000_000;
pub const SETTLEMENT_INSTRUCTION_ACCOUNT_COUNT: usize = 13;
pub const SPLIT_SETTLEMENT_INSTRUCTION_BASE_ACCOUNT_COUNT: usize = 13;
pub const SOLANA_LEGACY_PACKET_BYTES: usize = 1_232;
pub const SETTLEMENT_PACKET_SAFETY_MARGIN_BYTES: usize = 256;
pub const MAX_SETTLEMENT_SIGNED_BYTES: usize =
    SOLANA_LEGACY_PACKET_BYTES - SETTLEMENT_PACKET_SAFETY_MARGIN_BYTES;
pub const MAX_SETTLEMENT_INSTRUCTION_ACCOUNT_COUNT: usize =
    SPLIT_SETTLEMENT_INSTRUCTION_BASE_ACCOUNT_COUNT + MAX_RECIPIENTS;
pub const MAX_REFUND_CLAIM_WINDOW_SECONDS: i64 = 90 * 24 * 60 * 60;

// These byte arrays are the canonical decoded values of the fixed public IDs
// above. Keeping them as data removes a runtime panic from the hot path while
// the test suite proves the string/byte compatibility contract.
const SETTLEMENT_PROGRAM_BYTES: [u8; 32] = [1; 32];
const SPL_TOKEN_PROGRAM_BYTES: [u8; 32] = [
    6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180, 133, 237,
    95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
];

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SettlementCompileRequest {
    pub program_id: String,
    pub payer: String,
    pub merchant: String,
    pub source: String,
    pub mint: String,
    pub tax_vault: String,
    pub net_destination: String,
    pub token_program: String,
    pub nonce: u64,
    pub policy_sequence: u64,
    pub gross_atomic: String,
    pub tax_atomic: String,
    pub net_atomic: String,
    #[serde(deserialize_with = "deserialize_digest")]
    pub policy_digest: [u8; 32],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DocumentSettlementCompileRequest {
    pub program_id: String,
    pub payer: String,
    pub merchant: String,
    pub source: String,
    pub mint: String,
    pub tax_vault: String,
    pub net_destination: String,
    pub token_program: String,
    pub nonce: u64,
    pub policy_sequence: u64,
    pub gross_atomic: String,
    pub tax_atomic: String,
    pub net_atomic: String,
    pub policy_digest: [u8; 32],
    pub document_attestation_digest: [u8; 32],
    pub payment_scope_digest: [u8; 32],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SplitSettlementCompileRequest {
    pub program_id: String,
    pub payer: String,
    pub merchant: String,
    pub source: String,
    pub mint: String,
    pub tax_vault: String,
    pub token_program: String,
    pub nonce: u64,
    pub policy_sequence: u64,
    pub split_plan_version: u64,
    pub recipients: Vec<SplitRecipient>,
    pub gross_atomic: String,
    pub tax_atomic: String,
    pub net_atomic: String,
    #[serde(deserialize_with = "deserialize_digest")]
    pub policy_digest: [u8; 32],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SettlementAdjustmentCompileRequest {
    pub program_id: String,
    pub proposer: String,
    pub approver: String,
    pub merchant: String,
    pub mint: String,
    pub token_program: String,
    pub original_receipt: String,
    #[serde(with = "policy::serde_u64_string")]
    pub adjustment_nonce: u64,
    pub kind: u8,
    pub gross_sign: u8,
    pub gross_magnitude: String,
    pub tax_sign: u8,
    pub tax_magnitude: String,
    pub net_sign: u8,
    pub net_magnitude: String,
    pub adjustment_digest: [u8; 32],
    pub evidence_digest: [u8; 32],
    pub source_calculation_digest: [u8; 32],
    pub replacement_calculation_digest: [u8; 32],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SettlementRefundClaimCompileRequest {
    pub program_id: String,
    pub proposer: String,
    pub refund_funder: String,
    pub merchant: String,
    pub mint: String,
    pub token_program: String,
    pub original_receipt: String,
    pub adjustment: String,
    pub tax_vault: String,
    pub net_source: String,
    pub refund_destination: String,
    #[serde(with = "policy::serde_u64_string")]
    pub claim_nonce: u64,
    pub execute_after_unix: i64,
    pub expires_at_unix: i64,
    #[serde(deserialize_with = "deserialize_digest")]
    pub reference_digest: [u8; 32],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SettlementRefundExecutionCompileRequest {
    pub program_id: String,
    pub approver: String,
    pub refund_funder: String,
    pub merchant: String,
    pub mint: String,
    pub token_program: String,
    pub original_receipt: String,
    pub adjustment: String,
    pub tax_vault: String,
    pub net_source: String,
    pub refund_destination: String,
    #[serde(with = "policy::serde_u64_string")]
    pub claim_nonce: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SettlementRefundCancellationCompileRequest {
    pub program_id: String,
    pub governance_authority: String,
    pub merchant: String,
    pub mint: String,
    pub adjustment: String,
    #[serde(with = "policy::serde_u64_string")]
    pub claim_nonce: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompiledSettlementInstruction {
    pub status: String,
    pub program_id: String,
    pub instruction_data_base64: String,
    pub accounts: Vec<CompiledAccountMeta>,
    pub protocol_config: String,
    pub vault_authority: String,
    pub policy_attestation: String,
    pub receipt: String,
    pub asset_config: String,
    pub compute_budget_policy: ComputeBudgetPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompiledDocumentSettlementInstruction {
    pub status: String,
    pub program_id: String,
    pub instruction_data_base64: String,
    pub accounts: Vec<CompiledAccountMeta>,
    pub protocol_config: String,
    pub vault_authority: String,
    pub policy_attestation: String,
    pub receipt: String,
    pub asset_config: String,
    pub document_attestation_digest: Vec<u8>,
    pub payment_scope_digest: Vec<u8>,
    pub compute_budget_policy: ComputeBudgetPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompiledSplitSettlementInstruction {
    pub status: String,
    pub program_id: String,
    pub instruction_data_base64: String,
    pub accounts: Vec<CompiledAccountMeta>,
    pub protocol_config: String,
    pub vault_authority: String,
    pub policy_attestation: String,
    pub split_plan: String,
    pub split_plan_digest: Vec<u8>,
    pub receipt: String,
    pub asset_config: String,
    pub compute_budget_policy: ComputeBudgetPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComputeBudgetPolicy {
    pub policy_version: u32,
    pub compute_unit_limit: u32,
    pub micro_lamports_per_compute_unit: u64,
}

impl Default for ComputeBudgetPolicy {
    fn default() -> Self {
        Self {
            policy_version: COMPUTE_BUDGET_POLICY_VERSION,
            compute_unit_limit: DEFAULT_COMPUTE_UNIT_LIMIT,
            micro_lamports_per_compute_unit: DEFAULT_MICRO_LAMPORTS_PER_COMPUTE_UNIT,
        }
    }
}

impl ComputeBudgetPolicy {
    pub fn validate(&self) -> Result<(), SettlementCompileError> {
        if self.policy_version != COMPUTE_BUDGET_POLICY_VERSION
            || self.compute_unit_limit == 0
            || self.compute_unit_limit > MAX_COMPUTE_UNIT_LIMIT
            || self.micro_lamports_per_compute_unit > MAX_MICRO_LAMPORTS_PER_COMPUTE_UNIT
        {
            return Err(SettlementCompileError::InvalidComputeBudgetPolicy);
        }
        Ok(())
    }

    pub fn set_compute_unit_limit_data(&self) -> Vec<u8> {
        let mut data = vec![2_u8];
        data.extend_from_slice(&self.compute_unit_limit.to_le_bytes());
        data
    }

    pub fn set_compute_unit_price_data(&self) -> Vec<u8> {
        let mut data = vec![3_u8];
        data.extend_from_slice(&self.micro_lamports_per_compute_unit.to_le_bytes());
        data
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompiledSettlementAdjustmentInstruction {
    pub status: String,
    pub program_id: String,
    pub instruction_data_base64: String,
    pub accounts: Vec<CompiledAccountMeta>,
    pub adjustment: String,
    pub adjustment_registry: String,
    pub original_receipt: String,
    #[serde(with = "policy::serde_u64_string")]
    pub adjustment_nonce: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompiledSettlementRefundInstruction {
    pub status: String,
    pub program_id: String,
    pub instruction_data_base64: String,
    pub accounts: Vec<CompiledAccountMeta>,
    pub registry: String,
    pub adjustment: String,
    pub original_receipt: String,
    pub claim: String,
    #[serde(with = "policy::serde_u64_string")]
    pub claim_nonce: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompiledSettlementRefundCancellationInstruction {
    pub status: String,
    pub program_id: String,
    pub instruction_data_base64: String,
    pub accounts: Vec<CompiledAccountMeta>,
    pub registry: String,
    pub claim: String,
    #[serde(with = "policy::serde_u64_string")]
    pub claim_nonce: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RevenueStreamPlanCompileRequest {
    pub program_id: String,
    pub governance_authority: String,
    pub merchant: String,
    pub mint: String,
    pub token_program: String,
    pub tax_vault: String,
    pub recipient_owner: String,
    pub proposer: String,
    pub approver: String,
    pub operator: String,
    pub plan_digest: [u8; 32],
    pub policy_digest: [u8; 32],
    #[serde(with = "policy::serde_u64_string")]
    pub start_slot: u64,
    #[serde(default, with = "policy::serde_optional_u64_string")]
    pub end_slot: Option<u64>,
    #[serde(with = "policy::serde_u64_string")]
    pub max_total_atomic: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub max_claim_atomic: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RevenueStreamAccrualCompileRequest {
    pub program_id: String,
    pub operator: String,
    pub plan: String,
    pub source_receipt: String,
    #[serde(with = "policy::serde_u64_string")]
    pub recipient_atomic: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub finalized_slot: u64,
    pub evidence_digest: [u8; 32],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RevenueStreamClaimCompileRequest {
    pub program_id: String,
    pub operator: String,
    pub plan: String,
    pub mint: String,
    pub token_program: String,
    pub tax_vault: String,
    pub destination: String,
    #[serde(with = "policy::serde_u64_string")]
    pub nonce: u64,
    #[serde(with = "policy::serde_u64_string")]
    pub amount_atomic: u64,
    pub claim_digest: [u8; 32],
    pub evidence_digest: [u8; 32],
    pub execute_after_unix: i64,
    pub expires_at_unix: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RevenueStreamClaimExecutionCompileRequest {
    pub program_id: String,
    pub approver: String,
    pub plan: String,
    pub merchant: String,
    pub mint: String,
    pub token_program: String,
    pub tax_vault: String,
    pub destination: String,
    #[serde(with = "policy::serde_u64_string")]
    pub nonce: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RevenueStreamClaimCancellationCompileRequest {
    pub program_id: String,
    pub governance_authority: String,
    pub plan: String,
    #[serde(with = "policy::serde_u64_string")]
    pub nonce: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompiledRevenueStreamInstruction {
    pub status: String,
    pub program_id: String,
    pub instruction_data_base64: String,
    pub accounts: Vec<CompiledAccountMeta>,
    pub plan: String,
    pub accrual: Option<String>,
    pub claim: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompiledAccountMeta {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SettlementCompileError {
    #[error("invalid public key for {field}")]
    InvalidPublicKey { field: &'static str },
    #[error("invalid unsigned amount for {field}")]
    InvalidAmount { field: &'static str },
    #[error("settlement program id is not the configured protocol program")]
    UnsupportedProgramId,
    #[error("gross amount must be positive")]
    InvalidGrossAmount,
    #[error("tax plus net must equal gross")]
    AmountConservationViolation,
    #[error("policy digest must not be all zeroes")]
    EmptyPolicyDigest,
    #[error("document attestation digest must not be all zeroes")]
    EmptyDocumentAttestationDigest,
    #[error("payment scope digest must not be all zeroes")]
    EmptyPaymentScopeDigest,
    #[error("settlement nonce must be positive")]
    InvalidNonce,
    #[error("policy sequence must be positive")]
    InvalidPolicySequence,
    #[error("token program is not a supported SPL token program")]
    UnsupportedTokenProgram,
    #[error("source, tax vault, and destination token accounts must be distinct")]
    AliasedTokenAccounts,
    #[error("settlement account metas must contain unique public keys")]
    DuplicateAccountMeta,
    #[error("split plan is invalid: {message}")]
    InvalidSplitPlan { message: String },
    #[error("settlement wire budget or shape is invalid")]
    InvalidWireBudget,
    #[error("compute budget policy is unsupported or outside its safety bounds")]
    InvalidComputeBudgetPolicy,
    #[error("adjustment nonce must be positive")]
    InvalidAdjustmentNonce,
    #[error("adjustment kind is unsupported")]
    InvalidAdjustmentKind,
    #[error("adjustment signer authorities must be distinct")]
    AdjustmentAuthorityConflict,
    #[error("adjustment sign must be zero or one")]
    InvalidAdjustmentSign,
    #[error("adjustment magnitude is invalid for {field}")]
    InvalidAdjustmentMagnitude { field: &'static str },
    #[error("adjustment signed amounts do not conserve gross = tax + net")]
    AdjustmentAmountConservationViolation,
    #[error("adjustment digest must not be all zeroes")]
    EmptyAdjustmentDigest,
    #[error("adjustment evidence digest must not be all zeroes")]
    EmptyAdjustmentEvidenceDigest,
    #[error("adjustment original receipt must not be the default key")]
    InvalidAdjustmentReceipt,
    #[error("refund claim nonce must be positive")]
    InvalidRefundClaimNonce,
    #[error("refund claim execution window is invalid")]
    InvalidRefundClaimWindow,
    #[error("refund reference digest must not be all zeroes")]
    EmptyRefundReferenceDigest,
    #[error("refund claim signer authorities must be distinct")]
    RefundClaimAuthorityConflict,
    #[error("refund token accounts must be distinct")]
    AliasedRefundTokenAccounts,
    #[error("refund cancellation nonce must be positive")]
    InvalidRefundCancellationNonce,
    #[error("refund cancellation key must be a non-default public key for {field}")]
    InvalidRefundCancellationKey { field: &'static str },
    #[error("refund cancellation accounts must be distinct")]
    AliasedRefundCancellationAccounts,
    #[error("revenue stream plan authorities must be distinct")]
    RevenueStreamAuthorityConflict,
    #[error("revenue stream plan or claim field is invalid")]
    InvalidRevenueStreamInput,
    #[error("revenue stream digest must not be all zeroes")]
    EmptyRevenueStreamDigest,
    #[error("revenue stream nonce must be positive")]
    InvalidRevenueStreamNonce,
    #[error("revenue stream execution window is invalid")]
    InvalidRevenueStreamWindow,
    #[error("revenue stream amount exceeds its configured bound")]
    InvalidRevenueStreamAmount,
    #[error("revenue stream accounts must be distinct")]
    AliasedRevenueStreamAccounts,
}

pub fn settlement_program_id() -> Pubkey {
    Pubkey::new_from_array(SETTLEMENT_PROGRAM_BYTES)
}

pub fn spl_token_program_id() -> Pubkey {
    Pubkey::new_from_array(SPL_TOKEN_PROGRAM_BYTES)
}

/// Derives the canonical mint-scoped asset-admission PDA. The program remains
/// the final authority; this helper only prevents callers from substituting a
/// different account in a prepared transaction.
pub fn asset_config_pda(mint: &str) -> Result<(String, u8), SettlementCompileError> {
    let mint = parse_pubkey("mint", mint)?;
    let (address, bump) = Pubkey::find_program_address(
        &[b"asset-config", mint.as_ref()],
        &settlement_program_id(),
    );
    Ok((address.to_string(), bump))
}

/// Derives the canonical merchant/mint-scoped adjustment registry PDA and
/// returns its bump. The API and RPC verifier use this server-owned helper so
/// callers cannot substitute a registry account.
pub fn adjustment_registry_pda(
    merchant: &str,
    mint: &str,
) -> Result<(String, u8), SettlementCompileError> {
    let merchant = parse_pubkey("merchant", merchant)?;
    let mint = parse_pubkey("mint", mint)?;
    let (address, bump) = Pubkey::find_program_address(
        &[b"adjustment-registry", merchant.as_ref(), mint.as_ref()],
        &settlement_program_id(),
    );
    Ok((address.to_string(), bump))
}

/// Derives the payer's canonical associated token account without an RPC
/// lookup. The account must already exist; the settlement hot path never
/// creates an ATA or silently substitutes another payer-owned token account.
pub fn derive_source_token_account(
    payer: &str,
    mint: &str,
    token_program: &str,
) -> Result<String, SettlementCompileError> {
    let payer = parse_pubkey("payer", payer)?;
    let mint = parse_pubkey("mint", mint)?;
    let token_program = parse_pubkey("token_program", token_program)?;
    if token_program != spl_token_program_id()
        && token_program != parse_pubkey("token_program", TOKEN_2022_PROGRAM_ID)?
    {
        return Err(SettlementCompileError::UnsupportedTokenProgram);
    }
    let associated_token_program =
        parse_pubkey("associated_token_program", ASSOCIATED_TOKEN_PROGRAM_ID)?;
    let (address, _) = Pubkey::find_program_address(
        &[payer.as_ref(), token_program.as_ref(), mint.as_ref()],
        &associated_token_program,
    );
    Ok(address.to_string())
}

pub fn compile_settlement_instruction(
    request: &SettlementCompileRequest,
) -> Result<CompiledSettlementInstruction, SettlementCompileError> {
    let program_id = parse_pubkey("program_id", &request.program_id)?;
    if program_id != settlement_program_id() {
        return Err(SettlementCompileError::UnsupportedProgramId);
    }
    let payer = parse_pubkey("payer", &request.payer)?;
    let merchant = parse_pubkey("merchant", &request.merchant)?;
    let source = parse_pubkey("source", &request.source)?;
    let mint = parse_pubkey("mint", &request.mint)?;
    let tax_vault = parse_pubkey("tax_vault", &request.tax_vault)?;
    let net_destination = parse_pubkey("net_destination", &request.net_destination)?;
    let token_program = parse_pubkey("token_program", &request.token_program)?;

    if token_program != spl_token_program_id()
        && token_program != parse_pubkey("token_program", TOKEN_2022_PROGRAM_ID)?
    {
        return Err(SettlementCompileError::UnsupportedTokenProgram);
    }
    if request.nonce == 0 {
        return Err(SettlementCompileError::InvalidNonce);
    }
    if request.policy_sequence == 0 {
        return Err(SettlementCompileError::InvalidPolicySequence);
    }
    if request.policy_digest == [0; 32] {
        return Err(SettlementCompileError::EmptyPolicyDigest);
    }
    let compute_budget_policy = ComputeBudgetPolicy::default();
    compute_budget_policy.validate()?;

    let gross_atomic = parse_amount("gross_atomic", &request.gross_atomic)?;
    let tax_atomic = parse_amount("tax_atomic", &request.tax_atomic)?;
    let net_atomic = parse_amount("net_atomic", &request.net_atomic)?;
    if gross_atomic == 0 {
        return Err(SettlementCompileError::InvalidGrossAmount);
    }
    if tax_atomic.checked_add(net_atomic) != Some(gross_atomic) {
        return Err(SettlementCompileError::AmountConservationViolation);
    }
    if source == tax_vault || source == net_destination || tax_vault == net_destination {
        return Err(SettlementCompileError::AliasedTokenAccounts);
    }

    let (protocol_config, _) = Pubkey::find_program_address(&[b"protocol-config"], &program_id);
    let (vault_authority, _) = Pubkey::find_program_address(
        &[b"vault-authority", merchant.as_ref(), mint.as_ref()],
        &program_id,
    );
    let (policy_attestation, _) = Pubkey::find_program_address(
        &[
            b"policy-attestation",
            merchant.as_ref(),
            mint.as_ref(),
            &request.policy_sequence.to_le_bytes(),
        ],
        &program_id,
    );
    let (receipt, _) = Pubkey::find_program_address(
        &[
            b"receipt",
            payer.as_ref(),
            merchant.as_ref(),
            &request.nonce.to_le_bytes(),
        ],
        &program_id,
    );
    let (asset_config, _) = Pubkey::find_program_address(
        &[b"asset-config", mint.as_ref()],
        &program_id,
    );

    let accounts = vec![
        account(payer, true, true),
        account(protocol_config, false, false),
        account(merchant, false, false),
        account(source, false, true),
        account(mint, false, false),
        account(tax_vault, false, true),
        account(vault_authority, false, false),
        account(policy_attestation, false, false),
        account(net_destination, false, true),
        account(receipt, false, true),
        account(asset_config, false, false),
        account(token_program, false, false),
        account(
            parse_pubkey("system_program", SYSTEM_PROGRAM_ID)?,
            false,
            false,
        ),
    ];
    ensure_unique_account_keys(&accounts, &program_id)?;

    let mut instruction_data = settle_discriminator();
    instruction_data.extend_from_slice(&request.nonce.to_le_bytes());
    instruction_data.extend_from_slice(&request.policy_sequence.to_le_bytes());
    instruction_data.extend_from_slice(&gross_atomic.to_le_bytes());
    instruction_data.extend_from_slice(&tax_atomic.to_le_bytes());
    instruction_data.extend_from_slice(&net_atomic.to_le_bytes());
    instruction_data.extend_from_slice(&request.policy_digest);
    estimate_legacy_settlement_wire_bytes_with_compute_budget(
        accounts.len(),
        instruction_data.len(),
    )?;

    Ok(CompiledSettlementInstruction {
        status: "prepared_unsubmitted".to_string(),
        program_id: program_id.to_string(),
        instruction_data_base64: BASE64_STANDARD.encode(instruction_data),
        accounts,
        protocol_config: protocol_config.to_string(),
        vault_authority: vault_authority.to_string(),
        policy_attestation: policy_attestation.to_string(),
        receipt: receipt.to_string(),
        asset_config: asset_config.to_string(),
        compute_budget_policy,
    })
}

pub fn compile_document_settlement_instruction(
    request: &DocumentSettlementCompileRequest,
) -> Result<CompiledDocumentSettlementInstruction, SettlementCompileError> {
    let program_id = parse_pubkey("program_id", &request.program_id)?;
    if program_id != settlement_program_id() {
        return Err(SettlementCompileError::UnsupportedProgramId);
    }
    let payer = parse_pubkey("payer", &request.payer)?;
    let merchant = parse_pubkey("merchant", &request.merchant)?;
    let source = parse_pubkey("source", &request.source)?;
    let mint = parse_pubkey("mint", &request.mint)?;
    let tax_vault = parse_pubkey("tax_vault", &request.tax_vault)?;
    let net_destination = parse_pubkey("net_destination", &request.net_destination)?;
    let token_program = parse_supported_token_program(&request.token_program)?;
    if request.nonce == 0 {
        return Err(SettlementCompileError::InvalidNonce);
    }
    if request.policy_sequence == 0 {
        return Err(SettlementCompileError::InvalidPolicySequence);
    }
    if request.policy_digest == [0; 32] {
        return Err(SettlementCompileError::EmptyPolicyDigest);
    }
    if request.document_attestation_digest == [0; 32] {
        return Err(SettlementCompileError::EmptyDocumentAttestationDigest);
    }
    if request.payment_scope_digest == [0; 32] {
        return Err(SettlementCompileError::EmptyPaymentScopeDigest);
    }
    let compute_budget_policy = ComputeBudgetPolicy::default();
    compute_budget_policy.validate()?;
    let gross_atomic = parse_amount("gross_atomic", &request.gross_atomic)?;
    let tax_atomic = parse_amount("tax_atomic", &request.tax_atomic)?;
    let net_atomic = parse_amount("net_atomic", &request.net_atomic)?;
    if gross_atomic == 0 {
        return Err(SettlementCompileError::InvalidGrossAmount);
    }
    if tax_atomic.checked_add(net_atomic) != Some(gross_atomic) {
        return Err(SettlementCompileError::AmountConservationViolation);
    }
    if source == tax_vault || source == net_destination || tax_vault == net_destination {
        return Err(SettlementCompileError::AliasedTokenAccounts);
    }

    let (protocol_config, _) = Pubkey::find_program_address(&[b"protocol-config"], &program_id);
    let (vault_authority, _) = Pubkey::find_program_address(
        &[b"vault-authority", merchant.as_ref(), mint.as_ref()],
        &program_id,
    );
    let (policy_attestation, _) = Pubkey::find_program_address(
        &[
            b"policy-attestation",
            merchant.as_ref(),
            mint.as_ref(),
            &request.policy_sequence.to_le_bytes(),
        ],
        &program_id,
    );
    let (receipt, _) = Pubkey::find_program_address(
        &[
            b"document-receipt",
            payer.as_ref(),
            merchant.as_ref(),
            &request.nonce.to_le_bytes(),
        ],
        &program_id,
    );
    let (asset_config, _) = Pubkey::find_program_address(
        &[b"asset-config", mint.as_ref()],
        &program_id,
    );
    let accounts = vec![
        account(payer, true, true),
        account(protocol_config, false, false),
        account(merchant, false, false),
        account(source, false, true),
        account(mint, false, false),
        account(tax_vault, false, true),
        account(vault_authority, false, false),
        account(policy_attestation, false, false),
        account(net_destination, false, true),
        account(receipt, false, true),
        account(asset_config, false, false),
        account(token_program, false, false),
        account(
            parse_pubkey("system_program", SYSTEM_PROGRAM_ID)?,
            false,
            false,
        ),
    ];
    ensure_unique_account_keys(&accounts, &program_id)?;
    let mut instruction_data = instruction_discriminator("global:settle_document");
    instruction_data.extend_from_slice(&request.nonce.to_le_bytes());
    instruction_data.extend_from_slice(&request.policy_sequence.to_le_bytes());
    instruction_data.extend_from_slice(&gross_atomic.to_le_bytes());
    instruction_data.extend_from_slice(&tax_atomic.to_le_bytes());
    instruction_data.extend_from_slice(&net_atomic.to_le_bytes());
    instruction_data.extend_from_slice(&request.policy_digest);
    instruction_data.extend_from_slice(&request.document_attestation_digest);
    instruction_data.extend_from_slice(&request.payment_scope_digest);
    estimate_legacy_settlement_wire_bytes_with_compute_budget(
        accounts.len(),
        instruction_data.len(),
    )?;
    Ok(CompiledDocumentSettlementInstruction {
        status: "prepared_unsubmitted".to_string(),
        program_id: program_id.to_string(),
        instruction_data_base64: BASE64_STANDARD.encode(instruction_data),
        accounts,
        protocol_config: protocol_config.to_string(),
        vault_authority: vault_authority.to_string(),
        policy_attestation: policy_attestation.to_string(),
        receipt: receipt.to_string(),
        asset_config: asset_config.to_string(),
        document_attestation_digest: request.document_attestation_digest.to_vec(),
        payment_scope_digest: request.payment_scope_digest.to_vec(),
        compute_budget_policy,
    })
}

pub fn compile_settlement_adjustment_instruction(
    request: &SettlementAdjustmentCompileRequest,
) -> Result<CompiledSettlementAdjustmentInstruction, SettlementCompileError> {
    let program_id = parse_pubkey("program_id", &request.program_id)?;
    if program_id != settlement_program_id() {
        return Err(SettlementCompileError::UnsupportedProgramId);
    }
    let proposer = parse_pubkey("proposer", &request.proposer)?;
    let approver = parse_pubkey("approver", &request.approver)?;
    let merchant = parse_pubkey("merchant", &request.merchant)?;
    let mint = parse_pubkey("mint", &request.mint)?;
    let token_program = parse_pubkey("token_program", &request.token_program)?;
    if token_program != spl_token_program_id()
        && token_program != parse_pubkey("token_program", TOKEN_2022_PROGRAM_ID)?
    {
        return Err(SettlementCompileError::UnsupportedTokenProgram);
    }
    let original_receipt = parse_pubkey("original_receipt", &request.original_receipt)?;
    if original_receipt == Pubkey::default() {
        return Err(SettlementCompileError::InvalidAdjustmentReceipt);
    }
    if proposer == approver {
        return Err(SettlementCompileError::AdjustmentAuthorityConflict);
    }
    if request.adjustment_nonce == 0 {
        return Err(SettlementCompileError::InvalidAdjustmentNonce);
    }
    if !matches!(request.kind, 1..=5) {
        return Err(SettlementCompileError::InvalidAdjustmentKind);
    }
    let gross = parse_signed_magnitude(
        "gross_magnitude",
        request.gross_sign,
        &request.gross_magnitude,
    )?;
    let tax = parse_signed_magnitude("tax_magnitude", request.tax_sign, &request.tax_magnitude)?;
    let net = parse_signed_magnitude("net_magnitude", request.net_sign, &request.net_magnitude)?;
    validate_signed_adjustment(request.kind, gross, tax, net)?;
    if request.adjustment_digest == [0; 32] {
        return Err(SettlementCompileError::EmptyAdjustmentDigest);
    }
    if request.evidence_digest == [0; 32] {
        return Err(SettlementCompileError::EmptyAdjustmentEvidenceDigest);
    }

    let (protocol_config, _) = Pubkey::find_program_address(&[b"protocol-config"], &program_id);
    let (adjustment_registry, _) = Pubkey::find_program_address(
        &[b"adjustment-registry", merchant.as_ref(), mint.as_ref()],
        &program_id,
    );
    let (adjustment, _) = Pubkey::find_program_address(
        &[
            b"adjustment",
            original_receipt.as_ref(),
            &request.adjustment_nonce.to_le_bytes(),
        ],
        &program_id,
    );
    let accounts = vec![
        account(approver, true, true),
        account(proposer, false, false),
        account(merchant, false, false),
        account(mint, false, false),
        account(token_program, false, false),
        account(protocol_config, false, false),
        account(adjustment_registry, false, false),
        account(original_receipt, false, false),
        account(adjustment, false, true),
        account(
            parse_pubkey("system_program", SYSTEM_PROGRAM_ID)?,
            false,
            false,
        ),
    ];
    ensure_unique_account_keys(&accounts, &program_id)?;

    let mut instruction_data = instruction_discriminator("global:record_adjustment");
    instruction_data.extend_from_slice(&request.adjustment_nonce.to_le_bytes());
    instruction_data.push(request.kind);
    append_signed_amount(&mut instruction_data, gross);
    append_signed_amount(&mut instruction_data, tax);
    append_signed_amount(&mut instruction_data, net);
    instruction_data.extend_from_slice(&request.adjustment_digest);
    instruction_data.extend_from_slice(&request.evidence_digest);
    instruction_data.extend_from_slice(&request.source_calculation_digest);
    instruction_data.extend_from_slice(&request.replacement_calculation_digest);
    estimate_legacy_settlement_wire_bytes(accounts.len(), instruction_data.len())?;

    Ok(CompiledSettlementAdjustmentInstruction {
        status: "prepared_unsubmitted".to_string(),
        program_id: program_id.to_string(),
        instruction_data_base64: BASE64_STANDARD.encode(instruction_data),
        accounts,
        adjustment: adjustment.to_string(),
        adjustment_registry: adjustment_registry.to_string(),
        original_receipt: original_receipt.to_string(),
        adjustment_nonce: request.adjustment_nonce,
    })
}

pub fn compile_settlement_refund_claim_instruction(
    request: &SettlementRefundClaimCompileRequest,
) -> Result<CompiledSettlementRefundInstruction, SettlementCompileError> {
    let program_id = parse_pubkey("program_id", &request.program_id)?;
    validate_settlement_program_id(program_id)?;
    let proposer = parse_pubkey("proposer", &request.proposer)?;
    let refund_funder = parse_pubkey("refund_funder", &request.refund_funder)?;
    let merchant = parse_pubkey("merchant", &request.merchant)?;
    let mint = parse_pubkey("mint", &request.mint)?;
    let token_program = parse_supported_token_program(&request.token_program)?;
    let original_receipt = parse_pubkey("original_receipt", &request.original_receipt)?;
    let adjustment = parse_pubkey("adjustment", &request.adjustment)?;
    let tax_vault = parse_pubkey("tax_vault", &request.tax_vault)?;
    let net_source = parse_pubkey("net_source", &request.net_source)?;
    let refund_destination = parse_pubkey("refund_destination", &request.refund_destination)?;
    if proposer == refund_funder {
        return Err(SettlementCompileError::RefundClaimAuthorityConflict);
    }
    if request.claim_nonce == 0 {
        return Err(SettlementCompileError::InvalidRefundClaimNonce);
    }
    if request.reference_digest == [0; 32] {
        return Err(SettlementCompileError::EmptyRefundReferenceDigest);
    }
    let window = request
        .expires_at_unix
        .checked_sub(request.execute_after_unix)
        .ok_or(SettlementCompileError::InvalidRefundClaimWindow)?;
    if window <= 0 || window > MAX_REFUND_CLAIM_WINDOW_SECONDS {
        return Err(SettlementCompileError::InvalidRefundClaimWindow);
    }
    if tax_vault == net_source
        || tax_vault == refund_destination
        || net_source == refund_destination
    {
        return Err(SettlementCompileError::AliasedRefundTokenAccounts);
    }

    let (protocol_config, _) = Pubkey::find_program_address(&[b"protocol-config"], &program_id);
    let (registry, _) = Pubkey::find_program_address(
        &[b"adjustment-registry", merchant.as_ref(), mint.as_ref()],
        &program_id,
    );
    let (vault_authority, _) = Pubkey::find_program_address(
        &[b"vault-authority", merchant.as_ref(), mint.as_ref()],
        &program_id,
    );
    let (claim, _) = Pubkey::find_program_address(
        &[
            b"refund-claim",
            adjustment.as_ref(),
            &request.claim_nonce.to_le_bytes(),
        ],
        &program_id,
    );
    let accounts = vec![
        account(proposer, true, true),
        account(refund_funder, true, false),
        account(merchant, false, false),
        account(mint, false, false),
        account(token_program, false, false),
        account(protocol_config, false, false),
        account(registry, false, false),
        account(adjustment, false, false),
        account(original_receipt, false, false),
        account(tax_vault, false, false),
        account(vault_authority, false, false),
        account(net_source, false, false),
        account(refund_destination, false, false),
        account(claim, false, true),
        account(
            parse_pubkey("system_program", SYSTEM_PROGRAM_ID)?,
            false,
            false,
        ),
    ];
    ensure_unique_account_keys(&accounts, &program_id)?;

    let mut instruction_data = instruction_discriminator("global:create_adjustment_refund_claim");
    instruction_data.extend_from_slice(&request.claim_nonce.to_le_bytes());
    instruction_data.extend_from_slice(&request.reference_digest);
    instruction_data.extend_from_slice(&request.execute_after_unix.to_le_bytes());
    instruction_data.extend_from_slice(&request.expires_at_unix.to_le_bytes());
    estimate_legacy_settlement_wire_bytes(accounts.len(), instruction_data.len())?;

    Ok(CompiledSettlementRefundInstruction {
        status: "prepared_unsubmitted".to_string(),
        program_id: program_id.to_string(),
        instruction_data_base64: BASE64_STANDARD.encode(instruction_data),
        accounts,
        registry: registry.to_string(),
        adjustment: adjustment.to_string(),
        original_receipt: original_receipt.to_string(),
        claim: claim.to_string(),
        claim_nonce: request.claim_nonce,
    })
}

pub fn compile_settlement_refund_execution_instruction(
    request: &SettlementRefundExecutionCompileRequest,
) -> Result<CompiledSettlementRefundInstruction, SettlementCompileError> {
    let program_id = parse_pubkey("program_id", &request.program_id)?;
    validate_settlement_program_id(program_id)?;
    let approver = parse_pubkey("approver", &request.approver)?;
    let refund_funder = parse_pubkey("refund_funder", &request.refund_funder)?;
    let merchant = parse_pubkey("merchant", &request.merchant)?;
    let mint = parse_pubkey("mint", &request.mint)?;
    let token_program = parse_supported_token_program(&request.token_program)?;
    let original_receipt = parse_pubkey("original_receipt", &request.original_receipt)?;
    let adjustment = parse_pubkey("adjustment", &request.adjustment)?;
    let tax_vault = parse_pubkey("tax_vault", &request.tax_vault)?;
    let net_source = parse_pubkey("net_source", &request.net_source)?;
    let refund_destination = parse_pubkey("refund_destination", &request.refund_destination)?;
    if approver == refund_funder {
        return Err(SettlementCompileError::RefundClaimAuthorityConflict);
    }
    if request.claim_nonce == 0 {
        return Err(SettlementCompileError::InvalidRefundClaimNonce);
    }
    if tax_vault == net_source
        || tax_vault == refund_destination
        || net_source == refund_destination
    {
        return Err(SettlementCompileError::AliasedRefundTokenAccounts);
    }

    let (protocol_config, _) = Pubkey::find_program_address(&[b"protocol-config"], &program_id);
    let (registry, _) = Pubkey::find_program_address(
        &[b"adjustment-registry", merchant.as_ref(), mint.as_ref()],
        &program_id,
    );
    let (vault_authority, _) = Pubkey::find_program_address(
        &[b"vault-authority", merchant.as_ref(), mint.as_ref()],
        &program_id,
    );
    let (claim, _) = Pubkey::find_program_address(
        &[
            b"refund-claim",
            adjustment.as_ref(),
            &request.claim_nonce.to_le_bytes(),
        ],
        &program_id,
    );
    let accounts = vec![
        account(approver, true, false),
        account(refund_funder, true, false),
        account(merchant, false, false),
        account(mint, false, false),
        account(token_program, false, false),
        account(protocol_config, false, false),
        account(registry, false, false),
        account(adjustment, false, false),
        account(original_receipt, false, false),
        account(tax_vault, false, true),
        account(vault_authority, false, false),
        account(net_source, false, true),
        account(refund_destination, false, true),
        account(claim, false, true),
    ];
    ensure_unique_account_keys(&accounts, &program_id)?;

    let mut instruction_data = instruction_discriminator("global:execute_adjustment_refund");
    instruction_data.extend_from_slice(&request.claim_nonce.to_le_bytes());
    estimate_legacy_settlement_wire_bytes(accounts.len(), instruction_data.len())?;

    Ok(CompiledSettlementRefundInstruction {
        status: "prepared_unsubmitted".to_string(),
        program_id: program_id.to_string(),
        instruction_data_base64: BASE64_STANDARD.encode(instruction_data),
        accounts,
        registry: registry.to_string(),
        adjustment: adjustment.to_string(),
        original_receipt: original_receipt.to_string(),
        claim: claim.to_string(),
        claim_nonce: request.claim_nonce,
    })
}

/// Compiles the governance-only cancellation instruction for a pending refund
/// claim. Cancellation intentionally carries no token amounts or token-account
/// topology: the protocol only needs the registered governance authority,
/// merchant/mint registry, and deterministic claim PDA.
pub fn compile_settlement_refund_cancellation_instruction(
    request: &SettlementRefundCancellationCompileRequest,
) -> Result<CompiledSettlementRefundCancellationInstruction, SettlementCompileError> {
    let program_id = parse_pubkey("program_id", &request.program_id)?;
    validate_settlement_program_id(program_id)?;
    let governance_authority = parse_pubkey("governance_authority", &request.governance_authority)?;
    let merchant = parse_pubkey("merchant", &request.merchant)?;
    let mint = parse_pubkey("mint", &request.mint)?;
    let adjustment = parse_pubkey("adjustment", &request.adjustment)?;
    if request.claim_nonce == 0 {
        return Err(SettlementCompileError::InvalidRefundCancellationNonce);
    }
    for (field, key) in [
        ("governance_authority", governance_authority),
        ("merchant", merchant),
        ("mint", mint),
        ("adjustment", adjustment),
    ] {
        if key == Pubkey::default() {
            return Err(SettlementCompileError::InvalidRefundCancellationKey { field });
        }
    }

    let (registry, _) = Pubkey::find_program_address(
        &[b"adjustment-registry", merchant.as_ref(), mint.as_ref()],
        &program_id,
    );
    let (claim, _) = Pubkey::find_program_address(
        &[
            b"refund-claim",
            adjustment.as_ref(),
            &request.claim_nonce.to_le_bytes(),
        ],
        &program_id,
    );
    let accounts = vec![
        // The governance wallet is the canonical fee payer for this fixed
        // wallet handoff, so Solana's message marks it writable. The program
        // does not mutate the account; the writable bit is fee-payer driven.
        account(governance_authority, true, true),
        account(merchant, false, false),
        account(mint, false, false),
        account(registry, false, false),
        account(claim, false, true),
    ];
    if governance_authority == merchant
        || governance_authority == mint
        || governance_authority == adjustment
        || merchant == mint
    {
        return Err(SettlementCompileError::AliasedRefundCancellationAccounts);
    }
    ensure_unique_account_keys(&accounts, &program_id)?;

    let mut instruction_data = instruction_discriminator("global:cancel_adjustment_refund_claim");
    instruction_data.extend_from_slice(&request.claim_nonce.to_le_bytes());
    estimate_legacy_settlement_wire_bytes(accounts.len(), instruction_data.len())?;

    Ok(CompiledSettlementRefundCancellationInstruction {
        status: "prepared_unsubmitted".to_string(),
        program_id: program_id.to_string(),
        instruction_data_base64: BASE64_STANDARD.encode(instruction_data),
        accounts,
        registry: registry.to_string(),
        claim: claim.to_string(),
        claim_nonce: request.claim_nonce,
    })
}

pub fn revenue_stream_plan_pda(
    merchant: &str,
    mint: &str,
    plan_digest: [u8; 32],
) -> Result<String, SettlementCompileError> {
    let merchant = parse_pubkey("merchant", merchant)?;
    let mint = parse_pubkey("mint", mint)?;
    let (plan, _) = Pubkey::find_program_address(
        &[
            b"revenue-stream-plan",
            merchant.as_ref(),
            mint.as_ref(),
            &plan_digest,
        ],
        &settlement_program_id(),
    );
    Ok(plan.to_string())
}

pub fn revenue_stream_accrual_pda(
    plan: &str,
    source_receipt: &str,
) -> Result<String, SettlementCompileError> {
    let plan = parse_pubkey("plan", plan)?;
    let source_receipt = parse_pubkey("source_receipt", source_receipt)?;
    let (accrual, _) = Pubkey::find_program_address(
        &[
            b"revenue-stream-accrual",
            plan.as_ref(),
            source_receipt.as_ref(),
        ],
        &settlement_program_id(),
    );
    Ok(accrual.to_string())
}

pub fn revenue_stream_claim_pda(plan: &str, nonce: u64) -> Result<String, SettlementCompileError> {
    let plan = parse_pubkey("plan", plan)?;
    if nonce == 0 {
        return Err(SettlementCompileError::InvalidRevenueStreamNonce);
    }
    let (claim, _) = Pubkey::find_program_address(
        &[b"revenue-stream-claim", plan.as_ref(), &nonce.to_le_bytes()],
        &settlement_program_id(),
    );
    Ok(claim.to_string())
}

pub fn compile_revenue_stream_plan_instruction(
    request: &RevenueStreamPlanCompileRequest,
) -> Result<CompiledRevenueStreamInstruction, SettlementCompileError> {
    let program_id = parse_pubkey("program_id", &request.program_id)?;
    validate_settlement_program_id(program_id)?;
    let governance_authority = parse_pubkey("governance_authority", &request.governance_authority)?;
    let merchant = parse_pubkey("merchant", &request.merchant)?;
    let mint = parse_pubkey("mint", &request.mint)?;
    let token_program = parse_supported_token_program(&request.token_program)?;
    let tax_vault = parse_pubkey("tax_vault", &request.tax_vault)?;
    let recipient_owner = parse_pubkey("recipient_owner", &request.recipient_owner)?;
    let proposer = parse_pubkey("proposer", &request.proposer)?;
    let approver = parse_pubkey("approver", &request.approver)?;
    let operator = parse_pubkey("operator", &request.operator)?;
    let keys = [
        ("governance_authority", governance_authority),
        ("merchant", merchant),
        ("mint", mint),
        ("tax_vault", tax_vault),
        ("recipient_owner", recipient_owner),
        ("proposer", proposer),
        ("approver", approver),
        ("operator", operator),
    ];
    if keys.iter().any(|(_, key)| *key == Pubkey::default()) {
        return Err(SettlementCompileError::InvalidRevenueStreamInput);
    }
    if governance_authority == proposer
        || governance_authority == approver
        || governance_authority == operator
        || proposer == approver
        || proposer == operator
        || approver == operator
    {
        return Err(SettlementCompileError::RevenueStreamAuthorityConflict);
    }
    if request.plan_digest == [0; 32] || request.policy_digest == [0; 32] {
        return Err(SettlementCompileError::EmptyRevenueStreamDigest);
    }
    if request.start_slot == 0 || request.end_slot.is_some_and(|end| end < request.start_slot) {
        return Err(SettlementCompileError::InvalidRevenueStreamWindow);
    }
    if request.max_total_atomic == 0
        || request.max_claim_atomic == 0
        || request.max_claim_atomic > request.max_total_atomic
    {
        return Err(SettlementCompileError::InvalidRevenueStreamAmount);
    }
    let plan = revenue_stream_plan_pda(&request.merchant, &request.mint, request.plan_digest)?;
    let plan_key = parse_pubkey("plan", &plan)?;
    let (vault_authority, _) = Pubkey::find_program_address(
        &[b"vault-authority", merchant.as_ref(), mint.as_ref()],
        &program_id,
    );
    let accounts = vec![
        account(governance_authority, true, true),
        account(merchant, false, false),
        account(mint, false, false),
        account(token_program, false, false),
        account(tax_vault, false, false),
        account(vault_authority, false, false),
        account(plan_key, false, true),
        account(
            parse_pubkey("system_program", SYSTEM_PROGRAM_ID)?,
            false,
            false,
        ),
    ];
    ensure_unique_account_keys(&accounts, &program_id)?;
    let mut data = instruction_discriminator("global:initialize_revenue_stream_plan");
    data.extend_from_slice(&request.plan_digest);
    data.extend_from_slice(&request.policy_digest);
    data.extend_from_slice(recipient_owner.as_ref());
    data.extend_from_slice(proposer.as_ref());
    data.extend_from_slice(approver.as_ref());
    data.extend_from_slice(operator.as_ref());
    data.extend_from_slice(&request.start_slot.to_le_bytes());
    match request.end_slot {
        Some(end_slot) => {
            data.push(1);
            data.extend_from_slice(&end_slot.to_le_bytes());
        }
        None => data.push(0),
    }
    data.extend_from_slice(&request.max_total_atomic.to_le_bytes());
    data.extend_from_slice(&request.max_claim_atomic.to_le_bytes());
    estimate_legacy_settlement_wire_bytes(accounts.len(), data.len())?;
    Ok(CompiledRevenueStreamInstruction {
        status: "prepared_unsubmitted".to_string(),
        program_id: program_id.to_string(),
        instruction_data_base64: BASE64_STANDARD.encode(data),
        accounts,
        plan,
        accrual: None,
        claim: None,
    })
}

pub fn compile_revenue_stream_accrual_instruction(
    request: &RevenueStreamAccrualCompileRequest,
) -> Result<CompiledRevenueStreamInstruction, SettlementCompileError> {
    let program_id = parse_pubkey("program_id", &request.program_id)?;
    validate_settlement_program_id(program_id)?;
    let operator = parse_pubkey("operator", &request.operator)?;
    let plan = parse_pubkey("plan", &request.plan)?;
    let source_receipt = parse_pubkey("source_receipt", &request.source_receipt)?;
    if operator == Pubkey::default()
        || plan == Pubkey::default()
        || source_receipt == Pubkey::default()
        || request.recipient_atomic == 0
        || request.finalized_slot == 0
        || request.evidence_digest == [0; 32]
    {
        return Err(SettlementCompileError::InvalidRevenueStreamInput);
    }
    let (protocol_config, _) = Pubkey::find_program_address(&[b"protocol-config"], &program_id);
    let accrual = revenue_stream_accrual_pda(&request.plan, &request.source_receipt)?;
    let accounts = vec![
        account(operator, true, true),
        account(protocol_config, false, false),
        account(plan, false, true),
        account(source_receipt, false, false),
        account(parse_pubkey("accrual", &accrual)?, false, true),
        account(
            parse_pubkey("system_program", SYSTEM_PROGRAM_ID)?,
            false,
            false,
        ),
    ];
    ensure_unique_account_keys(&accounts, &program_id)?;
    let mut data = instruction_discriminator("global:record_revenue_stream_accrual");
    data.extend_from_slice(&request.recipient_atomic.to_le_bytes());
    data.extend_from_slice(&request.finalized_slot.to_le_bytes());
    data.extend_from_slice(&request.evidence_digest);
    estimate_legacy_settlement_wire_bytes(accounts.len(), data.len())?;
    Ok(CompiledRevenueStreamInstruction {
        status: "prepared_unsubmitted".to_string(),
        program_id: program_id.to_string(),
        instruction_data_base64: BASE64_STANDARD.encode(data),
        accounts,
        plan: plan.to_string(),
        accrual: Some(accrual),
        claim: None,
    })
}

pub fn compile_revenue_stream_claim_instruction(
    request: &RevenueStreamClaimCompileRequest,
) -> Result<CompiledRevenueStreamInstruction, SettlementCompileError> {
    let program_id = parse_pubkey("program_id", &request.program_id)?;
    validate_settlement_program_id(program_id)?;
    let operator = parse_pubkey("operator", &request.operator)?;
    let plan = parse_pubkey("plan", &request.plan)?;
    let mint = parse_pubkey("mint", &request.mint)?;
    let token_program = parse_supported_token_program(&request.token_program)?;
    let tax_vault = parse_pubkey("tax_vault", &request.tax_vault)?;
    let destination = parse_pubkey("destination", &request.destination)?;
    if request.nonce == 0 || request.amount_atomic == 0 {
        return Err(SettlementCompileError::InvalidRevenueStreamAmount);
    }
    if request.claim_digest == [0; 32] || request.evidence_digest == [0; 32] {
        return Err(SettlementCompileError::EmptyRevenueStreamDigest);
    }
    let window = request
        .expires_at_unix
        .checked_sub(request.execute_after_unix)
        .ok_or(SettlementCompileError::InvalidRevenueStreamWindow)?;
    if window <= 0 || window > MAX_REFUND_CLAIM_WINDOW_SECONDS {
        return Err(SettlementCompileError::InvalidRevenueStreamWindow);
    }
    if tax_vault == destination {
        return Err(SettlementCompileError::AliasedRevenueStreamAccounts);
    }
    let (protocol_config, _) = Pubkey::find_program_address(&[b"protocol-config"], &program_id);
    let claim = revenue_stream_claim_pda(&request.plan, request.nonce)?;
    let accounts = vec![
        account(operator, true, true),
        account(protocol_config, false, false),
        account(plan, false, true),
        account(mint, false, false),
        account(token_program, false, false),
        account(tax_vault, false, false),
        account(destination, false, false),
        account(parse_pubkey("claim", &claim)?, false, true),
        account(
            parse_pubkey("system_program", SYSTEM_PROGRAM_ID)?,
            false,
            false,
        ),
    ];
    ensure_unique_account_keys(&accounts, &program_id)?;
    let mut data = instruction_discriminator("global:create_revenue_stream_claim");
    data.extend_from_slice(&request.nonce.to_le_bytes());
    data.extend_from_slice(&request.amount_atomic.to_le_bytes());
    data.extend_from_slice(&request.claim_digest);
    data.extend_from_slice(&request.evidence_digest);
    data.extend_from_slice(&request.execute_after_unix.to_le_bytes());
    data.extend_from_slice(&request.expires_at_unix.to_le_bytes());
    estimate_legacy_settlement_wire_bytes(accounts.len(), data.len())?;
    Ok(CompiledRevenueStreamInstruction {
        status: "prepared_unsubmitted".to_string(),
        program_id: program_id.to_string(),
        instruction_data_base64: BASE64_STANDARD.encode(data),
        accounts,
        plan: plan.to_string(),
        accrual: None,
        claim: Some(claim),
    })
}

pub fn compile_revenue_stream_claim_execution_instruction(
    request: &RevenueStreamClaimExecutionCompileRequest,
) -> Result<CompiledRevenueStreamInstruction, SettlementCompileError> {
    let program_id = parse_pubkey("program_id", &request.program_id)?;
    validate_settlement_program_id(program_id)?;
    let approver = parse_pubkey("approver", &request.approver)?;
    let plan = parse_pubkey("plan", &request.plan)?;
    let merchant = parse_pubkey("merchant", &request.merchant)?;
    let mint = parse_pubkey("mint", &request.mint)?;
    let token_program = parse_supported_token_program(&request.token_program)?;
    let tax_vault = parse_pubkey("tax_vault", &request.tax_vault)?;
    let destination = parse_pubkey("destination", &request.destination)?;
    if request.nonce == 0
        || approver == Pubkey::default()
        || plan == Pubkey::default()
        || merchant == Pubkey::default()
    {
        return Err(SettlementCompileError::InvalidRevenueStreamInput);
    }
    if tax_vault == destination {
        return Err(SettlementCompileError::AliasedRevenueStreamAccounts);
    }
    let (protocol_config, _) = Pubkey::find_program_address(&[b"protocol-config"], &program_id);
    let (vault_authority, _) = Pubkey::find_program_address(
        &[b"vault-authority", merchant.as_ref(), mint.as_ref()],
        &program_id,
    );
    let claim = revenue_stream_claim_pda(&request.plan, request.nonce)?;
    let accounts = vec![
        account(approver, true, false),
        account(protocol_config, false, false),
        account(plan, false, true),
        account(mint, false, false),
        account(token_program, false, false),
        account(tax_vault, false, true),
        account(vault_authority, false, false),
        account(destination, false, true),
        account(parse_pubkey("claim", &claim)?, false, true),
    ];
    ensure_unique_account_keys(&accounts, &program_id)?;
    let mut data = instruction_discriminator("global:execute_revenue_stream_claim");
    data.extend_from_slice(&request.nonce.to_le_bytes());
    estimate_legacy_settlement_wire_bytes(accounts.len(), data.len())?;
    Ok(CompiledRevenueStreamInstruction {
        status: "prepared_unsubmitted".to_string(),
        program_id: program_id.to_string(),
        instruction_data_base64: BASE64_STANDARD.encode(data),
        accounts,
        plan: plan.to_string(),
        accrual: None,
        claim: Some(claim),
    })
}

pub fn compile_revenue_stream_claim_cancellation_instruction(
    request: &RevenueStreamClaimCancellationCompileRequest,
) -> Result<CompiledRevenueStreamInstruction, SettlementCompileError> {
    let program_id = parse_pubkey("program_id", &request.program_id)?;
    validate_settlement_program_id(program_id)?;
    let governance_authority = parse_pubkey("governance_authority", &request.governance_authority)?;
    let plan = parse_pubkey("plan", &request.plan)?;
    if request.nonce == 0 || governance_authority == Pubkey::default() || plan == Pubkey::default()
    {
        return Err(SettlementCompileError::InvalidRevenueStreamInput);
    }
    let claim = revenue_stream_claim_pda(&request.plan, request.nonce)?;
    let accounts = vec![
        account(governance_authority, true, false),
        account(plan, false, true),
        account(parse_pubkey("claim", &claim)?, false, true),
    ];
    ensure_unique_account_keys(&accounts, &program_id)?;
    let mut data = instruction_discriminator("global:cancel_revenue_stream_claim");
    data.extend_from_slice(&request.nonce.to_le_bytes());
    estimate_legacy_settlement_wire_bytes(accounts.len(), data.len())?;
    Ok(CompiledRevenueStreamInstruction {
        status: "prepared_unsubmitted".to_string(),
        program_id: program_id.to_string(),
        instruction_data_base64: BASE64_STANDARD.encode(data),
        accounts,
        plan: plan.to_string(),
        accrual: None,
        claim: Some(claim),
    })
}

pub fn compile_split_settlement_instruction(
    request: &SplitSettlementCompileRequest,
) -> Result<CompiledSplitSettlementInstruction, SettlementCompileError> {
    let program_id = parse_pubkey("program_id", &request.program_id)?;
    if program_id != settlement_program_id() {
        return Err(SettlementCompileError::UnsupportedProgramId);
    }
    let payer = parse_pubkey("payer", &request.payer)?;
    let merchant = parse_pubkey("merchant", &request.merchant)?;
    let source = parse_pubkey("source", &request.source)?;
    let mint = parse_pubkey("mint", &request.mint)?;
    let tax_vault = parse_pubkey("tax_vault", &request.tax_vault)?;
    let token_program = parse_pubkey("token_program", &request.token_program)?;
    if token_program != spl_token_program_id()
        && token_program != parse_pubkey("token_program", TOKEN_2022_PROGRAM_ID)?
    {
        return Err(SettlementCompileError::UnsupportedTokenProgram);
    }
    if request.nonce == 0 {
        return Err(SettlementCompileError::InvalidNonce);
    }
    if request.policy_sequence == 0 {
        return Err(SettlementCompileError::InvalidPolicySequence);
    }
    if request.policy_digest == [0; 32] {
        return Err(SettlementCompileError::EmptyPolicyDigest);
    }
    let compute_budget_policy = ComputeBudgetPolicy::default();
    compute_budget_policy.validate()?;
    let gross_atomic = parse_amount("gross_atomic", &request.gross_atomic)?;
    let tax_atomic = parse_amount("tax_atomic", &request.tax_atomic)?;
    let net_atomic = parse_amount("net_atomic", &request.net_atomic)?;
    if gross_atomic == 0 {
        return Err(SettlementCompileError::InvalidGrossAmount);
    }
    if tax_atomic.checked_add(net_atomic) != Some(gross_atomic) {
        return Err(SettlementCompileError::AmountConservationViolation);
    }

    let plan = SplitPlan::new(
        request.merchant.clone(),
        request.mint.clone(),
        request.split_plan_version,
        request.recipients.clone(),
    )
    .map_err(|error| SettlementCompileError::InvalidSplitPlan {
        message: error.to_string(),
    })?;
    let split_plan_digest =
        plan.digest()
            .map_err(|error| SettlementCompileError::InvalidSplitPlan {
                message: error.to_string(),
            })?;
    let recipient_keys = plan
        .recipients
        .iter()
        .map(|recipient| parse_pubkey("recipient", &recipient.token_account))
        .collect::<Result<Vec<_>, _>>()?;
    if source == tax_vault
        || recipient_keys.contains(&source)
        || recipient_keys.contains(&tax_vault)
    {
        return Err(SettlementCompileError::AliasedTokenAccounts);
    }

    let (protocol_config, _) = Pubkey::find_program_address(&[b"protocol-config"], &program_id);
    let (vault_authority, _) = Pubkey::find_program_address(
        &[b"vault-authority", merchant.as_ref(), mint.as_ref()],
        &program_id,
    );
    let (policy_attestation, _) = Pubkey::find_program_address(
        &[
            b"policy-attestation",
            merchant.as_ref(),
            mint.as_ref(),
            &request.policy_sequence.to_le_bytes(),
        ],
        &program_id,
    );
    let (split_plan, _) = Pubkey::find_program_address(
        &[
            b"split-plan",
            merchant.as_ref(),
            mint.as_ref(),
            &request.split_plan_version.to_le_bytes(),
        ],
        &program_id,
    );
    let (receipt, _) = Pubkey::find_program_address(
        &[
            b"receipt",
            payer.as_ref(),
            merchant.as_ref(),
            &request.nonce.to_le_bytes(),
        ],
        &program_id,
    );
    let (asset_config, _) = Pubkey::find_program_address(
        &[b"asset-config", mint.as_ref()],
        &program_id,
    );

    let mut accounts = vec![
        account(payer, true, true),
        account(protocol_config, false, false),
        account(merchant, false, false),
        account(source, false, true),
        account(mint, false, false),
        account(tax_vault, false, true),
        account(vault_authority, false, false),
        account(policy_attestation, false, false),
        account(split_plan, false, false),
        account(receipt, false, true),
        account(asset_config, false, false),
        account(token_program, false, false),
        account(
            parse_pubkey("system_program", SYSTEM_PROGRAM_ID)?,
            false,
            false,
        ),
    ];
    accounts.extend(
        recipient_keys
            .iter()
            .copied()
            .map(|recipient| account(recipient, false, true)),
    );
    ensure_unique_account_keys(&accounts, &program_id)?;

    let mut instruction_data = instruction_discriminator("global:settle_split");
    instruction_data.extend_from_slice(&request.nonce.to_le_bytes());
    instruction_data.extend_from_slice(&request.policy_sequence.to_le_bytes());
    instruction_data.extend_from_slice(&request.split_plan_version.to_le_bytes());
    instruction_data.extend_from_slice(&gross_atomic.to_le_bytes());
    instruction_data.extend_from_slice(&tax_atomic.to_le_bytes());
    instruction_data.extend_from_slice(&net_atomic.to_le_bytes());
    instruction_data.extend_from_slice(&request.policy_digest);
    estimate_legacy_settlement_wire_bytes_with_compute_budget(
        accounts.len(),
        instruction_data.len(),
    )?;

    Ok(CompiledSplitSettlementInstruction {
        status: "prepared_unsubmitted".to_string(),
        program_id: program_id.to_string(),
        instruction_data_base64: BASE64_STANDARD.encode(instruction_data),
        accounts,
        protocol_config: protocol_config.to_string(),
        vault_authority: vault_authority.to_string(),
        policy_attestation: policy_attestation.to_string(),
        split_plan: split_plan.to_string(),
        split_plan_digest: split_plan_digest.to_vec(),
        receipt: receipt.to_string(),
        asset_config: asset_config.to_string(),
        compute_budget_policy,
    })
}

/// Hashes the canonical JSON representation emitted by serde for the full
/// compiled instruction artifact. Instruction data alone is intentionally
/// insufficient because account and program substitution must be detectable.
pub fn compiled_instruction_digest(
    instruction: &CompiledSettlementInstruction,
) -> Result<[u8; 32], serde_json::Error> {
    Ok(Sha256::digest(serde_json::to_vec(instruction)?).into())
}

/// Estimates the serialized size of a one-signature legacy transaction with
/// one settlement instruction. The estimate models Solana short-vector
/// lengths, the program key, static account keys, the recent blockhash, and
/// the instruction payload. It is independent of RPC and wallet state.
pub fn estimate_legacy_settlement_wire_bytes(
    instruction_account_count: usize,
    instruction_data_bytes: usize,
) -> Result<usize, SettlementCompileError> {
    if instruction_account_count > u8::MAX as usize
        || instruction_data_bytes > u16::MAX as usize
        || instruction_account_count > MAX_SETTLEMENT_INSTRUCTION_ACCOUNT_COUNT
    {
        return Err(SettlementCompileError::InvalidWireBudget);
    }
    let static_account_count = instruction_account_count
        .checked_add(1)
        .ok_or(SettlementCompileError::InvalidWireBudget)?;
    let signature_bytes = short_vec_len(1)?
        .checked_add(64)
        .ok_or(SettlementCompileError::InvalidWireBudget)?;
    let message_bytes = 3_usize
        .checked_add(short_vec_len(static_account_count)?)
        .and_then(|bytes| bytes.checked_add(static_account_count.checked_mul(32)?))
        .and_then(|bytes| bytes.checked_add(32))
        .and_then(|bytes| bytes.checked_add(short_vec_len(1).ok()?))
        .and_then(|bytes| bytes.checked_add(1))
        .and_then(|bytes| bytes.checked_add(short_vec_len(instruction_account_count).ok()?))
        .and_then(|bytes| bytes.checked_add(instruction_account_count))
        .and_then(|bytes| bytes.checked_add(short_vec_len(instruction_data_bytes).ok()?))
        .and_then(|bytes| bytes.checked_add(instruction_data_bytes))
        .ok_or(SettlementCompileError::InvalidWireBudget)?;
    let total = signature_bytes
        .checked_add(message_bytes)
        .ok_or(SettlementCompileError::InvalidWireBudget)?;
    if total > MAX_SETTLEMENT_SIGNED_BYTES {
        return Err(SettlementCompileError::InvalidWireBudget);
    }
    Ok(total)
}

/// Estimates the serialized size of a one-signature legacy transaction with
/// the governed compute-budget pair followed by one settlement instruction.
/// This is intentionally conservative and network-independent.
pub fn estimate_legacy_settlement_wire_bytes_with_compute_budget(
    instruction_account_count: usize,
    instruction_data_bytes: usize,
) -> Result<usize, SettlementCompileError> {
    if instruction_account_count > u8::MAX as usize
        || instruction_data_bytes > u16::MAX as usize
        || instruction_account_count > MAX_SETTLEMENT_INSTRUCTION_ACCOUNT_COUNT
    {
        return Err(SettlementCompileError::InvalidWireBudget);
    }
    let static_account_count = instruction_account_count
        .checked_add(2)
        .ok_or(SettlementCompileError::InvalidWireBudget)?;
    let signature_bytes = short_vec_len(1)?
        .checked_add(64)
        .ok_or(SettlementCompileError::InvalidWireBudget)?;
    let compute_limit_data = 5_usize;
    let compute_price_data = 9_usize;
    let compute_budget_instruction_bytes =
        (1 + 1 + short_vec_len(compute_limit_data)? + compute_limit_data)
            .checked_add(1 + 1 + short_vec_len(compute_price_data)? + compute_price_data)
            .ok_or(SettlementCompileError::InvalidWireBudget)?;
    let settlement_instruction_bytes = 1_usize
        .checked_add(short_vec_len(instruction_account_count)?)
        .and_then(|bytes| bytes.checked_add(instruction_account_count))
        .and_then(|bytes| bytes.checked_add(short_vec_len(instruction_data_bytes).ok()?))
        .and_then(|bytes| bytes.checked_add(instruction_data_bytes))
        .ok_or(SettlementCompileError::InvalidWireBudget)?;
    let message_bytes = 3_usize
        .checked_add(short_vec_len(static_account_count)?)
        .and_then(|bytes| bytes.checked_add(static_account_count.checked_mul(32)?))
        .and_then(|bytes| bytes.checked_add(32))
        .and_then(|bytes| bytes.checked_add(short_vec_len(3).ok()?))
        .and_then(|bytes| bytes.checked_add(compute_budget_instruction_bytes))
        .and_then(|bytes| bytes.checked_add(settlement_instruction_bytes))
        .ok_or(SettlementCompileError::InvalidWireBudget)?;
    let total = signature_bytes
        .checked_add(message_bytes)
        .ok_or(SettlementCompileError::InvalidWireBudget)?;
    if total > MAX_SETTLEMENT_SIGNED_BYTES {
        return Err(SettlementCompileError::InvalidWireBudget);
    }
    Ok(total)
}

fn short_vec_len(value: usize) -> Result<usize, SettlementCompileError> {
    if value > u16::MAX as usize {
        return Err(SettlementCompileError::InvalidWireBudget);
    }
    let mut remaining = value;
    let mut length = 1_usize;
    while remaining >= 128 {
        remaining >>= 7;
        length = length
            .checked_add(1)
            .ok_or(SettlementCompileError::InvalidWireBudget)?;
    }
    Ok(length)
}

fn parse_pubkey(field: &'static str, value: &str) -> Result<Pubkey, SettlementCompileError> {
    Pubkey::from_str(value.trim()).map_err(|_| SettlementCompileError::InvalidPublicKey { field })
}

fn validate_settlement_program_id(program_id: Pubkey) -> Result<(), SettlementCompileError> {
    if program_id != settlement_program_id() {
        return Err(SettlementCompileError::UnsupportedProgramId);
    }
    Ok(())
}

fn parse_supported_token_program(value: &str) -> Result<Pubkey, SettlementCompileError> {
    let token_program = parse_pubkey("token_program", value)?;
    if token_program != spl_token_program_id()
        && token_program != parse_pubkey("token_program", TOKEN_2022_PROGRAM_ID)?
    {
        return Err(SettlementCompileError::UnsupportedTokenProgram);
    }
    Ok(token_program)
}

fn ensure_unique_account_keys(
    accounts: &[CompiledAccountMeta],
    program_id: &Pubkey,
) -> Result<(), SettlementCompileError> {
    let program_id = program_id.to_string();
    let mut seen = BTreeSet::from([program_id.as_str()]);
    if accounts
        .iter()
        .any(|account| !seen.insert(account.pubkey.as_str()))
    {
        return Err(SettlementCompileError::DuplicateAccountMeta);
    }
    Ok(())
}

fn parse_amount(field: &'static str, value: &str) -> Result<u64, SettlementCompileError> {
    value
        .parse::<u64>()
        .map_err(|_| SettlementCompileError::InvalidAmount { field })
}

fn parse_signed_magnitude(
    field: &'static str,
    sign: u8,
    magnitude: &str,
) -> Result<(u8, u64), SettlementCompileError> {
    if sign > 1 {
        return Err(SettlementCompileError::InvalidAdjustmentSign);
    }
    let magnitude = magnitude
        .parse::<u64>()
        .map_err(|_| SettlementCompileError::InvalidAdjustmentMagnitude { field })?;
    if magnitude == 0 && sign != 0 {
        return Err(SettlementCompileError::InvalidAdjustmentSign);
    }
    Ok((sign, magnitude))
}

fn validate_signed_adjustment(
    kind: u8,
    gross: (u8, u64),
    tax: (u8, u64),
    net: (u8, u64),
) -> Result<(), SettlementCompileError> {
    if matches!(kind, 1 | 2 | 4)
        && (gross.1 == 0 || gross.0 != 1 || (tax.1 > 0 && tax.0 != 1) || (net.1 > 0 && net.0 != 1))
    {
        return Err(SettlementCompileError::InvalidAdjustmentSign);
    }
    let result = if tax.0 == net.0 {
        let magnitude = tax
            .1
            .checked_add(net.1)
            .ok_or(SettlementCompileError::AdjustmentAmountConservationViolation)?;
        (tax.0, magnitude)
    } else if tax.1 >= net.1 {
        (tax.0, tax.1 - net.1)
    } else {
        (net.0, net.1 - tax.1)
    };
    if result != gross || (gross.1 == 0 && tax.1 == 0 && net.1 == 0) {
        return Err(SettlementCompileError::AdjustmentAmountConservationViolation);
    }
    Ok(())
}

fn append_signed_amount(payload: &mut Vec<u8>, amount: (u8, u64)) {
    payload.push(amount.0);
    payload.extend_from_slice(&amount.1.to_le_bytes());
}

fn account(pubkey: Pubkey, is_signer: bool, is_writable: bool) -> CompiledAccountMeta {
    CompiledAccountMeta {
        pubkey: pubkey.to_string(),
        is_signer,
        is_writable,
    }
}

fn settle_discriminator() -> Vec<u8> {
    instruction_discriminator("global:settle")
}

fn instruction_discriminator(name: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.finalize()[..8].to_vec()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct SettlementVector {
        schema: String,
        request: SettlementCompileRequest,
        expected: SettlementVectorExpected,
    }

    #[derive(Deserialize)]
    struct SettlementVectorExpected {
        status: String,
        program_id: String,
        instruction_data_base64_chunks: Vec<String>,
        accounts: Vec<CompiledAccountMeta>,
        protocol_config: String,
        vault_authority: String,
        policy_attestation: String,
        receipt: String,
        asset_config: String,
        compute_budget_policy: ComputeBudgetPolicy,
    }

    #[test]
    fn fixed_program_id_bytes_match_canonical_base58_contract() {
        assert_eq!(settlement_program_id().to_string(), SETTLEMENT_PROGRAM_ID);
        assert_eq!(spl_token_program_id().to_string(), SPL_TOKEN_PROGRAM_ID);
    }

    #[test]
    fn derives_the_mint_scoped_asset_admission_pda() {
        let mint = "GgBaCs3NCBuZN12kCJgAW63ydqohFkHEdfdEXBPzLHq";
        let (asset_config, bump) = asset_config_pda(mint).expect("valid mint");
        assert_eq!(
            asset_config,
            "2nqeGnn1BrXoKjG6H5cSFReSEuDxpb18GH3g8bkt3YVy"
        );
        assert!(bump > 0);
        assert_ne!(asset_config_pda("11111111111111111111111111111111").unwrap().0, asset_config);
    }

    #[test]
    fn matches_the_shared_cross_language_settlement_vector() {
        let vector: SettlementVector = serde_json::from_str(include_str!(
            "../../../test-vectors/settlement-instruction-v1.json"
        ))
        .expect("settlement vector must be valid JSON");
        assert_eq!(
            vector.schema,
            "solana-tax-protocol/settlement-instruction/v1"
        );

        let compiled = compile_settlement_instruction(&vector.request)
            .expect("the shared settlement vector must compile");
        assert_eq!(compiled.status, vector.expected.status);
        assert_eq!(compiled.program_id, vector.expected.program_id);
        assert_eq!(
            compiled.instruction_data_base64,
            vector.expected.instruction_data_base64_chunks.concat()
        );
        assert_eq!(compiled.accounts, vector.expected.accounts);
        assert_eq!(compiled.protocol_config, vector.expected.protocol_config);
        assert_eq!(compiled.vault_authority, vector.expected.vault_authority);
        assert_eq!(
            compiled.policy_attestation,
            vector.expected.policy_attestation
        );
        assert_eq!(compiled.receipt, vector.expected.receipt);
        assert_eq!(compiled.asset_config, vector.expected.asset_config);
        assert_eq!(
            compiled.compute_budget_policy,
            vector.expected.compute_budget_policy
        );
    }

    #[test]
    fn compiles_the_canonical_settlement_instruction() {
        let request = valid_request();
        let compiled = compile_settlement_instruction(&request).expect("valid instruction");

        assert_eq!(compiled.status, "prepared_unsubmitted");
        assert_eq!(compiled.accounts.len(), 13);
        assert_eq!(compiled.accounts[0].pubkey, request.payer);
        assert!(compiled.accounts[0].is_signer);
        assert!(compiled.accounts[0].is_writable);
        assert_eq!(compiled.accounts[1].pubkey, compiled.protocol_config);
        assert_eq!(compiled.accounts[7].pubkey, compiled.policy_attestation);
        assert_eq!(compiled.accounts[8].pubkey, request.net_destination);
        assert_eq!(compiled.accounts[9].pubkey, compiled.receipt);
        assert_eq!(compiled.accounts[10].pubkey, compiled.asset_config);
        assert_eq!(compiled.instruction_data_base64.len(), 108);
        assert_eq!(
            compiled.instruction_data_base64,
            "ryq5V5CDZtQJAAAAAAAAAAMAAAAAAAAAZAAAAAAAAAAZAAAAAAAAAEsAAAAAAAAABwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc="
        );
    }

    fn document_request() -> DocumentSettlementCompileRequest {
        DocumentSettlementCompileRequest {
            program_id: settlement_program_id().to_string(),
            payer: Pubkey::new_unique().to_string(),
            merchant: Pubkey::new_unique().to_string(),
            source: Pubkey::new_unique().to_string(),
            mint: Pubkey::new_unique().to_string(),
            tax_vault: Pubkey::new_unique().to_string(),
            net_destination: Pubkey::new_unique().to_string(),
            token_program: spl_token_program_id().to_string(),
            nonce: 11,
            policy_sequence: 3,
            gross_atomic: "1000".to_string(),
            tax_atomic: "100".to_string(),
            net_atomic: "900".to_string(),
            policy_digest: [7; 32],
            document_attestation_digest: [8; 32],
            payment_scope_digest: [9; 32],
        }
    }

    #[test]
    fn compiles_document_settlement_with_provenance_and_document_receipt() {
        let request = document_request();
        let compiled = compile_document_settlement_instruction(&request).expect("valid document");
        assert_eq!(compiled.accounts.len(), 13);
        assert_eq!(compiled.accounts[9].pubkey, compiled.receipt);
        assert_eq!(compiled.document_attestation_digest, vec![8; 32]);
        assert_eq!(compiled.payment_scope_digest, vec![9; 32]);
        let data = BASE64_STANDARD
            .decode(compiled.instruction_data_base64)
            .expect("instruction data");
        assert_eq!(
            &data[..8],
            &instruction_discriminator("global:settle_document")
        );
        assert_eq!(data.len(), 8 + (8 * 5) + (32 * 3));
    }

    #[test]
    fn document_settlement_rejects_missing_provenance_or_nonconserving_amounts() {
        let mut request = document_request();
        request.document_attestation_digest = [0; 32];
        assert_eq!(
            compile_document_settlement_instruction(&request),
            Err(SettlementCompileError::EmptyDocumentAttestationDigest)
        );
        let mut request = document_request();
        request.payment_scope_digest = [0; 32];
        assert_eq!(
            compile_document_settlement_instruction(&request),
            Err(SettlementCompileError::EmptyPaymentScopeDigest)
        );
        let mut request = document_request();
        request.net_atomic = "901".to_string();
        assert_eq!(
            compile_document_settlement_instruction(&request),
            Err(SettlementCompileError::AmountConservationViolation)
        );
    }

    #[test]
    fn compiles_a_bounded_split_settlement_with_ordered_remaining_accounts() {
        let request = split_request();
        let compiled = compile_split_settlement_instruction(&request).expect("valid split");

        assert_eq!(compiled.status, "prepared_unsubmitted");
        assert_eq!(
            compiled.accounts.len(),
            SPLIT_SETTLEMENT_INSTRUCTION_BASE_ACCOUNT_COUNT + 2
        );
        assert_eq!(compiled.accounts[8].pubkey, compiled.split_plan);
        assert_eq!(
            compiled.accounts[13].pubkey,
            request.recipients[0].token_account
        );
        assert_eq!(
            compiled.accounts[14].pubkey,
            request.recipients[1].token_account
        );
        assert_eq!(
            compiled.split_plan_digest,
            vec![
                245, 27, 219, 197, 243, 59, 244, 252, 226, 232, 224, 208, 52, 31, 115, 134, 54,
                180, 76, 219, 116, 190, 27, 48, 217, 210, 150, 151, 38, 71, 128, 199,
            ]
        );
    }

    #[test]
    fn split_compiler_rejects_duplicate_recipient_accounts() {
        let mut request = split_request();
        request.recipients[1].token_account = request.recipients[0].token_account.clone();
        assert!(matches!(
            compile_split_settlement_instruction(&request),
            Err(SettlementCompileError::InvalidSplitPlan { .. })
        ));
    }

    #[test]
    fn derives_the_pdas_from_the_program_domain() {
        let request = valid_request();
        let compiled = compile_settlement_instruction(&request).expect("valid instruction");
        let program_id = Pubkey::from_str(&request.program_id).expect("program id");
        let (expected_protocol_config, _) =
            Pubkey::find_program_address(&[b"protocol-config"], &program_id);
        let (expected_vault_authority, _) = Pubkey::find_program_address(
            &[
                b"vault-authority",
                Pubkey::from_str(&request.merchant)
                    .expect("merchant")
                    .as_ref(),
                Pubkey::from_str(&request.mint).expect("mint").as_ref(),
            ],
            &program_id,
        );

        assert_eq!(
            compiled.protocol_config,
            expected_protocol_config.to_string()
        );
        assert_eq!(
            compiled.vault_authority,
            expected_vault_authority.to_string()
        );
    }

    #[test]
    fn adjustment_registry_pda_is_derived_from_merchant_and_mint() {
        let merchant = Pubkey::new_from_array([31; 32]).to_string();
        let mint = Pubkey::new_from_array([32; 32]).to_string();
        let (actual, bump) = adjustment_registry_pda(&merchant, &mint).unwrap();
        let (expected, expected_bump) = Pubkey::find_program_address(
            &[
                b"adjustment-registry",
                Pubkey::new_from_array([31; 32]).as_ref(),
                Pubkey::new_from_array([32; 32]).as_ref(),
            ],
            &settlement_program_id(),
        );
        assert_eq!(actual, expected.to_string());
        assert_eq!(bump, expected_bump);
        assert!(adjustment_registry_pda(&merchant, "not-a-public-key").is_err());
    }

    #[test]
    fn rejects_non_conserving_amounts_before_serialization() {
        let mut request = valid_request();
        request.net_atomic = "74".to_string();

        assert!(matches!(
            compile_settlement_instruction(&request),
            Err(SettlementCompileError::AmountConservationViolation)
        ));
    }

    #[test]
    fn maximum_split_shape_stays_below_the_packet_ceiling_with_margin() {
        let estimate = estimate_legacy_settlement_wire_bytes(20, 88).expect("bounded shape");
        assert_eq!(estimate, 885);
        assert_eq!(SOLANA_LEGACY_PACKET_BYTES - estimate, 347);
        assert!(estimate <= MAX_SETTLEMENT_SIGNED_BYTES);
    }

    #[test]
    fn wire_budget_rejects_unrepresentable_instruction_shapes() {
        assert!(estimate_legacy_settlement_wire_bytes(256, 88).is_err());
        assert!(estimate_legacy_settlement_wire_bytes(20, 65_536).is_err());
    }

    #[test]
    fn rejects_aliasing_token_accounts() {
        let mut request = valid_request();
        request.net_destination = request.source.clone();

        assert!(matches!(
            compile_settlement_instruction(&request),
            Err(SettlementCompileError::AliasedTokenAccounts)
        ));
    }

    #[test]
    fn rejects_duplicate_static_account_metas_before_signing() {
        let mut request = valid_request();
        request.merchant = SYSTEM_PROGRAM_ID.to_string();

        assert!(matches!(
            compile_settlement_instruction(&request),
            Err(SettlementCompileError::DuplicateAccountMeta)
        ));
    }

    #[test]
    fn rejects_program_id_as_a_static_account_meta() {
        let mut request = valid_request();
        request.payer = request.program_id.clone();

        assert!(matches!(
            compile_settlement_instruction(&request),
            Err(SettlementCompileError::DuplicateAccountMeta)
        ));
    }

    #[test]
    fn rejects_unsupported_token_programs() {
        let mut request = valid_request();
        request.token_program = Pubkey::new_unique().to_string();

        let error = compile_settlement_instruction(&request).expect_err("must reject");
        assert_eq!(error, SettlementCompileError::UnsupportedTokenProgram);
    }

    #[test]
    fn compiles_the_canonical_on_chain_adjustment_evidence_instruction() {
        let request = valid_adjustment_request();
        let compiled = compile_settlement_adjustment_instruction(&request)
            .expect("valid adjustment evidence instruction");

        assert_eq!(compiled.status, "prepared_unsubmitted");
        assert_eq!(compiled.accounts.len(), 10);
        assert!(compiled.accounts[0].is_signer);
        assert!(compiled.accounts[0].is_writable);
        assert!(!compiled.accounts[1].is_signer);
        assert_eq!(compiled.accounts[7].pubkey, request.original_receipt);
        assert_eq!(compiled.instruction_data_base64.len(), 232);
        assert_eq!(compiled.adjustment_nonce, request.adjustment_nonce);
    }

    #[test]
    fn matches_the_shared_cross_language_adjustment_evidence_vector() {
        #[derive(Deserialize)]
        struct AdjustmentVector {
            schema: String,
            request: SettlementAdjustmentCompileRequest,
            expected: CompiledSettlementAdjustmentInstruction,
        }

        let vector: AdjustmentVector = serde_json::from_str(include_str!(
            "../../../test-vectors/settlement-adjustment-evidence-v1.json"
        ))
        .expect("adjustment evidence vector must be valid JSON");
        assert_eq!(
            vector.schema,
            "solana-tax-protocol/settlement-adjustment-evidence/v1"
        );
        let actual = compile_settlement_adjustment_instruction(&vector.request)
            .expect("shared adjustment vector must compile");
        assert_eq!(actual, vector.expected);
    }

    #[test]
    fn compiled_adjustment_nonce_serializes_as_an_exact_decimal_string() {
        let compiled = compile_settlement_adjustment_instruction(&valid_adjustment_request())
            .expect("valid adjustment evidence instruction");
        let encoded =
            serde_json::to_value(&compiled).expect("compiled instruction is serializable");
        assert_eq!(encoded["adjustment_nonce"], serde_json::json!("11"));
        let decoded: CompiledSettlementAdjustmentInstruction =
            serde_json::from_value(encoded).expect("canonical string must round-trip");
        assert_eq!(decoded.adjustment_nonce, 11);
    }

    #[test]
    fn adjustment_compiler_rejects_non_conserving_signed_deltas() {
        let mut request = valid_adjustment_request();
        request.net_magnitude = "41".to_string();

        assert!(matches!(
            compile_settlement_adjustment_instruction(&request),
            Err(SettlementCompileError::AdjustmentAmountConservationViolation)
        ));
    }

    #[test]
    fn adjustment_compiler_rejects_invalid_refund_sign_and_duplicate_authority() {
        let mut request = valid_adjustment_request();
        request.gross_sign = 0;
        assert!(matches!(
            compile_settlement_adjustment_instruction(&request),
            Err(SettlementCompileError::InvalidAdjustmentSign)
        ));

        let mut request = valid_adjustment_request();
        request.approver = request.proposer.clone();
        assert!(matches!(
            compile_settlement_adjustment_instruction(&request),
            Err(SettlementCompileError::AdjustmentAuthorityConflict)
        ));
    }

    #[test]
    fn adjustment_compiler_derives_a_receipt_bound_pda_and_rejects_zero_digests() {
        let request = valid_adjustment_request();
        let compiled = compile_settlement_adjustment_instruction(&request)
            .expect("valid adjustment evidence instruction");
        let program_id = settlement_program_id();
        let receipt = Pubkey::from_str(&request.original_receipt).expect("receipt");
        let (expected, _) = Pubkey::find_program_address(
            &[
                b"adjustment",
                receipt.as_ref(),
                &request.adjustment_nonce.to_le_bytes(),
            ],
            &program_id,
        );
        assert_eq!(compiled.adjustment, expected.to_string());

        let mut request = valid_adjustment_request();
        request.evidence_digest = [0; 32];
        assert!(matches!(
            compile_settlement_adjustment_instruction(&request),
            Err(SettlementCompileError::EmptyAdjustmentEvidenceDigest)
        ));
    }

    #[test]
    fn compiles_bounded_refund_claim_and_execution_shapes() {
        let claim_request = valid_refund_claim_request();
        let claim = compile_settlement_refund_claim_instruction(&claim_request)
            .expect("valid refund claim");
        assert_eq!(claim.accounts.len(), 15);
        assert!(claim.accounts[0].is_signer);
        assert!(claim.accounts[1].is_signer);
        assert_eq!(claim.accounts[13].pubkey, claim.claim);
        assert_eq!(claim.accounts[14].pubkey, SYSTEM_PROGRAM_ID);

        let execution = compile_settlement_refund_execution_instruction(
            &SettlementRefundExecutionCompileRequest {
                program_id: claim_request.program_id.clone(),
                approver: Pubkey::new_unique().to_string(),
                refund_funder: claim_request.refund_funder.clone(),
                merchant: claim_request.merchant.clone(),
                mint: claim_request.mint.clone(),
                token_program: claim_request.token_program.clone(),
                original_receipt: claim_request.original_receipt.clone(),
                adjustment: claim_request.adjustment.clone(),
                tax_vault: claim_request.tax_vault.clone(),
                net_source: claim_request.net_source.clone(),
                refund_destination: claim_request.refund_destination.clone(),
                claim_nonce: claim_request.claim_nonce,
            },
        )
        .expect("valid refund execution");
        assert_eq!(execution.accounts.len(), 14);
        assert!(execution.accounts[0].is_signer);
        assert!(execution.accounts[9].is_writable);
        assert_eq!(execution.claim, claim.claim);
    }

    #[test]
    fn compiles_exact_governed_refund_cancellation_shape() {
        let request = valid_refund_cancellation_request();
        let compiled = compile_settlement_refund_cancellation_instruction(&request)
            .expect("valid refund cancellation");
        assert_eq!(compiled.accounts.len(), 5);
        assert_eq!(compiled.accounts[0].pubkey, request.governance_authority);
        assert!(compiled.accounts[0].is_signer);
        assert!(compiled.accounts[0].is_writable);
        assert_eq!(compiled.accounts[1].pubkey, request.merchant);
        assert_eq!(compiled.accounts[2].pubkey, request.mint);
        assert_eq!(compiled.accounts[3].pubkey, compiled.registry);
        assert_eq!(compiled.accounts[4].pubkey, compiled.claim);
        assert!(compiled.accounts[4].is_writable);
        assert_eq!(compiled.claim_nonce, 17);

        let mut expected_data = instruction_discriminator("global:cancel_adjustment_refund_claim");
        expected_data.extend_from_slice(&17_u64.to_le_bytes());
        assert_eq!(
            compiled.instruction_data_base64,
            BASE64_STANDARD.encode(expected_data)
        );
    }

    #[test]
    fn refund_cancellation_rejects_zero_or_default_keys_and_aliases() {
        let mut request = valid_refund_cancellation_request();
        request.claim_nonce = 0;
        assert!(matches!(
            compile_settlement_refund_cancellation_instruction(&request),
            Err(SettlementCompileError::InvalidRefundCancellationNonce)
        ));

        let mut request = valid_refund_cancellation_request();
        request.governance_authority = Pubkey::default().to_string();
        assert!(matches!(
            compile_settlement_refund_cancellation_instruction(&request),
            Err(SettlementCompileError::InvalidRefundCancellationKey {
                field: "governance_authority"
            })
        ));

        let mut request = valid_refund_cancellation_request();
        request.merchant = request.governance_authority.clone();
        assert!(matches!(
            compile_settlement_refund_cancellation_instruction(&request),
            Err(SettlementCompileError::AliasedRefundCancellationAccounts)
        ));
    }

    #[test]
    fn matches_the_shared_cross_language_refund_claim_vector() {
        #[derive(Deserialize)]
        struct RefundVector {
            schema: String,
            request: SettlementRefundClaimCompileRequest,
            expected: CompiledSettlementRefundInstruction,
        }

        let vector: RefundVector = serde_json::from_str(include_str!(
            "../../../test-vectors/settlement-refund-claim-v1.json"
        ))
        .expect("refund claim vector must be valid JSON");
        assert_eq!(
            vector.schema,
            "solana-tax-protocol/settlement-refund-claim/v1"
        );
        let actual = compile_settlement_refund_claim_instruction(&vector.request)
            .expect("shared refund claim vector must compile");
        assert_eq!(actual, vector.expected);
    }

    #[test]
    fn refund_compiler_rejects_window_authority_and_account_aliases() {
        let mut request = valid_refund_claim_request();
        request.expires_at_unix = request.execute_after_unix;
        assert!(matches!(
            compile_settlement_refund_claim_instruction(&request),
            Err(SettlementCompileError::InvalidRefundClaimWindow)
        ));

        let mut request = valid_refund_claim_request();
        request.refund_funder = request.proposer.clone();
        assert!(matches!(
            compile_settlement_refund_claim_instruction(&request),
            Err(SettlementCompileError::RefundClaimAuthorityConflict)
        ));

        let mut request = valid_refund_claim_request();
        request.net_source = request.refund_destination.clone();
        assert!(matches!(
            compile_settlement_refund_claim_instruction(&request),
            Err(SettlementCompileError::AliasedRefundTokenAccounts)
        ));
    }

    #[test]
    fn rejects_zero_policy_digest_and_zero_sequences() {
        let mut request = valid_request();
        request.policy_digest = [0; 32];
        assert!(matches!(
            compile_settlement_instruction(&request),
            Err(SettlementCompileError::EmptyPolicyDigest)
        ));

        let mut request = valid_request();
        request.policy_sequence = 0;
        assert!(matches!(
            compile_settlement_instruction(&request),
            Err(SettlementCompileError::InvalidPolicySequence)
        ));
    }

    #[test]
    fn rejects_program_substitution() {
        let mut request = valid_request();
        request.program_id = Pubkey::new_unique().to_string();

        assert!(matches!(
            compile_settlement_instruction(&request),
            Err(SettlementCompileError::UnsupportedProgramId)
        ));
    }

    #[test]
    fn compiled_digest_covers_the_entire_artifact() {
        let instruction = compile_settlement_instruction(&valid_request()).expect("instruction");
        let original = compiled_instruction_digest(&instruction).expect("digest");
        let mut changed = instruction;
        changed.accounts[0].is_writable = !changed.accounts[0].is_writable;
        assert_ne!(
            original,
            compiled_instruction_digest(&changed).expect("digest")
        );
    }

    #[test]
    fn derives_a_canonical_payer_ata_without_rpc_or_ata_creation() {
        let payer = Pubkey::new_unique().to_string();
        let mint = Pubkey::new_unique().to_string();
        let first = derive_source_token_account(&payer, &mint, SPL_TOKEN_PROGRAM_ID)
            .expect("valid payer ATA");
        let second = derive_source_token_account(&payer, &mint, SPL_TOKEN_PROGRAM_ID)
            .expect("same payer ATA");
        assert_eq!(first, second);
        assert_ne!(first, payer);
        assert_ne!(first, mint);

        let token_2022 = derive_source_token_account(&payer, &mint, TOKEN_2022_PROGRAM_ID)
            .expect("valid Token-2022 payer ATA");
        assert_ne!(first, token_2022);
    }

    #[test]
    fn compiles_revenue_stream_plan_with_deterministic_pda_and_optional_window() {
        let request = valid_revenue_stream_plan_request();
        let compiled = compile_revenue_stream_plan_instruction(&request).expect("stream plan");
        assert_eq!(compiled.accounts.len(), 8);
        assert_eq!(compiled.accrual, None);
        assert_eq!(compiled.claim, None);
        assert_eq!(
            compiled.plan,
            revenue_stream_plan_pda(&request.merchant, &request.mint, request.plan_digest)
                .expect("plan PDA")
        );
        let bytes = BASE64_STANDARD
            .decode(compiled.instruction_data_base64)
            .expect("instruction bytes");
        assert_eq!(
            &bytes[..8],
            instruction_discriminator("global:initialize_revenue_stream_plan")
        );
        assert_eq!(bytes[8 + 32 + 32 + 32 + 32 + 32 + 32 + 8], 1);
    }

    #[test]
    fn revenue_stream_compilers_cover_accrual_claim_execute_and_cancel_shapes() {
        let plan_request = valid_revenue_stream_plan_request();
        let plan = compile_revenue_stream_plan_instruction(&plan_request).expect("plan");
        let accrual_request = RevenueStreamAccrualCompileRequest {
            program_id: settlement_program_id().to_string(),
            operator: plan_request.operator.clone(),
            plan: plan.plan.clone(),
            source_receipt: Pubkey::new_unique().to_string(),
            recipient_atomic: 25,
            finalized_slot: 101,
            evidence_digest: [11; 32],
        };
        let accrual =
            compile_revenue_stream_accrual_instruction(&accrual_request).expect("accrual");
        assert_eq!(accrual.accounts.len(), 6);
        assert_eq!(
            accrual.accrual.as_deref(),
            Some(
                revenue_stream_accrual_pda(&plan.plan, &accrual_request.source_receipt)
                    .expect("accrual PDA")
                    .as_str()
            )
        );

        let claim_request = RevenueStreamClaimCompileRequest {
            program_id: settlement_program_id().to_string(),
            operator: plan_request.operator.clone(),
            plan: plan.plan.clone(),
            mint: plan_request.mint.clone(),
            token_program: SPL_TOKEN_PROGRAM_ID.to_string(),
            tax_vault: plan_request.tax_vault.clone(),
            destination: Pubkey::new_unique().to_string(),
            nonce: 1,
            amount_atomic: 10,
            claim_digest: [12; 32],
            evidence_digest: [13; 32],
            execute_after_unix: 1_800_000_000,
            expires_at_unix: 1_800_000_600,
        };
        let claim = compile_revenue_stream_claim_instruction(&claim_request).expect("claim");
        assert_eq!(claim.accounts.len(), 9);
        assert_eq!(
            claim.claim.as_deref(),
            Some(
                revenue_stream_claim_pda(&plan.plan, claim_request.nonce)
                    .expect("claim PDA")
                    .as_str()
            )
        );

        let execute = compile_revenue_stream_claim_execution_instruction(
            &RevenueStreamClaimExecutionCompileRequest {
                program_id: settlement_program_id().to_string(),
                approver: plan_request.approver.clone(),
                plan: plan.plan.clone(),
                merchant: plan_request.merchant.clone(),
                mint: plan_request.mint.clone(),
                token_program: SPL_TOKEN_PROGRAM_ID.to_string(),
                tax_vault: plan_request.tax_vault.clone(),
                destination: claim_request.destination.clone(),
                nonce: 1,
            },
        )
        .expect("execution");
        assert_eq!(execute.accounts.len(), 9);
        assert_eq!(
            execute.accounts[6].pubkey,
            Pubkey::find_program_address(
                &[
                    b"vault-authority",
                    Pubkey::from_str(&plan_request.merchant)
                        .expect("merchant")
                        .as_ref(),
                    Pubkey::from_str(&plan_request.mint).expect("mint").as_ref(),
                ],
                &settlement_program_id(),
            )
            .0
            .to_string()
        );

        let cancel = compile_revenue_stream_claim_cancellation_instruction(
            &RevenueStreamClaimCancellationCompileRequest {
                program_id: settlement_program_id().to_string(),
                governance_authority: plan_request.governance_authority,
                plan: plan.plan,
                nonce: 1,
            },
        )
        .expect("cancellation");
        assert_eq!(cancel.accounts.len(), 3);
    }

    #[test]
    fn revenue_stream_compiler_rejects_authority_aliases_windows_and_account_aliases() {
        let mut request = valid_revenue_stream_plan_request();
        request.operator = request.approver.clone();
        assert!(matches!(
            compile_revenue_stream_plan_instruction(&request),
            Err(SettlementCompileError::RevenueStreamAuthorityConflict)
        ));

        let mut claim = valid_revenue_stream_claim_request();
        claim.expires_at_unix = claim.execute_after_unix;
        assert!(matches!(
            compile_revenue_stream_claim_instruction(&claim),
            Err(SettlementCompileError::InvalidRevenueStreamWindow)
        ));

        let mut claim = valid_revenue_stream_claim_request();
        claim.destination = claim.tax_vault.clone();
        assert!(matches!(
            compile_revenue_stream_claim_instruction(&claim),
            Err(SettlementCompileError::AliasedRevenueStreamAccounts)
        ));
    }

    fn valid_request() -> SettlementCompileRequest {
        SettlementCompileRequest {
            program_id: settlement_program_id().to_string(),
            payer: Pubkey::new_unique().to_string(),
            merchant: Pubkey::new_unique().to_string(),
            source: Pubkey::new_unique().to_string(),
            mint: Pubkey::new_unique().to_string(),
            tax_vault: Pubkey::new_unique().to_string(),
            net_destination: Pubkey::new_unique().to_string(),
            token_program: spl_token_program_id().to_string(),
            nonce: 9,
            policy_sequence: 3,
            gross_atomic: "100".to_string(),
            tax_atomic: "25".to_string(),
            net_atomic: "75".to_string(),
            policy_digest: [7; 32],
        }
    }

    fn split_request() -> SplitSettlementCompileRequest {
        SplitSettlementCompileRequest {
            program_id: settlement_program_id().to_string(),
            payer: "cGfHiC6Kgg3FpFZvgwGcswsCRtp4aBP2fzuXRQPizuN".to_string(),
            merchant: "8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR".to_string(),
            source: Pubkey::new_unique().to_string(),
            mint: "GgBaCs3NCBuZN12kCJgAW63ydqohFkHEdfdEXBPzLHq".to_string(),
            tax_vault: "LbUiWL3xVV8hTFYBVdbTNrpDo41NKS6o3LHHuDzjfcY".to_string(),
            token_program: SPL_TOKEN_PROGRAM_ID.to_string(),
            nonce: 9,
            policy_sequence: 3,
            split_plan_version: 1,
            recipients: vec![
                SplitRecipient {
                    token_account: "QWmroo4YnnMqYW3cnxWkFdaTxGD3P7vMSzwMHGbUzwF".to_string(),
                    basis_points: 7_000,
                },
                SplitRecipient {
                    token_account: "CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8".to_string(),
                    basis_points: 3_000,
                },
            ],
            gross_atomic: "100".to_string(),
            tax_atomic: "25".to_string(),
            net_atomic: "75".to_string(),
            policy_digest: [7; 32],
        }
    }

    fn valid_adjustment_request() -> SettlementAdjustmentCompileRequest {
        SettlementAdjustmentCompileRequest {
            program_id: settlement_program_id().to_string(),
            proposer: Pubkey::new_unique().to_string(),
            approver: Pubkey::new_unique().to_string(),
            merchant: Pubkey::new_unique().to_string(),
            mint: Pubkey::new_unique().to_string(),
            token_program: SPL_TOKEN_PROGRAM_ID.to_string(),
            original_receipt: Pubkey::new_unique().to_string(),
            adjustment_nonce: 11,
            kind: 1,
            gross_sign: 1,
            gross_magnitude: "100".to_string(),
            tax_sign: 1,
            tax_magnitude: "25".to_string(),
            net_sign: 1,
            net_magnitude: "75".to_string(),
            adjustment_digest: [3; 32],
            evidence_digest: [4; 32],
            source_calculation_digest: [5; 32],
            replacement_calculation_digest: [6; 32],
        }
    }

    fn valid_refund_claim_request() -> SettlementRefundClaimCompileRequest {
        SettlementRefundClaimCompileRequest {
            program_id: settlement_program_id().to_string(),
            proposer: Pubkey::new_unique().to_string(),
            refund_funder: Pubkey::new_unique().to_string(),
            merchant: Pubkey::new_unique().to_string(),
            mint: Pubkey::new_unique().to_string(),
            token_program: SPL_TOKEN_PROGRAM_ID.to_string(),
            original_receipt: Pubkey::new_unique().to_string(),
            adjustment: Pubkey::new_unique().to_string(),
            tax_vault: Pubkey::new_unique().to_string(),
            net_source: Pubkey::new_unique().to_string(),
            refund_destination: Pubkey::new_unique().to_string(),
            claim_nonce: 3,
            execute_after_unix: 1_800_000_000,
            expires_at_unix: 1_800_000_600,
            reference_digest: [9; 32],
        }
    }

    fn valid_refund_cancellation_request() -> SettlementRefundCancellationCompileRequest {
        SettlementRefundCancellationCompileRequest {
            program_id: settlement_program_id().to_string(),
            governance_authority: Pubkey::new_unique().to_string(),
            merchant: Pubkey::new_unique().to_string(),
            mint: Pubkey::new_unique().to_string(),
            adjustment: Pubkey::new_unique().to_string(),
            claim_nonce: 17,
        }
    }

    fn valid_revenue_stream_plan_request() -> RevenueStreamPlanCompileRequest {
        RevenueStreamPlanCompileRequest {
            program_id: settlement_program_id().to_string(),
            governance_authority: Pubkey::new_unique().to_string(),
            merchant: Pubkey::new_unique().to_string(),
            mint: Pubkey::new_unique().to_string(),
            token_program: SPL_TOKEN_PROGRAM_ID.to_string(),
            tax_vault: Pubkey::new_unique().to_string(),
            recipient_owner: Pubkey::new_unique().to_string(),
            proposer: Pubkey::new_unique().to_string(),
            approver: Pubkey::new_unique().to_string(),
            operator: Pubkey::new_unique().to_string(),
            plan_digest: [21; 32],
            policy_digest: [22; 32],
            start_slot: 100,
            end_slot: Some(200),
            max_total_atomic: 1_000,
            max_claim_atomic: 100,
        }
    }

    fn valid_revenue_stream_claim_request() -> RevenueStreamClaimCompileRequest {
        let plan_request = valid_revenue_stream_plan_request();
        RevenueStreamClaimCompileRequest {
            program_id: settlement_program_id().to_string(),
            operator: plan_request.operator,
            plan: revenue_stream_plan_pda(
                &plan_request.merchant,
                &plan_request.mint,
                plan_request.plan_digest,
            )
            .expect("plan PDA"),
            mint: plan_request.mint,
            token_program: SPL_TOKEN_PROGRAM_ID.to_string(),
            tax_vault: plan_request.tax_vault,
            destination: Pubkey::new_unique().to_string(),
            nonce: 1,
            amount_atomic: 10,
            claim_digest: [31; 32],
            evidence_digest: [32; 32],
            execute_after_unix: 1_800_000_000,
            expires_at_unix: 1_800_000_600,
        }
    }
}
