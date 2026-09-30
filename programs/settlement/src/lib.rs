#![deny(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]
// Anchor's generated dispatch wrappers preserve the published instruction
// ABI and therefore exceed Clippy's ordinary Rust helper threshold.
#![allow(clippy::too_many_arguments)]
// Anchor's stable proc-macros emit target-specific cfgs for SBF and host builds.
#![allow(unexpected_cfgs)]

use anchor_lang::{Ids, prelude::*};
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};
use sha2::{Digest, Sha256};

const SPLIT_PLAN_SCHEMA: &[u8] = b"solana-tax-protocol/split-plan/v1";
const MAX_SPLIT_RECIPIENTS: usize = 8;
const BASIS_POINTS_DENOMINATOR: u64 = 10_000;
const MAX_RELEASE_WINDOW_SECONDS: i64 = 90 * 24 * 60 * 60;

declare_id!("4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi");

#[program]
pub mod settlement {
    use super::*;

    pub fn initialize_protocol_config(ctx: Context<InitializeProtocolConfig>) -> Result<()> {
        let config = &mut ctx.accounts.protocol_config;
        config.authority = ctx.accounts.authority.key();
        config.paused = false;
        config.bump = ctx.bumps.protocol_config;
        emit!(ProtocolPauseChanged {
            authority: ctx.accounts.authority.key(),
            paused: false,
        });
        Ok(())
    }

    pub fn set_protocol_paused(ctx: Context<UpdateProtocolConfig>, paused: bool) -> Result<()> {
        validate_protocol_authority(&ctx.accounts.protocol_config, ctx.accounts.authority.key())?;
        ctx.accounts.protocol_config.paused = paused;
        emit!(ProtocolPauseChanged {
            authority: ctx.accounts.authority.key(),
            paused,
        });
        Ok(())
    }

    pub fn initialize_policy_issuer_registry(
        ctx: Context<InitializePolicyIssuerRegistry>,
        issuer: Pubkey,
    ) -> Result<()> {
        validate_new_issuer(issuer)?;
        validate_supported_mint_program(
            &ctx.accounts.token_program,
            &ctx.accounts.mint.to_account_info(),
        )?;
        let registry = &mut ctx.accounts.issuer_registry;
        registry.merchant = ctx.accounts.merchant.key();
        registry.mint = ctx.accounts.mint.key();
        registry.authority = ctx.accounts.authority.key();
        registry.issuer = issuer;
        registry.active = true;
        registry.bump = ctx.bumps.issuer_registry;
        Ok(())
    }

    pub fn rotate_policy_issuer(
        ctx: Context<UpdatePolicyIssuerRegistry>,
        issuer: Pubkey,
    ) -> Result<()> {
        validate_registry_update(
            &ctx.accounts.issuer_registry,
            ctx.accounts.authority.key(),
            issuer,
        )?;
        ctx.accounts.issuer_registry.issuer = issuer;
        Ok(())
    }

    pub fn set_policy_issuer_active(
        ctx: Context<UpdatePolicyIssuerRegistry>,
        active: bool,
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.issuer_registry.authority,
            ctx.accounts.authority.key(),
            SettlementError::RegistryAuthorityMismatch
        );
        ctx.accounts.issuer_registry.active = active;
        Ok(())
    }

    pub fn register_policy_attestation(
        ctx: Context<RegisterPolicyAttestation>,
        sequence: u64,
        gross_atomic: u64,
        tax_atomic: u64,
        net_atomic: u64,
        policy_digest: [u8; 32],
        expires_at_unix: i64,
    ) -> Result<()> {
        validate_registry_for_registration(
            &ctx.accounts.issuer_registry,
            ctx.accounts.merchant.key(),
            ctx.accounts.mint.key(),
            ctx.accounts.issuer.key(),
        )?;
        validate_supported_mint_program(
            &ctx.accounts.token_program,
            &ctx.accounts.mint.to_account_info(),
        )?;
        let now_unix = Clock::get()?.unix_timestamp;
        validate_registration(
            sequence,
            gross_atomic,
            tax_atomic,
            net_atomic,
            policy_digest,
            expires_at_unix,
            now_unix,
        )?;
        let policy = &mut ctx.accounts.policy_attestation;
        policy.merchant = ctx.accounts.merchant.key();
        policy.mint = ctx.accounts.mint.key();
        policy.issuer = ctx.accounts.issuer.key();
        policy.policy_digest = policy_digest;
        policy.sequence = sequence;
        policy.gross_atomic = gross_atomic;
        policy.tax_atomic = tax_atomic;
        policy.net_atomic = net_atomic;
        policy.expires_at_unix = expires_at_unix;
        policy.revoked = false;
        policy.bump = ctx.bumps.policy_attestation;
        Ok(())
    }

    pub fn initialize_split_plan(
        ctx: Context<InitializeSplitPlan>,
        version: u64,
        recipients: Vec<SplitRecipient>,
    ) -> Result<()> {
        validate_split_plan_shape(&recipients, version)?;
        validate_supported_mint_program(
            &ctx.accounts.token_program,
            &ctx.accounts.mint.to_account_info(),
        )?;
        let plan = &mut ctx.accounts.split_plan;
        plan.merchant = ctx.accounts.merchant.key();
        plan.mint = ctx.accounts.mint.key();
        plan.version = version;
        plan.recipients = recipients;
        plan.digest =
            calculate_split_plan_digest(plan.merchant, plan.mint, plan.version, &plan.recipients);
        plan.bump = ctx.bumps.split_plan;
        Ok(())
    }

    pub fn initialize_vault_release_registry(
        ctx: Context<InitializeVaultReleaseRegistry>,
        proposer: Pubkey,
        approver: Pubkey,
    ) -> Result<()> {
        validate_release_registry_authorities(
            ctx.accounts.governance_authority.key(),
            proposer,
            approver,
        )?;
        validate_supported_mint_program(
            &ctx.accounts.token_program,
            &ctx.accounts.mint.to_account_info(),
        )?;
        let registry = &mut ctx.accounts.registry;
        registry.merchant = ctx.accounts.merchant.key();
        registry.mint = ctx.accounts.mint.key();
        registry.governance_authority = ctx.accounts.governance_authority.key();
        registry.proposer = proposer;
        registry.approver = approver;
        registry.active = true;
        registry.bump = ctx.bumps.registry;
        Ok(())
    }

    pub fn update_vault_release_registry(
        ctx: Context<UpdateVaultReleaseRegistry>,
        proposer: Pubkey,
        approver: Pubkey,
        active: bool,
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.registry.governance_authority,
            ctx.accounts.governance_authority.key(),
            SettlementError::ReleaseRegistryAuthorityMismatch
        );
        validate_release_registry_authorities(
            ctx.accounts.governance_authority.key(),
            proposer,
            approver,
        )?;
        ctx.accounts.registry.proposer = proposer;
        ctx.accounts.registry.approver = approver;
        ctx.accounts.registry.active = active;
        Ok(())
    }

    pub fn initialize_adjustment_registry(
        ctx: Context<InitializeAdjustmentRegistry>,
        proposer: Pubkey,
        approver: Pubkey,
    ) -> Result<()> {
        validate_release_registry_authorities(
            ctx.accounts.governance_authority.key(),
            proposer,
            approver,
        )?;
        validate_supported_mint_program(
            &ctx.accounts.token_program,
            &ctx.accounts.mint.to_account_info(),
        )?;
        let registry = &mut ctx.accounts.registry;
        registry.merchant = ctx.accounts.merchant.key();
        registry.mint = ctx.accounts.mint.key();
        registry.governance_authority = ctx.accounts.governance_authority.key();
        registry.proposer = proposer;
        registry.approver = approver;
        registry.active = true;
        registry.bump = ctx.bumps.registry;
        Ok(())
    }

    pub fn update_adjustment_registry(
        ctx: Context<UpdateAdjustmentRegistry>,
        proposer: Pubkey,
        approver: Pubkey,
        active: bool,
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.registry.governance_authority,
            ctx.accounts.governance_authority.key(),
            SettlementError::AdjustmentRegistryAuthorityMismatch
        );
        validate_release_registry_authorities(
            ctx.accounts.governance_authority.key(),
            proposer,
            approver,
        )?;
        ctx.accounts.registry.proposer = proposer;
        ctx.accounts.registry.approver = approver;
        ctx.accounts.registry.active = active;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_vault_release_claim(
        ctx: Context<CreateVaultReleaseClaim>,
        nonce: u64,
        amount_atomic: u64,
        reference_digest: [u8; 32],
        execute_after_unix: i64,
        expires_at_unix: i64,
    ) -> Result<()> {
        validate_release_registry_scope(
            &ctx.accounts.registry,
            ctx.accounts.merchant.key(),
            ctx.accounts.mint.key(),
        )?;
        require!(
            ctx.accounts.registry.active,
            SettlementError::ReleaseRegistryInactive
        );
        require_keys_eq!(
            ctx.accounts.registry.proposer,
            ctx.accounts.proposer.key(),
            SettlementError::ReleaseProposerMismatch
        );
        validate_supported_release_accounts(
            &ctx.accounts.token_program,
            &ctx.accounts.mint,
            &ctx.accounts.tax_vault,
            &ctx.accounts.destination,
        )?;
        require_keys_eq!(
            ctx.accounts.tax_vault.owner,
            ctx.accounts.vault_authority.key(),
            SettlementError::ReleaseVaultAuthorityMismatch
        );
        require_keys_eq!(
            ctx.accounts.destination.mint,
            ctx.accounts.mint.key(),
            SettlementError::DestinationMintMismatch
        );
        require_keys_neq!(
            ctx.accounts.tax_vault.key(),
            ctx.accounts.destination.key(),
            SettlementError::AliasedTokenAccounts
        );
        validate_release_claim_inputs(
            amount_atomic,
            reference_digest,
            execute_after_unix,
            expires_at_unix,
            Clock::get()?.unix_timestamp,
        )?;

        let claim = &mut ctx.accounts.claim;
        claim.merchant = ctx.accounts.merchant.key();
        claim.mint = ctx.accounts.mint.key();
        claim.nonce = nonce;
        claim.amount_atomic = amount_atomic;
        claim.destination = ctx.accounts.destination.key();
        claim.reference_digest = reference_digest;
        claim.execute_after_unix = execute_after_unix;
        claim.expires_at_unix = expires_at_unix;
        claim.proposer = ctx.accounts.proposer.key();
        claim.executed = false;
        claim.cancelled = false;
        claim.bump = ctx.bumps.claim;

        emit!(VaultReleaseClaimCreated {
            claim: claim.key(),
            merchant: claim.merchant,
            mint: claim.mint,
            nonce,
            amount_atomic,
            destination: claim.destination,
            reference_digest,
            proposer: claim.proposer,
            execute_after_unix,
            expires_at_unix,
        });
        Ok(())
    }

    pub fn execute_vault_release(ctx: Context<ExecuteVaultRelease>, nonce: u64) -> Result<()> {
        let execution = ReleaseExecutionContext {
            merchant: ctx.accounts.merchant.key(),
            mint: ctx.accounts.mint.key(),
            vault_token_account: ctx.accounts.tax_vault.key(),
            destination: ctx.accounts.destination.key(),
            now_unix: Clock::get()?.unix_timestamp,
            vault_amount: ctx.accounts.tax_vault.amount,
        };
        validate_release_claim_for_execution(
            &ctx.accounts.registry,
            &ctx.accounts.claim,
            ctx.accounts.approver.key(),
            &execution,
        )?;
        require!(
            ctx.accounts.claim.nonce == nonce,
            SettlementError::ReleaseClaimBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.claim.destination,
            ctx.accounts.destination.key(),
            SettlementError::ReleaseDestinationMismatch
        );
        validate_supported_release_accounts(
            &ctx.accounts.token_program,
            &ctx.accounts.mint,
            &ctx.accounts.tax_vault,
            &ctx.accounts.destination,
        )?;
        require_keys_eq!(
            ctx.accounts.tax_vault.owner,
            ctx.accounts.vault_authority.key(),
            SettlementError::ReleaseVaultAuthorityMismatch
        );

        let merchant_key = ctx.accounts.merchant.key();
        let mint_key = ctx.accounts.mint.key();
        let signer_seeds: &[&[u8]] = &[
            b"vault-authority",
            merchant_key.as_ref(),
            mint_key.as_ref(),
            &[ctx.bumps.vault_authority],
        ];
        let transfer_accounts = TransferChecked {
            from: ctx.accounts.tax_vault.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.destination.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                transfer_accounts,
                &[signer_seeds],
            ),
            ctx.accounts.claim.amount_atomic,
            ctx.accounts.mint.decimals,
        )?;
        ctx.accounts.claim.executed = true;
        emit!(VaultReleaseExecuted {
            claim: ctx.accounts.claim.key(),
            merchant: ctx.accounts.claim.merchant,
            mint: ctx.accounts.claim.mint,
            nonce,
            amount_atomic: ctx.accounts.claim.amount_atomic,
            destination: ctx.accounts.claim.destination,
            reference_digest: ctx.accounts.claim.reference_digest,
            approver: ctx.accounts.approver.key(),
        });
        Ok(())
    }

    pub fn cancel_vault_release_claim(
        ctx: Context<CancelVaultReleaseClaim>,
        nonce: u64,
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.registry.governance_authority,
            ctx.accounts.governance_authority.key(),
            SettlementError::ReleaseRegistryAuthorityMismatch
        );
        validate_release_registry_scope(
            &ctx.accounts.registry,
            ctx.accounts.merchant.key(),
            ctx.accounts.mint.key(),
        )?;
        require_keys_eq!(
            ctx.accounts.claim.merchant,
            ctx.accounts.merchant.key(),
            SettlementError::ReleaseClaimBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.claim.mint,
            ctx.accounts.mint.key(),
            SettlementError::ReleaseClaimBindingMismatch
        );
        require!(
            ctx.accounts.claim.nonce == nonce,
            SettlementError::ReleaseClaimBindingMismatch
        );
        require!(
            !ctx.accounts.claim.executed && !ctx.accounts.claim.cancelled,
            SettlementError::ReleaseClaimAlreadyFinalized
        );
        ctx.accounts.claim.cancelled = true;
        emit!(VaultReleaseClaimCancelled {
            claim: ctx.accounts.claim.key(),
            merchant: ctx.accounts.claim.merchant,
            mint: ctx.accounts.claim.mint,
            nonce,
            reference_digest: ctx.accounts.claim.reference_digest,
            governance_authority: ctx.accounts.governance_authority.key(),
        });
        Ok(())
    }

    pub fn revoke_policy_attestation(ctx: Context<RevokePolicyAttestation>) -> Result<()> {
        validate_revocation_authority(
            &ctx.accounts.issuer_registry,
            &ctx.accounts.policy_attestation,
            ctx.accounts.authority.key(),
        )?;
        ctx.accounts.policy_attestation.revoked = true;
        Ok(())
    }

    pub fn settle(
        ctx: Context<Settle>,
        nonce: u64,
        policy_sequence: u64,
        gross_atomic: u64,
        tax_atomic: u64,
        net_atomic: u64,
        policy_digest: [u8; 32],
    ) -> Result<()> {
        validate_protocol_not_paused(&ctx.accounts.protocol_config)?;
        validate_policy_attestation(
            &ctx.accounts.policy_attestation,
            PolicySettlementBinding {
                merchant: ctx.accounts.merchant.key(),
                mint: ctx.accounts.mint.key(),
                sequence: policy_sequence,
                gross_atomic,
                tax_atomic,
                net_atomic,
                policy_digest,
            },
            Clock::get()?.unix_timestamp,
        )?;
        validate_settlement_amounts(gross_atomic, tax_atomic, net_atomic)?;
        validate_supported_token_accounts(
            &ctx.accounts.token_program,
            &ctx.accounts.mint,
            &ctx.accounts.source,
            &ctx.accounts.tax_vault,
            &ctx.accounts.net_destination,
        )?;
        require_keys_eq!(
            ctx.accounts.source.owner,
            ctx.accounts.payer.key(),
            SettlementError::SourceAuthorityMismatch
        );
        require_keys_eq!(
            ctx.accounts.source.mint,
            ctx.accounts.mint.key(),
            SettlementError::SourceMintMismatch
        );
        require_keys_eq!(
            ctx.accounts.tax_vault.mint,
            ctx.accounts.mint.key(),
            SettlementError::TaxVaultMintMismatch
        );
        require_keys_eq!(
            ctx.accounts.tax_vault.owner,
            ctx.accounts.vault_authority.key(),
            SettlementError::TaxVaultAuthorityMismatch
        );
        require_keys_eq!(
            ctx.accounts.net_destination.mint,
            ctx.accounts.mint.key(),
            SettlementError::DestinationMintMismatch
        );
        require_keys_neq!(
            ctx.accounts.source.key(),
            ctx.accounts.tax_vault.key(),
            SettlementError::AliasedTokenAccounts
        );
        require_keys_neq!(
            ctx.accounts.source.key(),
            ctx.accounts.net_destination.key(),
            SettlementError::AliasedTokenAccounts
        );
        require_keys_neq!(
            ctx.accounts.tax_vault.key(),
            ctx.accounts.net_destination.key(),
            SettlementError::AliasedTokenAccounts
        );
        require!(
            ctx.accounts.source.amount >= gross_atomic,
            SettlementError::InsufficientSourceFunds
        );

        if tax_atomic > 0 {
            let transfer_accounts = TransferChecked {
                from: ctx.accounts.source.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.tax_vault.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            };
            token_interface::transfer_checked(
                CpiContext::new(ctx.accounts.token_program.key(), transfer_accounts),
                tax_atomic,
                ctx.accounts.mint.decimals,
            )?;
        }

        if net_atomic > 0 {
            let transfer_accounts = TransferChecked {
                from: ctx.accounts.source.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.net_destination.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            };
            token_interface::transfer_checked(
                CpiContext::new(ctx.accounts.token_program.key(), transfer_accounts),
                net_atomic,
                ctx.accounts.mint.decimals,
            )?;
        }

        let receipt = &mut ctx.accounts.receipt;
        receipt.payer = ctx.accounts.payer.key();
        receipt.merchant = ctx.accounts.merchant.key();
        receipt.mint = ctx.accounts.mint.key();
        receipt.nonce = nonce;
        receipt.gross_atomic = gross_atomic;
        receipt.tax_atomic = tax_atomic;
        receipt.net_atomic = net_atomic;
        receipt.policy_digest = policy_digest;
        receipt.bump = ctx.bumps.receipt;

        emit!(SettlementExecuted {
            receipt: receipt.key(),
            payer: receipt.payer,
            merchant: receipt.merchant,
            mint: receipt.mint,
            nonce,
            gross_atomic,
            tax_atomic,
            net_atomic,
            policy_digest,
        });

        Ok(())
    }

    /// Settles a server-bound document payment. This is intentionally a
    /// separate instruction from aggregate `settle`: the receipt records the
    /// source document attestation and the exact whole-document/line-subset
    /// payment scope commitment.
    #[allow(clippy::too_many_arguments)]
    pub fn settle_document(
        ctx: Context<SettleDocument>,
        nonce: u64,
        policy_sequence: u64,
        gross_atomic: u64,
        tax_atomic: u64,
        net_atomic: u64,
        policy_digest: [u8; 32],
        document_attestation_digest: [u8; 32],
        payment_scope_digest: [u8; 32],
    ) -> Result<()> {
        validate_protocol_not_paused(&ctx.accounts.protocol_config)?;
        validate_policy_attestation(
            &ctx.accounts.policy_attestation,
            PolicySettlementBinding {
                merchant: ctx.accounts.merchant.key(),
                mint: ctx.accounts.mint.key(),
                sequence: policy_sequence,
                gross_atomic,
                tax_atomic,
                net_atomic,
                policy_digest,
            },
            Clock::get()?.unix_timestamp,
        )?;
        require!(
            document_attestation_digest != [0; 32],
            SettlementError::EmptyDocumentAttestationDigest
        );
        require!(
            payment_scope_digest != [0; 32],
            SettlementError::EmptyPaymentScopeDigest
        );
        validate_settlement_amounts(gross_atomic, tax_atomic, net_atomic)?;
        validate_supported_token_accounts(
            &ctx.accounts.token_program,
            &ctx.accounts.mint,
            &ctx.accounts.source,
            &ctx.accounts.tax_vault,
            &ctx.accounts.net_destination,
        )?;
        require_keys_eq!(
            ctx.accounts.source.owner,
            ctx.accounts.payer.key(),
            SettlementError::SourceAuthorityMismatch
        );
        require_keys_eq!(
            ctx.accounts.source.mint,
            ctx.accounts.mint.key(),
            SettlementError::SourceMintMismatch
        );
        require_keys_eq!(
            ctx.accounts.tax_vault.mint,
            ctx.accounts.mint.key(),
            SettlementError::TaxVaultMintMismatch
        );
        require_keys_eq!(
            ctx.accounts.tax_vault.owner,
            ctx.accounts.vault_authority.key(),
            SettlementError::TaxVaultAuthorityMismatch
        );
        require_keys_eq!(
            ctx.accounts.net_destination.mint,
            ctx.accounts.mint.key(),
            SettlementError::DestinationMintMismatch
        );
        require_keys_neq!(
            ctx.accounts.source.key(),
            ctx.accounts.tax_vault.key(),
            SettlementError::AliasedTokenAccounts
        );
        require_keys_neq!(
            ctx.accounts.source.key(),
            ctx.accounts.net_destination.key(),
            SettlementError::AliasedTokenAccounts
        );
        require_keys_neq!(
            ctx.accounts.tax_vault.key(),
            ctx.accounts.net_destination.key(),
            SettlementError::AliasedTokenAccounts
        );
        require!(
            ctx.accounts.source.amount >= gross_atomic,
            SettlementError::InsufficientSourceFunds
        );

        if tax_atomic > 0 {
            let transfer_accounts = TransferChecked {
                from: ctx.accounts.source.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.tax_vault.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            };
            token_interface::transfer_checked(
                CpiContext::new(ctx.accounts.token_program.key(), transfer_accounts),
                tax_atomic,
                ctx.accounts.mint.decimals,
            )?;
        }
        if net_atomic > 0 {
            let transfer_accounts = TransferChecked {
                from: ctx.accounts.source.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.net_destination.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            };
            token_interface::transfer_checked(
                CpiContext::new(ctx.accounts.token_program.key(), transfer_accounts),
                net_atomic,
                ctx.accounts.mint.decimals,
            )?;
        }

        let receipt = &mut ctx.accounts.receipt;
        receipt.payer = ctx.accounts.payer.key();
        receipt.merchant = ctx.accounts.merchant.key();
        receipt.mint = ctx.accounts.mint.key();
        receipt.nonce = nonce;
        receipt.gross_atomic = gross_atomic;
        receipt.tax_atomic = tax_atomic;
        receipt.net_atomic = net_atomic;
        receipt.policy_digest = policy_digest;
        receipt.document_attestation_digest = document_attestation_digest;
        receipt.payment_scope_digest = payment_scope_digest;
        receipt.bump = ctx.bumps.receipt;

        emit!(DocumentSettlementExecuted {
            receipt: receipt.key(),
            payer: receipt.payer,
            merchant: receipt.merchant,
            mint: receipt.mint,
            nonce,
            gross_atomic,
            tax_atomic,
            net_atomic,
            policy_digest,
            document_attestation_digest,
            payment_scope_digest,
        });
        Ok(())
    }

    pub fn record_adjustment(
        ctx: Context<RecordAdjustment>,
        input: AdjustmentRecordInput,
    ) -> Result<()> {
        validate_protocol_not_paused(&ctx.accounts.protocol_config)?;
        require!(
            input.adjustment_nonce > 0,
            SettlementError::InvalidAdjustmentNonce
        );
        require_keys_neq!(
            ctx.accounts.proposer.key(),
            ctx.accounts.approver.key(),
            SettlementError::InvalidAdjustmentAuthority
        );
        validate_supported_mint_program(
            &ctx.accounts.token_program,
            &ctx.accounts.mint.to_account_info(),
        )?;
        require_keys_eq!(
            ctx.accounts.registry.merchant,
            ctx.accounts.merchant.key(),
            SettlementError::AdjustmentRegistryBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.registry.mint,
            ctx.accounts.mint.key(),
            SettlementError::AdjustmentRegistryBindingMismatch
        );
        require!(
            ctx.accounts.registry.active,
            SettlementError::AdjustmentRegistryInactive
        );
        require_keys_eq!(
            ctx.accounts.registry.proposer,
            ctx.accounts.proposer.key(),
            SettlementError::AdjustmentProposerMismatch
        );
        require_keys_eq!(
            ctx.accounts.registry.approver,
            ctx.accounts.approver.key(),
            SettlementError::AdjustmentApproverMismatch
        );
        require_keys_eq!(
            ctx.accounts.original_receipt.merchant,
            ctx.accounts.merchant.key(),
            SettlementError::AdjustmentReceiptBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.original_receipt.mint,
            ctx.accounts.mint.key(),
            SettlementError::AdjustmentReceiptBindingMismatch
        );
        validate_adjustment_delta(
            input.kind,
            input.gross_sign,
            input.gross_magnitude,
            input.tax_sign,
            input.tax_magnitude,
            input.net_sign,
            input.net_magnitude,
        )?;
        require!(
            input.adjustment_digest != [0; 32],
            SettlementError::EmptyAdjustmentDigest
        );
        require!(
            input.evidence_digest != [0; 32],
            SettlementError::EmptyAdjustmentEvidenceDigest
        );
        require!(
            input.source_calculation_digest != [0; 32],
            SettlementError::EmptyAdjustmentCalculationDigest
        );

        let adjustment = &mut ctx.accounts.adjustment;
        adjustment.original_receipt = ctx.accounts.original_receipt.key();
        adjustment.merchant = ctx.accounts.merchant.key();
        adjustment.mint = ctx.accounts.original_receipt.mint;
        adjustment.proposer = ctx.accounts.proposer.key();
        adjustment.approver = ctx.accounts.approver.key();
        adjustment.nonce = input.adjustment_nonce;
        adjustment.kind = input.kind;
        adjustment.gross_sign = input.gross_sign;
        adjustment.gross_magnitude = input.gross_magnitude;
        adjustment.tax_sign = input.tax_sign;
        adjustment.tax_magnitude = input.tax_magnitude;
        adjustment.net_sign = input.net_sign;
        adjustment.net_magnitude = input.net_magnitude;
        adjustment.adjustment_digest = input.adjustment_digest;
        adjustment.evidence_digest = input.evidence_digest;
        adjustment.source_calculation_digest = input.source_calculation_digest;
        adjustment.replacement_calculation_digest = input.replacement_calculation_digest;
        adjustment.bump = ctx.bumps.adjustment;

        emit!(SettlementAdjustmentRecorded {
            adjustment: adjustment.key(),
            original_receipt: adjustment.original_receipt,
            merchant: adjustment.merchant,
            mint: adjustment.mint,
            nonce: input.adjustment_nonce,
            kind: input.kind,
            gross_sign: input.gross_sign,
            gross_magnitude: input.gross_magnitude,
            tax_sign: input.tax_sign,
            tax_magnitude: input.tax_magnitude,
            net_sign: input.net_sign,
            net_magnitude: input.net_magnitude,
            adjustment_digest: input.adjustment_digest,
            evidence_digest: input.evidence_digest,
            approver: adjustment.approver,
        });
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_adjustment_refund_claim(
        ctx: Context<CreateAdjustmentRefundClaim>,
        claim_nonce: u64,
        reference_digest: [u8; 32],
        execute_after_unix: i64,
        expires_at_unix: i64,
    ) -> Result<()> {
        validate_protocol_not_paused(&ctx.accounts.protocol_config)?;
        require!(
            claim_nonce > 0,
            SettlementError::InvalidAdjustmentRefundClaimNonce
        );
        validate_adjustment_registry_for_proposer(
            &ctx.accounts.registry,
            ctx.accounts.merchant.key(),
            ctx.accounts.mint.key(),
            ctx.accounts.proposer.key(),
        )?;
        require_keys_eq!(
            ctx.accounts.adjustment.approver,
            ctx.accounts.registry.approver,
            SettlementError::AdjustmentApprovalBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.adjustment.merchant,
            ctx.accounts.merchant.key(),
            SettlementError::AdjustmentReceiptBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.adjustment.mint,
            ctx.accounts.mint.key(),
            SettlementError::AdjustmentReceiptBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.adjustment.original_receipt,
            ctx.accounts.original_receipt.key(),
            SettlementError::AdjustmentReceiptBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.original_receipt.merchant,
            ctx.accounts.merchant.key(),
            SettlementError::AdjustmentReceiptBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.original_receipt.mint,
            ctx.accounts.mint.key(),
            SettlementError::AdjustmentReceiptBindingMismatch
        );
        validate_refund_movement_amounts(&ctx.accounts.adjustment)?;
        require!(
            ctx.accounts.proposer.key() != ctx.accounts.refund_funder.key(),
            SettlementError::InvalidAdjustmentAuthority
        );
        validate_supported_mint_program(
            &ctx.accounts.token_program,
            &ctx.accounts.mint.to_account_info(),
        )?;
        validate_refund_token_accounts(
            &ctx.accounts.token_program,
            &ctx.accounts.mint,
            &ctx.accounts.tax_vault,
            &ctx.accounts.net_source,
            &ctx.accounts.refund_destination,
            ctx.accounts.vault_authority.key(),
            ctx.accounts.refund_funder.key(),
            ctx.accounts.original_receipt.payer,
        )?;
        validate_release_claim_inputs(
            1,
            reference_digest,
            execute_after_unix,
            expires_at_unix,
            Clock::get()?.unix_timestamp,
        )?;

        let claim = &mut ctx.accounts.claim;
        claim.adjustment = ctx.accounts.adjustment.key();
        claim.original_receipt = ctx.accounts.original_receipt.key();
        claim.merchant = ctx.accounts.merchant.key();
        claim.mint = ctx.accounts.mint.key();
        claim.payer = ctx.accounts.original_receipt.payer;
        claim.refund_funder = ctx.accounts.refund_funder.key();
        claim.tax_vault = ctx.accounts.tax_vault.key();
        claim.net_source = ctx.accounts.net_source.key();
        claim.refund_destination = ctx.accounts.refund_destination.key();
        claim.nonce = claim_nonce;
        claim.gross_magnitude = ctx.accounts.adjustment.gross_magnitude;
        claim.tax_magnitude = ctx.accounts.adjustment.tax_magnitude;
        claim.net_magnitude = ctx.accounts.adjustment.net_magnitude;
        claim.reference_digest = reference_digest;
        claim.execute_after_unix = execute_after_unix;
        claim.expires_at_unix = expires_at_unix;
        claim.proposer = ctx.accounts.proposer.key();
        claim.executed = false;
        claim.cancelled = false;
        claim.bump = ctx.bumps.claim;

        emit!(AdjustmentRefundClaimCreated {
            claim: claim.key(),
            adjustment: claim.adjustment,
            original_receipt: claim.original_receipt,
            merchant: claim.merchant,
            mint: claim.mint,
            nonce: claim_nonce,
            gross_magnitude: claim.gross_magnitude,
            tax_magnitude: claim.tax_magnitude,
            net_magnitude: claim.net_magnitude,
            refund_funder: claim.refund_funder,
            refund_destination: claim.refund_destination,
            proposer: claim.proposer,
            reference_digest,
            execute_after_unix,
            expires_at_unix,
        });
        Ok(())
    }

    pub fn execute_adjustment_refund(
        ctx: Context<ExecuteAdjustmentRefund>,
        claim_nonce: u64,
    ) -> Result<()> {
        validate_protocol_not_paused(&ctx.accounts.protocol_config)?;
        require!(
            claim_nonce > 0,
            SettlementError::InvalidAdjustmentRefundClaimNonce
        );
        validate_adjustment_registry_for_approver(
            &ctx.accounts.registry,
            ctx.accounts.merchant.key(),
            ctx.accounts.mint.key(),
            ctx.accounts.approver.key(),
        )?;
        require_keys_neq!(
            ctx.accounts.approver.key(),
            ctx.accounts.refund_funder.key(),
            SettlementError::InvalidAdjustmentAuthority
        );
        require!(
            ctx.accounts.claim.nonce == claim_nonce,
            SettlementError::AdjustmentRefundClaimBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.claim.adjustment,
            ctx.accounts.adjustment.key(),
            SettlementError::AdjustmentRefundClaimBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.claim.original_receipt,
            ctx.accounts.original_receipt.key(),
            SettlementError::AdjustmentRefundClaimBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.claim.merchant,
            ctx.accounts.merchant.key(),
            SettlementError::AdjustmentRefundClaimBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.claim.mint,
            ctx.accounts.mint.key(),
            SettlementError::AdjustmentRefundClaimBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.claim.payer,
            ctx.accounts.original_receipt.payer,
            SettlementError::AdjustmentRefundClaimBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.claim.refund_funder,
            ctx.accounts.refund_funder.key(),
            SettlementError::AdjustmentRefundClaimBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.claim.tax_vault,
            ctx.accounts.tax_vault.key(),
            SettlementError::AdjustmentRefundClaimBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.claim.net_source,
            ctx.accounts.net_source.key(),
            SettlementError::AdjustmentRefundClaimBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.claim.refund_destination,
            ctx.accounts.refund_destination.key(),
            SettlementError::AdjustmentRefundClaimBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.claim.proposer,
            ctx.accounts.registry.proposer,
            SettlementError::AdjustmentRefundClaimBindingMismatch
        );
        require!(
            !ctx.accounts.claim.executed && !ctx.accounts.claim.cancelled,
            SettlementError::AdjustmentRefundClaimAlreadyFinalized
        );
        let now_unix = Clock::get()?.unix_timestamp;
        require!(
            ctx.accounts.claim.execute_after_unix <= now_unix
                && now_unix <= ctx.accounts.claim.expires_at_unix,
            SettlementError::AdjustmentRefundClaimOutsideWindow
        );
        require_keys_eq!(
            ctx.accounts.adjustment.approver,
            ctx.accounts.registry.approver,
            SettlementError::AdjustmentApprovalBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.adjustment.merchant,
            ctx.accounts.merchant.key(),
            SettlementError::AdjustmentReceiptBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.adjustment.mint,
            ctx.accounts.mint.key(),
            SettlementError::AdjustmentReceiptBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.adjustment.original_receipt,
            ctx.accounts.original_receipt.key(),
            SettlementError::AdjustmentReceiptBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.original_receipt.merchant,
            ctx.accounts.merchant.key(),
            SettlementError::AdjustmentReceiptBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.original_receipt.mint,
            ctx.accounts.mint.key(),
            SettlementError::AdjustmentReceiptBindingMismatch
        );
        validate_refund_movement_amounts(&ctx.accounts.adjustment)?;
        require!(
            ctx.accounts.claim.gross_magnitude == ctx.accounts.adjustment.gross_magnitude
                && ctx.accounts.claim.tax_magnitude == ctx.accounts.adjustment.tax_magnitude
                && ctx.accounts.claim.net_magnitude == ctx.accounts.adjustment.net_magnitude,
            SettlementError::AdjustmentRefundClaimBindingMismatch
        );
        validate_supported_mint_program(
            &ctx.accounts.token_program,
            &ctx.accounts.mint.to_account_info(),
        )?;
        validate_refund_token_accounts(
            &ctx.accounts.token_program,
            &ctx.accounts.mint,
            &ctx.accounts.tax_vault,
            &ctx.accounts.net_source,
            &ctx.accounts.refund_destination,
            ctx.accounts.vault_authority.key(),
            ctx.accounts.refund_funder.key(),
            ctx.accounts.original_receipt.payer,
        )?;
        require!(
            ctx.accounts.tax_vault.amount >= ctx.accounts.claim.tax_magnitude,
            SettlementError::AdjustmentTaxVaultInsufficientFunds
        );
        require!(
            ctx.accounts.net_source.amount >= ctx.accounts.claim.net_magnitude,
            SettlementError::AdjustmentRefundSourceInsufficientFunds
        );

        if ctx.accounts.claim.tax_magnitude > 0 {
            let merchant_key = ctx.accounts.merchant.key();
            let mint_key = ctx.accounts.mint.key();
            let signer_seeds: &[&[u8]] = &[
                b"vault-authority",
                merchant_key.as_ref(),
                mint_key.as_ref(),
                &[ctx.bumps.vault_authority],
            ];
            let transfer_accounts = TransferChecked {
                from: ctx.accounts.tax_vault.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.refund_destination.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            };
            token_interface::transfer_checked(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.key(),
                    transfer_accounts,
                    &[signer_seeds],
                ),
                ctx.accounts.claim.tax_magnitude,
                ctx.accounts.mint.decimals,
            )?;
        }

        if ctx.accounts.claim.net_magnitude > 0 {
            let transfer_accounts = TransferChecked {
                from: ctx.accounts.net_source.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.refund_destination.to_account_info(),
                authority: ctx.accounts.refund_funder.to_account_info(),
            };
            token_interface::transfer_checked(
                CpiContext::new(ctx.accounts.token_program.key(), transfer_accounts),
                ctx.accounts.claim.net_magnitude,
                ctx.accounts.mint.decimals,
            )?;
        }

        ctx.accounts.claim.executed = true;
        emit!(AdjustmentRefundExecuted {
            claim: ctx.accounts.claim.key(),
            adjustment: ctx.accounts.claim.adjustment,
            original_receipt: ctx.accounts.claim.original_receipt,
            merchant: ctx.accounts.claim.merchant,
            mint: ctx.accounts.claim.mint,
            nonce: claim_nonce,
            gross_magnitude: ctx.accounts.claim.gross_magnitude,
            tax_magnitude: ctx.accounts.claim.tax_magnitude,
            net_magnitude: ctx.accounts.claim.net_magnitude,
            refund_funder: ctx.accounts.refund_funder.key(),
            refund_destination: ctx.accounts.refund_destination.key(),
            approver: ctx.accounts.approver.key(),
            reference_digest: ctx.accounts.claim.reference_digest,
        });
        Ok(())
    }

    pub fn cancel_adjustment_refund_claim(
        ctx: Context<CancelAdjustmentRefundClaim>,
        claim_nonce: u64,
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.registry.governance_authority,
            ctx.accounts.governance_authority.key(),
            SettlementError::AdjustmentRegistryAuthorityMismatch
        );
        validate_adjustment_registry_scope(
            &ctx.accounts.registry,
            ctx.accounts.merchant.key(),
            ctx.accounts.mint.key(),
        )?;
        require!(
            ctx.accounts.claim.nonce == claim_nonce,
            SettlementError::AdjustmentRefundClaimBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.claim.merchant,
            ctx.accounts.merchant.key(),
            SettlementError::AdjustmentRefundClaimBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.claim.mint,
            ctx.accounts.mint.key(),
            SettlementError::AdjustmentRefundClaimBindingMismatch
        );
        require!(
            !ctx.accounts.claim.executed && !ctx.accounts.claim.cancelled,
            SettlementError::AdjustmentRefundClaimAlreadyFinalized
        );
        ctx.accounts.claim.cancelled = true;
        emit!(AdjustmentRefundClaimCancelled {
            claim: ctx.accounts.claim.key(),
            adjustment: ctx.accounts.claim.adjustment,
            merchant: ctx.accounts.claim.merchant,
            mint: ctx.accounts.claim.mint,
            nonce: claim_nonce,
            governance_authority: ctx.accounts.governance_authority.key(),
            reference_digest: ctx.accounts.claim.reference_digest,
        });
        Ok(())
    }

    // The argument order is the versioned wire contract; collapsing amounts
    // into a struct would change Anchor serialization and the signed-message
    // compatibility boundary.
    #[allow(clippy::too_many_arguments)]
    pub fn settle_split<'info>(
        ctx: Context<'info, SettleSplit<'info>>,
        nonce: u64,
        policy_sequence: u64,
        split_plan_version: u64,
        gross_atomic: u64,
        tax_atomic: u64,
        net_atomic: u64,
        policy_digest: [u8; 32],
    ) -> Result<()> {
        validate_protocol_not_paused(&ctx.accounts.protocol_config)?;
        validate_policy_attestation(
            &ctx.accounts.policy_attestation,
            PolicySettlementBinding {
                merchant: ctx.accounts.merchant.key(),
                mint: ctx.accounts.mint.key(),
                sequence: policy_sequence,
                gross_atomic,
                tax_atomic,
                net_atomic,
                policy_digest,
            },
            Clock::get()?.unix_timestamp,
        )?;
        require_keys_eq!(
            ctx.accounts.split_plan.merchant,
            ctx.accounts.merchant.key(),
            SettlementError::SplitPlanBindingMismatch
        );
        require_keys_eq!(
            ctx.accounts.split_plan.mint,
            ctx.accounts.mint.key(),
            SettlementError::SplitPlanBindingMismatch
        );
        require!(
            ctx.accounts.split_plan.version == split_plan_version,
            SettlementError::SplitPlanBindingMismatch
        );
        require!(
            ctx.accounts.split_plan.digest
                == calculate_split_plan_digest(
                    ctx.accounts.split_plan.merchant,
                    ctx.accounts.split_plan.mint,
                    ctx.accounts.split_plan.version,
                    &ctx.accounts.split_plan.recipients,
                ),
            SettlementError::SplitPlanDigestMismatch
        );
        validate_settlement_amounts(gross_atomic, tax_atomic, net_atomic)?;
        validate_supported_split_base_accounts(
            &ctx.accounts.token_program,
            &ctx.accounts.mint,
            &ctx.accounts.source,
            &ctx.accounts.tax_vault,
        )?;
        require_keys_eq!(
            ctx.accounts.source.owner,
            ctx.accounts.payer.key(),
            SettlementError::SourceAuthorityMismatch
        );
        require_keys_eq!(
            ctx.accounts.source.mint,
            ctx.accounts.mint.key(),
            SettlementError::SourceMintMismatch
        );
        require_keys_eq!(
            ctx.accounts.tax_vault.mint,
            ctx.accounts.mint.key(),
            SettlementError::TaxVaultMintMismatch
        );
        require_keys_eq!(
            ctx.accounts.tax_vault.owner,
            ctx.accounts.vault_authority.key(),
            SettlementError::TaxVaultAuthorityMismatch
        );
        require!(
            ctx.accounts.source.amount >= gross_atomic,
            SettlementError::InsufficientSourceFunds
        );
        require_keys_neq!(
            ctx.accounts.source.key(),
            ctx.accounts.tax_vault.key(),
            SettlementError::AliasedTokenAccounts
        );

        let allocations = allocate_split_amounts(&ctx.accounts.split_plan, net_atomic)?;
        validate_split_recipient_accounts(
            &ctx.accounts.split_plan,
            &ctx.accounts.token_program,
            &ctx.accounts.mint,
            &ctx.accounts.source,
            &ctx.accounts.tax_vault,
            ctx.remaining_accounts,
        )?;

        if tax_atomic > 0 {
            let transfer_accounts = TransferChecked {
                from: ctx.accounts.source.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.tax_vault.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            };
            token_interface::transfer_checked(
                CpiContext::new(ctx.accounts.token_program.key(), transfer_accounts),
                tax_atomic,
                ctx.accounts.mint.decimals,
            )?;
        }

        for (index, amount_atomic) in allocations
            .iter()
            .take(ctx.accounts.split_plan.recipients.len())
            .enumerate()
        {
            if *amount_atomic == 0 {
                continue;
            }
            let transfer_accounts = TransferChecked {
                from: ctx.accounts.source.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.remaining_accounts[index].clone(),
                authority: ctx.accounts.payer.to_account_info(),
            };
            token_interface::transfer_checked(
                CpiContext::new(ctx.accounts.token_program.key(), transfer_accounts),
                *amount_atomic,
                ctx.accounts.mint.decimals,
            )?;
        }

        let receipt = &mut ctx.accounts.receipt;
        receipt.payer = ctx.accounts.payer.key();
        receipt.merchant = ctx.accounts.merchant.key();
        receipt.mint = ctx.accounts.mint.key();
        receipt.nonce = nonce;
        receipt.gross_atomic = gross_atomic;
        receipt.tax_atomic = tax_atomic;
        receipt.net_atomic = net_atomic;
        receipt.policy_digest = policy_digest;
        receipt.bump = ctx.bumps.receipt;

        emit!(SplitSettlementExecuted {
            receipt: receipt.key(),
            payer: receipt.payer,
            merchant: receipt.merchant,
            mint: receipt.mint,
            nonce,
            gross_atomic,
            tax_atomic,
            net_atomic,
            split_plan_version,
            split_plan_digest: ctx.accounts.split_plan.digest,
            policy_digest,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeProtocolConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + ProtocolConfig::INIT_SPACE,
        seeds = [b"protocol-config"],
        bump,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(version: u64)]
pub struct InitializeSplitPlan<'info> {
    #[account(mut)]
    pub merchant: Signer<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    #[account(
        init,
        payer = merchant,
        space = 8 + SplitPlan::INIT_SPACE,
        seeds = [b"split-plan", merchant.key().as_ref(), mint.key().as_ref(), &version.to_le_bytes()],
        bump,
    )]
    pub split_plan: Account<'info, SplitPlan>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(proposer: Pubkey, approver: Pubkey)]
pub struct InitializeVaultReleaseRegistry<'info> {
    #[account(mut)]
    pub governance_authority: Signer<'info>,
    pub merchant: SystemAccount<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    #[account(
        init,
        payer = governance_authority,
        space = 8 + VaultReleaseRegistry::INIT_SPACE,
        seeds = [b"vault-release-registry", merchant.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub registry: Account<'info, VaultReleaseRegistry>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateVaultReleaseRegistry<'info> {
    pub governance_authority: Signer<'info>,
    pub merchant: SystemAccount<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        seeds = [b"vault-release-registry", merchant.key().as_ref(), mint.key().as_ref()],
        bump = registry.bump,
    )]
    pub registry: Account<'info, VaultReleaseRegistry>,
}

#[derive(Accounts)]
#[instruction(nonce: u64)]
pub struct CreateVaultReleaseClaim<'info> {
    #[account(mut)]
    pub proposer: Signer<'info>,
    pub merchant: SystemAccount<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    #[account(
        seeds = [b"vault-release-registry", merchant.key().as_ref(), mint.key().as_ref()],
        bump = registry.bump,
    )]
    pub registry: Account<'info, VaultReleaseRegistry>,
    pub tax_vault: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: constrained by the deterministic vault-authority PDA and used as the token signer.
    #[account(
        seeds = [b"vault-authority", merchant.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,
    pub destination: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init,
        payer = proposer,
        space = 8 + ReleaseClaim::INIT_SPACE,
        seeds = [b"release-claim", merchant.key().as_ref(), mint.key().as_ref(), &nonce.to_le_bytes()],
        bump,
    )]
    pub claim: Account<'info, ReleaseClaim>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(nonce: u64)]
pub struct ExecuteVaultRelease<'info> {
    pub approver: Signer<'info>,
    pub merchant: SystemAccount<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    #[account(
        seeds = [b"vault-release-registry", merchant.key().as_ref(), mint.key().as_ref()],
        bump = registry.bump,
    )]
    pub registry: Account<'info, VaultReleaseRegistry>,
    #[account(mut)]
    pub tax_vault: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: constrained by the deterministic vault-authority PDA and used as the token signer.
    #[account(
        seeds = [b"vault-authority", merchant.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub destination: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"release-claim", merchant.key().as_ref(), mint.key().as_ref(), &nonce.to_le_bytes()],
        bump = claim.bump,
    )]
    pub claim: Account<'info, ReleaseClaim>,
}

#[derive(Accounts)]
#[instruction(nonce: u64)]
pub struct CancelVaultReleaseClaim<'info> {
    pub governance_authority: Signer<'info>,
    pub merchant: SystemAccount<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        seeds = [b"vault-release-registry", merchant.key().as_ref(), mint.key().as_ref()],
        bump = registry.bump,
    )]
    pub registry: Account<'info, VaultReleaseRegistry>,
    #[account(
        mut,
        seeds = [b"release-claim", merchant.key().as_ref(), mint.key().as_ref(), &nonce.to_le_bytes()],
        bump = claim.bump,
    )]
    pub claim: Account<'info, ReleaseClaim>,
}

#[derive(Accounts)]
pub struct UpdateProtocolConfig<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"protocol-config"],
        bump = protocol_config.bump,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
}

#[derive(Accounts)]
#[instruction(issuer: Pubkey)]
pub struct InitializePolicyIssuerRegistry<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// Merchant identity is bound into the registry PDA and policy attestations.
    pub merchant: SystemAccount<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    #[account(
        init,
        payer = authority,
        space = 8 + PolicyIssuerRegistry::INIT_SPACE,
        seeds = [b"policy-issuer-registry", merchant.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub issuer_registry: Account<'info, PolicyIssuerRegistry>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdatePolicyIssuerRegistry<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [
            b"policy-issuer-registry",
            issuer_registry.merchant.as_ref(),
            issuer_registry.mint.as_ref()
        ],
        bump = issuer_registry.bump,
    )]
    pub issuer_registry: Account<'info, PolicyIssuerRegistry>,
}

#[derive(Accounts)]
#[instruction(proposer: Pubkey, approver: Pubkey)]
pub struct InitializeAdjustmentRegistry<'info> {
    #[account(mut)]
    pub governance_authority: Signer<'info>,
    pub merchant: SystemAccount<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    #[account(
        init,
        payer = governance_authority,
        space = 8 + SettlementAdjustmentRegistry::INIT_SPACE,
        seeds = [b"adjustment-registry", merchant.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub registry: Account<'info, SettlementAdjustmentRegistry>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateAdjustmentRegistry<'info> {
    pub governance_authority: Signer<'info>,
    pub merchant: SystemAccount<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        seeds = [b"adjustment-registry", merchant.key().as_ref(), mint.key().as_ref()],
        bump = registry.bump,
    )]
    pub registry: Account<'info, SettlementAdjustmentRegistry>,
}

#[derive(Accounts)]
#[instruction(claim_nonce: u64)]
pub struct CreateAdjustmentRefundClaim<'info> {
    #[account(mut)]
    pub proposer: Signer<'info>,
    pub refund_funder: Signer<'info>,
    pub merchant: SystemAccount<'info>,
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Interface<'info, TokenInterface>,
    #[account(
        seeds = [b"protocol-config"],
        bump = protocol_config.bump,
    )]
    pub protocol_config: Box<Account<'info, ProtocolConfig>>,
    #[account(
        seeds = [b"adjustment-registry", merchant.key().as_ref(), mint.key().as_ref()],
        bump = registry.bump,
    )]
    pub registry: Box<Account<'info, SettlementAdjustmentRegistry>>,
    #[account(
        seeds = [b"adjustment", original_receipt.key().as_ref(), &adjustment.nonce.to_le_bytes()],
        bump = adjustment.bump,
    )]
    pub adjustment: Box<Account<'info, SettlementAdjustment>>,
    #[account(
        seeds = [
            b"receipt",
            original_receipt.payer.as_ref(),
            merchant.key().as_ref(),
            &original_receipt.nonce.to_le_bytes(),
        ],
        bump = original_receipt.bump,
    )]
    pub original_receipt: Box<Account<'info, SettlementReceipt>>,
    #[account(mut)]
    pub tax_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    /// CHECK: constrained by the deterministic vault-authority PDA and only used as a token signer.
    #[account(
        seeds = [b"vault-authority", merchant.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,
    pub net_source: Box<InterfaceAccount<'info, TokenAccount>>,
    pub refund_destination: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        init,
        payer = proposer,
        space = 8 + AdjustmentRefundClaim::INIT_SPACE,
        seeds = [b"refund-claim", adjustment.key().as_ref(), &claim_nonce.to_le_bytes()],
        bump,
    )]
    pub claim: Box<Account<'info, AdjustmentRefundClaim>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(claim_nonce: u64)]
pub struct ExecuteAdjustmentRefund<'info> {
    pub approver: Signer<'info>,
    pub refund_funder: Signer<'info>,
    pub merchant: SystemAccount<'info>,
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Interface<'info, TokenInterface>,
    #[account(
        seeds = [b"protocol-config"],
        bump = protocol_config.bump,
    )]
    pub protocol_config: Box<Account<'info, ProtocolConfig>>,
    #[account(
        seeds = [b"adjustment-registry", merchant.key().as_ref(), mint.key().as_ref()],
        bump = registry.bump,
    )]
    pub registry: Box<Account<'info, SettlementAdjustmentRegistry>>,
    #[account(
        seeds = [b"adjustment", original_receipt.key().as_ref(), &adjustment.nonce.to_le_bytes()],
        bump = adjustment.bump,
    )]
    pub adjustment: Box<Account<'info, SettlementAdjustment>>,
    #[account(
        seeds = [
            b"receipt",
            original_receipt.payer.as_ref(),
            merchant.key().as_ref(),
            &original_receipt.nonce.to_le_bytes(),
        ],
        bump = original_receipt.bump,
    )]
    pub original_receipt: Box<Account<'info, SettlementReceipt>>,
    #[account(mut)]
    pub tax_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    /// CHECK: constrained by the deterministic vault-authority PDA and only used as a token signer.
    #[account(
        seeds = [b"vault-authority", merchant.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub net_source: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut)]
    pub refund_destination: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [b"refund-claim", adjustment.key().as_ref(), &claim_nonce.to_le_bytes()],
        bump = claim.bump,
    )]
    pub claim: Box<Account<'info, AdjustmentRefundClaim>>,
}

#[derive(Accounts)]
#[instruction(claim_nonce: u64)]
pub struct CancelAdjustmentRefundClaim<'info> {
    pub governance_authority: Signer<'info>,
    pub merchant: SystemAccount<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        seeds = [b"adjustment-registry", merchant.key().as_ref(), mint.key().as_ref()],
        bump = registry.bump,
    )]
    pub registry: Account<'info, SettlementAdjustmentRegistry>,
    #[account(
        mut,
        seeds = [b"refund-claim", claim.adjustment.as_ref(), &claim_nonce.to_le_bytes()],
        bump = claim.bump,
    )]
    pub claim: Account<'info, AdjustmentRefundClaim>,
}

#[derive(Accounts)]
#[instruction(input: AdjustmentRecordInput)]
pub struct RecordAdjustment<'info> {
    #[account(mut)]
    pub approver: Signer<'info>,
    pub proposer: SystemAccount<'info>,
    pub merchant: SystemAccount<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    #[account(
        seeds = [b"protocol-config"],
        bump = protocol_config.bump,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(
        seeds = [b"adjustment-registry", merchant.key().as_ref(), mint.key().as_ref()],
        bump = registry.bump,
    )]
    pub registry: Account<'info, SettlementAdjustmentRegistry>,
    #[account(
        seeds = [
            b"receipt",
            original_receipt.payer.as_ref(),
            merchant.key().as_ref(),
            &original_receipt.nonce.to_le_bytes(),
        ],
        bump = original_receipt.bump,
    )]
    pub original_receipt: Account<'info, SettlementReceipt>,
    #[account(
        init,
        payer = approver,
        space = 8 + SettlementAdjustment::INIT_SPACE,
        seeds = [b"adjustment", original_receipt.key().as_ref(), &input.adjustment_nonce.to_le_bytes()],
        bump,
    )]
    pub adjustment: Account<'info, SettlementAdjustment>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(nonce: u64, policy_sequence: u64)]
pub struct Settle<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        seeds = [b"protocol-config"],
        bump = protocol_config.bump,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    /// Merchant identity is used in the receipt and vault-authority PDA domain.
    pub merchant: SystemAccount<'info>,
    #[account(mut)]
    pub source: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub tax_vault: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: address is constrained by the vault-authority PDA seeds and only used as token authority.
    #[account(
        seeds = [b"vault-authority", merchant.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"policy-attestation", merchant.key().as_ref(), mint.key().as_ref(), &policy_sequence.to_le_bytes()],
        bump = policy_attestation.bump,
    )]
    pub policy_attestation: Account<'info, PolicyAttestation>,
    #[account(mut)]
    pub net_destination: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init,
        payer = payer,
        space = 8 + SettlementReceipt::INIT_SPACE,
        seeds = [b"receipt", payer.key().as_ref(), merchant.key().as_ref(), &nonce.to_le_bytes()],
        bump
    )]
    pub receipt: Account<'info, SettlementReceipt>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(nonce: u64, policy_sequence: u64)]
pub struct SettleDocument<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        seeds = [b"protocol-config"],
        bump = protocol_config.bump,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    pub merchant: SystemAccount<'info>,
    #[account(mut)]
    pub source: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub tax_vault: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: constrained by the vault-authority PDA seeds and used only as token authority.
    #[account(
        seeds = [b"vault-authority", merchant.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"policy-attestation", merchant.key().as_ref(), mint.key().as_ref(), &policy_sequence.to_le_bytes()],
        bump = policy_attestation.bump,
    )]
    pub policy_attestation: Account<'info, PolicyAttestation>,
    #[account(mut)]
    pub net_destination: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init,
        payer = payer,
        space = 8 + DocumentSettlementReceipt::INIT_SPACE,
        seeds = [b"document-receipt", payer.key().as_ref(), merchant.key().as_ref(), &nonce.to_le_bytes()],
        bump
    )]
    pub receipt: Account<'info, DocumentSettlementReceipt>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(nonce: u64, policy_sequence: u64, split_plan_version: u64)]
pub struct SettleSplit<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        seeds = [b"protocol-config"],
        bump = protocol_config.bump,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    pub merchant: SystemAccount<'info>,
    #[account(mut)]
    pub source: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub tax_vault: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: constrained by the vault-authority PDA seeds and used only as token authority.
    #[account(
        seeds = [b"vault-authority", merchant.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"policy-attestation", merchant.key().as_ref(), mint.key().as_ref(), &policy_sequence.to_le_bytes()],
        bump = policy_attestation.bump,
    )]
    pub policy_attestation: Account<'info, PolicyAttestation>,
    #[account(
        seeds = [b"split-plan", merchant.key().as_ref(), mint.key().as_ref(), &split_plan_version.to_le_bytes()],
        bump = split_plan.bump,
    )]
    pub split_plan: Account<'info, SplitPlan>,
    #[account(
        init,
        payer = payer,
        space = 8 + SettlementReceipt::INIT_SPACE,
        seeds = [b"receipt", payer.key().as_ref(), merchant.key().as_ref(), &nonce.to_le_bytes()],
        bump
    )]
    pub receipt: Account<'info, SettlementReceipt>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(sequence: u64)]
pub struct RegisterPolicyAttestation<'info> {
    #[account(mut)]
    pub issuer: Signer<'info>,
    /// Merchant identity is bound into the policy PDA and settlement receipt.
    pub merchant: SystemAccount<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    #[account(
        seeds = [b"policy-issuer-registry", merchant.key().as_ref(), mint.key().as_ref()],
        bump = issuer_registry.bump,
    )]
    pub issuer_registry: Account<'info, PolicyIssuerRegistry>,
    #[account(
        init,
        payer = issuer,
        space = 8 + PolicyAttestation::INIT_SPACE,
        seeds = [b"policy-attestation", merchant.key().as_ref(), mint.key().as_ref(), &sequence.to_le_bytes()],
        bump,
    )]
    pub policy_attestation: Account<'info, PolicyAttestation>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RevokePolicyAttestation<'info> {
    pub authority: Signer<'info>,
    #[account(mut)]
    pub policy_attestation: Account<'info, PolicyAttestation>,
    #[account(
        seeds = [
            b"policy-issuer-registry",
            policy_attestation.merchant.as_ref(),
            policy_attestation.mint.as_ref()
        ],
        bump = issuer_registry.bump,
    )]
    pub issuer_registry: Account<'info, PolicyIssuerRegistry>,
}

#[account]
#[derive(InitSpace)]
pub struct ProtocolConfig {
    pub authority: Pubkey,
    pub paused: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct PolicyAttestation {
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub issuer: Pubkey,
    pub policy_digest: [u8; 32],
    pub sequence: u64,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub expires_at_unix: i64,
    pub revoked: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct SplitPlan {
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub version: u64,
    pub digest: [u8; 32],
    #[max_len(8)]
    pub recipients: Vec<SplitRecipient>,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct SplitRecipient {
    pub token_account: Pubkey,
    pub basis_points: u16,
}

/// Versioned record-only wire payload. Keeping this as an Anchor struct avoids
/// an oversized generated wrapper while preserving the field order used by
/// the settlement-client compiler and golden vector.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct AdjustmentRecordInput {
    pub adjustment_nonce: u64,
    pub kind: u8,
    pub gross_sign: u8,
    pub gross_magnitude: u64,
    pub tax_sign: u8,
    pub tax_magnitude: u64,
    pub net_sign: u8,
    pub net_magnitude: u64,
    pub adjustment_digest: [u8; 32],
    pub evidence_digest: [u8; 32],
    pub source_calculation_digest: [u8; 32],
    pub replacement_calculation_digest: [u8; 32],
}

#[account]
#[derive(InitSpace)]
pub struct PolicyIssuerRegistry {
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub authority: Pubkey,
    pub issuer: Pubkey,
    pub active: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct VaultReleaseRegistry {
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub governance_authority: Pubkey,
    pub proposer: Pubkey,
    pub approver: Pubkey,
    pub active: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct SettlementAdjustmentRegistry {
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub governance_authority: Pubkey,
    pub proposer: Pubkey,
    pub approver: Pubkey,
    pub active: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct ReleaseClaim {
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub nonce: u64,
    pub amount_atomic: u64,
    pub destination: Pubkey,
    pub reference_digest: [u8; 32],
    pub execute_after_unix: i64,
    pub expires_at_unix: i64,
    pub proposer: Pubkey,
    pub executed: bool,
    pub cancelled: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct SettlementReceipt {
    pub payer: Pubkey,
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub nonce: u64,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub policy_digest: [u8; 32],
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct DocumentSettlementReceipt {
    pub payer: Pubkey,
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub nonce: u64,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub policy_digest: [u8; 32],
    pub document_attestation_digest: [u8; 32],
    pub payment_scope_digest: [u8; 32],
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct SettlementAdjustment {
    pub original_receipt: Pubkey,
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub proposer: Pubkey,
    pub approver: Pubkey,
    pub nonce: u64,
    pub kind: u8,
    pub gross_sign: u8,
    pub gross_magnitude: u64,
    pub tax_sign: u8,
    pub tax_magnitude: u64,
    pub net_sign: u8,
    pub net_magnitude: u64,
    pub adjustment_digest: [u8; 32],
    pub evidence_digest: [u8; 32],
    pub source_calculation_digest: [u8; 32],
    pub replacement_calculation_digest: [u8; 32],
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct AdjustmentRefundClaim {
    pub adjustment: Pubkey,
    pub original_receipt: Pubkey,
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub payer: Pubkey,
    pub refund_funder: Pubkey,
    pub tax_vault: Pubkey,
    pub net_source: Pubkey,
    pub refund_destination: Pubkey,
    pub nonce: u64,
    pub gross_magnitude: u64,
    pub tax_magnitude: u64,
    pub net_magnitude: u64,
    pub reference_digest: [u8; 32],
    pub execute_after_unix: i64,
    pub expires_at_unix: i64,
    pub proposer: Pubkey,
    pub executed: bool,
    pub cancelled: bool,
    pub bump: u8,
}

#[event]
pub struct SettlementExecuted {
    pub receipt: Pubkey,
    pub payer: Pubkey,
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub nonce: u64,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub policy_digest: [u8; 32],
}

#[event]
pub struct SplitSettlementExecuted {
    pub receipt: Pubkey,
    pub payer: Pubkey,
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub nonce: u64,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub split_plan_version: u64,
    pub split_plan_digest: [u8; 32],
    pub policy_digest: [u8; 32],
}

#[event]
pub struct DocumentSettlementExecuted {
    pub receipt: Pubkey,
    pub payer: Pubkey,
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub nonce: u64,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub policy_digest: [u8; 32],
    pub document_attestation_digest: [u8; 32],
    pub payment_scope_digest: [u8; 32],
}

#[event]
pub struct SettlementAdjustmentRecorded {
    pub adjustment: Pubkey,
    pub original_receipt: Pubkey,
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub nonce: u64,
    pub kind: u8,
    pub gross_sign: u8,
    pub gross_magnitude: u64,
    pub tax_sign: u8,
    pub tax_magnitude: u64,
    pub net_sign: u8,
    pub net_magnitude: u64,
    pub adjustment_digest: [u8; 32],
    pub evidence_digest: [u8; 32],
    pub approver: Pubkey,
}

#[event]
pub struct AdjustmentRefundClaimCreated {
    pub claim: Pubkey,
    pub adjustment: Pubkey,
    pub original_receipt: Pubkey,
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub nonce: u64,
    pub gross_magnitude: u64,
    pub tax_magnitude: u64,
    pub net_magnitude: u64,
    pub refund_funder: Pubkey,
    pub refund_destination: Pubkey,
    pub proposer: Pubkey,
    pub reference_digest: [u8; 32],
    pub execute_after_unix: i64,
    pub expires_at_unix: i64,
}

#[event]
pub struct AdjustmentRefundExecuted {
    pub claim: Pubkey,
    pub adjustment: Pubkey,
    pub original_receipt: Pubkey,
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub nonce: u64,
    pub gross_magnitude: u64,
    pub tax_magnitude: u64,
    pub net_magnitude: u64,
    pub refund_funder: Pubkey,
    pub refund_destination: Pubkey,
    pub approver: Pubkey,
    pub reference_digest: [u8; 32],
}

#[event]
pub struct AdjustmentRefundClaimCancelled {
    pub claim: Pubkey,
    pub adjustment: Pubkey,
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub nonce: u64,
    pub governance_authority: Pubkey,
    pub reference_digest: [u8; 32],
}

#[event]
pub struct ProtocolPauseChanged {
    pub authority: Pubkey,
    pub paused: bool,
}

#[event]
pub struct VaultReleaseClaimCreated {
    pub claim: Pubkey,
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub nonce: u64,
    pub amount_atomic: u64,
    pub destination: Pubkey,
    pub reference_digest: [u8; 32],
    pub proposer: Pubkey,
    pub execute_after_unix: i64,
    pub expires_at_unix: i64,
}

#[event]
pub struct VaultReleaseExecuted {
    pub claim: Pubkey,
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub nonce: u64,
    pub amount_atomic: u64,
    pub destination: Pubkey,
    pub reference_digest: [u8; 32],
    pub approver: Pubkey,
}

#[event]
pub struct VaultReleaseClaimCancelled {
    pub claim: Pubkey,
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub nonce: u64,
    pub reference_digest: [u8; 32],
    pub governance_authority: Pubkey,
}

fn validate_settlement_amounts(gross_atomic: u64, tax_atomic: u64, net_atomic: u64) -> Result<()> {
    require!(gross_atomic > 0, SettlementError::ZeroGrossAmount);
    require!(
        tax_atomic.checked_add(net_atomic) == Some(gross_atomic),
        SettlementError::AmountConservationViolation
    );
    Ok(())
}

fn validate_adjustment_delta(
    kind: u8,
    gross_sign: u8,
    gross_magnitude: u64,
    tax_sign: u8,
    tax_magnitude: u64,
    net_sign: u8,
    net_magnitude: u64,
) -> Result<()> {
    require!(
        (1..=5).contains(&kind),
        SettlementError::InvalidAdjustmentKind
    );
    for (sign, magnitude) in [
        (gross_sign, gross_magnitude),
        (tax_sign, tax_magnitude),
        (net_sign, net_magnitude),
    ] {
        require!(sign <= 1, SettlementError::InvalidAdjustmentSign);
        require!(
            magnitude != 0 || sign == 0,
            SettlementError::InvalidAdjustmentSign
        );
    }
    require!(
        gross_magnitude != 0 || tax_magnitude != 0 || net_magnitude != 0,
        SettlementError::ZeroAdjustmentDelta
    );
    if matches!(kind, 1 | 2 | 4) {
        require!(
            gross_sign == 1
                && (tax_magnitude == 0 || tax_sign == 1)
                && (net_magnitude == 0 || net_sign == 1),
            SettlementError::InvalidAdjustmentSign
        );
    }

    let (result_sign, result_magnitude) = if tax_sign == net_sign {
        (
            tax_sign,
            tax_magnitude
                .checked_add(net_magnitude)
                .ok_or(error!(SettlementError::AdjustmentArithmeticOverflow))?,
        )
    } else if tax_magnitude >= net_magnitude {
        (tax_sign, tax_magnitude - net_magnitude)
    } else {
        (net_sign, net_magnitude - tax_magnitude)
    };
    require!(
        result_sign == gross_sign && result_magnitude == gross_magnitude,
        SettlementError::AdjustmentConservationViolation
    );
    Ok(())
}

fn validate_refund_movement_amounts(adjustment: &SettlementAdjustment) -> Result<()> {
    require!(
        adjustment.kind == 1,
        SettlementError::UnsupportedAdjustmentMovement
    );
    validate_adjustment_delta(
        adjustment.kind,
        adjustment.gross_sign,
        adjustment.gross_magnitude,
        adjustment.tax_sign,
        adjustment.tax_magnitude,
        adjustment.net_sign,
        adjustment.net_magnitude,
    )?;
    require!(
        adjustment.gross_sign == 1
            && (adjustment.tax_magnitude == 0 || adjustment.tax_sign == 1)
            && (adjustment.net_magnitude == 0 || adjustment.net_sign == 1),
        SettlementError::InvalidAdjustmentSign
    );
    Ok(())
}

fn validate_protocol_authority(config: &ProtocolConfig, authority: Pubkey) -> Result<()> {
    require_keys_eq!(
        config.authority,
        authority,
        SettlementError::ProtocolAuthorityMismatch
    );
    Ok(())
}

fn validate_protocol_not_paused(config: &ProtocolConfig) -> Result<()> {
    require!(!config.paused, SettlementError::ProtocolPaused);
    Ok(())
}

fn validate_supported_mint_layout(mint_len: usize) -> Result<()> {
    require!(mint_len == 82, SettlementError::UnsupportedTokenExtension);
    Ok(())
}

fn validate_supported_token_layout(
    mint_len: usize,
    source_len: usize,
    tax_vault_len: usize,
    destination_len: usize,
) -> Result<()> {
    validate_supported_mint_layout(mint_len)?;
    require!(
        source_len == 165 && tax_vault_len == 165 && destination_len == 165,
        SettlementError::UnsupportedTokenExtension
    );
    Ok(())
}

fn validate_supported_mint_program(
    token_program: &Interface<'_, TokenInterface>,
    mint: &AccountInfo<'_>,
) -> Result<()> {
    let token_program_key = token_program.key();
    require!(
        TokenInterface::ids().contains(&token_program_key),
        SettlementError::UnsupportedTokenProgram
    );
    require_keys_eq!(
        *mint.owner,
        token_program_key,
        SettlementError::TokenProgramMismatch
    );
    validate_supported_mint_layout(mint.data_len())
}

fn validate_supported_token_accounts(
    token_program: &Interface<'_, TokenInterface>,
    mint: &InterfaceAccount<'_, Mint>,
    source: &InterfaceAccount<'_, TokenAccount>,
    tax_vault: &InterfaceAccount<'_, TokenAccount>,
    destination: &InterfaceAccount<'_, TokenAccount>,
) -> Result<()> {
    let token_program_key = token_program.key();
    let mint_info = mint.to_account_info();
    let source_info = source.to_account_info();
    let tax_vault_info = tax_vault.to_account_info();
    let destination_info = destination.to_account_info();
    require_keys_eq!(
        *mint_info.owner,
        token_program_key,
        SettlementError::TokenProgramMismatch
    );
    require_keys_eq!(
        *source_info.owner,
        token_program_key,
        SettlementError::TokenProgramMismatch
    );
    require_keys_eq!(
        *tax_vault_info.owner,
        token_program_key,
        SettlementError::TokenProgramMismatch
    );
    require_keys_eq!(
        *destination_info.owner,
        token_program_key,
        SettlementError::TokenProgramMismatch
    );
    require_keys_eq!(
        tax_vault.mint,
        mint.key(),
        SettlementError::TaxVaultMintMismatch
    );
    require_keys_eq!(
        destination.mint,
        mint.key(),
        SettlementError::DestinationMintMismatch
    );
    validate_supported_token_layout(
        mint_info.data_len(),
        source_info.data_len(),
        tax_vault_info.data_len(),
        destination_info.data_len(),
    )
}

fn validate_supported_split_base_accounts(
    token_program: &Interface<'_, TokenInterface>,
    mint: &InterfaceAccount<'_, Mint>,
    source: &InterfaceAccount<'_, TokenAccount>,
    tax_vault: &InterfaceAccount<'_, TokenAccount>,
) -> Result<()> {
    let token_program_key = token_program.key();
    let mint_info = mint.to_account_info();
    let source_info = source.to_account_info();
    let tax_vault_info = tax_vault.to_account_info();
    require_keys_eq!(
        *mint_info.owner,
        token_program_key,
        SettlementError::TokenProgramMismatch
    );
    require_keys_eq!(
        *source_info.owner,
        token_program_key,
        SettlementError::TokenProgramMismatch
    );
    require_keys_eq!(
        *tax_vault_info.owner,
        token_program_key,
        SettlementError::TokenProgramMismatch
    );
    validate_supported_token_layout(
        mint_info.data_len(),
        source_info.data_len(),
        tax_vault_info.data_len(),
        tax_vault_info.data_len(),
    )
}

fn validate_split_plan_shape(recipients: &[SplitRecipient], version: u64) -> Result<()> {
    require!(version > 0, SettlementError::InvalidSplitPlanVersion);
    require!(
        !recipients.is_empty() && recipients.len() <= MAX_SPLIT_RECIPIENTS,
        SettlementError::InvalidSplitRecipientCount
    );
    let mut total = 0_u64;
    for (index, recipient) in recipients.iter().enumerate() {
        require!(
            recipient.token_account != Pubkey::default(),
            SettlementError::InvalidSplitRecipient
        );
        require!(
            recipient.basis_points > 0,
            SettlementError::InvalidSplitBasisPoints
        );
        total = total
            .checked_add(u64::from(recipient.basis_points))
            .ok_or(error!(SettlementError::SplitBasisPointsOverflow))?;
        require!(
            !recipients[..index]
                .iter()
                .any(|previous| previous.token_account == recipient.token_account),
            SettlementError::DuplicateSplitRecipient
        );
    }
    require!(
        total == BASIS_POINTS_DENOMINATOR,
        SettlementError::SplitBasisPointsDoNotSum
    );
    Ok(())
}

fn calculate_split_plan_digest(
    merchant: Pubkey,
    mint: Pubkey,
    version: u64,
    recipients: &[SplitRecipient],
) -> [u8; 32] {
    let mut payload = Vec::with_capacity(128);
    payload.extend_from_slice(SPLIT_PLAN_SCHEMA);
    payload.push(0);
    payload.extend_from_slice(merchant.as_ref());
    payload.extend_from_slice(mint.as_ref());
    payload.extend_from_slice(&version.to_le_bytes());
    payload.push(recipients.len() as u8);
    for recipient in recipients {
        payload.extend_from_slice(recipient.token_account.as_ref());
        payload.extend_from_slice(&recipient.basis_points.to_le_bytes());
    }
    Sha256::digest(payload).into()
}

fn allocate_split_amounts(
    plan: &SplitPlan,
    net_atomic: u64,
) -> Result<[u64; MAX_SPLIT_RECIPIENTS]> {
    validate_split_plan_shape(&plan.recipients, plan.version)?;
    let mut amounts = [0_u64; MAX_SPLIT_RECIPIENTS];
    let mut remainders = [0_u128; MAX_SPLIT_RECIPIENTS];
    let mut floor_sum = 0_u64;
    for (index, recipient) in plan.recipients.iter().enumerate() {
        let numerator = u128::from(net_atomic)
            .checked_mul(u128::from(recipient.basis_points))
            .ok_or(error!(SettlementError::SplitArithmeticOverflow))?;
        amounts[index] = u64::try_from(numerator / u128::from(BASIS_POINTS_DENOMINATOR))
            .map_err(|_| error!(SettlementError::SplitArithmeticOverflow))?;
        remainders[index] = numerator % u128::from(BASIS_POINTS_DENOMINATOR);
        floor_sum = floor_sum
            .checked_add(amounts[index])
            .ok_or(error!(SettlementError::SplitArithmeticOverflow))?;
    }
    let residual = net_atomic
        .checked_sub(floor_sum)
        .ok_or(error!(SettlementError::SplitConservationViolation))?;
    let residual_count = usize::try_from(residual)
        .map_err(|_| error!(SettlementError::SplitConservationViolation))?;
    require!(
        residual_count <= plan.recipients.len(),
        SettlementError::SplitConservationViolation
    );
    let mut selected = [false; MAX_SPLIT_RECIPIENTS];
    for _ in 0..residual_count {
        let mut best: Option<usize> = None;
        for index in 0..plan.recipients.len() {
            if selected[index]
                || best.is_some_and(|best_index| remainders[index] <= remainders[best_index])
            {
                continue;
            }
            best = Some(index);
        }
        let index = best.ok_or(error!(SettlementError::SplitConservationViolation))?;
        selected[index] = true;
        amounts[index] = amounts[index]
            .checked_add(1)
            .ok_or(error!(SettlementError::SplitArithmeticOverflow))?;
    }
    let total = amounts[..plan.recipients.len()]
        .iter()
        .try_fold(0_u64, |sum, amount| sum.checked_add(*amount))
        .ok_or(error!(SettlementError::SplitArithmeticOverflow))?;
    require!(
        total == net_atomic,
        SettlementError::SplitConservationViolation
    );
    Ok(amounts)
}

fn validate_split_recipient_accounts(
    plan: &SplitPlan,
    token_program: &Interface<'_, TokenInterface>,
    mint: &InterfaceAccount<'_, Mint>,
    source: &InterfaceAccount<'_, TokenAccount>,
    tax_vault: &InterfaceAccount<'_, TokenAccount>,
    remaining_accounts: &[AccountInfo<'_>],
) -> Result<()> {
    require!(
        remaining_accounts.len() == plan.recipients.len(),
        SettlementError::SplitRecipientAccountCountMismatch
    );
    for (index, expected) in plan.recipients.iter().enumerate() {
        let account_info = &remaining_accounts[index];
        require_keys_eq!(
            account_info.key(),
            expected.token_account,
            SettlementError::SplitRecipientAccountOrderMismatch
        );
        require_keys_eq!(
            *account_info.owner,
            token_program.key(),
            SettlementError::TokenProgramMismatch
        );
        require!(
            account_info.data_len() == 165,
            SettlementError::UnsupportedTokenExtension
        );
        let data = account_info.data.borrow();
        let mut data_slice: &[u8] = &data;
        let recipient = TokenAccount::try_deserialize(&mut data_slice)?;
        require_keys_eq!(
            recipient.mint,
            mint.key(),
            SettlementError::SplitRecipientMintMismatch
        );
        require_keys_neq!(
            account_info.key(),
            source.key(),
            SettlementError::AliasedTokenAccounts
        );
        require_keys_neq!(
            account_info.key(),
            tax_vault.key(),
            SettlementError::AliasedTokenAccounts
        );
    }
    Ok(())
}

fn validate_new_issuer(issuer: Pubkey) -> Result<()> {
    require!(issuer != Pubkey::default(), SettlementError::InvalidIssuer);
    Ok(())
}

fn validate_registry_scope(
    registry: &PolicyIssuerRegistry,
    merchant: Pubkey,
    mint: Pubkey,
) -> Result<()> {
    require_keys_eq!(
        registry.merchant,
        merchant,
        SettlementError::RegistryBindingMismatch
    );
    require_keys_eq!(
        registry.mint,
        mint,
        SettlementError::RegistryBindingMismatch
    );
    Ok(())
}

fn validate_registry_for_registration(
    registry: &PolicyIssuerRegistry,
    merchant: Pubkey,
    mint: Pubkey,
    issuer: Pubkey,
) -> Result<()> {
    validate_registry_scope(registry, merchant, mint)?;
    require!(registry.active, SettlementError::RegistryInactive);
    require_keys_eq!(
        registry.issuer,
        issuer,
        SettlementError::PolicyIssuerMismatch
    );
    Ok(())
}

fn validate_registry_update(
    registry: &PolicyIssuerRegistry,
    authority: Pubkey,
    new_issuer: Pubkey,
) -> Result<()> {
    require_keys_eq!(
        registry.authority,
        authority,
        SettlementError::RegistryAuthorityMismatch
    );
    validate_new_issuer(new_issuer)
}

fn validate_revocation_authority(
    registry: &PolicyIssuerRegistry,
    policy: &PolicyAttestation,
    authority: Pubkey,
) -> Result<()> {
    validate_registry_scope(registry, policy.merchant, policy.mint)?;
    require_keys_eq!(
        registry.authority,
        authority,
        SettlementError::RegistryAuthorityMismatch
    );
    Ok(())
}

fn validate_release_registry_authorities(
    governance_authority: Pubkey,
    proposer: Pubkey,
    approver: Pubkey,
) -> Result<()> {
    require!(
        governance_authority != Pubkey::default()
            && proposer != Pubkey::default()
            && approver != Pubkey::default(),
        SettlementError::InvalidReleaseAuthority
    );
    require!(
        governance_authority != proposer
            && governance_authority != approver
            && proposer != approver,
        SettlementError::InvalidReleaseAuthority
    );
    Ok(())
}

fn validate_release_registry_scope(
    registry: &VaultReleaseRegistry,
    merchant: Pubkey,
    mint: Pubkey,
) -> Result<()> {
    require_keys_eq!(
        registry.merchant,
        merchant,
        SettlementError::ReleaseRegistryBindingMismatch
    );
    require_keys_eq!(
        registry.mint,
        mint,
        SettlementError::ReleaseRegistryBindingMismatch
    );
    Ok(())
}

fn validate_adjustment_registry_scope(
    registry: &SettlementAdjustmentRegistry,
    merchant: Pubkey,
    mint: Pubkey,
) -> Result<()> {
    require_keys_eq!(
        registry.merchant,
        merchant,
        SettlementError::AdjustmentRegistryBindingMismatch
    );
    require_keys_eq!(
        registry.mint,
        mint,
        SettlementError::AdjustmentRegistryBindingMismatch
    );
    Ok(())
}

fn validate_adjustment_registry_for_proposer(
    registry: &SettlementAdjustmentRegistry,
    merchant: Pubkey,
    mint: Pubkey,
    proposer: Pubkey,
) -> Result<()> {
    validate_adjustment_registry_scope(registry, merchant, mint)?;
    require!(registry.active, SettlementError::AdjustmentRegistryInactive);
    require_keys_eq!(
        registry.proposer,
        proposer,
        SettlementError::AdjustmentProposerMismatch
    );
    Ok(())
}

fn validate_adjustment_registry_for_approver(
    registry: &SettlementAdjustmentRegistry,
    merchant: Pubkey,
    mint: Pubkey,
    approver: Pubkey,
) -> Result<()> {
    validate_adjustment_registry_scope(registry, merchant, mint)?;
    require!(registry.active, SettlementError::AdjustmentRegistryInactive);
    require_keys_eq!(
        registry.approver,
        approver,
        SettlementError::AdjustmentApproverMismatch
    );
    Ok(())
}

fn validate_refund_token_accounts<'info>(
    token_program: &Interface<'info, TokenInterface>,
    mint: &InterfaceAccount<'info, Mint>,
    tax_vault: &InterfaceAccount<'info, TokenAccount>,
    net_source: &InterfaceAccount<'info, TokenAccount>,
    refund_destination: &InterfaceAccount<'info, TokenAccount>,
    vault_authority: Pubkey,
    refund_funder: Pubkey,
    payer: Pubkey,
) -> Result<()> {
    let token_program_key = token_program.key();
    let mint_info = mint.to_account_info();
    let tax_vault_info = tax_vault.to_account_info();
    let net_source_info = net_source.to_account_info();
    let destination_info = refund_destination.to_account_info();
    for info in [
        &mint_info,
        &tax_vault_info,
        &net_source_info,
        &destination_info,
    ] {
        require_keys_eq!(
            *info.owner,
            token_program_key,
            SettlementError::TokenProgramMismatch
        );
    }
    require_keys_eq!(
        tax_vault.mint,
        mint.key(),
        SettlementError::TaxVaultMintMismatch
    );
    require_keys_eq!(
        net_source.mint,
        mint.key(),
        SettlementError::AdjustmentRefundSourceMintMismatch
    );
    require_keys_eq!(
        refund_destination.mint,
        mint.key(),
        SettlementError::DestinationMintMismatch
    );
    require_keys_eq!(
        tax_vault.owner,
        vault_authority,
        SettlementError::TaxVaultAuthorityMismatch
    );
    require_keys_eq!(
        net_source.owner,
        refund_funder,
        SettlementError::AdjustmentRefundSourceAuthorityMismatch
    );
    require_keys_eq!(
        refund_destination.owner,
        payer,
        SettlementError::AdjustmentRefundDestinationAuthorityMismatch
    );
    require_keys_neq!(
        tax_vault.key(),
        net_source.key(),
        SettlementError::AliasedTokenAccounts
    );
    require_keys_neq!(
        tax_vault.key(),
        refund_destination.key(),
        SettlementError::AliasedTokenAccounts
    );
    require_keys_neq!(
        net_source.key(),
        refund_destination.key(),
        SettlementError::AliasedTokenAccounts
    );
    validate_supported_token_layout(
        mint_info.data_len(),
        tax_vault_info.data_len(),
        net_source_info.data_len(),
        destination_info.data_len(),
    )
}

fn validate_release_claim_inputs(
    amount_atomic: u64,
    reference_digest: [u8; 32],
    execute_after_unix: i64,
    expires_at_unix: i64,
    now_unix: i64,
) -> Result<()> {
    require!(amount_atomic > 0, SettlementError::InvalidReleaseAmount);
    require!(
        reference_digest != [0; 32],
        SettlementError::InvalidReleaseReference
    );
    require!(
        execute_after_unix >= now_unix,
        SettlementError::InvalidReleaseWindow
    );
    let window = expires_at_unix
        .checked_sub(execute_after_unix)
        .ok_or(error!(SettlementError::InvalidReleaseWindow))?;
    require!(
        window > 0 && window <= MAX_RELEASE_WINDOW_SECONDS,
        SettlementError::InvalidReleaseWindow
    );
    Ok(())
}

fn validate_supported_release_accounts(
    token_program: &Interface<'_, TokenInterface>,
    mint: &InterfaceAccount<'_, Mint>,
    tax_vault: &InterfaceAccount<'_, TokenAccount>,
    destination: &InterfaceAccount<'_, TokenAccount>,
) -> Result<()> {
    let token_program_key = token_program.key();
    let mint_info = mint.to_account_info();
    let tax_vault_info = tax_vault.to_account_info();
    let destination_info = destination.to_account_info();
    require_keys_eq!(
        *mint_info.owner,
        token_program_key,
        SettlementError::TokenProgramMismatch
    );
    require_keys_eq!(
        *tax_vault_info.owner,
        token_program_key,
        SettlementError::TokenProgramMismatch
    );
    require_keys_eq!(
        *destination_info.owner,
        token_program_key,
        SettlementError::TokenProgramMismatch
    );
    require_keys_eq!(
        tax_vault.mint,
        mint.key(),
        SettlementError::TaxVaultMintMismatch
    );
    require_keys_eq!(
        destination.mint,
        mint.key(),
        SettlementError::DestinationMintMismatch
    );
    validate_supported_token_layout(
        mint_info.data_len(),
        tax_vault_info.data_len(),
        tax_vault_info.data_len(),
        destination_info.data_len(),
    )
}

#[derive(Clone, Copy)]
struct ReleaseExecutionContext {
    merchant: Pubkey,
    mint: Pubkey,
    vault_token_account: Pubkey,
    destination: Pubkey,
    now_unix: i64,
    vault_amount: u64,
}

fn validate_release_claim_for_execution(
    registry: &VaultReleaseRegistry,
    claim: &ReleaseClaim,
    approver: Pubkey,
    execution: &ReleaseExecutionContext,
) -> Result<()> {
    validate_release_registry_scope(registry, execution.merchant, execution.mint)?;
    require!(registry.active, SettlementError::ReleaseRegistryInactive);
    require_keys_eq!(
        registry.approver,
        approver,
        SettlementError::ReleaseApproverMismatch
    );
    require_keys_eq!(
        claim.merchant,
        execution.merchant,
        SettlementError::ReleaseClaimBindingMismatch
    );
    require_keys_eq!(
        claim.mint,
        execution.mint,
        SettlementError::ReleaseClaimBindingMismatch
    );
    require_keys_eq!(
        claim.proposer,
        registry.proposer,
        SettlementError::ReleaseClaimBindingMismatch
    );
    require_keys_neq!(
        claim.proposer,
        approver,
        SettlementError::InvalidReleaseAuthority
    );
    require_keys_eq!(
        claim.destination,
        execution.destination,
        SettlementError::ReleaseDestinationMismatch
    );
    require_keys_neq!(
        execution.vault_token_account,
        execution.destination,
        SettlementError::AliasedTokenAccounts
    );
    require!(
        !claim.executed && !claim.cancelled,
        SettlementError::ReleaseClaimAlreadyFinalized
    );
    require!(
        execution.now_unix >= claim.execute_after_unix
            && execution.now_unix <= claim.expires_at_unix,
        SettlementError::ReleaseClaimOutsideWindow
    );
    require!(
        execution.vault_amount >= claim.amount_atomic,
        SettlementError::ReleaseVaultInsufficientFunds
    );
    Ok(())
}

#[error_code]
pub enum SettlementError {
    #[msg("protocol governance authority does not match the signer")]
    ProtocolAuthorityMismatch,
    #[msg("settlement is paused by protocol governance")]
    ProtocolPaused,
    #[msg("gross amount must be positive")]
    ZeroGrossAmount,
    #[msg("tax plus net must equal gross")]
    AmountConservationViolation,
    #[msg("policy digest must not be all zeroes")]
    EmptyPolicyDigest,
    #[msg("document attestation digest must not be all zeroes")]
    EmptyDocumentAttestationDigest,
    #[msg("payment scope digest must not be all zeroes")]
    EmptyPaymentScopeDigest,
    #[msg("source token account is not owned by the payer")]
    SourceAuthorityMismatch,
    #[msg("source token account mint does not match the supplied mint")]
    SourceMintMismatch,
    #[msg("tax vault mint does not match the supplied mint")]
    TaxVaultMintMismatch,
    #[msg("tax vault is not controlled by the expected vault authority PDA")]
    TaxVaultAuthorityMismatch,
    #[msg("destination token account mint does not match the supplied mint")]
    DestinationMintMismatch,
    #[msg("source token account does not have enough funds")]
    InsufficientSourceFunds,
    #[msg("source, tax vault, and destination token accounts must be distinct")]
    AliasedTokenAccounts,
    #[msg("policy attestation issuer does not match the signer")]
    PolicyIssuerMismatch,
    #[msg("token program is not a supported SPL token program")]
    UnsupportedTokenProgram,
    #[msg("token accounts and mint must use the same token program")]
    TokenProgramMismatch,
    #[msg("the token mint or account uses an unsupported extension")]
    UnsupportedTokenExtension,
    #[msg("policy issuer registry authority does not match the signer")]
    RegistryAuthorityMismatch,
    #[msg("policy issuer registry does not match the merchant and mint scope")]
    RegistryBindingMismatch,
    #[msg("policy issuer registry is inactive")]
    RegistryInactive,
    #[msg("issuer public key must not be the default key")]
    InvalidIssuer,
    #[msg("policy attestation is revoked")]
    PolicyRevoked,
    #[msg("policy attestation has expired")]
    PolicyExpired,
    #[msg("policy attestation does not match the settlement")]
    PolicyBindingMismatch,
    #[msg("policy sequence must be positive")]
    InvalidPolicySequence,
    #[msg("policy expiry must be in the future")]
    InvalidPolicyExpiry,
    #[msg("split plan version must be positive")]
    InvalidSplitPlanVersion,
    #[msg("split plan recipient count is outside the bounded limit")]
    InvalidSplitRecipientCount,
    #[msg("split plan recipient is invalid")]
    InvalidSplitRecipient,
    #[msg("split plan recipient basis points must be positive")]
    InvalidSplitBasisPoints,
    #[msg("split plan basis points overflowed")]
    SplitBasisPointsOverflow,
    #[msg("split plan basis points must sum to 10000")]
    SplitBasisPointsDoNotSum,
    #[msg("split plan contains a duplicate recipient")]
    DuplicateSplitRecipient,
    #[msg("split plan does not match the settlement")]
    SplitPlanBindingMismatch,
    #[msg("split plan digest does not match its canonical content")]
    SplitPlanDigestMismatch,
    #[msg("split allocation arithmetic overflowed")]
    SplitArithmeticOverflow,
    #[msg("split allocation did not conserve the net amount")]
    SplitConservationViolation,
    #[msg("split recipient account count does not match the immutable plan")]
    SplitRecipientAccountCountMismatch,
    #[msg("split recipient account order does not match the immutable plan")]
    SplitRecipientAccountOrderMismatch,
    #[msg("split recipient token account mint does not match the settlement mint")]
    SplitRecipientMintMismatch,
    #[msg("vault release authorities must be non-default and pairwise distinct")]
    InvalidReleaseAuthority,
    #[msg("vault release registry governance authority does not match the signer")]
    ReleaseRegistryAuthorityMismatch,
    #[msg("vault release registry does not match the merchant and mint scope")]
    ReleaseRegistryBindingMismatch,
    #[msg("vault release registry is inactive")]
    ReleaseRegistryInactive,
    #[msg("vault release proposal authority does not match the registry")]
    ReleaseProposerMismatch,
    #[msg("vault release approval authority does not match the registry")]
    ReleaseApproverMismatch,
    #[msg("vault release amount must be positive")]
    InvalidReleaseAmount,
    #[msg("vault release reference digest must not be all zeroes")]
    InvalidReleaseReference,
    #[msg("vault release execution window is invalid or exceeds the bound")]
    InvalidReleaseWindow,
    #[msg("vault release claim is outside its execution window")]
    ReleaseClaimOutsideWindow,
    #[msg("vault release claim is already finalized")]
    ReleaseClaimAlreadyFinalized,
    #[msg("vault release claim does not match the scoped accounts")]
    ReleaseClaimBindingMismatch,
    #[msg("vault release destination does not match the claim")]
    ReleaseDestinationMismatch,
    #[msg("vault release vault authority does not match the deterministic PDA")]
    ReleaseVaultAuthorityMismatch,
    #[msg("vault release vault does not have enough funds")]
    ReleaseVaultInsufficientFunds,
    #[msg("adjustment nonce must be positive")]
    InvalidAdjustmentNonce,
    #[msg("adjustment kind is unsupported")]
    InvalidAdjustmentKind,
    #[msg("adjustment signer authorities must be distinct")]
    InvalidAdjustmentAuthority,
    #[msg("adjustment registry governance authority does not match the signer")]
    AdjustmentRegistryAuthorityMismatch,
    #[msg("adjustment registry does not match the merchant and mint scope")]
    AdjustmentRegistryBindingMismatch,
    #[msg("adjustment registry is inactive")]
    AdjustmentRegistryInactive,
    #[msg("adjustment proposer does not match the registered authority")]
    AdjustmentProposerMismatch,
    #[msg("adjustment approver does not match the registered authority")]
    AdjustmentApproverMismatch,
    #[msg("adjustment receipt does not match the merchant scope")]
    AdjustmentReceiptBindingMismatch,
    #[msg("adjustment sign must be zero or one and zero magnitudes must be unsigned")]
    InvalidAdjustmentSign,
    #[msg("adjustment delta must not be zero")]
    ZeroAdjustmentDelta,
    #[msg("adjustment arithmetic overflowed")]
    AdjustmentArithmeticOverflow,
    #[msg("adjustment delta does not conserve gross = tax + net")]
    AdjustmentConservationViolation,
    #[msg("adjustment digest must not be all zeroes")]
    EmptyAdjustmentDigest,
    #[msg("adjustment evidence digest must not be all zeroes")]
    EmptyAdjustmentEvidenceDigest,
    #[msg("adjustment calculation digest must not be all zeroes")]
    EmptyAdjustmentCalculationDigest,
    #[msg("this adjustment kind has no governed refund movement contract")]
    UnsupportedAdjustmentMovement,
    #[msg("adjustment approval evidence does not match the active approver")]
    AdjustmentApprovalBindingMismatch,
    #[msg("adjustment refund claim does not match its bound accounts")]
    AdjustmentRefundClaimBindingMismatch,
    #[msg("adjustment refund claim nonce must be positive")]
    InvalidAdjustmentRefundClaimNonce,
    #[msg("adjustment refund claim is outside its execution window")]
    AdjustmentRefundClaimOutsideWindow,
    #[msg("adjustment refund claim is already finalized")]
    AdjustmentRefundClaimAlreadyFinalized,
    #[msg("adjustment refund source mint does not match the adjustment mint")]
    AdjustmentRefundSourceMintMismatch,
    #[msg("adjustment refund source is not owned by the refund funder")]
    AdjustmentRefundSourceAuthorityMismatch,
    #[msg("adjustment refund destination is not owned by the original payer")]
    AdjustmentRefundDestinationAuthorityMismatch,
    #[msg("adjustment tax vault does not have enough funds")]
    AdjustmentTaxVaultInsufficientFunds,
    #[msg("adjustment refund source does not have enough funds")]
    AdjustmentRefundSourceInsufficientFunds,
}

fn validate_registration(
    sequence: u64,
    gross_atomic: u64,
    tax_atomic: u64,
    net_atomic: u64,
    policy_digest: [u8; 32],
    expires_at_unix: i64,
    now_unix: i64,
) -> Result<()> {
    require!(sequence > 0, SettlementError::InvalidPolicySequence);
    require!(policy_digest != [0; 32], SettlementError::EmptyPolicyDigest);
    require!(
        expires_at_unix > now_unix,
        SettlementError::InvalidPolicyExpiry
    );
    validate_settlement_amounts(gross_atomic, tax_atomic, net_atomic)
}

#[derive(Clone, Copy)]
struct PolicySettlementBinding {
    merchant: Pubkey,
    mint: Pubkey,
    sequence: u64,
    gross_atomic: u64,
    tax_atomic: u64,
    net_atomic: u64,
    policy_digest: [u8; 32],
}

fn validate_policy_attestation(
    policy: &PolicyAttestation,
    binding: PolicySettlementBinding,
    now_unix: i64,
) -> Result<()> {
    require_keys_eq!(
        policy.merchant,
        binding.merchant,
        SettlementError::PolicyBindingMismatch
    );
    require_keys_eq!(
        policy.mint,
        binding.mint,
        SettlementError::PolicyBindingMismatch
    );
    require!(
        policy.sequence == binding.sequence,
        SettlementError::PolicyBindingMismatch
    );
    require!(
        policy.gross_atomic == binding.gross_atomic,
        SettlementError::PolicyBindingMismatch
    );
    require!(
        policy.tax_atomic == binding.tax_atomic,
        SettlementError::PolicyBindingMismatch
    );
    require!(
        policy.net_atomic == binding.net_atomic,
        SettlementError::PolicyBindingMismatch
    );
    require!(
        policy.policy_digest == binding.policy_digest,
        SettlementError::PolicyBindingMismatch
    );
    require!(!policy.revoked, SettlementError::PolicyRevoked);
    require!(
        policy.expires_at_unix > now_unix,
        SettlementError::PolicyExpired
    );
    validate_settlement_amounts(binding.gross_atomic, binding.tax_atomic, binding.net_atomic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::str::FromStr;

    #[test]
    fn adjustment_delta_accepts_negative_refund_and_mixed_correction() {
        assert!(validate_adjustment_delta(1, 1, 100, 1, 25, 1, 75).is_ok());
        assert!(validate_adjustment_delta(3, 0, 10, 1, 4, 0, 14).is_ok());
    }

    #[test]
    fn adjustment_delta_rejects_non_conservation_and_overflow() {
        assert!(validate_adjustment_delta(1, 1, 100, 1, 25, 1, 74).is_err());
        assert!(validate_adjustment_delta(3, 0, u64::MAX, 0, 1, 0, 1).is_err());
    }

    #[test]
    fn adjustment_delta_rejects_positive_refund_and_zero_evidence() {
        assert!(validate_adjustment_delta(1, 0, 100, 0, 25, 0, 75).is_err());
        assert!(validate_adjustment_delta(1, 1, 0, 0, 0, 0, 0).is_err());
    }

    #[test]
    fn refund_movement_accepts_only_a_negative_refund_adjustment() {
        let adjustment = refund_adjustment_fixture();
        assert!(validate_refund_movement_amounts(&adjustment).is_ok());

        let mut correction = refund_adjustment_fixture();
        correction.kind = 3;
        assert!(validate_refund_movement_amounts(&correction).is_err());

        let mut positive = refund_adjustment_fixture();
        positive.gross_sign = 0;
        assert!(validate_refund_movement_amounts(&positive).is_err());
    }

    #[test]
    fn refund_movement_rejects_a_zero_leg_magnitude_with_negative_sign() {
        let mut adjustment = refund_adjustment_fixture();
        adjustment.tax_magnitude = 0;
        adjustment.tax_sign = 1;
        assert!(validate_refund_movement_amounts(&adjustment).is_err());
    }

    fn refund_adjustment_fixture() -> SettlementAdjustment {
        SettlementAdjustment {
            original_receipt: Pubkey::new_from_array([1; 32]),
            merchant: Pubkey::new_from_array([2; 32]),
            mint: Pubkey::new_from_array([3; 32]),
            proposer: Pubkey::new_from_array([4; 32]),
            approver: Pubkey::new_from_array([5; 32]),
            nonce: 1,
            kind: 1,
            gross_sign: 1,
            gross_magnitude: 100,
            tax_sign: 1,
            tax_magnitude: 25,
            net_sign: 1,
            net_magnitude: 75,
            adjustment_digest: [6; 32],
            evidence_digest: [7; 32],
            source_calculation_digest: [8; 32],
            replacement_calculation_digest: [0; 32],
            bump: 1,
        }
    }

    #[test]
    fn accepts_conserving_amounts() {
        assert!(validate_settlement_amounts(1_155, 155, 1_000).is_ok());
    }

    #[test]
    fn rejects_non_conserving_amounts() {
        assert!(validate_settlement_amounts(1_155, 154, 1_000).is_err());
    }

    #[test]
    fn rejects_zero_gross() {
        assert!(validate_settlement_amounts(0, 0, 0).is_err());
    }

    fn split_plan_fixture() -> SplitPlan {
        SplitPlan {
            merchant: Pubkey::from_str("8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR")
                .expect("merchant"),
            mint: Pubkey::from_str("GgBaCs3NCBuZN12kCJgAW63ydqohFkHEdfdEXBPzLHq").expect("mint"),
            version: 1,
            digest: [0; 32],
            recipients: vec![
                SplitRecipient {
                    token_account: Pubkey::from_str("QWmroo4YnnMqYW3cnxWkFdaTxGD3P7vMSzwMHGbUzwF")
                        .expect("recipient"),
                    basis_points: 7_000,
                },
                SplitRecipient {
                    token_account: Pubkey::from_str("CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8")
                        .expect("recipient"),
                    basis_points: 3_000,
                },
            ],
            bump: 1,
        }
    }

    #[test]
    fn split_plan_matches_shared_digest_and_allocation_vectors() {
        let plan = split_plan_fixture();
        assert_eq!(
            calculate_split_plan_digest(plan.merchant, plan.mint, plan.version, &plan.recipients,),
            [
                245, 27, 219, 197, 243, 59, 244, 252, 226, 232, 224, 208, 52, 31, 115, 134, 54,
                180, 76, 219, 116, 190, 27, 48, 217, 210, 150, 151, 38, 71, 128, 199,
            ]
        );
        let allocations = allocate_split_amounts(&plan, 1).expect("allocation");
        assert_eq!(&allocations[..2], &[1, 0]);
        let allocations = allocate_split_amounts(&plan, u64::MAX).expect("allocation");
        assert_eq!(
            &allocations[..2],
            &[12_912_720_851_596_686_131, 5_534_023_222_112_865_484]
        );
    }

    #[test]
    fn split_plan_rejects_duplicate_and_over_bound_recipients() {
        let plan = split_plan_fixture();
        let duplicate = vec![
            plan.recipients[0].clone(),
            SplitRecipient {
                token_account: plan.recipients[0].token_account,
                basis_points: 3_000,
            },
        ];
        assert!(validate_split_plan_shape(&duplicate, 1).is_err());
        assert!(validate_split_plan_shape(&plan.recipients, 0).is_err());
    }

    proptest! {
        #[test]
        fn split_allocation_conserves_for_bounded_two_way_plans(
            first_bps in 1_u16..10_000_u16,
            net_atomic in any::<u64>(),
        ) {
            let mut plan = split_plan_fixture();
            plan.recipients[0].basis_points = first_bps;
            plan.recipients[1].basis_points = 10_000 - first_bps;
            let allocations = allocate_split_amounts(&plan, net_atomic)
                .expect("bounded split must conserve");
            prop_assert_eq!(
                allocations[..2].iter().copied().sum::<u64>(),
                net_atomic,
            );
        }
    }

    proptest! {
        #[test]
        fn amount_validation_accepts_only_nonzero_exact_u64_conservation(
            tax_atomic in any::<u64>(),
            net_atomic in any::<u64>(),
        ) {
            let gross_atomic = tax_atomic.wrapping_add(net_atomic);
            let expected_valid = gross_atomic > 0
                && tax_atomic.checked_add(net_atomic) == Some(gross_atomic);

            prop_assert_eq!(
                validate_settlement_amounts(gross_atomic, tax_atomic, net_atomic).is_ok(),
                expected_valid,
            );
        }
    }

    fn policy_fixture() -> PolicyAttestation {
        PolicyAttestation {
            merchant: Pubkey::new_from_array([1; 32]),
            mint: Pubkey::new_from_array([2; 32]),
            issuer: Pubkey::new_from_array([3; 32]),
            policy_digest: [4; 32],
            sequence: 7,
            gross_atomic: 1_155,
            tax_atomic: 155,
            net_atomic: 1_000,
            expires_at_unix: 1_800_000_300,
            revoked: false,
            bump: 1,
        }
    }

    #[test]
    fn accepts_an_exact_active_policy_attestation() {
        let policy = policy_fixture();
        assert!(
            validate_policy_attestation(
                &policy,
                PolicySettlementBinding {
                    merchant: policy.merchant,
                    mint: policy.mint,
                    sequence: policy.sequence,
                    gross_atomic: policy.gross_atomic,
                    tax_atomic: policy.tax_atomic,
                    net_atomic: policy.net_atomic,
                    policy_digest: policy.policy_digest,
                },
                1_800_000_100,
            )
            .is_ok()
        );
    }

    #[test]
    fn rejects_revoked_or_expired_policy_attestations() {
        let mut policy = policy_fixture();
        policy.revoked = true;
        assert!(
            validate_policy_attestation(
                &policy,
                PolicySettlementBinding {
                    merchant: policy.merchant,
                    mint: policy.mint,
                    sequence: policy.sequence,
                    gross_atomic: policy.gross_atomic,
                    tax_atomic: policy.tax_atomic,
                    net_atomic: policy.net_atomic,
                    policy_digest: policy.policy_digest,
                },
                1_800_000_100,
            )
            .is_err()
        );
        policy.revoked = false;
        assert!(
            validate_policy_attestation(
                &policy,
                PolicySettlementBinding {
                    merchant: policy.merchant,
                    mint: policy.mint,
                    sequence: policy.sequence,
                    gross_atomic: policy.gross_atomic,
                    tax_atomic: policy.tax_atomic,
                    net_atomic: policy.net_atomic,
                    policy_digest: policy.policy_digest,
                },
                1_800_000_300,
            )
            .is_err()
        );
    }

    fn registry_fixture() -> PolicyIssuerRegistry {
        PolicyIssuerRegistry {
            merchant: Pubkey::new_from_array([1; 32]),
            mint: Pubkey::new_from_array([2; 32]),
            authority: Pubkey::new_from_array([5; 32]),
            issuer: Pubkey::new_from_array([3; 32]),
            active: true,
            bump: 9,
        }
    }

    #[test]
    fn accepts_only_the_active_issuer_for_registration() {
        let registry = registry_fixture();
        assert!(
            validate_registry_for_registration(
                &registry,
                registry.merchant,
                registry.mint,
                registry.issuer,
            )
            .is_ok()
        );
        assert!(
            validate_registry_for_registration(
                &registry,
                registry.merchant,
                registry.mint,
                Pubkey::new_from_array([6; 32]),
            )
            .is_err()
        );
        assert!(
            validate_registry_for_registration(
                &registry,
                Pubkey::new_from_array([7; 32]),
                registry.mint,
                registry.issuer,
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_inactive_registry_and_unauthorized_rotation() {
        let mut registry = registry_fixture();
        registry.active = false;
        assert!(
            validate_registry_for_registration(
                &registry,
                registry.merchant,
                registry.mint,
                registry.issuer,
            )
            .is_err()
        );

        registry.active = true;
        assert!(
            validate_registry_update(
                &registry,
                registry.authority,
                Pubkey::new_from_array([8; 32]),
            )
            .is_ok()
        );
        assert!(
            validate_registry_update(
                &registry,
                Pubkey::new_from_array([6; 32]),
                Pubkey::new_from_array([8; 32]),
            )
            .is_err()
        );
        assert!(
            validate_registry_update(&registry, registry.authority, Pubkey::default(),).is_err()
        );
    }

    #[test]
    fn revocation_requires_registry_authority_and_matching_scope() {
        let registry = registry_fixture();
        let policy = policy_fixture();
        assert!(validate_revocation_authority(&registry, &policy, registry.authority,).is_ok());
        assert!(validate_revocation_authority(&registry, &policy, registry.issuer,).is_err());
        let mut foreign_policy = policy;
        foreign_policy.mint = Pubkey::new_from_array([9; 32]);
        assert!(
            validate_revocation_authority(&registry, &foreign_policy, registry.authority,).is_err()
        );
    }

    #[test]
    fn accepts_only_extension_free_token_account_layouts() {
        assert!(validate_supported_token_layout(82, 165, 165, 165).is_ok());
        assert!(validate_supported_token_layout(83, 165, 165, 165).is_err());
        assert!(validate_supported_token_layout(82, 166, 165, 165).is_err());
        assert!(validate_supported_token_layout(82, 165, 166, 165).is_err());
        assert!(validate_supported_token_layout(82, 165, 165, 166).is_err());
    }

    fn protocol_fixture(paused: bool) -> ProtocolConfig {
        ProtocolConfig {
            authority: Pubkey::new_from_array([8; 32]),
            paused,
            bump: 3,
        }
    }

    #[test]
    fn accepts_settlement_when_protocol_is_unpaused() {
        assert!(validate_protocol_not_paused(&protocol_fixture(false)).is_ok());
    }

    #[test]
    fn rejects_settlement_when_protocol_is_paused() {
        assert!(validate_protocol_not_paused(&protocol_fixture(true)).is_err());
    }

    #[test]
    fn pause_state_requires_the_configured_authority() {
        let config = protocol_fixture(false);
        assert!(validate_protocol_authority(&config, config.authority).is_ok());
        assert!(validate_protocol_authority(&config, Pubkey::new_from_array([9; 32])).is_err());
    }

    fn release_registry_fixture() -> VaultReleaseRegistry {
        VaultReleaseRegistry {
            merchant: Pubkey::new_from_array([1; 32]),
            mint: Pubkey::new_from_array([2; 32]),
            governance_authority: Pubkey::new_from_array([3; 32]),
            proposer: Pubkey::new_from_array([4; 32]),
            approver: Pubkey::new_from_array([5; 32]),
            active: true,
            bump: 6,
        }
    }

    fn release_claim_fixture() -> ReleaseClaim {
        ReleaseClaim {
            merchant: Pubkey::new_from_array([1; 32]),
            mint: Pubkey::new_from_array([2; 32]),
            nonce: 9,
            amount_atomic: 100,
            destination: Pubkey::new_from_array([7; 32]),
            reference_digest: [8; 32],
            execute_after_unix: 1_800_000_100,
            expires_at_unix: 1_800_000_300,
            proposer: Pubkey::new_from_array([4; 32]),
            executed: false,
            cancelled: false,
            bump: 10,
        }
    }

    #[test]
    fn release_claim_requires_distinct_roles_and_bounded_window() {
        let registry = release_registry_fixture();
        assert!(
            validate_release_registry_authorities(
                registry.governance_authority,
                registry.proposer,
                registry.approver,
            )
            .is_ok()
        );
        assert!(
            validate_release_registry_authorities(
                registry.governance_authority,
                registry.proposer,
                registry.proposer,
            )
            .is_err()
        );
        assert!(
            validate_release_claim_inputs(
                100,
                [8; 32],
                1_800_000_100,
                1_800_000_300,
                1_800_000_000,
            )
            .is_ok()
        );
        assert!(
            validate_release_claim_inputs(
                100,
                [8; 32],
                1_800_000_100,
                1_800_000_300 + MAX_RELEASE_WINDOW_SECONDS,
                1_800_000_000,
            )
            .is_err()
        );
        assert!(
            validate_release_claim_inputs(0, [8; 32], 1_800_000_100, 1_800_000_300, 1_800_000_000,)
                .is_err()
        );
    }

    #[test]
    fn release_claim_execution_is_fail_closed_and_single_use() {
        let registry = release_registry_fixture();
        let claim = release_claim_fixture();
        let execution = ReleaseExecutionContext {
            merchant: registry.merchant,
            mint: registry.mint,
            vault_token_account: Pubkey::new_from_array([6; 32]),
            destination: claim.destination,
            now_unix: 1_800_000_200,
            vault_amount: 100,
        };
        assert!(
            validate_release_claim_for_execution(&registry, &claim, registry.approver, &execution,)
                .is_ok()
        );
        assert!(
            validate_release_claim_for_execution(&registry, &claim, claim.proposer, &execution,)
                .is_err()
        );
        let mut outside_window = execution;
        outside_window.now_unix = 1_800_000_301;
        assert!(
            validate_release_claim_for_execution(
                &registry,
                &claim,
                registry.approver,
                &outside_window,
            )
            .is_err()
        );
        let mut insufficient_funds = execution;
        insufficient_funds.vault_amount = 99;
        assert!(
            validate_release_claim_for_execution(
                &registry,
                &claim,
                registry.approver,
                &insufficient_funds,
            )
            .is_err()
        );
        let mut executed = claim.clone();
        executed.executed = true;
        assert!(
            validate_release_claim_for_execution(
                &registry,
                &executed,
                registry.approver,
                &execution,
            )
            .is_err()
        );
    }
}
