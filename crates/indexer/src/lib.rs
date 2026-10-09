#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use anchor_lang::prelude::Pubkey;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use reqwest::Url;
use revenue_streaming::{
    AssetId as RevenueAssetId, Digest as RevenueDigest, FinalityCommitment,
    FinalizedSettlementAllocationProof, RecipientId as RevenueRecipientId,
    Signature as RevenueSignature,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use settlement_adjustments::{
    AdjustmentDelta, RefundBalanceProof, RefundBalanceSnapshot, prove_refund_balance_deltas,
};
use sha2::{Digest, Sha256};
use solana_signature::Signature;
use std::str::FromStr;
use std::sync::{
    Arc,
    atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};
use thiserror::Error;
use uuid::Uuid;

pub const MAX_SCAN_RANGE_SLOTS: u64 = 10_000;
pub const MAX_RPC_PAGE_SLOTS: u64 = 128;
pub const MAX_RPC_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
pub const DEFAULT_RPC_TIMEOUT: Duration = Duration::from_secs(2);
pub const MAX_PROVIDER_ATTEMPTS: usize = 3;
pub const MAX_RAW_EVIDENCE_BYTES: usize = 1024 * 1024;
pub const MAX_RAW_TRANSACTION_BYTES: usize = 512 * 1024;
pub const MAX_PROVIDER_HEALTH_SCORE: i32 = 8;
pub const MIN_PROVIDER_HEALTH_SCORE: i32 = -8;
pub const PROVIDER_COOLDOWN_BASE: Duration = Duration::from_millis(25);
pub const MAX_PROVIDER_COOLDOWN: Duration = Duration::from_secs(5);
pub const SETTLEMENT_PROGRAM_ID: &str = "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi";
pub const STABLECOIN_ASSET_KIND: u8 = 1;
pub const ASSET_CONFIG_ACCOUNT_DISCRIMINATOR: [u8; 8] =
    [0x39, 0x70, 0xf7, 0xa6, 0xf7, 0x40, 0x8c, 0x17];
pub const ASSET_CONFIG_ACCOUNT_DATA_LEN: usize = 108;
pub const SETTLEMENT_ROUTE_VERIFICATION_SCHEMA: &str = "settlement-route-verification/v2";
pub const SETTLEMENT_EVENT_DISCRIMINATOR: [u8; 8] = [198, 221, 154, 28, 173, 8, 114, 6];
pub const DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR: [u8; 8] = [101, 58, 10, 108, 29, 185, 246, 2];
pub const SPLIT_SETTLEMENT_EVENT_DISCRIMINATOR: [u8; 8] = [50, 129, 102, 109, 249, 142, 125, 30];
pub const APPLICABILITY_SETTLEMENT_EVENT_DISCRIMINATOR: [u8; 8] =
    [183, 111, 70, 137, 145, 33, 22, 37];
pub const APPLICABILITY_DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR: [u8; 8] =
    [97, 174, 200, 140, 146, 33, 72, 238];
pub const PROTOCOL_PAUSE_EVENT_DISCRIMINATOR: [u8; 8] = [67, 33, 235, 73, 71, 124, 172, 110];
pub const ADJUSTMENT_REFUND_EXECUTED_EVENT_DISCRIMINATOR: [u8; 8] =
    [112, 130, 243, 73, 61, 70, 35, 70];
pub const ADJUSTMENT_REFUND_CLAIM_CREATED_EVENT_DISCRIMINATOR: [u8; 8] =
    [40, 169, 108, 67, 121, 205, 147, 49];
pub const ADJUSTMENT_REFUND_CLAIM_CANCELLED_EVENT_DISCRIMINATOR: [u8; 8] =
    [201, 89, 79, 76, 75, 107, 22, 3];
pub const SETTLEMENT_ADJUSTMENT_RECORDED_EVENT_DISCRIMINATOR: [u8; 8] =
    [208, 86, 69, 183, 44, 203, 236, 75];
pub const MAX_ROUTE_RPC_RESULT_BYTES: usize = 256 * 1024;
pub const MAX_ROUTE_ACCOUNT_DATA_BYTES: usize = 256;
pub const MAX_REFUND_CLAIM_ACCOUNT_DATA_BYTES: usize = 512;
pub const APPLICABILITY_BINDING_ACCOUNT_DATA_LEN: usize = 8 + 32 + 32 + 32 + 16 + 32 + 1;
pub const REFUND_CLAIM_ACCOUNT_DATA_LEN: usize = 8 + (9 * 32) + (4 * 8) + 32 + (2 * 8) + 32 + 3;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalizedPage {
    pub range: ScanRange,
    pub observed_finalized_slot: u64,
    pub receipts: Vec<IndexedReceipt>,
    pub split_settlements: Vec<IndexedSplitSettlement>,
    pub adjustment_records: Vec<IndexedSettlementAdjustment>,
    pub refund_executions: Vec<IndexedAdjustmentRefundExecution>,
    pub raw_evidence: Vec<RawChainEvidence>,
    pub raw_transactions: Vec<RawChainTransaction>,
    pub protocol_pause_changes: Vec<ProtocolPauseChangedEvent>,
    pub complete: bool,
}

impl FinalizedPage {
    pub fn validate(&self, planned_range: ScanRange) -> Result<(), IndexerError> {
        planned_range.validate()?;
        if self.range != planned_range {
            return Err(IndexerError::ProviderRangeMismatch);
        }
        if !self.complete {
            return Err(IndexerError::IncompleteRange);
        }
        if self.observed_finalized_slot < self.range.end_slot {
            return Err(IndexerError::FinalityViolation);
        }
        let mut signatures = std::collections::HashSet::with_capacity(self.receipts.len());
        for receipt in &self.receipts {
            receipt.validate(self.range, self.observed_finalized_slot)?;
            if !signatures.insert(&receipt.signature) {
                return Err(IndexerError::ConflictingEvidence);
            }
        }
        let mut split_signatures =
            std::collections::HashSet::with_capacity(self.split_settlements.len());
        for split in &self.split_settlements {
            split.validate(self.range, self.observed_finalized_slot)?;
            if !split_signatures.insert(&split.signature) {
                return Err(IndexerError::ConflictingEvidence);
            }
            let Some(receipt) = self
                .receipts
                .iter()
                .find(|receipt| receipt.signature == split.signature)
            else {
                return Err(IndexerError::ConflictingEvidence);
            };
            if receipt.gross_atomic != split.gross_atomic
                || receipt.tax_atomic != split.tax_atomic
                || receipt.net_atomic != split.net_atomic
                || receipt.evidence_digest != split.evidence_digest
            {
                return Err(IndexerError::ConflictingEvidence);
            }
        }
        let mut refund_execution_keys =
            std::collections::HashSet::with_capacity(self.refund_executions.len());
        for execution in &self.refund_executions {
            execution.validate(self.range, self.observed_finalized_slot)?;
            if !refund_execution_keys.insert((&execution.signature, execution.event_index)) {
                return Err(IndexerError::ConflictingEvidence);
            }
        }
        let mut adjustment_record_keys =
            std::collections::HashSet::with_capacity(self.adjustment_records.len());
        for record in &self.adjustment_records {
            record.validate(self.range, self.observed_finalized_slot)?;
            if !adjustment_record_keys.insert((&record.signature, record.event_index)) {
                return Err(IndexerError::ConflictingEvidence);
            }
        }
        let mut evidence_keys = std::collections::HashSet::with_capacity(self.raw_evidence.len());
        for evidence in &self.raw_evidence {
            evidence.validate(self.range, self.observed_finalized_slot)?;
            if !evidence_keys.insert((&evidence.signature, evidence.event_index)) {
                return Err(IndexerError::ConflictingEvidence);
            }
        }
        for execution in &self.refund_executions {
            let Some(evidence) = self.raw_evidence.iter().find(|evidence| {
                evidence.signature == execution.signature
                    && evidence.event_index == execution.event_index
            }) else {
                return Err(IndexerError::InvalidEvidence);
            };
            if evidence.event_discriminator != ADJUSTMENT_REFUND_EXECUTED_EVENT_DISCRIMINATOR
                || evidence.evidence_digest != execution.evidence_digest
            {
                return Err(IndexerError::ConflictingEvidence);
            }
        }
        for record in &self.adjustment_records {
            let Some(evidence) = self.raw_evidence.iter().find(|evidence| {
                evidence.signature == record.signature && evidence.event_index == record.event_index
            }) else {
                return Err(IndexerError::InvalidEvidence);
            };
            if evidence.event_discriminator != SETTLEMENT_ADJUSTMENT_RECORDED_EVENT_DISCRIMINATOR
                || evidence.evidence_digest != record.evidence_digest
            {
                return Err(IndexerError::ConflictingEvidence);
            }
        }
        let mut transaction_keys =
            std::collections::HashSet::with_capacity(self.raw_transactions.len());
        for transaction in &self.raw_transactions {
            transaction.validate(self.range, self.observed_finalized_slot)?;
            if !transaction_keys.insert((&transaction.signature, transaction.transaction_index)) {
                return Err(IndexerError::ConflictingEvidence);
            }
        }
        let mut pause_keys =
            std::collections::HashSet::with_capacity(self.protocol_pause_changes.len());
        for change in &self.protocol_pause_changes {
            change.validate(self.range, self.observed_finalized_slot)?;
            if !pause_keys.insert((&change.signature, change.event_index)) {
                return Err(IndexerError::ConflictingEvidence);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ProviderError {
    #[error("provider transport failed: {0}")]
    Transport(String),
    #[error("provider response exceeded the configured size limit")]
    ResponseTooLarge,
    #[error("provider returned an invalid JSON-RPC response: {0}")]
    InvalidResponse(String),
    #[error("provider returned a JSON-RPC error: {0}")]
    Rpc(String),
    #[error("provider returned an invalid finalized page: {0}")]
    InvalidPage(#[from] IndexerError),
    #[error("provider returned a non-finalized response")]
    NonFinalized,
}

/// A finalized-only provider port. Implementations may use HTTP JSON-RPC,
/// Geyser, or a managed provider, but must translate provider-specific data
/// into this bounded, canonical page before it reaches the coordinator.
#[async_trait]
pub trait FinalizedPageSource: Send + Sync {
    async fn fetch_finalized_page(&self, range: ScanRange) -> Result<FinalizedPage, ProviderError>;
}

#[derive(Clone)]
pub struct FailoverPageSource {
    providers: Vec<ProviderSlot>,
    attempt_timeout: Duration,
}

#[derive(Clone)]
struct ProviderSlot {
    source: Arc<dyn FinalizedPageSource>,
    health: AdaptiveProviderHealth,
}

struct ProviderHealthState {
    score: AtomicI32,
    consecutive_failures: AtomicU32,
    cooldown_until_millis: AtomicU64,
}

/// Process-local, bounded provider health used only to order failover
/// attempts. It is deliberately disposable and cannot authorize settlement or
/// represent chain finality.
#[derive(Clone)]
pub struct AdaptiveProviderHealth {
    state: Arc<ProviderHealthState>,
    started_at: Instant,
}

impl AdaptiveProviderHealth {
    pub fn new() -> Self {
        Self {
            state: Arc::new(ProviderHealthState {
                score: AtomicI32::new(0),
                consecutive_failures: AtomicU32::new(0),
                cooldown_until_millis: AtomicU64::new(0),
            }),
            started_at: Instant::now(),
        }
    }

    pub fn score(&self) -> i32 {
        self.state.score.load(Ordering::Acquire)
    }

    pub fn consecutive_failures(&self) -> u32 {
        self.state.consecutive_failures.load(Ordering::Acquire)
    }

    pub fn cooldown_remaining(&self) -> Duration {
        let now = self.elapsed_millis();
        let cooldown_until = self.state.cooldown_until_millis.load(Ordering::Acquire);
        Duration::from_millis(cooldown_until.saturating_sub(now))
    }

    pub fn is_available(&self) -> bool {
        self.cooldown_remaining().is_zero()
    }

    pub fn record_success(&self) {
        self.state
            .score
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |score| {
                Some(score.saturating_add(1).min(MAX_PROVIDER_HEALTH_SCORE))
            })
            .ok();
        self.state.consecutive_failures.store(0, Ordering::Release);
        self.state.cooldown_until_millis.store(0, Ordering::Release);
    }

    pub fn record_failure(&self) {
        self.state
            .score
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |score| {
                Some(score.saturating_sub(1).max(MIN_PROVIDER_HEALTH_SCORE))
            })
            .ok();
        let failures = self
            .state
            .consecutive_failures
            .fetch_add(1, Ordering::AcqRel)
            .saturating_add(1);
        let exponent = failures.saturating_sub(1).min(7);
        let multiplier = 1_u64 << exponent;
        let cooldown_millis = PROVIDER_COOLDOWN_BASE
            .as_millis()
            .saturating_mul(multiplier as u128)
            .min(MAX_PROVIDER_COOLDOWN.as_millis()) as u64;
        let proposed = self.elapsed_millis().saturating_add(cooldown_millis);
        let mut observed = self.state.cooldown_until_millis.load(Ordering::Acquire);
        while observed < proposed {
            match self.state.cooldown_until_millis.compare_exchange_weak(
                observed,
                proposed,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(current) => observed = current,
            }
        }
    }

    fn elapsed_millis(&self) -> u64 {
        self.started_at.elapsed().as_millis().min(u64::MAX as u128) as u64
    }
}

impl Default for AdaptiveProviderHealth {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderHealthSnapshot {
    pub provider_index: usize,
    pub score: i32,
    pub consecutive_failures: u32,
    pub cooldown_remaining: Duration,
}

impl FailoverPageSource {
    pub fn new(
        providers: Vec<Arc<dyn FinalizedPageSource>>,
        attempt_timeout: Duration,
    ) -> Result<Self, ProviderError> {
        if providers.is_empty() || providers.len() > MAX_PROVIDER_ATTEMPTS {
            return Err(ProviderError::InvalidResponse(
                "provider pool must contain between 1 and 3 endpoints".to_string(),
            ));
        }
        if !(Duration::from_millis(50)..=Duration::from_secs(10)).contains(&attempt_timeout) {
            return Err(ProviderError::InvalidResponse(
                "provider attempt timeout must be between 50ms and 10s".to_string(),
            ));
        }
        Ok(Self {
            providers: providers
                .into_iter()
                .map(|source| ProviderSlot {
                    source,
                    health: AdaptiveProviderHealth::new(),
                })
                .collect(),
            attempt_timeout,
        })
    }

    pub fn health_snapshot(&self) -> Vec<ProviderHealthSnapshot> {
        self.providers
            .iter()
            .enumerate()
            .map(|(provider_index, provider)| ProviderHealthSnapshot {
                provider_index,
                score: provider.health.score(),
                consecutive_failures: provider.health.consecutive_failures(),
                cooldown_remaining: provider.health.cooldown_remaining(),
            })
            .collect()
    }

    /// Probe the provider pool without requiring the configured start slot to
    /// be finalized yet. A reachable provider may legitimately report
    /// `NonFinalized` when a fresh local validator has not produced the start
    /// slot; transport and malformed-response failures remain failures.
    pub async fn probe(&self, start_slot: u64) -> Result<(), ProviderError> {
        let range = ScanRange {
            start_slot,
            end_slot: start_slot,
        };
        let mut last_error = None;
        for provider_index in self.provider_order() {
            let provider = &self.providers[provider_index];
            match tokio::time::timeout(
                self.attempt_timeout,
                provider.source.fetch_finalized_page(range),
            )
            .await
            {
                Ok(Ok(_)) | Ok(Err(ProviderError::NonFinalized)) => {
                    self.record_success(provider_index);
                    return Ok(());
                }
                Ok(Err(error)) => {
                    self.record_failure(provider_index);
                    last_error = Some(error);
                }
                Err(_) => {
                    self.record_failure(provider_index);
                    last_error = Some(ProviderError::Transport(
                        "provider attempt deadline exceeded".to_string(),
                    ));
                }
            }
        }
        match last_error {
            Some(error) => Err(error),
            None => Err(ProviderError::Transport(
                "provider probe produced no result".to_string(),
            )),
        }
    }

    fn provider_order(&self) -> Vec<usize> {
        let mut available = self
            .providers
            .iter()
            .enumerate()
            .filter(|(_, provider)| provider.health.is_available())
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if available.is_empty()
            && let Some((index, _)) = self
                .providers
                .iter()
                .enumerate()
                .min_by_key(|(_, provider)| provider.health.cooldown_remaining())
        {
            available.push(index);
        }
        available.sort_by_key(|index| {
            (
                std::cmp::Reverse(self.providers[*index].health.score()),
                *index,
            )
        });
        available
    }

    fn record_success(&self, provider_index: usize) {
        self.providers[provider_index].health.record_success();
    }

    fn record_failure(&self, provider_index: usize) {
        self.providers[provider_index].health.record_failure();
    }
}

#[async_trait]
impl FinalizedPageSource for FailoverPageSource {
    async fn fetch_finalized_page(&self, range: ScanRange) -> Result<FinalizedPage, ProviderError> {
        let mut last_error = None;
        for provider_index in self.provider_order() {
            let provider = &self.providers[provider_index];
            match tokio::time::timeout(
                self.attempt_timeout,
                provider.source.fetch_finalized_page(range),
            )
            .await
            {
                Ok(Ok(page)) => {
                    self.record_success(provider_index);
                    return Ok(page);
                }
                Ok(Err(error)) => {
                    self.record_failure(provider_index);
                    last_error = Some(error);
                }
                Err(_) => {
                    self.record_failure(provider_index);
                    last_error = Some(ProviderError::Transport(
                        "provider attempt deadline exceeded".to_string(),
                    ));
                }
            }
        }
        match last_error {
            Some(error) => Err(error),
            None => Err(ProviderError::Transport(
                "provider pool returned no result".to_string(),
            )),
        }
    }
}

/// Transport port for Solana JSON-RPC. Keeping this below the provider makes
/// request-shape and finality tests independent of a network or vendor.
#[async_trait]
pub trait JsonRpcTransport: Send + Sync {
    async fn call(&self, method: &str, params: Value) -> Result<Value, ProviderError>;
}

#[async_trait]
impl<T: JsonRpcTransport + ?Sized> JsonRpcTransport for &T {
    async fn call(&self, method: &str, params: Value) -> Result<Value, ProviderError> {
        (**self).call(method, params).await
    }
}

/// Server-owned identity used by the read-only vault reconciler. The caller
/// supplies no expected balance; that value is derived from persisted,
/// finalized projection evidence by the storage adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultBalanceTarget {
    pub tax_vault: String,
    pub mint: String,
    pub token_program: String,
}

/// Server-owned topology for the first governed Refund balance proof. Every
/// address is resolved from the approved claim/route evidence; callers must
/// not be allowed to choose arbitrary accounts at the reconciliation boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundBalanceTarget {
    pub claim: String,
    pub adjustment: String,
    pub original_receipt: String,
    pub merchant: String,
    pub claim_nonce: u64,
    pub tax_vault: String,
    pub refund_source: String,
    pub payer_destination: String,
    pub mint: String,
    pub token_program: String,
    pub vault_authority: String,
    pub refund_funder: String,
    pub payer: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FinalizedRefundBalances {
    pub slot: u64,
    pub tax_vault_atomic: u64,
    pub refund_source_atomic: u64,
    pub payer_destination_atomic: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FinalizedVaultBalance {
    pub slot: u64,
    pub amount_atomic: u64,
}

/// Server-owned identity for a receipt-scoped applicability binding. Every
/// field is resolved from the stored settlement event/quote; callers cannot
/// use this target to select an arbitrary merchant-side account.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicabilityBindingTarget {
    pub binding: String,
    pub receipt: String,
    pub merchant: String,
    pub mint: String,
    pub context_id: [u8; 16],
    pub attestation_digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FinalizedApplicabilityBinding {
    pub slot: u64,
    pub binding: [u8; 32],
    pub receipt: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub context_id: [u8; 16],
    pub attestation_digest: [u8; 32],
    pub bump: u8,
}

#[async_trait]
pub trait FinalizedApplicabilityBindingSource: Send + Sync {
    async fn fetch_finalized_applicability_binding(
        &self,
        target: &ApplicabilityBindingTarget,
    ) -> Result<FinalizedApplicabilityBinding, ProviderError>;
}

/// Dedicated finalized account reader for applicability-bound settlement.
/// This workflow is intentionally outside quote, compilation, wallet signing,
/// and settlement submission paths.
pub struct SolanaRpcApplicabilityBindingSource<T> {
    transport: T,
}

impl<T> SolanaRpcApplicabilityBindingSource<T> {
    pub const fn new(transport: T) -> Self {
        Self { transport }
    }
}

#[async_trait]
impl<T: JsonRpcTransport> FinalizedApplicabilityBindingSource
    for SolanaRpcApplicabilityBindingSource<T>
{
    async fn fetch_finalized_applicability_binding(
        &self,
        target: &ApplicabilityBindingTarget,
    ) -> Result<FinalizedApplicabilityBinding, ProviderError> {
        if target.context_id == [0; 16] || target.attestation_digest == [0; 32] {
            return Err(ProviderError::InvalidResponse(
                "applicability binding target contains an empty identity or digest".to_string(),
            ));
        }
        let binding = Pubkey::from_str(target.binding.trim()).map_err(|_| {
            ProviderError::InvalidResponse(
                "applicability binding address is not a valid public key".to_string(),
            )
        })?;
        let receipt = Pubkey::from_str(target.receipt.trim()).map_err(|_| {
            ProviderError::InvalidResponse(
                "applicability receipt is not a valid public key".to_string(),
            )
        })?;
        let merchant = Pubkey::from_str(target.merchant.trim()).map_err(|_| {
            ProviderError::InvalidResponse(
                "applicability merchant is not a valid public key".to_string(),
            )
        })?;
        let mint = Pubkey::from_str(target.mint.trim()).map_err(|_| {
            ProviderError::InvalidResponse(
                "applicability mint is not a valid public key".to_string(),
            )
        })?;
        let (expected_binding, expected_bump) = Pubkey::find_program_address(
            &[b"applicability-binding", receipt.as_ref()],
            &settlement_client::settlement_program_id(),
        );
        if binding != expected_binding {
            return Err(ProviderError::InvalidResponse(
                "applicability binding PDA does not match the receipt".to_string(),
            ));
        }
        let response = self
            .transport
            .call(
                "getMultipleAccounts",
                json!([[binding.to_string()], {
                    "commitment": "finalized",
                    "encoding": "base64"
                }]),
            )
            .await?;
        let encoded = serde_json::to_vec(&response)
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        if encoded.len() > MAX_ROUTE_RPC_RESULT_BYTES {
            return Err(ProviderError::ResponseTooLarge);
        }
        let slot = response
            .get("context")
            .and_then(|value| value.get("slot"))
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                ProviderError::InvalidResponse(
                    "applicability binding finalized slot is invalid".to_string(),
                )
            })?;
        let values = response
            .get("value")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ProviderError::InvalidResponse(
                    "applicability binding account value is not an array".to_string(),
                )
            })?;
        if values.len() != 1 {
            return Err(ProviderError::InvalidResponse(
                "applicability binding response contains an unexpected account count".to_string(),
            ));
        }
        let account = parse_route_account_bounded(
            &values[0],
            &binding.to_string(),
            APPLICABILITY_BINDING_ACCOUNT_DATA_LEN,
        )
        .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        if account.executable || account.owner != settlement_client::SETTLEMENT_PROGRAM_ID {
            return Err(ProviderError::InvalidResponse(
                "applicability binding owner or executable flag is invalid".to_string(),
            ));
        }
        if account.data.len() != APPLICABILITY_BINDING_ACCOUNT_DATA_LEN {
            return Err(ProviderError::InvalidResponse(
                "applicability binding uses an unsupported account layout".to_string(),
            ));
        }
        let mut discriminator = Sha256::new();
        discriminator.update(b"account:ApplicabilitySettlementBinding");
        let discriminator = discriminator.finalize();
        if account.data.get(..8) != Some(&discriminator[..8]) {
            return Err(ProviderError::InvalidResponse(
                "applicability binding discriminator is invalid".to_string(),
            ));
        }
        let actual_receipt = read_pubkey_field(&account, 8, "applicability receipt")
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        let actual_merchant = read_pubkey_field(&account, 40, "applicability merchant")
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        let actual_mint = read_pubkey_field(&account, 72, "applicability mint")
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        let actual_context_id = account.data.get(104..120).ok_or_else(|| {
            ProviderError::InvalidResponse("applicability context ID is missing".to_string())
        })?;
        let actual_attestation_digest = account.data.get(120..152).ok_or_else(|| {
            ProviderError::InvalidResponse(
                "applicability attestation digest is missing".to_string(),
            )
        })?;
        let actual_bump = *account.data.get(152).ok_or_else(|| {
            ProviderError::InvalidResponse("applicability binding bump is missing".to_string())
        })?;
        if actual_receipt != receipt
            || actual_merchant != merchant
            || actual_mint != mint
            || actual_context_id != target.context_id
            || actual_attestation_digest != target.attestation_digest
            || actual_bump != expected_bump
        {
            return Err(ProviderError::InvalidResponse(
                "applicability binding account does not match the server-owned target".to_string(),
            ));
        }
        Ok(FinalizedApplicabilityBinding {
            slot,
            binding: binding.to_bytes(),
            receipt: actual_receipt.to_bytes(),
            merchant: actual_merchant.to_bytes(),
            mint: actual_mint.to_bytes(),
            context_id: target.context_id,
            attestation_digest: target.attestation_digest,
            bump: actual_bump,
        })
    }
}

#[async_trait]
pub trait FinalizedVaultBalanceSource: Send + Sync {
    async fn fetch_finalized_vault_balance(
        &self,
        target: &VaultBalanceTarget,
    ) -> Result<FinalizedVaultBalance, ProviderError>;
}

#[async_trait]
pub trait FinalizedRefundBalanceSource: Send + Sync {
    async fn fetch_finalized_refund_balances(
        &self,
        target: &RefundBalanceTarget,
    ) -> Result<FinalizedRefundBalances, ProviderError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundBalanceProofTarget {
    pub signature: String,
    pub topology: RefundBalanceTarget,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub reference_digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FinalizedRefundBalanceTransition {
    pub slot: u64,
    pub tax_vault_before_atomic: u64,
    pub tax_vault_after_atomic: u64,
    pub refund_source_before_atomic: u64,
    pub refund_source_after_atomic: u64,
    pub payer_destination_before_atomic: u64,
    pub payer_destination_after_atomic: u64,
}

#[async_trait]
pub trait FinalizedRefundBalanceTransitionSource: Send + Sync {
    async fn fetch_finalized_refund_balance_transition(
        &self,
        target: &RefundBalanceProofTarget,
    ) -> Result<FinalizedRefundBalanceTransition, ProviderError>;
}

/// The only refund identity accepted by the proof orchestrator. Account
/// topology omitted by the event is resolved from the finalized claim PDA.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundClaimTopologyRequest {
    pub claim: [u8; 32],
    pub adjustment: [u8; 32],
    pub original_receipt: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub claim_nonce: u64,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub refund_funder: [u8; 32],
    pub refund_destination: [u8; 32],
    pub reference_digest: [u8; 32],
}

#[async_trait]
pub trait FinalizedRefundBalanceProofSource: Send + Sync {
    async fn fetch_finalized_refund_balance_proof(
        &self,
        request: &RefundClaimTopologyRequest,
        signature: &str,
    ) -> Result<(RefundBalanceProofTarget, FinalizedRefundBalanceTransition), ProviderError>;
}

pub const MAX_SIGNATURE_PAGE_SIZE: usize = 1_000;

/// A bounded discovery result from `getSignaturesForAddress`. This is not
/// finalized transaction evidence and cannot advance an indexer checkpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalizedSignatureInfo {
    pub signature: String,
    pub slot: u64,
    pub succeeded: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalizedSignaturePage {
    pub address: String,
    pub before: Option<String>,
    pub entries: Vec<FinalizedSignatureInfo>,
    pub next_before: Option<String>,
    pub complete: bool,
}

#[async_trait]
pub trait FinalizedSignaturePageSource: Send + Sync {
    async fn fetch_finalized_signature_page(
        &self,
        address: &str,
        before: Option<&str>,
        limit: usize,
    ) -> Result<FinalizedSignaturePage, ProviderError>;
}

/// Provides one bounded, finalized signature page for targeted recovery.
/// Callers own the page budget and must fetch full finalized evidence before
/// mutating any projection.
pub struct SolanaRpcSignaturePageSource<T> {
    transport: T,
}

impl<T> SolanaRpcSignaturePageSource<T> {
    pub const fn new(transport: T) -> Self {
        Self { transport }
    }
}

impl<T: JsonRpcTransport> SolanaRpcSignaturePageSource<T> {
    pub async fn fetch_page(
        &self,
        address: &str,
        before: Option<&str>,
        limit: usize,
    ) -> Result<FinalizedSignaturePage, ProviderError> {
        let address = parse_signature_page_address(address)?;
        if !(1..=MAX_SIGNATURE_PAGE_SIZE).contains(&limit) {
            return Err(ProviderError::InvalidResponse(
                "signature page limit must be between 1 and 1000".to_string(),
            ));
        }
        let before = before.map(parse_signature_cursor).transpose()?;
        let mut options = serde_json::Map::from_iter([
            (
                "commitment".to_string(),
                Value::String("finalized".to_string()),
            ),
            ("limit".to_string(), Value::from(limit as u64)),
        ]);
        if let Some(before) = before.as_deref() {
            options.insert("before".to_string(), Value::String(before.to_string()));
        }
        let value = self
            .transport
            .call(
                "getSignaturesForAddress",
                json!([address, Value::Object(options)]),
            )
            .await?;
        let encoded = serde_json::to_vec(&value)
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        if encoded.len() > MAX_RPC_RESPONSE_BYTES {
            return Err(ProviderError::ResponseTooLarge);
        }
        let values = value.as_array().ok_or_else(|| {
            ProviderError::InvalidResponse(
                "getSignaturesForAddress result must be an array".to_string(),
            )
        })?;
        if values.len() > limit {
            return Err(ProviderError::InvalidResponse(
                "signature page contains more entries than requested".to_string(),
            ));
        }
        let mut entries = Vec::with_capacity(values.len());
        let mut seen = std::collections::HashSet::with_capacity(values.len());
        for value in values {
            let object = value.as_object().ok_or_else(|| {
                ProviderError::InvalidResponse("signature entry must be an object".to_string())
            })?;
            let signature = parse_signature_cursor(
                object
                    .get("signature")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ProviderError::InvalidResponse(
                            "signature entry is missing signature".to_string(),
                        )
                    })?,
            )?;
            if !seen.insert(signature.clone()) {
                return Err(ProviderError::InvalidResponse(
                    "signature page contains duplicate signatures".to_string(),
                ));
            }
            let slot = object.get("slot").and_then(Value::as_u64).ok_or_else(|| {
                ProviderError::InvalidResponse("signature entry slot must be a u64".to_string())
            })?;
            if object.get("confirmationStatus").and_then(Value::as_str) != Some("finalized") {
                return Err(ProviderError::NonFinalized);
            }
            let succeeded = object
                .get("err")
                .ok_or_else(|| {
                    ProviderError::InvalidResponse("signature entry err is missing".to_string())
                })?
                .is_null();
            entries.push(FinalizedSignatureInfo {
                signature,
                slot,
                succeeded,
            });
        }
        let next_before = if values.len() == limit {
            entries.last().map(|entry| entry.signature.clone())
        } else {
            None
        };
        if next_before.is_some() && next_before.as_deref() == before.as_deref() {
            return Err(ProviderError::InvalidResponse(
                "signature page cursor did not make progress".to_string(),
            ));
        }
        Ok(FinalizedSignaturePage {
            address,
            before,
            entries,
            next_before,
            complete: values.len() < limit,
        })
    }
}

#[async_trait]
impl<T: JsonRpcTransport> FinalizedSignaturePageSource for SolanaRpcSignaturePageSource<T> {
    async fn fetch_finalized_signature_page(
        &self,
        address: &str,
        before: Option<&str>,
        limit: usize,
    ) -> Result<FinalizedSignaturePage, ProviderError> {
        self.fetch_page(address, before, limit).await
    }
}

#[derive(Clone)]
pub struct FailoverSignaturePageSource {
    providers: Vec<SignaturePageProviderSlot>,
    attempt_timeout: Duration,
}

#[derive(Clone)]
struct SignaturePageProviderSlot {
    source: Arc<dyn FinalizedSignaturePageSource>,
    health: AdaptiveProviderHealth,
}

impl FailoverSignaturePageSource {
    pub fn new(
        providers: Vec<Arc<dyn FinalizedSignaturePageSource>>,
        attempt_timeout: Duration,
    ) -> Result<Self, ProviderError> {
        if providers.is_empty() || providers.len() > MAX_PROVIDER_ATTEMPTS {
            return Err(ProviderError::InvalidResponse(
                "signature provider pool must contain 1 to 3 endpoints".to_string(),
            ));
        }
        if !(Duration::from_millis(50)..=Duration::from_secs(10)).contains(&attempt_timeout) {
            return Err(ProviderError::InvalidResponse(
                "signature provider timeout must be between 50ms and 10s".to_string(),
            ));
        }
        Ok(Self {
            providers: providers
                .into_iter()
                .map(|source| SignaturePageProviderSlot {
                    source,
                    health: AdaptiveProviderHealth::new(),
                })
                .collect(),
            attempt_timeout,
        })
    }

    fn provider_order(&self) -> Vec<usize> {
        let mut available = self
            .providers
            .iter()
            .enumerate()
            .filter(|(_, provider)| provider.health.is_available())
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if available.is_empty()
            && let Some((index, _)) = self
                .providers
                .iter()
                .enumerate()
                .min_by_key(|(_, provider)| provider.health.cooldown_remaining())
        {
            available.push(index);
        }
        available.sort_by_key(|index| {
            (
                std::cmp::Reverse(self.providers[*index].health.score()),
                *index,
            )
        });
        available
    }
}

#[async_trait]
impl FinalizedSignaturePageSource for FailoverSignaturePageSource {
    async fn fetch_finalized_signature_page(
        &self,
        address: &str,
        before: Option<&str>,
        limit: usize,
    ) -> Result<FinalizedSignaturePage, ProviderError> {
        let mut last_error = None;
        for provider_index in self.provider_order() {
            let provider = &self.providers[provider_index];
            match tokio::time::timeout(
                self.attempt_timeout,
                provider
                    .source
                    .fetch_finalized_signature_page(address, before, limit),
            )
            .await
            {
                Ok(Ok(page)) => {
                    provider.health.record_success();
                    return Ok(page);
                }
                Ok(Err(error)) => {
                    provider.health.record_failure();
                    last_error = Some(error);
                }
                Err(_) => {
                    provider.health.record_failure();
                    last_error = Some(ProviderError::Transport(
                        "signature provider attempt deadline exceeded".to_string(),
                    ));
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            ProviderError::Transport("signature provider pool returned no result".to_string())
        }))
    }
}

#[async_trait]
pub trait FinalizedTransactionSource: Send + Sync {
    async fn fetch_finalized_transaction(
        &self,
        signature: &str,
    ) -> Result<DecodedBlock, ProviderError>;
}

pub struct SolanaRpcTransactionSource<T, D> {
    transport: T,
    decoder: D,
}

impl<T, D> SolanaRpcTransactionSource<T, D> {
    pub const fn new(transport: T, decoder: D) -> Self {
        Self { transport, decoder }
    }
}

#[async_trait]
impl<T, D> FinalizedTransactionSource for SolanaRpcTransactionSource<T, D>
where
    T: JsonRpcTransport,
    D: ReceiptDecoder,
{
    async fn fetch_finalized_transaction(
        &self,
        signature: &str,
    ) -> Result<DecodedBlock, ProviderError> {
        let signature = parse_signature_cursor(signature)?;
        let value = self
            .transport
            .call(
                "getTransaction",
                json!([
                    signature,
                    {
                        "commitment": "finalized",
                        "encoding": "json",
                        "maxSupportedTransactionVersion": 0
                    }
                ]),
            )
            .await?;
        let encoded = serde_json::to_vec(&value)
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        if encoded.len() > MAX_RPC_RESPONSE_BYTES {
            return Err(ProviderError::ResponseTooLarge);
        }
        let transaction = value.as_object().ok_or_else(|| {
            ProviderError::InvalidResponse(
                "finalized getTransaction result must be an object".to_string(),
            )
        })?;
        let slot = transaction
            .get("slot")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                ProviderError::InvalidResponse(
                    "finalized transaction slot must be a u64".to_string(),
                )
            })?;
        let primary_signature = transaction
            .get("transaction")
            .and_then(|value| value.get("signatures"))
            .and_then(Value::as_array)
            .and_then(|signatures| signatures.first())
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProviderError::InvalidResponse(
                    "finalized transaction primary signature is missing".to_string(),
                )
            })?;
        if primary_signature != signature {
            return Err(ProviderError::InvalidResponse(
                "finalized transaction signature does not match the requested target".to_string(),
            ));
        }
        self.decoder
            .decode_block(slot, json!({"transactions": [value]}))
            .await
    }
}

#[derive(Clone)]
pub struct FailoverTransactionSource {
    providers: Vec<TransactionProviderSlot>,
    attempt_timeout: Duration,
}

#[derive(Clone)]
struct TransactionProviderSlot {
    source: Arc<dyn FinalizedTransactionSource>,
    health: AdaptiveProviderHealth,
}

impl FailoverTransactionSource {
    pub fn new(
        providers: Vec<Arc<dyn FinalizedTransactionSource>>,
        attempt_timeout: Duration,
    ) -> Result<Self, ProviderError> {
        if providers.is_empty() || providers.len() > MAX_PROVIDER_ATTEMPTS {
            return Err(ProviderError::InvalidResponse(
                "transaction provider pool must contain 1 to 3 endpoints".to_string(),
            ));
        }
        if !(Duration::from_millis(50)..=Duration::from_secs(10)).contains(&attempt_timeout) {
            return Err(ProviderError::InvalidResponse(
                "transaction provider timeout must be between 50ms and 10s".to_string(),
            ));
        }
        Ok(Self {
            providers: providers
                .into_iter()
                .map(|source| TransactionProviderSlot {
                    source,
                    health: AdaptiveProviderHealth::new(),
                })
                .collect(),
            attempt_timeout,
        })
    }

    fn provider_order(&self) -> Vec<usize> {
        let mut available = self
            .providers
            .iter()
            .enumerate()
            .filter(|(_, provider)| provider.health.is_available())
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if available.is_empty()
            && let Some((index, _)) = self
                .providers
                .iter()
                .enumerate()
                .min_by_key(|(_, provider)| provider.health.cooldown_remaining())
        {
            available.push(index);
        }
        available.sort_by_key(|index| {
            (
                std::cmp::Reverse(self.providers[*index].health.score()),
                *index,
            )
        });
        available
    }
}

#[async_trait]
impl FinalizedTransactionSource for FailoverTransactionSource {
    async fn fetch_finalized_transaction(
        &self,
        signature: &str,
    ) -> Result<DecodedBlock, ProviderError> {
        let mut last_error = None;
        for provider_index in self.provider_order() {
            let provider = &self.providers[provider_index];
            match tokio::time::timeout(
                self.attempt_timeout,
                provider.source.fetch_finalized_transaction(signature),
            )
            .await
            {
                Ok(Ok(block)) => {
                    provider.health.record_success();
                    return Ok(block);
                }
                Ok(Err(error)) => {
                    provider.health.record_failure();
                    last_error = Some(error);
                }
                Err(_) => {
                    provider.health.record_failure();
                    last_error = Some(ProviderError::Transport(
                        "transaction provider attempt deadline exceeded".to_string(),
                    ));
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            ProviderError::Transport("transaction provider pool returned no result".to_string())
        }))
    }
}

fn parse_signature_page_address(value: &str) -> Result<String, ProviderError> {
    let address = Pubkey::from_str(value.trim()).map_err(|_| {
        ProviderError::InvalidResponse(
            "signature page address is not a valid public key".to_string(),
        )
    })?;
    Ok(address.to_string())
}

fn parse_signature_cursor(value: &str) -> Result<String, ProviderError> {
    let signature = Signature::from_str(value.trim()).map_err(|_| {
        ProviderError::InvalidResponse(
            "signature cursor is not a valid base58 signature".to_string(),
        )
    })?;
    let canonical = signature.to_string();
    if canonical != value.trim() {
        return Err(ProviderError::InvalidResponse(
            "signature cursor must use canonical base58".to_string(),
        ));
    }
    Ok(canonical)
}

/// Validates and canonicalizes a Solana transaction signature at a control
/// boundary. The returned value is safe to use as the sole signature cursor.
pub fn canonical_signature(value: &str) -> Result<String, ProviderError> {
    parse_signature_cursor(value)
}

/// Public account configuration supplied by the tenant route registry. The
/// verifier accepts this as data only; it never treats it as proof of chain
/// state until [`verify_settlement_route`] completes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SettlementRouteConfig {
    pub route_id: uuid::Uuid,
    pub version: i32,
    pub merchant_account: String,
    pub mint: String,
    pub tax_vault: String,
    pub net_destination: String,
    pub token_program: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct VerifiedRouteAccount {
    pub address: String,
    pub owner: String,
    pub executable: bool,
    pub lamports: u64,
    pub data_len: usize,
    pub data_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SettlementRouteVerification {
    pub route_id: uuid::Uuid,
    pub route_version: i32,
    pub merchant_account: String,
    pub mint: String,
    pub tax_vault: String,
    pub net_destination: String,
    pub token_program: String,
    pub asset_config: String,
    pub asset_kind: u8,
    pub asset_enabled: bool,
    pub asset_config_bump: u8,
    pub verification_schema: &'static str,
    pub verification_slot: u64,
    pub verification_commitment: &'static str,
    pub mint_decimals: u8,
    pub accounts: Vec<VerifiedRouteAccount>,
    pub verification_digest: [u8; 32],
}

/// Server-owned destination topology for a revenue-stream payout. This is
/// configuration only until [`verify_revenue_stream_destination_route`]
/// produces finalized account evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RevenueStreamDestinationRouteConfig {
    pub route_id: uuid::Uuid,
    pub version: i32,
    pub recipient_owner: String,
    pub mint: String,
    pub destination_token_account: String,
    pub token_program: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RevenueStreamDestinationRouteVerification {
    pub route_id: uuid::Uuid,
    pub route_version: i32,
    pub recipient_owner: String,
    pub mint: String,
    pub destination_token_account: String,
    pub token_program: String,
    pub verification_slot: u64,
    pub verification_commitment: &'static str,
    pub mint_decimals: u8,
    pub accounts: Vec<VerifiedRouteAccount>,
    pub verification_digest: [u8; 32],
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum RevenueStreamDestinationVerificationError {
    #[error("invalid revenue stream destination route: {0}")]
    InvalidRoute(String),
    #[error("revenue stream destination verification provider failed: {0}")]
    Provider(#[from] ProviderError),
    #[error("revenue stream destination verification response is invalid: {0}")]
    InvalidResponse(String),
    #[error("revenue stream destination account is missing: {0}")]
    MissingAccount(String),
    #[error("revenue stream destination account owner does not match the token program: {0}")]
    AccountOwnerMismatch(&'static str),
    #[error("revenue stream destination account has an unsupported layout: {0}")]
    UnsupportedAccountLayout(&'static str),
    #[error("revenue stream destination token account is bound to a different mint")]
    TokenAccountMintMismatch,
    #[error("revenue stream destination token authority does not match the recipient owner")]
    TokenAuthorityMismatch,
}

/// Server-owned funding topology supplied by the approved configuration. It
/// is data only until [`verify_refund_funding_topology`] proves the finalized
/// chain accounts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RefundFundingTopologyConfig {
    pub configuration_id: uuid::Uuid,
    pub version: i32,
    pub configuration_digest: [u8; 32],
    pub merchant_account: String,
    pub mint: String,
    pub tax_vault: String,
    pub vault_authority: String,
    pub refund_funder: String,
    pub net_source: String,
    pub token_program: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RefundFundingTopologyVerification {
    pub configuration_id: uuid::Uuid,
    pub configuration_version: i32,
    pub configuration_digest: [u8; 32],
    pub merchant_account: String,
    pub mint: String,
    pub tax_vault: String,
    pub vault_authority: String,
    pub refund_funder: String,
    pub net_source: String,
    pub token_program: String,
    pub verification_slot: u64,
    pub verification_commitment: &'static str,
    pub mint_decimals: u8,
    pub accounts: Vec<VerifiedRouteAccount>,
    pub verification_digest: [u8; 32],
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum RefundFundingVerificationError {
    #[error("invalid refund funding topology: {0}")]
    InvalidTopology(String),
    #[error("refund funding verification provider failed: {0}")]
    Provider(#[from] ProviderError),
    #[error("refund funding verification response is invalid: {0}")]
    InvalidResponse(String),
    #[error("refund funding account is missing: {0}")]
    MissingAccount(String),
    #[error("refund funding account owner does not match the required program: {0}")]
    AccountOwnerMismatch(&'static str),
    #[error("refund funding account has an unsupported layout: {0}")]
    UnsupportedAccountLayout(&'static str),
    #[error("refund funding token account is bound to a different mint: {0}")]
    TokenAccountMintMismatch(&'static str),
    #[error("refund funding token authority does not match the configured authority: {0}")]
    TokenAuthorityMismatch(&'static str),
    #[error("configured vault authority does not match the deterministic PDA")]
    VaultAuthorityMismatch,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum RouteVerificationError {
    #[error("invalid settlement route: {0}")]
    InvalidRoute(String),
    #[error("route verification provider failed: {0}")]
    Provider(#[from] ProviderError),
    #[error("route verification response is invalid: {0}")]
    InvalidResponse(String),
    #[error("route account is missing: {0}")]
    MissingAccount(String),
    #[error("route account owner does not match the required program: {0}")]
    AccountOwnerMismatch(&'static str),
    #[error("route account has an unsupported layout: {0}")]
    UnsupportedAccountLayout(&'static str),
    #[error("route token account is bound to a different mint: {0}")]
    TokenAccountMintMismatch(&'static str),
    #[error("tax-vault token authority does not match the deterministic vault PDA")]
    VaultAuthorityMismatch,
    #[error("asset admission configuration is disabled")]
    AssetConfigDisabled,
    #[error("asset admission configuration is not a stablecoin")]
    AssetConfigNotStablecoin,
    #[error("asset admission configuration does not match the route: {0}")]
    AssetConfigMismatch(&'static str),
}

#[derive(Clone, Debug)]
struct RouteAccountSnapshot {
    address: String,
    owner: String,
    executable: bool,
    lamports: u64,
    data: Vec<u8>,
}

/// Verifies the four public accounts required by a settlement route at a
/// finalized commitment. This is deliberately a separate provisioning
/// operation; quote and settlement compilation must not call it.
pub async fn verify_settlement_route<T: JsonRpcTransport + ?Sized>(
    transport: &T,
    route: &SettlementRouteConfig,
) -> Result<SettlementRouteVerification, RouteVerificationError> {
    let keys = validate_route_config(route)?;
    let asset_config_address = keys.asset_config.to_string();
    let addresses = [
        route.merchant_account.as_str(),
        route.mint.as_str(),
        route.tax_vault.as_str(),
        route.net_destination.as_str(),
        asset_config_address.as_str(),
    ];
    let response = transport
        .call(
            "getMultipleAccounts",
            json!([
                addresses,
                { "commitment": "finalized", "encoding": "base64" }
            ]),
        )
        .await?;
    let encoded_response = serde_json::to_vec(&response)
        .map_err(|error| RouteVerificationError::InvalidResponse(error.to_string()))?;
    if encoded_response.len() > MAX_ROUTE_RPC_RESULT_BYTES {
        return Err(RouteVerificationError::InvalidResponse(
            "getMultipleAccounts response exceeds the bounded route limit".to_string(),
        ));
    }
    let context = response
        .get("context")
        .and_then(Value::as_object)
        .ok_or_else(|| RouteVerificationError::InvalidResponse("context is missing".to_string()))?;
    let slot = context.get("slot").and_then(Value::as_u64).ok_or_else(|| {
        RouteVerificationError::InvalidResponse("context.slot is invalid".to_string())
    })?;
    let values = response
        .get("value")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            RouteVerificationError::InvalidResponse("value is not an array".to_string())
        })?;
    if values.len() != addresses.len() {
        return Err(RouteVerificationError::InvalidResponse(
            "getMultipleAccounts returned an unexpected account count".to_string(),
        ));
    }
    let snapshots = values
        .iter()
        .zip(addresses)
        .map(|(value, address)| parse_route_account(value, address))
        .collect::<Result<Vec<_>, _>>()?;

    let merchant = &snapshots[0];
    if merchant.executable
        || merchant.owner != settlement_client::SYSTEM_PROGRAM_ID
        || !merchant.data.is_empty()
    {
        return Err(RouteVerificationError::AccountOwnerMismatch(
            "merchant_account",
        ));
    }

    let mint = &snapshots[1];
    verify_token_account_container(mint, &keys.token_program, "mint", 82)?;
    let mint_decimals = *mint
        .data
        .get(44)
        .ok_or(RouteVerificationError::UnsupportedAccountLayout("mint"))?;

    let tax_vault = &snapshots[2];
    verify_token_account_container(tax_vault, &keys.token_program, "tax_vault", 165)?;
    verify_token_mint(tax_vault, &keys.mint, "tax_vault")?;
    let tax_owner = read_pubkey_field(tax_vault, 32, "tax_vault")?;
    if tax_owner != keys.vault_authority {
        return Err(RouteVerificationError::VaultAuthorityMismatch);
    }

    let destination = &snapshots[3];
    verify_token_account_container(destination, &keys.token_program, "net_destination", 165)?;
    verify_token_mint(destination, &keys.mint, "net_destination")?;

    let asset_config = &snapshots[4];
    let asset_config_fields = verify_asset_config(asset_config, &keys, mint_decimals)?;

    let accounts = snapshots
        .into_iter()
        .map(route_account_evidence)
        .collect::<Vec<_>>();
    let verification_digest =
        route_verification_digest(route, &keys.asset_config, slot, mint_decimals, &accounts);
    Ok(SettlementRouteVerification {
        route_id: route.route_id,
        route_version: route.version,
        merchant_account: route.merchant_account.clone(),
        mint: route.mint.clone(),
        tax_vault: route.tax_vault.clone(),
        net_destination: route.net_destination.clone(),
        token_program: route.token_program.clone(),
        asset_config: keys.asset_config.to_string(),
        asset_kind: asset_config_fields.asset_kind,
        asset_enabled: asset_config_fields.enabled,
        asset_config_bump: asset_config_fields.bump,
        verification_schema: SETTLEMENT_ROUTE_VERIFICATION_SCHEMA,
        verification_slot: slot,
        verification_commitment: "finalized",
        mint_decimals,
        accounts,
        verification_digest,
    })
}

/// Verifies the mint and recipient token account for a revenue payout route at
/// one finalized commitment. It deliberately accepts only extension-free
/// legacy SPL or Token-2022 account layouts until extension semantics have a
/// separately audited compatibility contract.
pub async fn verify_revenue_stream_destination_route<T: JsonRpcTransport + ?Sized>(
    transport: &T,
    route: &RevenueStreamDestinationRouteConfig,
) -> Result<RevenueStreamDestinationRouteVerification, RevenueStreamDestinationVerificationError> {
    if route.route_id.is_nil() || route.version <= 0 {
        return Err(RevenueStreamDestinationVerificationError::InvalidRoute(
            "route identity and positive version are required".to_string(),
        ));
    }
    let recipient_owner =
        parse_destination_route_pubkey("recipient_owner", &route.recipient_owner)?;
    let mint = parse_destination_route_pubkey("mint", &route.mint)?;
    let destination = parse_destination_route_pubkey(
        "destination_token_account",
        &route.destination_token_account,
    )?;
    let token_program = parse_destination_route_pubkey("token_program", &route.token_program)?;
    let spl_token =
        parse_destination_route_pubkey("token_program", settlement_client::SPL_TOKEN_PROGRAM_ID)?;
    let token_2022 =
        parse_destination_route_pubkey("token_program", settlement_client::TOKEN_2022_PROGRAM_ID)?;
    if token_program != spl_token && token_program != token_2022 {
        return Err(RevenueStreamDestinationVerificationError::InvalidRoute(
            "token program is not supported".to_string(),
        ));
    }
    if mint == destination || recipient_owner == destination {
        return Err(RevenueStreamDestinationVerificationError::InvalidRoute(
            "destination route addresses must be distinct".to_string(),
        ));
    }
    let addresses = [
        route.mint.as_str(),
        route.destination_token_account.as_str(),
    ];
    let response = transport
        .call(
            "getMultipleAccounts",
            json!([addresses, { "commitment": "finalized", "encoding": "base64" }]),
        )
        .await?;
    let encoded_response = serde_json::to_vec(&response).map_err(|error| {
        RevenueStreamDestinationVerificationError::InvalidResponse(error.to_string())
    })?;
    if encoded_response.len() > MAX_ROUTE_RPC_RESULT_BYTES {
        return Err(RevenueStreamDestinationVerificationError::InvalidResponse(
            "getMultipleAccounts response exceeds the bounded route limit".to_string(),
        ));
    }
    let context = response
        .get("context")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            RevenueStreamDestinationVerificationError::InvalidResponse(
                "context is missing".to_string(),
            )
        })?;
    let slot = context.get("slot").and_then(Value::as_u64).ok_or_else(|| {
        RevenueStreamDestinationVerificationError::InvalidResponse(
            "context.slot is invalid".to_string(),
        )
    })?;
    let values = response
        .get("value")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            RevenueStreamDestinationVerificationError::InvalidResponse(
                "value is not an array".to_string(),
            )
        })?;
    if values.len() != addresses.len() {
        return Err(RevenueStreamDestinationVerificationError::InvalidResponse(
            "getMultipleAccounts returned an unexpected account count".to_string(),
        ));
    }
    let snapshots = values
        .iter()
        .zip(addresses)
        .map(|(value, address)| {
            parse_route_account(value, address).map_err(map_destination_route_error)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mint_account = &snapshots[0];
    verify_destination_token_account_container(mint_account, &token_program, "mint", 82)?;
    let mint_decimals = *mint_account
        .data
        .get(44)
        .ok_or(RevenueStreamDestinationVerificationError::UnsupportedAccountLayout("mint"))?;
    let destination_account = &snapshots[1];
    verify_destination_token_account_container(
        destination_account,
        &token_program,
        "destination_token_account",
        165,
    )?;
    let destination_mint = read_destination_pubkey_field(destination_account, 0, "destination")?;
    if destination_mint != mint {
        return Err(RevenueStreamDestinationVerificationError::TokenAccountMintMismatch);
    }
    let destination_authority =
        read_destination_pubkey_field(destination_account, 32, "destination")?;
    if destination_authority != recipient_owner {
        return Err(RevenueStreamDestinationVerificationError::TokenAuthorityMismatch);
    }
    let accounts = snapshots
        .into_iter()
        .map(route_account_evidence)
        .collect::<Vec<_>>();
    let verification_digest =
        revenue_destination_verification_digest(route, slot, mint_decimals, &accounts);
    Ok(RevenueStreamDestinationRouteVerification {
        route_id: route.route_id,
        route_version: route.version,
        recipient_owner: route.recipient_owner.clone(),
        mint: route.mint.clone(),
        destination_token_account: route.destination_token_account.clone(),
        token_program: route.token_program.clone(),
        verification_slot: slot,
        verification_commitment: "finalized",
        mint_decimals,
        accounts,
        verification_digest,
    })
}

/// Verifies the complete legacy-SPL refund funding topology at one finalized
/// slot. Token-2022 is deliberately rejected until its extension semantics
/// have a separate compatibility decision and validator evidence.
pub async fn verify_refund_funding_topology<T: JsonRpcTransport + ?Sized>(
    transport: &T,
    topology: &RefundFundingTopologyConfig,
) -> Result<RefundFundingTopologyVerification, RefundFundingVerificationError> {
    let keys = validate_refund_funding_topology(topology)?;
    let addresses = [
        topology.merchant_account.as_str(),
        topology.mint.as_str(),
        topology.tax_vault.as_str(),
        topology.refund_funder.as_str(),
        topology.net_source.as_str(),
    ];
    let response = transport
        .call(
            "getMultipleAccounts",
            json!([
                addresses,
                { "commitment": "finalized", "encoding": "base64" }
            ]),
        )
        .await?;
    let encoded_response = serde_json::to_vec(&response)
        .map_err(|error| RefundFundingVerificationError::InvalidResponse(error.to_string()))?;
    if encoded_response.len() > MAX_ROUTE_RPC_RESULT_BYTES {
        return Err(RefundFundingVerificationError::InvalidResponse(
            "getMultipleAccounts response exceeds the bounded refund funding limit".to_string(),
        ));
    }
    let slot = response
        .get("context")
        .and_then(|value| value.get("slot"))
        .and_then(Value::as_u64)
        .filter(|slot| *slot > 0)
        .ok_or_else(|| {
            RefundFundingVerificationError::InvalidResponse(
                "finalized verification slot is invalid".to_string(),
            )
        })?;
    let values = response
        .get("value")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            RefundFundingVerificationError::InvalidResponse(
                "refund funding account values are not an array".to_string(),
            )
        })?;
    if values.len() != addresses.len() {
        return Err(RefundFundingVerificationError::InvalidResponse(
            "refund funding response contains an unexpected account count".to_string(),
        ));
    }
    let snapshots = values
        .iter()
        .zip(addresses)
        .map(|(value, address)| {
            parse_route_account_bounded(value, address, MAX_ROUTE_ACCOUNT_DATA_BYTES).map_err(
                |error| match error {
                    RouteVerificationError::MissingAccount(address) => {
                        RefundFundingVerificationError::MissingAccount(address)
                    }
                    RouteVerificationError::InvalidResponse(message) => {
                        RefundFundingVerificationError::InvalidResponse(message)
                    }
                    _ => RefundFundingVerificationError::InvalidResponse(error.to_string()),
                },
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    let merchant = &snapshots[0];
    verify_system_account(merchant, "merchant_account")?;
    let mint = &snapshots[1];
    verify_token_container(mint, &keys.token_program, "mint", 82)?;
    let mint_decimals = mint.data.get(44).copied().ok_or(
        RefundFundingVerificationError::UnsupportedAccountLayout("mint"),
    )?;
    let tax_vault = &snapshots[2];
    verify_token_container(tax_vault, &keys.token_program, "tax_vault", 165)?;
    verify_token_mint_for_refund(tax_vault, &keys.mint, "tax_vault")?;
    verify_token_authority_for_refund(tax_vault, &keys.vault_authority, "tax_vault")?;
    let refund_funder = &snapshots[3];
    verify_system_account(refund_funder, "refund_funder")?;
    let net_source = &snapshots[4];
    verify_token_container(net_source, &keys.token_program, "net_source", 165)?;
    verify_token_mint_for_refund(net_source, &keys.mint, "net_source")?;
    verify_token_authority_for_refund(net_source, &keys.refund_funder, "net_source")?;

    let accounts = snapshots
        .into_iter()
        .map(route_account_evidence)
        .collect::<Vec<_>>();
    let verification_digest =
        refund_funding_verification_digest(topology, slot, mint_decimals, &accounts);
    Ok(RefundFundingTopologyVerification {
        configuration_id: topology.configuration_id,
        configuration_version: topology.version,
        configuration_digest: topology.configuration_digest,
        merchant_account: topology.merchant_account.clone(),
        mint: topology.mint.clone(),
        tax_vault: topology.tax_vault.clone(),
        vault_authority: topology.vault_authority.clone(),
        refund_funder: topology.refund_funder.clone(),
        net_source: topology.net_source.clone(),
        token_program: topology.token_program.clone(),
        verification_slot: slot,
        verification_commitment: "finalized",
        mint_decimals,
        accounts,
        verification_digest,
    })
}

/// Finalized token-account reader for the reconciliation boundary. It proves
/// the account container before reading the canonical SPL amount field. The
/// base 165-byte layout is intentional: extension-bearing Token-2022 accounts
/// fail closed until their semantics have a dedicated compatibility decision.
pub struct SolanaRpcVaultBalanceSource<T> {
    transport: T,
}

impl<T> SolanaRpcVaultBalanceSource<T> {
    pub const fn new(transport: T) -> Self {
        Self { transport }
    }
}

#[async_trait]
impl<T: JsonRpcTransport> FinalizedVaultBalanceSource for SolanaRpcVaultBalanceSource<T> {
    async fn fetch_finalized_vault_balance(
        &self,
        target: &VaultBalanceTarget,
    ) -> Result<FinalizedVaultBalance, ProviderError> {
        let vault = Pubkey::from_str(target.tax_vault.trim()).map_err(|_| {
            ProviderError::InvalidResponse("vault address is not a valid public key".to_string())
        })?;
        let mint = Pubkey::from_str(target.mint.trim()).map_err(|_| {
            ProviderError::InvalidResponse("vault mint is not a valid public key".to_string())
        })?;
        let token_program = Pubkey::from_str(target.token_program.trim()).map_err(|_| {
            ProviderError::InvalidResponse(
                "vault token program is not a valid public key".to_string(),
            )
        })?;
        if target.token_program == settlement_client::TOKEN_2022_PROGRAM_ID {
            return Err(ProviderError::InvalidResponse(
                "Token-2022 vault reconciliation requires a dedicated extension contract"
                    .to_string(),
            ));
        }
        if target.token_program != settlement_client::SPL_TOKEN_PROGRAM_ID {
            return Err(ProviderError::InvalidResponse(
                "vault reconciliation token program is not supported".to_string(),
            ));
        }
        let response = self
            .transport
            .call(
                "getMultipleAccounts",
                json!([[vault.to_string()], {"commitment": "finalized", "encoding": "base64"}]),
            )
            .await?;
        let encoded = serde_json::to_vec(&response)
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        if encoded.len() > MAX_ROUTE_RPC_RESULT_BYTES {
            return Err(ProviderError::ResponseTooLarge);
        }
        let slot = response
            .get("context")
            .and_then(|value| value.get("slot"))
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                ProviderError::InvalidResponse("finalized slot is invalid".to_string())
            })?;
        let values = response
            .get("value")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ProviderError::InvalidResponse("account value is not an array".to_string())
            })?;
        if values.len() != 1 {
            return Err(ProviderError::InvalidResponse(
                "vault balance response contains an unexpected account count".to_string(),
            ));
        }
        let account = parse_route_account(&values[0], target.tax_vault.trim())
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        if account.executable || account.owner != token_program.to_string() {
            return Err(ProviderError::InvalidResponse(
                "vault account owner or executable flag is invalid".to_string(),
            ));
        }
        if account.data.len() != 165 {
            return Err(ProviderError::InvalidResponse(
                "vault account uses an unsupported token layout".to_string(),
            ));
        }
        let actual_mint = read_pubkey_field(&account, 0, "tax_vault")
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        if actual_mint != mint {
            return Err(ProviderError::InvalidResponse(
                "vault account mint does not match the target".to_string(),
            ));
        }
        let amount_bytes = account.data.get(64..72).ok_or_else(|| {
            ProviderError::InvalidResponse("vault amount field is missing".to_string())
        })?;
        let amount_atomic = u64::from_le_bytes(amount_bytes.try_into().map_err(|_| {
            ProviderError::InvalidResponse("vault amount field is malformed".to_string())
        })?);
        Ok(FinalizedVaultBalance {
            slot,
            amount_atomic,
        })
    }
}

/// Finalized multi-account reader for the Refund balance-proof boundary.
/// Keeping the three accounts in one RPC call guarantees that the returned
/// balances share one provider context slot before the pure domain proof runs.
pub struct SolanaRpcRefundBalanceSource<T> {
    transport: T,
}

impl<T> SolanaRpcRefundBalanceSource<T> {
    pub const fn new(transport: T) -> Self {
        Self { transport }
    }
}

#[async_trait]
impl<T: JsonRpcTransport> FinalizedRefundBalanceSource for SolanaRpcRefundBalanceSource<T> {
    async fn fetch_finalized_refund_balances(
        &self,
        target: &RefundBalanceTarget,
    ) -> Result<FinalizedRefundBalances, ProviderError> {
        let claim = parse_refund_pubkey(&target.claim, "refund claim")?;
        if target.claim_nonce == 0 {
            return Err(ProviderError::InvalidResponse(
                "refund claim nonce must be positive".to_string(),
            ));
        }
        let tax_vault = parse_refund_pubkey(&target.tax_vault, "tax vault")?;
        let refund_source = parse_refund_pubkey(&target.refund_source, "refund source")?;
        let payer_destination =
            parse_refund_pubkey(&target.payer_destination, "payer destination")?;
        let mint = parse_refund_pubkey(&target.mint, "refund mint")?;
        let token_program = parse_refund_pubkey(&target.token_program, "token program")?;
        let vault_authority = parse_refund_pubkey(&target.vault_authority, "vault authority")?;
        let refund_funder = parse_refund_pubkey(&target.refund_funder, "refund funder")?;
        let payer = parse_refund_pubkey(&target.payer, "payer")?;
        if token_program
            != Pubkey::from_str(settlement_client::SPL_TOKEN_PROGRAM_ID).map_err(|_| {
                ProviderError::InvalidResponse("canonical token program is invalid".to_string())
            })?
        {
            return Err(ProviderError::InvalidResponse(
                "refund balance proof supports only legacy SPL Token accounts".to_string(),
            ));
        }
        if tax_vault == refund_source
            || tax_vault == payer_destination
            || refund_source == payer_destination
        {
            return Err(ProviderError::InvalidResponse(
                "refund balance accounts must be pairwise distinct".to_string(),
            ));
        }
        let addresses = [
            claim.to_string(),
            tax_vault.to_string(),
            refund_source.to_string(),
            payer_destination.to_string(),
        ];
        let response = self
            .transport
            .call(
                "getMultipleAccounts",
                json!([addresses, { "commitment": "finalized", "encoding": "base64" }]),
            )
            .await?;
        let encoded = serde_json::to_vec(&response)
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        if encoded.len() > MAX_ROUTE_RPC_RESULT_BYTES {
            return Err(ProviderError::ResponseTooLarge);
        }
        let slot = response
            .get("context")
            .and_then(|value| value.get("slot"))
            .and_then(Value::as_u64)
            .filter(|slot| *slot > 0)
            .ok_or_else(|| {
                ProviderError::InvalidResponse(
                    "refund balance response finalized slot is invalid".to_string(),
                )
            })?;
        let values = response
            .get("value")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ProviderError::InvalidResponse("refund balance values are not an array".to_string())
            })?;
        if values.len() != addresses.len() {
            return Err(ProviderError::InvalidResponse(
                "refund balance response contains an unexpected account count".to_string(),
            ));
        }
        let snapshots = values
            .iter()
            .zip(addresses.iter())
            .map(|(value, address)| {
                parse_route_account_bounded(value, address, MAX_REFUND_CLAIM_ACCOUNT_DATA_BYTES)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        let claim_binding = RefundClaimBinding {
            claim,
            adjustment: parse_refund_pubkey(&target.adjustment, "adjustment")?,
            original_receipt: parse_refund_pubkey(&target.original_receipt, "original receipt")?,
            merchant: parse_refund_pubkey(&target.merchant, "merchant")?,
            mint,
            payer,
            refund_funder,
            tax_vault,
            refund_source,
            payer_destination,
            claim_nonce: target.claim_nonce,
        };
        verify_refund_claim_account(&snapshots[0], &claim_binding)?;
        let tax_vault = verify_refund_token_account(
            &snapshots[1],
            &token_program,
            &mint,
            &vault_authority,
            "tax_vault",
        )?;
        let refund_source = verify_refund_token_account(
            &snapshots[2],
            &token_program,
            &mint,
            &refund_funder,
            "refund_source",
        )?;
        let payer_destination = verify_refund_token_account(
            &snapshots[3],
            &token_program,
            &mint,
            &payer,
            "payer_destination",
        )?;
        Ok(FinalizedRefundBalances {
            slot,
            tax_vault_atomic: token_amount(tax_vault, "tax_vault")?,
            refund_source_atomic: token_amount(refund_source, "refund_source")?,
            payer_destination_atomic: token_amount(payer_destination, "payer_destination")?,
        })
    }
}

/// Reads transaction-bound token deltas. Unlike the diagnostic post-state
/// reader above, this source obtains both sides from one finalized
/// `getTransaction` response and therefore preserves the transaction slot as
/// the proof boundary.
pub struct SolanaRpcRefundBalanceTransitionSource<T> {
    transport: T,
    verify_claim: bool,
}

impl<T> SolanaRpcRefundBalanceTransitionSource<T> {
    pub const fn new(transport: T) -> Self {
        Self {
            transport,
            verify_claim: true,
        }
    }

    const fn without_claim_validation(transport: T) -> Self {
        Self {
            transport,
            verify_claim: false,
        }
    }
}

#[async_trait]
impl<T: JsonRpcTransport> FinalizedRefundBalanceTransitionSource
    for SolanaRpcRefundBalanceTransitionSource<T>
{
    async fn fetch_finalized_refund_balance_transition(
        &self,
        target: &RefundBalanceProofTarget,
    ) -> Result<FinalizedRefundBalanceTransition, ProviderError> {
        let signature = canonical_signature(&target.signature)?;
        validate_refund_proof_target(target)?;
        let value = self
            .transport
            .call(
                "getTransaction",
                json!([
                    signature,
                    {
                        "commitment": "finalized",
                        "encoding": "json",
                        "maxSupportedTransactionVersion": 0
                    }
                ]),
            )
            .await?;
        let encoded = serde_json::to_vec(&value)
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        if encoded.len() > MAX_RPC_RESPONSE_BYTES {
            return Err(ProviderError::ResponseTooLarge);
        }
        let transaction = value.as_object().ok_or_else(|| {
            ProviderError::InvalidResponse(
                "refund proof transaction response must be an object".to_string(),
            )
        })?;
        let slot = transaction
            .get("slot")
            .and_then(Value::as_u64)
            .filter(|slot| *slot > 0)
            .ok_or_else(|| {
                ProviderError::InvalidResponse(
                    "refund proof transaction slot is invalid".to_string(),
                )
            })?;
        let primary_signature = transaction
            .get("transaction")
            .and_then(|value| value.get("signatures"))
            .and_then(Value::as_array)
            .and_then(|signatures| signatures.first())
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProviderError::InvalidResponse(
                    "refund proof transaction primary signature is missing".to_string(),
                )
            })?;
        if primary_signature != signature {
            return Err(ProviderError::InvalidResponse(
                "refund proof transaction signature does not match the target".to_string(),
            ));
        }
        let meta = transaction
            .get("meta")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                ProviderError::InvalidResponse(
                    "refund proof transaction metadata is missing".to_string(),
                )
            })?;
        if !meta.get("err").is_some_and(Value::is_null) {
            return Err(ProviderError::InvalidResponse(
                "refund proof transaction did not execute successfully".to_string(),
            ));
        }
        verify_refund_execution_log(transaction, target)?;
        let account_keys = parse_transaction_account_keys(transaction)?;
        let tax_vault_index = unique_account_index(&account_keys, &target.topology.tax_vault)?;
        let refund_source_index =
            unique_account_index(&account_keys, &target.topology.refund_source)?;
        let payer_destination_index =
            unique_account_index(&account_keys, &target.topology.payer_destination)?;
        let token_program =
            canonical_pubkey_string(&target.topology.token_program, "token program")?;
        let mint = canonical_pubkey_string(&target.topology.mint, "refund mint")?;
        let vault_authority =
            canonical_pubkey_string(&target.topology.vault_authority, "vault authority")?;
        let refund_funder =
            canonical_pubkey_string(&target.topology.refund_funder, "refund funder")?;
        let payer = canonical_pubkey_string(&target.topology.payer, "payer")?;
        let tax_before = transaction_token_balance(
            meta,
            "preTokenBalances",
            tax_vault_index,
            &mint,
            &token_program,
            &vault_authority,
        )?;
        let tax_after = transaction_token_balance(
            meta,
            "postTokenBalances",
            tax_vault_index,
            &mint,
            &token_program,
            &vault_authority,
        )?;
        let source_before = transaction_token_balance(
            meta,
            "preTokenBalances",
            refund_source_index,
            &mint,
            &token_program,
            &refund_funder,
        )?;
        let source_after = transaction_token_balance(
            meta,
            "postTokenBalances",
            refund_source_index,
            &mint,
            &token_program,
            &refund_funder,
        )?;
        let destination_before = transaction_token_balance(
            meta,
            "preTokenBalances",
            payer_destination_index,
            &mint,
            &token_program,
            &payer,
        )?;
        let destination_after = transaction_token_balance(
            meta,
            "postTokenBalances",
            payer_destination_index,
            &mint,
            &token_program,
            &payer,
        )?;
        if self.verify_claim {
            let claim_source = SolanaRpcRefundBalanceSource::new(&self.transport);
            claim_source
                .fetch_finalized_refund_balances(&target.topology)
                .await?;
        }
        Ok(FinalizedRefundBalanceTransition {
            slot,
            tax_vault_before_atomic: tax_before,
            tax_vault_after_atomic: tax_after,
            refund_source_before_atomic: source_before,
            refund_source_after_atomic: source_after,
            payer_destination_before_atomic: destination_before,
            payer_destination_after_atomic: destination_after,
        })
    }
}

/// Resolves the omitted refund topology from the finalized claim and then
/// proves the transaction-bound balance transition. The claim read and the
/// transaction read share one transport/provider attempt; the failover layer
/// retries this complete request rather than mixing observations.
pub struct SolanaRpcRefundBalanceProofSource<T> {
    transport: T,
}

impl<T> SolanaRpcRefundBalanceProofSource<T> {
    pub const fn new(transport: T) -> Self {
        Self { transport }
    }
}

#[async_trait]
impl<T: JsonRpcTransport> FinalizedRefundBalanceProofSource
    for SolanaRpcRefundBalanceProofSource<T>
{
    async fn fetch_finalized_refund_balance_proof(
        &self,
        request: &RefundClaimTopologyRequest,
        signature: &str,
    ) -> Result<(RefundBalanceProofTarget, FinalizedRefundBalanceTransition), ProviderError> {
        let claim = Pubkey::new_from_array(request.claim);
        let response = self
            .transport
            .call(
                "getMultipleAccounts",
                json!([[claim.to_string()], { "commitment": "finalized", "encoding": "base64" }]),
            )
            .await?;
        let encoded = serde_json::to_vec(&response)
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        if encoded.len() > MAX_ROUTE_RPC_RESULT_BYTES {
            return Err(ProviderError::ResponseTooLarge);
        }
        let _claim_slot = response
            .get("context")
            .and_then(|value| value.get("slot"))
            .and_then(Value::as_u64)
            .filter(|slot| *slot > 0)
            .ok_or_else(|| {
                ProviderError::InvalidResponse(
                    "refund claim topology finalized slot is invalid".to_string(),
                )
            })?;
        let values = response
            .get("value")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ProviderError::InvalidResponse(
                    "refund claim topology values are not an array".to_string(),
                )
            })?;
        if values.len() != 1 {
            return Err(ProviderError::InvalidResponse(
                "refund claim topology response contains an unexpected account count".to_string(),
            ));
        }
        let account = parse_route_account_bounded(
            &values[0],
            &claim.to_string(),
            MAX_REFUND_CLAIM_ACCOUNT_DATA_BYTES,
        )
        .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        let target = resolve_refund_balance_target(&account, request)?;
        let transition_source =
            SolanaRpcRefundBalanceTransitionSource::without_claim_validation(&self.transport);
        let transition = transition_source
            .fetch_finalized_refund_balance_transition(&RefundBalanceProofTarget {
                signature: canonical_signature(signature)?,
                topology: target.clone(),
                gross_atomic: request.gross_atomic,
                tax_atomic: request.tax_atomic,
                net_atomic: request.net_atomic,
                reference_digest: request.reference_digest,
            })
            .await?;
        Ok((
            RefundBalanceProofTarget {
                signature: canonical_signature(signature)?,
                topology: target,
                gross_atomic: request.gross_atomic,
                tax_atomic: request.tax_atomic,
                net_atomic: request.net_atomic,
                reference_digest: request.reference_digest,
            },
            transition,
        ))
    }
}

fn resolve_refund_balance_target(
    account: &RouteAccountSnapshot,
    request: &RefundClaimTopologyRequest,
) -> Result<RefundBalanceTarget, ProviderError> {
    let expected_claim = Pubkey::find_program_address(
        &[
            b"refund-claim",
            request.adjustment.as_ref(),
            &request.claim_nonce.to_le_bytes(),
        ],
        &settlement_client::settlement_program_id(),
    )
    .0;
    if expected_claim.to_bytes() != request.claim {
        return Err(ProviderError::InvalidResponse(
            "refund claim PDA does not match the indexed execution".to_string(),
        ));
    }
    if account.executable || account.owner != settlement_client::SETTLEMENT_PROGRAM_ID {
        return Err(ProviderError::InvalidResponse(
            "refund claim owner or executable flag is invalid".to_string(),
        ));
    }
    if account.data.len() != REFUND_CLAIM_ACCOUNT_DATA_LEN {
        return Err(ProviderError::InvalidResponse(
            "refund claim uses an unsupported account layout".to_string(),
        ));
    }
    let mut discriminator = Sha256::new();
    discriminator.update(b"account:AdjustmentRefundClaim");
    if account.data.get(..8) != Some(&discriminator.finalize()[..8]) {
        return Err(ProviderError::InvalidResponse(
            "refund claim discriminator is invalid".to_string(),
        ));
    }
    let read_key = |offset: usize, field: &'static str| {
        read_pubkey_field(account, offset, field)
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))
    };
    let adjustment = read_key(8, "adjustment")?;
    let original_receipt = read_key(40, "original receipt")?;
    let merchant = read_key(72, "merchant")?;
    let mint = read_key(104, "mint")?;
    let payer = read_key(136, "payer")?;
    let refund_funder = read_key(168, "refund funder")?;
    let tax_vault = read_key(200, "tax vault")?;
    let refund_source = read_key(232, "refund source")?;
    let refund_destination = read_key(264, "refund destination")?;
    if adjustment.to_bytes() != request.adjustment
        || original_receipt.to_bytes() != request.original_receipt
        || merchant.to_bytes() != request.merchant
        || mint.to_bytes() != request.mint
        || refund_funder.to_bytes() != request.refund_funder
        || refund_destination.to_bytes() != request.refund_destination
    {
        return Err(ProviderError::InvalidResponse(
            "refund claim topology does not match the indexed execution".to_string(),
        ));
    }
    if read_u64_field(account, 296, "refund claim nonce")? != request.claim_nonce
        || read_u64_field(account, 304, "refund claim gross")? != request.gross_atomic
        || read_u64_field(account, 312, "refund claim tax")? != request.tax_atomic
        || read_u64_field(account, 320, "refund claim net")? != request.net_atomic
    {
        return Err(ProviderError::InvalidResponse(
            "refund claim amounts or nonce do not match the indexed execution".to_string(),
        ));
    }
    if account.data.get(328..360) != Some(request.reference_digest.as_slice()) {
        return Err(ProviderError::InvalidResponse(
            "refund claim reference digest does not match the indexed execution".to_string(),
        ));
    }
    let execute_after = read_i64_field(account, 360, "refund claim execute window")?;
    let expires_at = read_i64_field(account, 368, "refund claim execute window")?;
    if execute_after > expires_at
        || account.data.get(408) != Some(&1)
        || account.data.get(409) != Some(&0)
    {
        return Err(ProviderError::InvalidResponse(
            "refund claim is not valid executed evidence".to_string(),
        ));
    }
    let vault_authority = Pubkey::find_program_address(
        &[b"vault-authority", merchant.as_ref(), mint.as_ref()],
        &settlement_client::settlement_program_id(),
    )
    .0;
    Ok(RefundBalanceTarget {
        claim: Pubkey::new_from_array(request.claim).to_string(),
        adjustment: adjustment.to_string(),
        original_receipt: original_receipt.to_string(),
        merchant: merchant.to_string(),
        claim_nonce: request.claim_nonce,
        tax_vault: tax_vault.to_string(),
        refund_source: refund_source.to_string(),
        payer_destination: refund_destination.to_string(),
        mint: mint.to_string(),
        token_program: settlement_client::SPL_TOKEN_PROGRAM_ID.to_string(),
        vault_authority: vault_authority.to_string(),
        refund_funder: refund_funder.to_string(),
        payer: payer.to_string(),
    })
}

struct RefundProofProviderSlot {
    source: Arc<dyn FinalizedRefundBalanceProofSource>,
    health: AdaptiveProviderHealth,
}

/// Bounded failover for claim resolution plus transaction-bound proof. A
/// provider either returns the complete pair or contributes no evidence.
pub struct FailoverRefundBalanceProofSource {
    providers: Vec<RefundProofProviderSlot>,
    attempt_timeout: Duration,
}

impl FailoverRefundBalanceProofSource {
    pub fn new(
        providers: Vec<Arc<dyn FinalizedRefundBalanceProofSource>>,
        attempt_timeout: Duration,
    ) -> Result<Self, ProviderError> {
        if providers.is_empty() || providers.len() > MAX_PROVIDER_ATTEMPTS {
            return Err(ProviderError::InvalidResponse(
                "refund proof provider pool must contain between 1 and 3 endpoints".to_string(),
            ));
        }
        if !(Duration::from_millis(50)..=Duration::from_secs(10)).contains(&attempt_timeout) {
            return Err(ProviderError::InvalidResponse(
                "refund proof provider timeout must be between 50ms and 10s".to_string(),
            ));
        }
        Ok(Self {
            providers: providers
                .into_iter()
                .map(|source| RefundProofProviderSlot {
                    source,
                    health: AdaptiveProviderHealth::default(),
                })
                .collect(),
            attempt_timeout,
        })
    }

    pub async fn fetch_finalized_refund_balance_proof(
        &self,
        request: &RefundClaimTopologyRequest,
        signature: &str,
    ) -> Result<(RefundBalanceProofTarget, FinalizedRefundBalanceTransition), ProviderError> {
        let mut order = self
            .providers
            .iter()
            .enumerate()
            .filter(|(_, provider)| provider.health.is_available())
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if order.is_empty()
            && let Some((index, _)) = self
                .providers
                .iter()
                .enumerate()
                .min_by_key(|(_, provider)| provider.health.cooldown_remaining())
        {
            order.push(index);
        }
        order.sort_by_key(|index| {
            (
                std::cmp::Reverse(self.providers[*index].health.score()),
                *index,
            )
        });
        let mut last_error = None;
        for index in order {
            let provider = &self.providers[index];
            match tokio::time::timeout(
                self.attempt_timeout,
                provider
                    .source
                    .fetch_finalized_refund_balance_proof(request, signature),
            )
            .await
            {
                Ok(Ok(proof)) => {
                    provider.health.record_success();
                    return Ok(proof);
                }
                Ok(Err(error)) => {
                    provider.health.record_failure();
                    last_error = Some(error);
                }
                Err(_) => {
                    provider.health.record_failure();
                    last_error = Some(ProviderError::Transport(
                        "refund proof provider attempt deadline exceeded".to_string(),
                    ));
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            ProviderError::Transport("refund proof provider pool returned no result".to_string())
        }))
    }
}

#[async_trait]
impl FinalizedRefundBalanceProofSource for FailoverRefundBalanceProofSource {
    async fn fetch_finalized_refund_balance_proof(
        &self,
        request: &RefundClaimTopologyRequest,
        signature: &str,
    ) -> Result<(RefundBalanceProofTarget, FinalizedRefundBalanceTransition), ProviderError> {
        self.fetch_finalized_refund_balance_proof(request, signature)
            .await
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum RefundProofWorkerError {
    #[error("refund proof candidate is invalid: {0}")]
    InvalidCandidate(String),
    #[error("refund proof provider failed: {0}")]
    Provider(#[from] ProviderError),
    #[error("refund proof arithmetic failed: {0}")]
    Arithmetic(String),
    #[error("refund proof persistence failed: {0}")]
    Persistence(String),
}

/// A server-owned candidate selected from the indexed execution projection.
/// The topology request is assembled by the storage boundary from finalized
/// claim evidence; it is never accepted from an operator or browser request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundProofCandidate {
    pub stream_key: String,
    pub lease_token: Uuid,
    pub execution: IndexedAdjustmentRefundExecution,
    pub topology_request: RefundClaimTopologyRequest,
}

impl RefundProofCandidate {
    pub fn validate(&self) -> Result<(), RefundProofWorkerError> {
        if self.stream_key.trim().is_empty()
            || self.stream_key.len() > 256
            || self.stream_key != self.stream_key.trim()
            || self.lease_token.get_version_num() != 7
            || self.execution.slot == 0
        {
            return Err(RefundProofWorkerError::InvalidCandidate(
                "stream key or finalized execution slot is invalid".to_string(),
            ));
        }
        self.execution.validate_identity().map_err(|_| {
            RefundProofWorkerError::InvalidCandidate(
                "indexed refund execution identity is invalid".to_string(),
            )
        })?;
        let request = &self.topology_request;
        if request.claim != self.execution.claim
            || request.adjustment != self.execution.adjustment
            || request.original_receipt != self.execution.original_receipt
            || request.merchant != self.execution.merchant
            || request.mint != self.execution.mint
            || request.claim_nonce != self.execution.nonce
            || request.gross_atomic != self.execution.gross_magnitude
            || request.tax_atomic != self.execution.tax_magnitude
            || request.net_atomic != self.execution.net_magnitude
            || request.refund_funder != self.execution.refund_funder
            || request.refund_destination != self.execution.refund_destination
            || request.reference_digest != self.execution.reference_digest
        {
            return Err(RefundProofWorkerError::InvalidCandidate(
                "claim topology request is not bound to the indexed execution".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundProofEvidence {
    pub stream_key: String,
    pub lease_token: Uuid,
    pub execution: IndexedAdjustmentRefundExecution,
    pub topology: RefundBalanceTarget,
    pub proof: RefundBalanceProof,
}

#[async_trait]
pub trait RefundProofCandidateSource: Send + Sync {
    async fn claim_next_refund_proof_candidate(
        &self,
    ) -> Result<Option<RefundProofCandidate>, RefundProofWorkerError>;

    async fn record_refund_proof_failure(
        &self,
        _candidate: &RefundProofCandidate,
        _reason: &str,
    ) -> Result<(), RefundProofWorkerError> {
        Ok(())
    }
}

#[async_trait]
pub trait RefundProofSink: Send + Sync {
    async fn persist_refund_proof(
        &self,
        evidence: RefundProofEvidence,
    ) -> Result<(), RefundProofWorkerError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RefundProofCycleResult {
    NoCandidate,
    Persisted { signature: String, slot: u64 },
}

/// One-candidate, bounded proof cycle. The production adapter supplies the
/// PostgreSQL claim/persistence ports; this worker owns only the trust-boundary
/// sequencing and exact arithmetic. It never creates a paid state.
pub struct RefundProofWorker {
    candidates: Arc<dyn RefundProofCandidateSource>,
    provider: Arc<dyn FinalizedRefundBalanceProofSource>,
    sink: Arc<dyn RefundProofSink>,
}

impl RefundProofWorker {
    pub fn new(
        candidates: Arc<dyn RefundProofCandidateSource>,
        provider: Arc<dyn FinalizedRefundBalanceProofSource>,
        sink: Arc<dyn RefundProofSink>,
    ) -> Self {
        Self {
            candidates,
            provider,
            sink,
        }
    }

    pub async fn process_one(&self) -> Result<RefundProofCycleResult, RefundProofWorkerError> {
        let Some(candidate) = self.candidates.claim_next_refund_proof_candidate().await? else {
            return Ok(RefundProofCycleResult::NoCandidate);
        };
        let result = self.process_candidate(&candidate).await;
        if let Err(error) = &result {
            self.candidates
                .record_refund_proof_failure(&candidate, refund_proof_failure_code(error))
                .await?;
        }
        result
    }

    async fn process_candidate(
        &self,
        candidate: &RefundProofCandidate,
    ) -> Result<RefundProofCycleResult, RefundProofWorkerError> {
        candidate.validate()?;

        let (target, transition) = self
            .provider
            .fetch_finalized_refund_balance_proof(
                &candidate.topology_request,
                &candidate.execution.signature,
            )
            .await
            .map_err(RefundProofWorkerError::Provider)?;
        validate_refund_proof_target_binding(candidate, &target)?;

        let proof = prove_refund_balance_deltas(
            AdjustmentDelta {
                gross_atomic: -i128::from(candidate.execution.gross_magnitude),
                tax_atomic: -i128::from(candidate.execution.tax_magnitude),
                net_atomic: -i128::from(candidate.execution.net_magnitude),
            },
            RefundBalanceSnapshot {
                finalized_slot: transition.slot,
                tax_vault_atomic: transition.tax_vault_before_atomic,
                refund_source_atomic: transition.refund_source_before_atomic,
                payer_destination_atomic: transition.payer_destination_before_atomic,
            },
            RefundBalanceSnapshot {
                finalized_slot: transition.slot,
                tax_vault_atomic: transition.tax_vault_after_atomic,
                refund_source_atomic: transition.refund_source_after_atomic,
                payer_destination_atomic: transition.payer_destination_after_atomic,
            },
        )
        .map_err(|error| RefundProofWorkerError::Arithmetic(error.to_string()))?;
        let slot = proof.finalized_slot;
        let signature = candidate.execution.signature.clone();
        self.sink
            .persist_refund_proof(RefundProofEvidence {
                stream_key: candidate.stream_key.clone(),
                lease_token: candidate.lease_token,
                execution: candidate.execution.clone(),
                topology: target.topology,
                proof,
            })
            .await?;
        Ok(RefundProofCycleResult::Persisted { signature, slot })
    }
}

fn refund_proof_failure_code(error: &RefundProofWorkerError) -> &'static str {
    match error {
        RefundProofWorkerError::InvalidCandidate(_) => "invalid_candidate",
        RefundProofWorkerError::Provider(_) => "provider_failure",
        RefundProofWorkerError::Arithmetic(_) => "arithmetic_failure",
        RefundProofWorkerError::Persistence(_) => "persistence_failure",
    }
}

fn validate_refund_proof_target_binding(
    candidate: &RefundProofCandidate,
    target: &RefundBalanceProofTarget,
) -> Result<(), RefundProofWorkerError> {
    validate_refund_proof_target(target).map_err(RefundProofWorkerError::Provider)?;
    let execution = &candidate.execution;
    let request = &candidate.topology_request;
    if target.signature != execution.signature
        || target.gross_atomic != execution.gross_magnitude
        || target.tax_atomic != execution.tax_magnitude
        || target.net_atomic != execution.net_magnitude
        || target.reference_digest != execution.reference_digest
        || target.topology.claim_nonce != execution.nonce
        || target.topology.token_program != settlement_client::SPL_TOKEN_PROGRAM_ID
    {
        return Err(RefundProofWorkerError::InvalidCandidate(
            "provider proof target is not bound to the indexed execution".to_string(),
        ));
    }
    for (value, expected, field) in [
        (&target.topology.claim, request.claim, "claim"),
        (
            &target.topology.adjustment,
            request.adjustment,
            "adjustment",
        ),
        (
            &target.topology.original_receipt,
            request.original_receipt,
            "original receipt",
        ),
        (&target.topology.merchant, request.merchant, "merchant"),
        (&target.topology.mint, request.mint, "mint"),
        (
            &target.topology.refund_funder,
            request.refund_funder,
            "refund funder",
        ),
        (
            &target.topology.payer_destination,
            request.refund_destination,
            "refund destination",
        ),
    ] {
        let parsed = parse_refund_pubkey(value, field).map_err(RefundProofWorkerError::Provider)?;
        if parsed.to_bytes() != expected {
            return Err(RefundProofWorkerError::InvalidCandidate(format!(
                "provider proof {field} is not bound to the indexed execution"
            )));
        }
    }
    for (value, field) in [
        (&target.topology.tax_vault, "tax vault"),
        (&target.topology.refund_source, "refund source"),
        (&target.topology.vault_authority, "vault authority"),
        (&target.topology.payer, "payer"),
    ] {
        parse_refund_pubkey(value, field).map_err(RefundProofWorkerError::Provider)?;
    }
    Ok(())
}

fn validate_refund_proof_target(target: &RefundBalanceProofTarget) -> Result<(), ProviderError> {
    if target.gross_atomic == 0
        || target.tax_atomic.checked_add(target.net_atomic) != Some(target.gross_atomic)
        || target.reference_digest == [0; 32]
    {
        return Err(ProviderError::InvalidResponse(
            "refund proof target amounts or reference digest are invalid".to_string(),
        ));
    }
    if target.topology.claim_nonce == 0 {
        return Err(ProviderError::InvalidResponse(
            "refund proof claim nonce must be positive".to_string(),
        ));
    }
    Ok(())
}

fn canonical_pubkey_string(value: &str, field: &str) -> Result<String, ProviderError> {
    Ok(parse_refund_pubkey(value, field)?.to_string())
}

fn parse_transaction_account_keys(
    transaction: &serde_json::Map<String, Value>,
) -> Result<Vec<String>, ProviderError> {
    let keys = transaction
        .get("transaction")
        .and_then(|value| value.get("message"))
        .and_then(|value| value.get("accountKeys"))
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ProviderError::InvalidResponse("refund proof account keys are missing".to_string())
        })?;
    if keys.is_empty() || keys.len() > 256 {
        return Err(ProviderError::InvalidResponse(
            "refund proof account-key list is outside the bounded range".to_string(),
        ));
    }
    if transaction
        .get("meta")
        .and_then(|meta| meta.get("loadedAddresses"))
        .is_some_and(|loaded| !loaded.is_null())
    {
        return Err(ProviderError::InvalidResponse(
            "refund proof does not support loaded address tables".to_string(),
        ));
    }
    keys.iter()
        .map(|key| {
            let value = key
                .as_str()
                .or_else(|| key.get("pubkey").and_then(Value::as_str))
                .ok_or_else(|| {
                    ProviderError::InvalidResponse(
                        "refund proof account key is not a public key".to_string(),
                    )
                })?;
            canonical_pubkey_string(value, "transaction account key")
        })
        .collect()
}

fn unique_account_index(account_keys: &[String], address: &str) -> Result<u64, ProviderError> {
    let address = canonical_pubkey_string(address, "refund account")?;
    let matches = account_keys
        .iter()
        .enumerate()
        .filter_map(|(index, key)| (key == &address).then_some(index as u64))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(ProviderError::InvalidResponse(
            "refund proof account must occur exactly once in transaction keys".to_string(),
        ));
    }
    Ok(matches[0])
}

fn transaction_token_balance(
    meta: &serde_json::Map<String, Value>,
    field: &str,
    account_index: u64,
    expected_mint: &str,
    expected_program: &str,
    expected_owner: &str,
) -> Result<u64, ProviderError> {
    let entries = meta
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| ProviderError::InvalidResponse(format!("{field} is missing")))?;
    if entries.len() > 256 {
        return Err(ProviderError::InvalidResponse(format!(
            "{field} is too large"
        )));
    }
    let matches = entries
        .iter()
        .filter(|entry| entry.get("accountIndex").and_then(Value::as_u64) == Some(account_index))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(ProviderError::InvalidResponse(format!(
            "{field} must contain exactly one entry for the refund account"
        )));
    }
    let entry = matches[0];
    let mint = entry
        .get("mint")
        .and_then(Value::as_str)
        .ok_or_else(|| ProviderError::InvalidResponse(format!("{field} mint is missing")))?;
    if canonical_pubkey_string(mint, "token balance mint")? != expected_mint {
        return Err(ProviderError::InvalidResponse(format!(
            "{field} mint is not bound"
        )));
    }
    let owner = entry
        .get("owner")
        .and_then(Value::as_str)
        .ok_or_else(|| ProviderError::InvalidResponse(format!("{field} owner is missing")))?;
    if canonical_pubkey_string(owner, "token balance owner")? != expected_owner {
        return Err(ProviderError::InvalidResponse(format!(
            "{field} owner is not bound"
        )));
    }
    let program = entry
        .get("programId")
        .and_then(Value::as_str)
        .ok_or_else(|| ProviderError::InvalidResponse(format!("{field} programId is missing")))?;
    if canonical_pubkey_string(program, "token balance program")? != expected_program {
        return Err(ProviderError::InvalidResponse(format!(
            "{field} program is not bound"
        )));
    }
    let amount = entry
        .get("uiTokenAmount")
        .and_then(|value| value.get("amount"))
        .and_then(Value::as_str)
        .ok_or_else(|| ProviderError::InvalidResponse(format!("{field} amount is missing")))?;
    let parsed = amount
        .parse::<u64>()
        .map_err(|_| ProviderError::InvalidResponse(format!("{field} amount is not a u64")))?;
    if parsed.to_string() != amount {
        return Err(ProviderError::InvalidResponse(format!(
            "{field} amount is not canonical"
        )));
    }
    Ok(parsed)
}

fn verify_refund_execution_log(
    transaction: &serde_json::Map<String, Value>,
    target: &RefundBalanceProofTarget,
) -> Result<(), ProviderError> {
    let logs = transaction
        .get("meta")
        .and_then(|meta| meta.get("logMessages"))
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ProviderError::InvalidResponse("refund proof logs are missing".to_string())
        })?;
    if logs.len() > 1_024 {
        return Err(ProviderError::InvalidResponse(
            "refund proof logs are too large".to_string(),
        ));
    }
    let mut matches = 0usize;
    for log in logs {
        let Some(log) = log.as_str() else {
            return Err(ProviderError::InvalidResponse(
                "refund proof log message is invalid".to_string(),
            ));
        };
        let Some(encoded) = log.strip_prefix("Program data: ") else {
            continue;
        };
        let data = BASE64.decode(encoded).map_err(|error| {
            ProviderError::InvalidResponse(format!("refund proof event is not base64: {error}"))
        })?;
        if data.len() < 8 || data[..8] != ADJUSTMENT_REFUND_EXECUTED_EVENT_DISCRIMINATOR {
            continue;
        }
        matches += 1;
        let event = AdjustmentRefundExecutedEvent::decode(&data)?;
        let topology = &target.topology;
        let expected = [
            (
                event.claim,
                parse_refund_pubkey(&topology.claim, "refund claim")?,
            ),
            (
                event.adjustment,
                parse_refund_pubkey(&topology.adjustment, "adjustment")?,
            ),
            (
                event.original_receipt,
                parse_refund_pubkey(&topology.original_receipt, "original receipt")?,
            ),
            (
                event.merchant,
                parse_refund_pubkey(&topology.merchant, "merchant")?,
            ),
            (
                event.mint,
                parse_refund_pubkey(&topology.mint, "refund mint")?,
            ),
            (
                event.refund_funder,
                parse_refund_pubkey(&topology.refund_funder, "refund funder")?,
            ),
            (
                event.refund_destination,
                parse_refund_pubkey(&topology.payer_destination, "payer destination")?,
            ),
        ];
        if expected
            .iter()
            .any(|(actual, expected)| actual != expected.as_ref())
            || event.nonce != topology.claim_nonce
            || event.gross_magnitude != target.gross_atomic
            || event.tax_magnitude != target.tax_atomic
            || event.net_magnitude != target.net_atomic
            || event.reference_digest != target.reference_digest
        {
            return Err(ProviderError::InvalidResponse(
                "refund proof event does not match the indexed execution target".to_string(),
            ));
        }
    }
    if matches != 1 {
        return Err(ProviderError::InvalidResponse(
            "refund proof requires exactly one matching execution event".to_string(),
        ));
    }
    Ok(())
}

fn parse_refund_pubkey(value: &str, field: &str) -> Result<Pubkey, ProviderError> {
    Pubkey::from_str(value.trim())
        .map_err(|_| ProviderError::InvalidResponse(format!("{field} is not a valid public key")))
}

struct RefundClaimBinding {
    claim: Pubkey,
    adjustment: Pubkey,
    original_receipt: Pubkey,
    merchant: Pubkey,
    mint: Pubkey,
    payer: Pubkey,
    refund_funder: Pubkey,
    tax_vault: Pubkey,
    refund_source: Pubkey,
    payer_destination: Pubkey,
    claim_nonce: u64,
}

fn verify_refund_claim_account(
    account: &RouteAccountSnapshot,
    binding: &RefundClaimBinding,
) -> Result<(), ProviderError> {
    let (expected_claim, _) = Pubkey::find_program_address(
        &[
            b"refund-claim",
            binding.adjustment.as_ref(),
            &binding.claim_nonce.to_le_bytes(),
        ],
        &settlement_client::settlement_program_id(),
    );
    if expected_claim != binding.claim {
        return Err(ProviderError::InvalidResponse(
            "refund claim PDA does not match adjustment and nonce".to_string(),
        ));
    }
    if account.executable || account.owner != settlement_client::SETTLEMENT_PROGRAM_ID {
        return Err(ProviderError::InvalidResponse(
            "refund claim owner or executable flag is invalid".to_string(),
        ));
    }
    if account.data.len() != REFUND_CLAIM_ACCOUNT_DATA_LEN {
        return Err(ProviderError::InvalidResponse(
            "refund claim uses an unsupported account layout".to_string(),
        ));
    }
    let mut discriminator = Sha256::new();
    discriminator.update(b"account:AdjustmentRefundClaim");
    let discriminator = discriminator.finalize();
    if account.data.get(..8) != Some(&discriminator[..8]) {
        return Err(ProviderError::InvalidResponse(
            "refund claim discriminator is invalid".to_string(),
        ));
    }
    let expected_fields = [
        (8, &binding.adjustment, "adjustment"),
        (40, &binding.original_receipt, "original receipt"),
        (72, &binding.merchant, "merchant"),
        (104, &binding.mint, "mint"),
        (136, &binding.payer, "payer"),
        (168, &binding.refund_funder, "refund funder"),
        (200, &binding.tax_vault, "tax vault"),
        (232, &binding.refund_source, "refund source"),
        (264, &binding.payer_destination, "payer destination"),
    ];
    for (offset, expected, field) in expected_fields {
        let actual = read_pubkey_field(account, offset, field)
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        if actual != *expected {
            return Err(ProviderError::InvalidResponse(format!(
                "refund claim {field} binding does not match the target"
            )));
        }
    }
    let nonce = read_u64_field(account, 296, "refund claim nonce")?;
    if nonce != binding.claim_nonce {
        return Err(ProviderError::InvalidResponse(
            "refund claim nonce does not match the target".to_string(),
        ));
    }
    let execute_after = read_i64_field(account, 360, "refund claim execute window")?;
    let expires_at = read_i64_field(account, 368, "refund claim execute window")?;
    if execute_after > expires_at {
        return Err(ProviderError::InvalidResponse(
            "refund claim execution window is invalid".to_string(),
        ));
    }
    if account.data.get(408) != Some(&1) || account.data.get(409) != Some(&0) {
        return Err(ProviderError::InvalidResponse(
            "refund claim is not executed and exclusively finalized".to_string(),
        ));
    }
    Ok(())
}

fn read_u64_field(
    account: &RouteAccountSnapshot,
    offset: usize,
    field: &'static str,
) -> Result<u64, ProviderError> {
    let bytes = account
        .data
        .get(offset..offset + 8)
        .ok_or_else(|| ProviderError::InvalidResponse(format!("{field} is missing")))?;
    Ok(u64::from_le_bytes(bytes.try_into().map_err(|_| {
        ProviderError::InvalidResponse(format!("{field} is malformed"))
    })?))
}

fn read_i64_field(
    account: &RouteAccountSnapshot,
    offset: usize,
    field: &'static str,
) -> Result<i64, ProviderError> {
    let bytes = account
        .data
        .get(offset..offset + 8)
        .ok_or_else(|| ProviderError::InvalidResponse(format!("{field} is missing")))?;
    Ok(i64::from_le_bytes(bytes.try_into().map_err(|_| {
        ProviderError::InvalidResponse(format!("{field} is malformed"))
    })?))
}

fn verify_refund_token_account<'a>(
    account: &'a RouteAccountSnapshot,
    token_program: &Pubkey,
    mint: &Pubkey,
    expected_owner: &Pubkey,
    field: &'static str,
) -> Result<&'a RouteAccountSnapshot, ProviderError> {
    if account.executable || account.owner != token_program.to_string() {
        return Err(ProviderError::InvalidResponse(format!(
            "{field} owner or executable flag is invalid"
        )));
    }
    if account.data.len() != 165 {
        return Err(ProviderError::InvalidResponse(format!(
            "{field} uses an unsupported token layout"
        )));
    }
    let actual_mint = read_pubkey_field(account, 0, field)
        .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
    if actual_mint != *mint {
        return Err(ProviderError::InvalidResponse(format!(
            "{field} mint does not match the target"
        )));
    }
    let actual_owner = read_pubkey_field(account, 32, field)
        .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
    if actual_owner != *expected_owner {
        return Err(ProviderError::InvalidResponse(format!(
            "{field} owner does not match the bound authority"
        )));
    }
    Ok(account)
}

fn token_amount(account: &RouteAccountSnapshot, field: &'static str) -> Result<u64, ProviderError> {
    let amount_bytes = account.data.get(64..72).ok_or_else(|| {
        ProviderError::InvalidResponse(format!("{field} amount field is missing"))
    })?;
    Ok(u64::from_le_bytes(amount_bytes.try_into().map_err(
        |_| ProviderError::InvalidResponse(format!("{field} amount field is malformed")),
    )?))
}

struct VaultProviderSlot {
    source: Arc<dyn FinalizedVaultBalanceSource>,
    health: AdaptiveProviderHealth,
}

struct ApplicabilityBindingProviderSlot {
    source: Arc<dyn FinalizedApplicabilityBindingSource>,
    health: AdaptiveProviderHealth,
}

/// Bounded provider failover for one complete finalized applicability-account
/// proof. The result always comes from one provider; fields are never merged.
pub struct FailoverApplicabilityBindingSource {
    providers: Vec<ApplicabilityBindingProviderSlot>,
    attempt_timeout: Duration,
}

impl FailoverApplicabilityBindingSource {
    pub fn new(
        providers: Vec<Arc<dyn FinalizedApplicabilityBindingSource>>,
        attempt_timeout: Duration,
    ) -> Result<Self, ProviderError> {
        if providers.is_empty() || providers.len() > MAX_PROVIDER_ATTEMPTS {
            return Err(ProviderError::InvalidResponse(
                "applicability provider pool must contain between 1 and 3 endpoints".to_string(),
            ));
        }
        if !(Duration::from_millis(50)..=Duration::from_secs(10)).contains(&attempt_timeout) {
            return Err(ProviderError::InvalidResponse(
                "applicability provider attempt timeout must be between 50ms and 10s".to_string(),
            ));
        }
        Ok(Self {
            providers: providers
                .into_iter()
                .map(|source| ApplicabilityBindingProviderSlot {
                    source,
                    health: AdaptiveProviderHealth::default(),
                })
                .collect(),
            attempt_timeout,
        })
    }

    fn provider_order(&self) -> Vec<usize> {
        let mut available = self
            .providers
            .iter()
            .enumerate()
            .filter(|(_, provider)| provider.health.is_available())
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if available.is_empty()
            && let Some((index, _)) = self
                .providers
                .iter()
                .enumerate()
                .min_by_key(|(_, provider)| provider.health.cooldown_remaining())
        {
            available.push(index);
        }
        available.sort_by_key(|index| {
            (
                std::cmp::Reverse(self.providers[*index].health.score()),
                *index,
            )
        });
        available
    }

    pub async fn fetch_finalized_applicability_binding(
        &self,
        target: &ApplicabilityBindingTarget,
    ) -> Result<FinalizedApplicabilityBinding, ProviderError> {
        let mut last_error = None;
        for provider_index in self.provider_order() {
            let provider = &self.providers[provider_index];
            match tokio::time::timeout(
                self.attempt_timeout,
                provider
                    .source
                    .fetch_finalized_applicability_binding(target),
            )
            .await
            {
                Ok(Ok(binding)) => {
                    provider.health.record_success();
                    return Ok(binding);
                }
                Ok(Err(error)) => {
                    provider.health.record_failure();
                    last_error = Some(error);
                }
                Err(_) => {
                    provider.health.record_failure();
                    last_error = Some(ProviderError::Transport(
                        "applicability provider attempt deadline exceeded".to_string(),
                    ));
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            ProviderError::Transport("applicability provider pool returned no result".to_string())
        }))
    }
}

#[async_trait]
impl FinalizedApplicabilityBindingSource for FailoverApplicabilityBindingSource {
    async fn fetch_finalized_applicability_binding(
        &self,
        target: &ApplicabilityBindingTarget,
    ) -> Result<FinalizedApplicabilityBinding, ProviderError> {
        Self::fetch_finalized_applicability_binding(self, target).await
    }
}

pub struct FailoverVaultBalanceSource {
    providers: Vec<VaultProviderSlot>,
    attempt_timeout: Duration,
}

impl FailoverVaultBalanceSource {
    pub fn new(
        providers: Vec<Arc<dyn FinalizedVaultBalanceSource>>,
        attempt_timeout: Duration,
    ) -> Result<Self, ProviderError> {
        if providers.is_empty() || providers.len() > MAX_PROVIDER_ATTEMPTS {
            return Err(ProviderError::InvalidResponse(
                "vault provider pool must contain between 1 and 3 endpoints".to_string(),
            ));
        }
        if !(Duration::from_millis(50)..=Duration::from_secs(10)).contains(&attempt_timeout) {
            return Err(ProviderError::InvalidResponse(
                "vault provider attempt timeout must be between 50ms and 10s".to_string(),
            ));
        }
        Ok(Self {
            providers: providers
                .into_iter()
                .map(|source| VaultProviderSlot {
                    source,
                    health: AdaptiveProviderHealth::default(),
                })
                .collect(),
            attempt_timeout,
        })
    }

    pub async fn fetch_finalized_vault_balance(
        &self,
        target: &VaultBalanceTarget,
    ) -> Result<FinalizedVaultBalance, ProviderError> {
        let mut order = self
            .providers
            .iter()
            .enumerate()
            .filter(|(_, provider)| provider.health.is_available())
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if order.is_empty()
            && let Some((index, _)) = self
                .providers
                .iter()
                .enumerate()
                .min_by_key(|(_, provider)| provider.health.cooldown_remaining())
        {
            order.push(index);
        }
        order.sort_by_key(|index| {
            (
                std::cmp::Reverse(self.providers[*index].health.score()),
                *index,
            )
        });
        let mut last_error = None;
        for index in order {
            let provider = &self.providers[index];
            match tokio::time::timeout(
                self.attempt_timeout,
                provider.source.fetch_finalized_vault_balance(target),
            )
            .await
            {
                Ok(Ok(balance)) => {
                    provider.health.record_success();
                    return Ok(balance);
                }
                Ok(Err(error)) => {
                    provider.health.record_failure();
                    last_error = Some(error);
                }
                Err(_) => {
                    provider.health.record_failure();
                    last_error = Some(ProviderError::Transport(
                        "vault provider attempt deadline exceeded".to_string(),
                    ));
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            ProviderError::Transport("vault provider pool returned no result".to_string())
        }))
    }
}

struct RefundProviderSlot {
    source: Arc<dyn FinalizedRefundBalanceSource>,
    health: AdaptiveProviderHealth,
}

/// Bounded, deadline-aware failover for the refund balance proof reader.
/// Provider failover never combines responses: one provider must return the
/// complete three-account snapshot used by the domain proof.
pub struct FailoverRefundBalanceSource {
    providers: Vec<RefundProviderSlot>,
    attempt_timeout: Duration,
}

impl FailoverRefundBalanceSource {
    pub fn new(
        providers: Vec<Arc<dyn FinalizedRefundBalanceSource>>,
        attempt_timeout: Duration,
    ) -> Result<Self, ProviderError> {
        if providers.is_empty() || providers.len() > MAX_PROVIDER_ATTEMPTS {
            return Err(ProviderError::InvalidResponse(
                "refund provider pool must contain between 1 and 3 endpoints".to_string(),
            ));
        }
        if !(Duration::from_millis(50)..=Duration::from_secs(10)).contains(&attempt_timeout) {
            return Err(ProviderError::InvalidResponse(
                "refund provider attempt timeout must be between 50ms and 10s".to_string(),
            ));
        }
        Ok(Self {
            providers: providers
                .into_iter()
                .map(|source| RefundProviderSlot {
                    source,
                    health: AdaptiveProviderHealth::default(),
                })
                .collect(),
            attempt_timeout,
        })
    }

    pub async fn fetch_finalized_refund_balances(
        &self,
        target: &RefundBalanceTarget,
    ) -> Result<FinalizedRefundBalances, ProviderError> {
        let mut order = self
            .providers
            .iter()
            .enumerate()
            .filter(|(_, provider)| provider.health.is_available())
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if order.is_empty()
            && let Some((index, _)) = self
                .providers
                .iter()
                .enumerate()
                .min_by_key(|(_, provider)| provider.health.cooldown_remaining())
        {
            order.push(index);
        }
        order.sort_by_key(|index| {
            (
                std::cmp::Reverse(self.providers[*index].health.score()),
                *index,
            )
        });
        let mut last_error = None;
        for index in order {
            let provider = &self.providers[index];
            match tokio::time::timeout(
                self.attempt_timeout,
                provider.source.fetch_finalized_refund_balances(target),
            )
            .await
            {
                Ok(Ok(balance)) => {
                    provider.health.record_success();
                    return Ok(balance);
                }
                Ok(Err(error)) => {
                    provider.health.record_failure();
                    last_error = Some(error);
                }
                Err(_) => {
                    provider.health.record_failure();
                    last_error = Some(ProviderError::Transport(
                        "refund provider attempt deadline exceeded".to_string(),
                    ));
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            ProviderError::Transport("refund provider pool returned no result".to_string())
        }))
    }
}

struct RefundTransitionProviderSlot {
    source: Arc<dyn FinalizedRefundBalanceTransitionSource>,
    health: AdaptiveProviderHealth,
}

/// Bounded failover for the complete transaction-bound proof request. A
/// successful result always contains one provider's transaction and claim
/// observations; responses are never mixed across endpoints.
pub struct FailoverRefundBalanceTransitionSource {
    providers: Vec<RefundTransitionProviderSlot>,
    attempt_timeout: Duration,
}

impl FailoverRefundBalanceTransitionSource {
    pub fn new(
        providers: Vec<Arc<dyn FinalizedRefundBalanceTransitionSource>>,
        attempt_timeout: Duration,
    ) -> Result<Self, ProviderError> {
        if providers.is_empty() || providers.len() > MAX_PROVIDER_ATTEMPTS {
            return Err(ProviderError::InvalidResponse(
                "refund transition provider pool must contain between 1 and 3 endpoints"
                    .to_string(),
            ));
        }
        if !(Duration::from_millis(50)..=Duration::from_secs(10)).contains(&attempt_timeout) {
            return Err(ProviderError::InvalidResponse(
                "refund transition provider attempt timeout must be between 50ms and 10s"
                    .to_string(),
            ));
        }
        Ok(Self {
            providers: providers
                .into_iter()
                .map(|source| RefundTransitionProviderSlot {
                    source,
                    health: AdaptiveProviderHealth::default(),
                })
                .collect(),
            attempt_timeout,
        })
    }

    pub async fn fetch_finalized_refund_balance_transition(
        &self,
        target: &RefundBalanceProofTarget,
    ) -> Result<FinalizedRefundBalanceTransition, ProviderError> {
        let mut order = self
            .providers
            .iter()
            .enumerate()
            .filter(|(_, provider)| provider.health.is_available())
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if order.is_empty()
            && let Some((index, _)) = self
                .providers
                .iter()
                .enumerate()
                .min_by_key(|(_, provider)| provider.health.cooldown_remaining())
        {
            order.push(index);
        }
        order.sort_by_key(|index| {
            (
                std::cmp::Reverse(self.providers[*index].health.score()),
                *index,
            )
        });
        let mut last_error = None;
        for index in order {
            let provider = &self.providers[index];
            match tokio::time::timeout(
                self.attempt_timeout,
                provider
                    .source
                    .fetch_finalized_refund_balance_transition(target),
            )
            .await
            {
                Ok(Ok(transition)) => {
                    provider.health.record_success();
                    return Ok(transition);
                }
                Ok(Err(error)) => {
                    provider.health.record_failure();
                    last_error = Some(error);
                }
                Err(_) => {
                    provider.health.record_failure();
                    last_error = Some(ProviderError::Transport(
                        "refund transition provider attempt deadline exceeded".to_string(),
                    ));
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            ProviderError::Transport(
                "refund transition provider pool returned no result".to_string(),
            )
        }))
    }
}

struct ValidatedRouteKeys {
    mint: Pubkey,
    token_program: Pubkey,
    vault_authority: Pubkey,
    asset_config: Pubkey,
    asset_config_bump: u8,
}

struct ValidatedRefundFundingKeys {
    mint: Pubkey,
    token_program: Pubkey,
    vault_authority: Pubkey,
    refund_funder: Pubkey,
}

fn validate_refund_funding_topology(
    topology: &RefundFundingTopologyConfig,
) -> Result<ValidatedRefundFundingKeys, RefundFundingVerificationError> {
    if topology.configuration_id.is_nil() || topology.version <= 0 {
        return Err(RefundFundingVerificationError::InvalidTopology(
            "configuration identity and positive version are required".to_string(),
        ));
    }
    let merchant = parse_refund_funding_pubkey("merchant_account", &topology.merchant_account)?;
    let mint = parse_refund_funding_pubkey("mint", &topology.mint)?;
    let tax_vault = parse_refund_funding_pubkey("tax_vault", &topology.tax_vault)?;
    let vault_authority =
        parse_refund_funding_pubkey("vault_authority", &topology.vault_authority)?;
    let refund_funder = parse_refund_funding_pubkey("refund_funder", &topology.refund_funder)?;
    let net_source = parse_refund_funding_pubkey("net_source", &topology.net_source)?;
    let token_program = parse_refund_funding_pubkey("token_program", &topology.token_program)?;
    let spl_token =
        parse_refund_funding_pubkey("token_program", settlement_client::SPL_TOKEN_PROGRAM_ID)?;
    if token_program != spl_token {
        return Err(RefundFundingVerificationError::InvalidTopology(
            "refund funding activation supports only the legacy SPL Token program".to_string(),
        ));
    }
    let unique = [merchant, mint, tax_vault, refund_funder, net_source];
    if unique.iter().enumerate().any(|(index, value)| {
        unique
            .iter()
            .enumerate()
            .any(|(other_index, other)| index != other_index && value == other)
    }) {
        return Err(RefundFundingVerificationError::InvalidTopology(
            "refund funding account addresses must be distinct".to_string(),
        ));
    }
    let (derived_vault_authority, _) = Pubkey::find_program_address(
        &[b"vault-authority", merchant.as_ref(), mint.as_ref()],
        &settlement_client::settlement_program_id(),
    );
    if vault_authority != derived_vault_authority {
        return Err(RefundFundingVerificationError::VaultAuthorityMismatch);
    }
    Ok(ValidatedRefundFundingKeys {
        mint,
        token_program,
        vault_authority,
        refund_funder,
    })
}

fn parse_refund_funding_pubkey(
    field: &'static str,
    value: &str,
) -> Result<Pubkey, RefundFundingVerificationError> {
    Pubkey::from_str(value.trim()).map_err(|_| {
        RefundFundingVerificationError::InvalidTopology(format!(
            "{field} is not a valid Solana public key"
        ))
    })
}

fn verify_system_account(
    account: &RouteAccountSnapshot,
    field: &'static str,
) -> Result<(), RefundFundingVerificationError> {
    if account.executable
        || account.owner != settlement_client::SYSTEM_PROGRAM_ID
        || !account.data.is_empty()
    {
        return Err(RefundFundingVerificationError::AccountOwnerMismatch(field));
    }
    Ok(())
}

fn verify_token_container(
    account: &RouteAccountSnapshot,
    token_program: &Pubkey,
    field: &'static str,
    expected_len: usize,
) -> Result<(), RefundFundingVerificationError> {
    if account.executable || account.owner != token_program.to_string() {
        return Err(RefundFundingVerificationError::AccountOwnerMismatch(field));
    }
    if account.data.len() != expected_len {
        return Err(RefundFundingVerificationError::UnsupportedAccountLayout(
            field,
        ));
    }
    Ok(())
}

fn verify_token_mint_for_refund(
    account: &RouteAccountSnapshot,
    mint: &Pubkey,
    field: &'static str,
) -> Result<(), RefundFundingVerificationError> {
    let actual = read_refund_pubkey_field(account, 0, field)?;
    if actual != *mint {
        return Err(RefundFundingVerificationError::TokenAccountMintMismatch(
            field,
        ));
    }
    Ok(())
}

fn verify_token_authority_for_refund(
    account: &RouteAccountSnapshot,
    authority: &Pubkey,
    field: &'static str,
) -> Result<(), RefundFundingVerificationError> {
    let actual = read_refund_pubkey_field(account, 32, field)?;
    if actual != *authority {
        return Err(RefundFundingVerificationError::TokenAuthorityMismatch(
            field,
        ));
    }
    Ok(())
}

fn read_refund_pubkey_field(
    account: &RouteAccountSnapshot,
    offset: usize,
    field: &'static str,
) -> Result<Pubkey, RefundFundingVerificationError> {
    let bytes = account.data.get(offset..offset.saturating_add(32)).ok_or(
        RefundFundingVerificationError::UnsupportedAccountLayout(field),
    )?;
    Pubkey::try_from(bytes)
        .map_err(|_| RefundFundingVerificationError::UnsupportedAccountLayout(field))
}

fn refund_funding_verification_digest(
    topology: &RefundFundingTopologyConfig,
    slot: u64,
    mint_decimals: u8,
    accounts: &[VerifiedRouteAccount],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"solana-tax-protocol/refund-funding-verification/v1");
    hasher.update(topology.configuration_id.as_bytes());
    hasher.update(topology.version.to_le_bytes());
    hasher.update(topology.configuration_digest);
    for value in [
        &topology.merchant_account,
        &topology.mint,
        &topology.tax_vault,
        &topology.vault_authority,
        &topology.refund_funder,
        &topology.net_source,
        &topology.token_program,
    ] {
        write_route_text(&mut hasher, value);
    }
    hasher.update(slot.to_le_bytes());
    hasher.update([mint_decimals]);
    for account in accounts {
        write_route_text(&mut hasher, &account.address);
        write_route_text(&mut hasher, &account.owner);
        hasher.update([u8::from(account.executable)]);
        hasher.update(account.lamports.to_le_bytes());
        hasher.update((account.data_len as u64).to_le_bytes());
        hasher.update(account.data_digest);
    }
    hasher.finalize().into()
}

fn validate_route_config(
    route: &SettlementRouteConfig,
) -> Result<ValidatedRouteKeys, RouteVerificationError> {
    if route.route_id.is_nil() || route.version <= 0 {
        return Err(RouteVerificationError::InvalidRoute(
            "route identity and positive version are required".to_string(),
        ));
    }
    let merchant = parse_route_pubkey("merchant_account", &route.merchant_account)?;
    let mint = parse_route_pubkey("mint", &route.mint)?;
    let tax_vault = parse_route_pubkey("tax_vault", &route.tax_vault)?;
    let net_destination = parse_route_pubkey("net_destination", &route.net_destination)?;
    let token_program = parse_route_pubkey("token_program", &route.token_program)?;
    let spl_token = parse_route_pubkey("token_program", settlement_client::SPL_TOKEN_PROGRAM_ID)?;
    let token_2022 = parse_route_pubkey("token_program", settlement_client::TOKEN_2022_PROGRAM_ID)?;
    if token_program != spl_token && token_program != token_2022 {
        return Err(RouteVerificationError::InvalidRoute(
            "token program is not supported".to_string(),
        ));
    }
    if [merchant, mint, tax_vault, net_destination]
        .into_iter()
        .enumerate()
        .any(|(index, value)| {
            [merchant, mint, tax_vault, net_destination]
                .into_iter()
                .enumerate()
                .any(|(other_index, other)| index != other_index && value == other)
        })
    {
        return Err(RouteVerificationError::InvalidRoute(
            "route account addresses must be distinct".to_string(),
        ));
    }
    let (vault_authority, _) = Pubkey::find_program_address(
        &[b"vault-authority", merchant.as_ref(), mint.as_ref()],
        &settlement_client::settlement_program_id(),
    );
    let (asset_config, asset_config_bump) = Pubkey::find_program_address(
        &[b"asset-config", mint.as_ref()],
        &settlement_client::settlement_program_id(),
    );
    Ok(ValidatedRouteKeys {
        mint,
        token_program,
        vault_authority,
        asset_config,
        asset_config_bump,
    })
}

fn parse_route_pubkey(field: &'static str, value: &str) -> Result<Pubkey, RouteVerificationError> {
    Pubkey::from_str(value.trim()).map_err(|_| {
        RouteVerificationError::InvalidRoute(format!("{field} is not a valid public key"))
    })
}

fn parse_route_account(
    value: &Value,
    expected_address: &str,
) -> Result<RouteAccountSnapshot, RouteVerificationError> {
    parse_route_account_bounded(value, expected_address, MAX_ROUTE_ACCOUNT_DATA_BYTES)
}

fn parse_route_account_bounded(
    value: &Value,
    expected_address: &str,
    max_data_bytes: usize,
) -> Result<RouteAccountSnapshot, RouteVerificationError> {
    if value.is_null() {
        return Err(RouteVerificationError::MissingAccount(
            expected_address.to_string(),
        ));
    }
    let object = value.as_object().ok_or_else(|| {
        RouteVerificationError::InvalidResponse("account value is not an object".to_string())
    })?;
    let owner = object.get("owner").and_then(Value::as_str).ok_or_else(|| {
        RouteVerificationError::InvalidResponse("account owner is missing".to_string())
    })?;
    parse_route_pubkey("account owner", owner)?;
    let executable = object
        .get("executable")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            RouteVerificationError::InvalidResponse("account executable is invalid".to_string())
        })?;
    let lamports = object
        .get("lamports")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            RouteVerificationError::InvalidResponse("account lamports is invalid".to_string())
        })?;
    let data = object
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            RouteVerificationError::InvalidResponse("account data is invalid".to_string())
        })?;
    if data.len() != 2 || data[1].as_str() != Some("base64") {
        return Err(RouteVerificationError::InvalidResponse(
            "account data must use base64 encoding".to_string(),
        ));
    }
    let encoded = data[0].as_str().ok_or_else(|| {
        RouteVerificationError::InvalidResponse("account base64 data is not a string".to_string())
    })?;
    let bytes = BASE64.decode(encoded).map_err(|error| {
        RouteVerificationError::InvalidResponse(format!("account base64 data is invalid: {error}"))
    })?;
    if bytes.len() > max_data_bytes {
        return Err(RouteVerificationError::InvalidResponse(
            "account data exceeds the bounded route limit".to_string(),
        ));
    }
    Ok(RouteAccountSnapshot {
        address: expected_address.to_string(),
        owner: owner.to_string(),
        executable,
        lamports,
        data: bytes,
    })
}

fn verify_token_account_container(
    account: &RouteAccountSnapshot,
    token_program: &Pubkey,
    field: &'static str,
    expected_len: usize,
) -> Result<(), RouteVerificationError> {
    if account.executable || account.owner != token_program.to_string() {
        return Err(RouteVerificationError::AccountOwnerMismatch(field));
    }
    if account.data.len() != expected_len {
        return Err(RouteVerificationError::UnsupportedAccountLayout(field));
    }
    Ok(())
}

fn verify_token_mint(
    account: &RouteAccountSnapshot,
    mint: &Pubkey,
    field: &'static str,
) -> Result<(), RouteVerificationError> {
    let actual = read_pubkey_field(account, 0, field)?;
    if actual != *mint {
        return Err(RouteVerificationError::TokenAccountMintMismatch(field));
    }
    Ok(())
}

struct VerifiedAssetConfigFields {
    asset_kind: u8,
    enabled: bool,
    bump: u8,
}

fn verify_asset_config(
    account: &RouteAccountSnapshot,
    keys: &ValidatedRouteKeys,
    mint_decimals: u8,
) -> Result<VerifiedAssetConfigFields, RouteVerificationError> {
    if account.executable || account.owner != settlement_client::settlement_program_id().to_string()
    {
        return Err(RouteVerificationError::AccountOwnerMismatch("asset_config"));
    }
    if account.data.len() != ASSET_CONFIG_ACCOUNT_DATA_LEN {
        return Err(RouteVerificationError::UnsupportedAccountLayout(
            "asset_config",
        ));
    }
    if account.data[..8] != ASSET_CONFIG_ACCOUNT_DISCRIMINATOR {
        return Err(RouteVerificationError::UnsupportedAccountLayout(
            "asset_config",
        ));
    }
    if read_pubkey_field(account, 8, "asset_config")? != keys.mint {
        return Err(RouteVerificationError::AssetConfigMismatch("mint"));
    }
    if read_pubkey_field(account, 40, "asset_config")? != keys.token_program {
        return Err(RouteVerificationError::AssetConfigMismatch("token program"));
    }
    if account.data[72] != mint_decimals {
        return Err(RouteVerificationError::AssetConfigMismatch("decimals"));
    }
    if account.data[73] != STABLECOIN_ASSET_KIND {
        return Err(RouteVerificationError::AssetConfigNotStablecoin);
    }
    if account.data[106] == 0 {
        return Err(RouteVerificationError::AssetConfigDisabled);
    }
    if account.data[107] != keys.asset_config_bump {
        return Err(RouteVerificationError::AssetConfigMismatch("PDA bump"));
    }
    if account.address != keys.asset_config.to_string() {
        return Err(RouteVerificationError::AssetConfigMismatch("PDA address"));
    }
    Ok(VerifiedAssetConfigFields {
        asset_kind: account.data[73],
        enabled: account.data[106] != 0,
        bump: account.data[107],
    })
}

fn read_pubkey_field(
    account: &RouteAccountSnapshot,
    offset: usize,
    field: &'static str,
) -> Result<Pubkey, RouteVerificationError> {
    let bytes = account
        .data
        .get(offset..offset.saturating_add(32))
        .ok_or(RouteVerificationError::UnsupportedAccountLayout(field))?;
    Pubkey::try_from(bytes).map_err(|_| RouteVerificationError::UnsupportedAccountLayout(field))
}

fn route_account_evidence(account: RouteAccountSnapshot) -> VerifiedRouteAccount {
    let mut hasher = Sha256::new();
    hasher.update(b"solana-tax-protocol/route-account/v1");
    write_route_text(&mut hasher, &account.address);
    write_route_text(&mut hasher, &account.owner);
    hasher.update([u8::from(account.executable)]);
    hasher.update(account.lamports.to_le_bytes());
    hasher.update(&account.data);
    VerifiedRouteAccount {
        address: account.address,
        owner: account.owner,
        executable: account.executable,
        lamports: account.lamports,
        data_len: account.data.len(),
        data_digest: hasher.finalize().into(),
    }
}

fn route_verification_digest(
    route: &SettlementRouteConfig,
    asset_config: &Pubkey,
    slot: u64,
    mint_decimals: u8,
    accounts: &[VerifiedRouteAccount],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"solana-tax-protocol/settlement-route-verification/v2");
    write_route_text(&mut hasher, SETTLEMENT_ROUTE_VERIFICATION_SCHEMA);
    hasher.update(route.route_id.as_bytes());
    hasher.update(route.version.to_le_bytes());
    write_route_text(&mut hasher, &route.merchant_account);
    write_route_text(&mut hasher, &route.mint);
    write_route_text(&mut hasher, &route.tax_vault);
    write_route_text(&mut hasher, &route.net_destination);
    write_route_text(&mut hasher, &route.token_program);
    write_route_text(&mut hasher, &asset_config.to_string());
    hasher.update(slot.to_le_bytes());
    write_route_text(&mut hasher, "finalized");
    hasher.update([mint_decimals]);
    for account in accounts {
        write_route_text(&mut hasher, &account.address);
        write_route_text(&mut hasher, &account.owner);
        hasher.update([u8::from(account.executable)]);
        hasher.update(account.lamports.to_le_bytes());
        hasher.update((account.data_len as u64).to_le_bytes());
        hasher.update(account.data_digest);
    }
    hasher.finalize().into()
}

fn map_destination_route_error(
    error: RouteVerificationError,
) -> RevenueStreamDestinationVerificationError {
    match error {
        RouteVerificationError::Provider(error) => {
            RevenueStreamDestinationVerificationError::Provider(error)
        }
        RouteVerificationError::MissingAccount(address) => {
            RevenueStreamDestinationVerificationError::MissingAccount(address)
        }
        RouteVerificationError::InvalidResponse(message) => {
            RevenueStreamDestinationVerificationError::InvalidResponse(message)
        }
        RouteVerificationError::InvalidRoute(message) => {
            RevenueStreamDestinationVerificationError::InvalidRoute(message)
        }
        RouteVerificationError::AccountOwnerMismatch(field) => {
            RevenueStreamDestinationVerificationError::AccountOwnerMismatch(field)
        }
        RouteVerificationError::UnsupportedAccountLayout(field) => {
            RevenueStreamDestinationVerificationError::UnsupportedAccountLayout(field)
        }
        RouteVerificationError::TokenAccountMintMismatch(_) => {
            RevenueStreamDestinationVerificationError::TokenAccountMintMismatch
        }
        RouteVerificationError::VaultAuthorityMismatch => {
            RevenueStreamDestinationVerificationError::InvalidResponse(
                "unexpected vault authority error while parsing destination route".to_string(),
            )
        }
        RouteVerificationError::AssetConfigDisabled
        | RouteVerificationError::AssetConfigNotStablecoin
        | RouteVerificationError::AssetConfigMismatch(_) => {
            RevenueStreamDestinationVerificationError::InvalidResponse(
                "unexpected asset admission error while parsing destination route".to_string(),
            )
        }
    }
}

fn parse_destination_route_pubkey(
    field: &'static str,
    value: &str,
) -> Result<Pubkey, RevenueStreamDestinationVerificationError> {
    Pubkey::from_str(value.trim()).map_err(|_| {
        RevenueStreamDestinationVerificationError::InvalidRoute(format!(
            "{field} is not a valid public key"
        ))
    })
}

fn verify_destination_token_account_container(
    account: &RouteAccountSnapshot,
    token_program: &Pubkey,
    field: &'static str,
    expected_len: usize,
) -> Result<(), RevenueStreamDestinationVerificationError> {
    if account.executable || account.owner != token_program.to_string() {
        return Err(RevenueStreamDestinationVerificationError::AccountOwnerMismatch(field));
    }
    if account.data.len() != expected_len {
        return Err(RevenueStreamDestinationVerificationError::UnsupportedAccountLayout(field));
    }
    Ok(())
}

fn read_destination_pubkey_field(
    account: &RouteAccountSnapshot,
    offset: usize,
    field: &'static str,
) -> Result<Pubkey, RevenueStreamDestinationVerificationError> {
    let bytes = account
        .data
        .get(offset..offset.saturating_add(32))
        .ok_or(RevenueStreamDestinationVerificationError::UnsupportedAccountLayout(field))?;
    Pubkey::try_from(bytes)
        .map_err(|_| RevenueStreamDestinationVerificationError::UnsupportedAccountLayout(field))
}

fn revenue_destination_verification_digest(
    route: &RevenueStreamDestinationRouteConfig,
    slot: u64,
    mint_decimals: u8,
    accounts: &[VerifiedRouteAccount],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"solana-tax-protocol/revenue-stream-destination-verification/v1");
    hasher.update(route.route_id.as_bytes());
    hasher.update(route.version.to_le_bytes());
    write_route_text(&mut hasher, &route.recipient_owner);
    write_route_text(&mut hasher, &route.mint);
    write_route_text(&mut hasher, &route.destination_token_account);
    write_route_text(&mut hasher, &route.token_program);
    hasher.update(slot.to_le_bytes());
    write_route_text(&mut hasher, "finalized");
    hasher.update([mint_decimals]);
    for account in accounts {
        write_route_text(&mut hasher, &account.address);
        write_route_text(&mut hasher, &account.owner);
        hasher.update([u8::from(account.executable)]);
        hasher.update(account.lamports.to_le_bytes());
        hasher.update((account.data_len as u64).to_le_bytes());
        hasher.update(account.data_digest);
    }
    hasher.finalize().into()
}

fn write_route_text(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

#[derive(Clone)]
pub struct ReqwestJsonRpcTransport {
    client: reqwest::Client,
    endpoint: Url,
    next_request_id: Arc<AtomicU64>,
    max_response_bytes: usize,
}

impl ReqwestJsonRpcTransport {
    pub fn new(
        endpoint: &str,
        timeout: Duration,
        max_response_bytes: usize,
    ) -> Result<Self, ProviderError> {
        Self::build(endpoint, timeout, max_response_bytes, false)
    }

    /// Build a transport for the explicitly local Docker development profile.
    ///
    /// This is intentionally separate from [`Self::new`]. It permits a
    /// container-network HTTP hostname such as `validator`, which has no TLS
    /// identity, and must never be enabled by production configuration.
    pub fn new_for_local_development(
        endpoint: &str,
        timeout: Duration,
        max_response_bytes: usize,
    ) -> Result<Self, ProviderError> {
        Self::build(endpoint, timeout, max_response_bytes, true)
    }

    fn build(
        endpoint: &str,
        timeout: Duration,
        max_response_bytes: usize,
        allow_insecure_http: bool,
    ) -> Result<Self, ProviderError> {
        let endpoint = Url::parse(endpoint)
            .map_err(|error| ProviderError::InvalidResponse(format!("invalid RPC URL: {error}")))?;
        let host = endpoint.host_str().unwrap_or_default();
        let local_http = matches!(endpoint.scheme(), "http")
            && matches!(host, "localhost" | "127.0.0.1" | "::1");
        let explicitly_allowed_http = endpoint.scheme() == "http" && allow_insecure_http;
        if endpoint.scheme() != "https" && !local_http && !explicitly_allowed_http {
            return Err(ProviderError::InvalidResponse(
                "RPC endpoint must use HTTPS; HTTP is allowed only for local development"
                    .to_string(),
            ));
        }
        if endpoint.username() != "" || endpoint.password().is_some() {
            return Err(ProviderError::InvalidResponse(
                "RPC endpoint must not contain credentials".to_string(),
            ));
        }
        if !(Duration::from_millis(50)..=Duration::from_secs(10)).contains(&timeout) {
            return Err(ProviderError::InvalidResponse(
                "RPC timeout must be between 50ms and 10s".to_string(),
            ));
        }
        if !(1024..=MAX_RPC_RESPONSE_BYTES).contains(&max_response_bytes) {
            return Err(ProviderError::InvalidResponse(
                "RPC response limit is outside the bounded range".to_string(),
            ));
        }
        let client = reqwest::Client::builder()
            .connect_timeout(timeout)
            .timeout(timeout)
            .pool_max_idle_per_host(16)
            .http2_adaptive_window(true)
            .build()
            .map_err(|error| ProviderError::Transport(error.to_string()))?;
        Ok(Self {
            client,
            endpoint,
            next_request_id: Arc::new(AtomicU64::new(1)),
            max_response_bytes,
        })
    }
}

#[derive(Serialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: &'static str,
    id: u64,
    method: &'a str,
    params: Value,
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: u64,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

#[async_trait]
impl JsonRpcTransport for ReqwestJsonRpcTransport {
    async fn call(&self, method: &str, params: Value) -> Result<Value, ProviderError> {
        if method.is_empty() || method.len() > 128 {
            return Err(ProviderError::InvalidResponse(
                "RPC method name is invalid".to_string(),
            ));
        }
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let response = self
            .client
            .post(self.endpoint.clone())
            .json(&JsonRpcRequest {
                jsonrpc: "2.0",
                id: request_id,
                method,
                params,
            })
            .send()
            .await
            .map_err(|error| ProviderError::Transport(error.to_string()))?;
        if !response.status().is_success() {
            return Err(ProviderError::Transport(format!(
                "RPC HTTP status {}",
                response.status()
            )));
        }
        if response
            .content_length()
            .is_some_and(|length| length > self.max_response_bytes as u64)
        {
            return Err(ProviderError::ResponseTooLarge);
        }
        let mut body = Vec::new();
        let mut response = response;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| ProviderError::Transport(error.to_string()))?
        {
            if body.len().saturating_add(chunk.len()) > self.max_response_bytes {
                return Err(ProviderError::ResponseTooLarge);
            }
            body.extend_from_slice(&chunk);
        }
        let envelope: JsonRpcResponse = serde_json::from_slice(&body)
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        if envelope.jsonrpc != "2.0" || envelope.id != request_id {
            return Err(ProviderError::InvalidResponse(
                "JSON-RPC version or request ID mismatch".to_string(),
            ));
        }
        if let Some(error) = envelope.error {
            return Err(ProviderError::Rpc(format!(
                "{} ({})",
                error.message, error.code
            )));
        }
        envelope.result.ok_or_else(|| {
            ProviderError::InvalidResponse("JSON-RPC response has no result".to_string())
        })
    }
}

#[async_trait]
pub trait ReceiptDecoder: Send + Sync {
    async fn decode_block(&self, slot: u64, block: Value) -> Result<DecodedBlock, ProviderError>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DecodedBlock {
    pub receipts: Vec<IndexedReceipt>,
    pub split_settlements: Vec<IndexedSplitSettlement>,
    pub adjustment_records: Vec<IndexedSettlementAdjustment>,
    pub refund_executions: Vec<IndexedAdjustmentRefundExecution>,
    pub raw_evidence: Vec<RawChainEvidence>,
    pub raw_transactions: Vec<RawChainTransaction>,
    pub protocol_pause_changes: Vec<ProtocolPauseChangedEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawChainTransaction {
    pub signature: String,
    pub slot: u64,
    pub transaction_index: u32,
    pub payload: Vec<u8>,
    pub payload_digest: [u8; 32],
}

impl RawChainTransaction {
    pub fn validate(
        &self,
        range: ScanRange,
        observed_finalized_slot: u64,
    ) -> Result<(), IndexerError> {
        if self.signature.trim().is_empty()
            || self.signature.len() > 128
            || self.payload.is_empty()
            || self.payload.len() > MAX_RAW_TRANSACTION_BYTES
            || self.payload_digest == [0; 32]
            || self.payload_digest
                != raw_transaction_digest(
                    &self.signature,
                    self.slot,
                    self.transaction_index,
                    &self.payload,
                )
        {
            return Err(IndexerError::InvalidRawTransaction);
        }
        if self.slot < range.start_slot
            || self.slot > range.end_slot
            || self.slot > observed_finalized_slot
        {
            return Err(IndexerError::RawTransactionOutsideRange);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawChainEvidence {
    pub signature: String,
    pub slot: u64,
    pub event_index: u16,
    pub event_discriminator: [u8; 8],
    pub event_data: Vec<u8>,
    pub evidence_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolPauseChangedEvent {
    pub signature: String,
    pub slot: u64,
    pub event_index: u16,
    pub authority: [u8; 32],
    pub paused: bool,
    pub evidence_digest: [u8; 32],
}

impl ProtocolPauseChangedEvent {
    pub fn validate(
        &self,
        range: ScanRange,
        observed_finalized_slot: u64,
    ) -> Result<(), IndexerError> {
        if self.signature.trim().is_empty()
            || self.signature.len() > 128
            || self.authority == [0; 32]
            || self.evidence_digest == [0; 32]
            || self.evidence_digest
                != protocol_pause_evidence_digest(
                    &self.signature,
                    self.slot,
                    self.event_index,
                    self.authority,
                    self.paused,
                )
        {
            return Err(IndexerError::InvalidEvidence);
        }
        if self.slot < range.start_slot
            || self.slot > range.end_slot
            || self.slot > observed_finalized_slot
        {
            return Err(IndexerError::EvidenceOutsideRange);
        }
        Ok(())
    }

    fn decode(
        signature: &str,
        slot: u64,
        event_index: u16,
        data: &[u8],
    ) -> Result<Self, ProviderError> {
        if data.len() != 8 + 32 + 1 || data[..8] != PROTOCOL_PAUSE_EVENT_DISCRIMINATOR {
            return Err(ProviderError::InvalidResponse(
                "protocol pause event has an unexpected discriminator or length".to_string(),
            ));
        }
        let mut offset = 8;
        let authority = read_array::<32>(data, &mut offset)?;
        let paused = match data[offset] {
            0 => false,
            1 => true,
            _ => {
                return Err(ProviderError::InvalidResponse(
                    "protocol pause event contains a non-canonical bool".to_string(),
                ));
            }
        };
        let evidence_digest =
            protocol_pause_evidence_digest(signature, slot, event_index, authority, paused);
        let event = Self {
            signature: signature.to_string(),
            slot,
            event_index,
            authority,
            paused,
            evidence_digest,
        };
        event
            .validate(
                ScanRange {
                    start_slot: slot,
                    end_slot: slot,
                },
                slot,
            )
            .map_err(|_| {
                ProviderError::InvalidResponse(
                    "protocol pause event violates identity or finality invariants".to_string(),
                )
            })?;
        Ok(event)
    }
}

impl RawChainEvidence {
    pub fn validate(
        &self,
        range: ScanRange,
        observed_finalized_slot: u64,
    ) -> Result<(), IndexerError> {
        if self.signature.trim().is_empty()
            || self.signature.len() > 128
            || self.event_data.len() < 8
            || self.event_data.len() > MAX_RAW_EVIDENCE_BYTES
            || self.event_data[..8] != self.event_discriminator
            || self.evidence_digest == [0; 32]
            || self.evidence_digest
                != settlement_evidence_digest(
                    &self.signature,
                    self.slot,
                    self.event_index,
                    &self.event_data,
                )
        {
            return Err(IndexerError::InvalidEvidence);
        }
        if self.slot < range.start_slot
            || self.slot > range.end_slot
            || self.slot > observed_finalized_slot
        {
            return Err(IndexerError::EvidenceOutsideRange);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettlementExecutedEvent {
    pub receipt: [u8; 32],
    pub payer: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub nonce: u64,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub policy_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentSettlementExecutedEvent {
    pub receipt: [u8; 32],
    pub payer: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub nonce: u64,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub policy_digest: [u8; 32],
    pub document_attestation_digest: [u8; 32],
    pub payment_scope_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicabilitySettlementExecutedEvent {
    pub receipt: [u8; 32],
    pub binding: [u8; 32],
    pub payer: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub nonce: u64,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub policy_digest: [u8; 32],
    pub document_attestation_digest: Option<[u8; 32]>,
    pub payment_scope_digest: Option<[u8; 32]>,
    pub context_id: [u8; 16],
    pub applicability_attestation_digest: [u8; 32],
}

impl ApplicabilitySettlementExecutedEvent {
    pub fn decode(data: &[u8]) -> Result<Self, ProviderError> {
        let discriminator = data.get(..8).ok_or_else(|| {
            ProviderError::InvalidResponse(
                "applicability settlement event is shorter than its discriminator".to_string(),
            )
        })?;
        let is_document = discriminator == APPLICABILITY_DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR;
        let expected_len = if is_document {
            8 + (32 * 6) + (8 * 4) + (32 * 2) + 16 + 32
        } else {
            8 + (32 * 5) + (8 * 4) + 32 + 16 + 32
        };
        if data.len() != expected_len
            || (is_document
                && discriminator != APPLICABILITY_DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR)
            || (!is_document && discriminator != APPLICABILITY_SETTLEMENT_EVENT_DISCRIMINATOR)
        {
            return Err(ProviderError::InvalidResponse(
                "applicability settlement event has an unexpected discriminator or length"
                    .to_string(),
            ));
        }
        let mut offset = 8;
        let receipt = read_array::<32>(data, &mut offset)?;
        let binding = read_array::<32>(data, &mut offset)?;
        let payer = read_array::<32>(data, &mut offset)?;
        let merchant = read_array::<32>(data, &mut offset)?;
        let mint = read_array::<32>(data, &mut offset)?;
        let nonce = read_u64_le(data, &mut offset)?;
        let gross_atomic = read_u64_le(data, &mut offset)?;
        let tax_atomic = read_u64_le(data, &mut offset)?;
        let net_atomic = read_u64_le(data, &mut offset)?;
        let policy_digest = read_array::<32>(data, &mut offset)?;
        let (document_attestation_digest, payment_scope_digest) = if is_document {
            (
                Some(read_array::<32>(data, &mut offset)?),
                Some(read_array::<32>(data, &mut offset)?),
            )
        } else {
            (None, None)
        };
        let context_id = read_array::<16>(data, &mut offset)?;
        let applicability_attestation_digest = read_array::<32>(data, &mut offset)?;
        if offset != data.len()
            || receipt == [0; 32]
            || binding == [0; 32]
            || payer == [0; 32]
            || merchant == [0; 32]
            || mint == [0; 32]
            || nonce == 0
            || gross_atomic == 0
            || tax_atomic.checked_add(net_atomic) != Some(gross_atomic)
            || policy_digest == [0; 32]
            || context_id == [0; 16]
            || applicability_attestation_digest == [0; 32]
            || document_attestation_digest.is_some_and(|digest| digest == [0; 32])
            || payment_scope_digest.is_some_and(|digest| digest == [0; 32])
        {
            return Err(ProviderError::InvalidResponse(
                "applicability settlement event violates identity, provenance, or amount invariants"
                    .to_string(),
            ));
        }
        Ok(Self {
            receipt,
            binding,
            payer,
            merchant,
            mint,
            nonce,
            gross_atomic,
            tax_atomic,
            net_atomic,
            policy_digest,
            document_attestation_digest,
            payment_scope_digest,
            context_id,
            applicability_attestation_digest,
        })
    }

    pub fn validate_binding(
        &self,
        binding: &FinalizedApplicabilityBinding,
    ) -> Result<(), IndexerError> {
        if self.receipt != binding.receipt
            || self.binding != binding.binding
            || self.merchant != binding.merchant
            || self.mint != binding.mint
            || self.context_id != binding.context_id
            || self.applicability_attestation_digest != binding.attestation_digest
        {
            return Err(IndexerError::ConflictingEvidence);
        }
        Ok(())
    }
}

impl DocumentSettlementExecutedEvent {
    fn decode(data: &[u8]) -> Result<Self, ProviderError> {
        if data.len() != 8 + (32 * 7) + (8 * 4)
            || data[..8] != DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR
        {
            return Err(ProviderError::InvalidResponse(
                "document settlement event has an unexpected discriminator or length".to_string(),
            ));
        }
        let mut offset = 8;
        let receipt = read_array::<32>(data, &mut offset)?;
        let payer = read_array::<32>(data, &mut offset)?;
        let merchant = read_array::<32>(data, &mut offset)?;
        let mint = read_array::<32>(data, &mut offset)?;
        let nonce = read_u64_le(data, &mut offset)?;
        let gross_atomic = read_u64_le(data, &mut offset)?;
        let tax_atomic = read_u64_le(data, &mut offset)?;
        let net_atomic = read_u64_le(data, &mut offset)?;
        let policy_digest = read_array::<32>(data, &mut offset)?;
        let document_attestation_digest = read_array::<32>(data, &mut offset)?;
        let payment_scope_digest = read_array::<32>(data, &mut offset)?;
        if offset != data.len()
            || receipt == [0; 32]
            || payer == [0; 32]
            || merchant == [0; 32]
            || mint == [0; 32]
            || nonce == 0
            || gross_atomic == 0
            || tax_atomic.checked_add(net_atomic) != Some(gross_atomic)
            || policy_digest == [0; 32]
            || document_attestation_digest == [0; 32]
            || payment_scope_digest == [0; 32]
        {
            return Err(ProviderError::InvalidResponse(
                "document settlement event violates identity, provenance, or amount invariants"
                    .to_string(),
            ));
        }
        Ok(Self {
            receipt,
            payer,
            merchant,
            mint,
            nonce,
            gross_atomic,
            tax_atomic,
            net_atomic,
            policy_digest,
            document_attestation_digest,
            payment_scope_digest,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IndexedSplitSettlement {
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
    pub split_plan_version: u64,
    pub split_plan_digest: [u8; 32],
    pub policy_digest: [u8; 32],
    pub evidence_digest: [u8; 32],
}

impl IndexedSplitSettlement {
    fn validate_identity(&self) -> Result<(), IndexerError> {
        if self.signature.trim().is_empty()
            || self.signature.len() > 128
            || self.receipt == [0; 32]
            || self.payer == [0; 32]
            || self.merchant == [0; 32]
            || self.mint == [0; 32]
            || self.gross_atomic == 0
            || self.tax_atomic.checked_add(self.net_atomic) != Some(self.gross_atomic)
            || self.split_plan_version == 0
            || self.split_plan_digest == [0; 32]
            || self.policy_digest == [0; 32]
            || self.evidence_digest == [0; 32]
        {
            return Err(IndexerError::InvalidSplitSettlement);
        }
        Ok(())
    }

    fn validate(&self, range: ScanRange, observed_finalized_slot: u64) -> Result<(), IndexerError> {
        self.validate_identity()?;
        if self.slot < range.start_slot || self.slot > range.end_slot {
            return Err(IndexerError::SplitSettlementOutsideRange);
        }
        if self.slot > observed_finalized_slot {
            return Err(IndexerError::InvalidSplitSettlement);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitSettlementExecutedEvent {
    pub receipt: [u8; 32],
    pub payer: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub nonce: u64,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub split_plan_version: u64,
    pub split_plan_digest: [u8; 32],
    pub policy_digest: [u8; 32],
}

impl SplitSettlementExecutedEvent {
    fn decode(data: &[u8]) -> Result<Self, ProviderError> {
        if data.len() != 8 + (32 * 4) + (8 * 5) + (32 * 2)
            || data[..8] != SPLIT_SETTLEMENT_EVENT_DISCRIMINATOR
        {
            return Err(ProviderError::InvalidResponse(
                "split settlement event has an unexpected Anchor discriminator or length"
                    .to_string(),
            ));
        }
        let mut offset = 8;
        let receipt = read_array::<32>(data, &mut offset)?;
        let payer = read_array::<32>(data, &mut offset)?;
        let merchant = read_array::<32>(data, &mut offset)?;
        let mint = read_array::<32>(data, &mut offset)?;
        let nonce = read_u64_le(data, &mut offset)?;
        let gross_atomic = read_u64_le(data, &mut offset)?;
        let tax_atomic = read_u64_le(data, &mut offset)?;
        let net_atomic = read_u64_le(data, &mut offset)?;
        let split_plan_version = read_u64_le(data, &mut offset)?;
        let split_plan_digest = read_array::<32>(data, &mut offset)?;
        let policy_digest = read_array::<32>(data, &mut offset)?;
        if offset != data.len()
            || receipt == [0; 32]
            || payer == [0; 32]
            || merchant == [0; 32]
            || mint == [0; 32]
            || gross_atomic == 0
            || tax_atomic.checked_add(net_atomic) != Some(gross_atomic)
            || split_plan_version == 0
            || split_plan_digest == [0; 32]
            || policy_digest == [0; 32]
        {
            return Err(ProviderError::InvalidResponse(
                "split settlement event violates identity or amount conservation".to_string(),
            ));
        }
        Ok(Self {
            receipt,
            payer,
            merchant,
            mint,
            nonce,
            gross_atomic,
            tax_atomic,
            net_atomic,
            split_plan_version,
            split_plan_digest,
            policy_digest,
        })
    }
}

/// Maps an already validated finalized split projection plus a server-resolved
/// recipient allocation into the revenue domain's finality proof contract.
/// The recipient and amount must come from an immutable, tenant-scoped split
/// plan projection; this function deliberately has no browser/API input path.
pub fn build_revenue_stream_allocation_proof(
    split: &IndexedSplitSettlement,
    recipient: [u8; 32],
    recipient_atomic: u64,
    observed_finalized_slot: u64,
) -> Result<FinalizedSettlementAllocationProof, IndexerError> {
    split.validate(
        ScanRange {
            start_slot: split.slot,
            end_slot: split.slot,
        },
        observed_finalized_slot,
    )?;
    let signature =
        Signature::from_str(&split.signature).map_err(|_| IndexerError::InvalidSplitSettlement)?;
    let proof = FinalizedSettlementAllocationProof {
        source_signature: RevenueSignature(signature.as_ref().to_vec()),
        source_slot: split.slot,
        observed_finalized_slot,
        commitment: FinalityCommitment::Finalized,
        receipt_digest: RevenueDigest(split.evidence_digest),
        plan_digest: RevenueDigest(split.split_plan_digest),
        policy_digest: RevenueDigest(split.policy_digest),
        asset: RevenueAssetId(split.mint),
        recipient: RevenueRecipientId(recipient),
        gross_atomic: split.gross_atomic,
        tax_atomic: split.tax_atomic,
        net_atomic: split.net_atomic,
        recipient_atomic,
    };
    revenue_streaming::finalized_accrual_from_proof(proof.clone())
        .map(|_| proof)
        .map_err(|_| IndexerError::InvalidSplitSettlement)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IndexedSettlementAdjustment {
    pub signature: String,
    pub slot: u64,
    pub event_index: u16,
    pub adjustment: [u8; 32],
    pub original_receipt: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub nonce: u64,
    pub kind: u8,
    pub gross_sign: u8,
    pub gross_magnitude: u64,
    pub tax_sign: u8,
    pub tax_magnitude: u64,
    pub net_sign: u8,
    pub net_magnitude: u64,
    pub adjustment_digest: [u8; 32],
    pub adjustment_evidence_digest: [u8; 32],
    pub evidence_digest: [u8; 32],
    pub approver: [u8; 32],
}

impl IndexedSettlementAdjustment {
    fn validate_identity(&self) -> Result<(), IndexerError> {
        if self.signature.trim().is_empty()
            || self.signature.len() > 128
            || self.adjustment == [0; 32]
            || self.original_receipt == [0; 32]
            || self.merchant == [0; 32]
            || self.mint == [0; 32]
            || self.nonce == 0
            || !(1..=5).contains(&self.kind)
            || self.adjustment_digest == [0; 32]
            || self.adjustment_evidence_digest == [0; 32]
            || self.evidence_digest == [0; 32]
            || self.approver == [0; 32]
        {
            return Err(IndexerError::InvalidSettlementAdjustment);
        }
        for (sign, magnitude) in [
            (self.gross_sign, self.gross_magnitude),
            (self.tax_sign, self.tax_magnitude),
            (self.net_sign, self.net_magnitude),
        ] {
            if sign > 1 || (magnitude == 0 && sign == 1) {
                return Err(IndexerError::InvalidSettlementAdjustment);
            }
        }
        if self.gross_magnitude == 0 && self.tax_magnitude == 0 && self.net_magnitude == 0 {
            return Err(IndexerError::InvalidSettlementAdjustment);
        }
        if matches!(self.kind, 1 | 2 | 4)
            && (self.gross_sign != 1
                || (self.tax_magnitude != 0 && self.tax_sign != 1)
                || (self.net_magnitude != 0 && self.net_sign != 1))
        {
            return Err(IndexerError::InvalidSettlementAdjustment);
        }
        let (result_sign, result_magnitude) = if self.tax_sign == self.net_sign {
            (
                self.tax_sign,
                self.tax_magnitude
                    .checked_add(self.net_magnitude)
                    .ok_or(IndexerError::InvalidSettlementAdjustment)?,
            )
        } else if self.tax_magnitude >= self.net_magnitude {
            (self.tax_sign, self.tax_magnitude - self.net_magnitude)
        } else {
            (self.net_sign, self.net_magnitude - self.tax_magnitude)
        };
        if result_sign != self.gross_sign || result_magnitude != self.gross_magnitude {
            return Err(IndexerError::InvalidSettlementAdjustment);
        }
        Ok(())
    }

    fn validate(&self, range: ScanRange, observed_finalized_slot: u64) -> Result<(), IndexerError> {
        self.validate_identity()?;
        if self.slot < range.start_slot
            || self.slot > range.end_slot
            || self.slot > observed_finalized_slot
        {
            return Err(IndexerError::SettlementAdjustmentOutsideRange);
        }
        Ok(())
    }
}

struct SettlementAdjustmentRecordedEvent {
    adjustment: [u8; 32],
    original_receipt: [u8; 32],
    merchant: [u8; 32],
    mint: [u8; 32],
    nonce: u64,
    kind: u8,
    gross_sign: u8,
    gross_magnitude: u64,
    tax_sign: u8,
    tax_magnitude: u64,
    net_sign: u8,
    net_magnitude: u64,
    adjustment_digest: [u8; 32],
    evidence_digest: [u8; 32],
    approver: [u8; 32],
}

impl SettlementAdjustmentRecordedEvent {
    fn decode(data: &[u8]) -> Result<Self, ProviderError> {
        const EVENT_BYTES: usize = 8 + (32 * 5) + 8 + 1 + (3 * 9) + (32 * 2);
        if data.len() != EVENT_BYTES
            || data[..8] != SETTLEMENT_ADJUSTMENT_RECORDED_EVENT_DISCRIMINATOR
        {
            return Err(ProviderError::InvalidResponse(
                "settlement adjustment event has an unexpected discriminator or length".to_string(),
            ));
        }
        let mut offset = 8;
        let event = Self {
            adjustment: read_array::<32>(data, &mut offset)?,
            original_receipt: read_array::<32>(data, &mut offset)?,
            merchant: read_array::<32>(data, &mut offset)?,
            mint: read_array::<32>(data, &mut offset)?,
            nonce: read_u64_le(data, &mut offset)?,
            kind: read_u8(data, &mut offset)?,
            gross_sign: read_u8(data, &mut offset)?,
            gross_magnitude: read_u64_le(data, &mut offset)?,
            tax_sign: read_u8(data, &mut offset)?,
            tax_magnitude: read_u64_le(data, &mut offset)?,
            net_sign: read_u8(data, &mut offset)?,
            net_magnitude: read_u64_le(data, &mut offset)?,
            adjustment_digest: read_array::<32>(data, &mut offset)?,
            evidence_digest: read_array::<32>(data, &mut offset)?,
            approver: read_array::<32>(data, &mut offset)?,
        };
        if offset != data.len() {
            return Err(ProviderError::InvalidResponse(
                "settlement adjustment event has trailing bytes".to_string(),
            ));
        }
        let indexed = IndexedSettlementAdjustment {
            signature: "adjustment-event-validation".to_string(),
            slot: 0,
            event_index: 0,
            adjustment: event.adjustment,
            original_receipt: event.original_receipt,
            merchant: event.merchant,
            mint: event.mint,
            nonce: event.nonce,
            kind: event.kind,
            gross_sign: event.gross_sign,
            gross_magnitude: event.gross_magnitude,
            tax_sign: event.tax_sign,
            tax_magnitude: event.tax_magnitude,
            net_sign: event.net_sign,
            net_magnitude: event.net_magnitude,
            adjustment_digest: event.adjustment_digest,
            adjustment_evidence_digest: event.evidence_digest,
            evidence_digest: [1; 32],
            approver: event.approver,
        };
        indexed.validate_identity().map_err(|_| {
            ProviderError::InvalidResponse(
                "settlement adjustment event violates identity or conservation invariants"
                    .to_string(),
            )
        })?;
        Ok(event)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IndexedAdjustmentRefundExecution {
    pub signature: String,
    pub slot: u64,
    pub event_index: u16,
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
    pub approver: [u8; 32],
    pub reference_digest: [u8; 32],
    pub evidence_digest: [u8; 32],
}

impl IndexedAdjustmentRefundExecution {
    fn validate_identity(&self) -> Result<(), IndexerError> {
        if self.signature.trim().is_empty()
            || self.signature.len() > 128
            || self.claim == [0; 32]
            || self.adjustment == [0; 32]
            || self.original_receipt == [0; 32]
            || self.merchant == [0; 32]
            || self.mint == [0; 32]
            || self.nonce == 0
            || self.gross_magnitude == 0
            || self.tax_magnitude.checked_add(self.net_magnitude) != Some(self.gross_magnitude)
            || self.refund_funder == [0; 32]
            || self.refund_destination == [0; 32]
            || self.approver == [0; 32]
            || self.reference_digest == [0; 32]
            || self.evidence_digest == [0; 32]
        {
            return Err(IndexerError::InvalidAdjustmentRefundExecution);
        }
        if self.refund_destination == self.refund_funder
            || self.approver == self.refund_funder
            || self.approver == self.refund_destination
        {
            return Err(IndexerError::InvalidAdjustmentRefundExecution);
        }
        Ok(())
    }

    fn validate(&self, range: ScanRange, observed_finalized_slot: u64) -> Result<(), IndexerError> {
        self.validate_identity()?;
        if self.slot < range.start_slot
            || self.slot > range.end_slot
            || self.slot > observed_finalized_slot
        {
            return Err(IndexerError::AdjustmentRefundExecutionOutsideRange);
        }
        Ok(())
    }
}

struct AdjustmentRefundExecutedEvent {
    claim: [u8; 32],
    adjustment: [u8; 32],
    original_receipt: [u8; 32],
    merchant: [u8; 32],
    mint: [u8; 32],
    nonce: u64,
    gross_magnitude: u64,
    tax_magnitude: u64,
    net_magnitude: u64,
    refund_funder: [u8; 32],
    refund_destination: [u8; 32],
    approver: [u8; 32],
    reference_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalizedRefundClaimEvidence {
    pub signature: String,
    pub slot: u64,
    pub event_index: u16,
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
    pub evidence_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalizedRefundClaimCancellationEvidence {
    pub signature: String,
    pub slot: u64,
    pub event_index: u16,
    pub claim: [u8; 32],
    pub adjustment: [u8; 32],
    pub merchant: [u8; 32],
    pub mint: [u8; 32],
    pub nonce: u64,
    pub governance_authority: [u8; 32],
    pub reference_digest: [u8; 32],
    pub evidence_digest: [u8; 32],
}

impl FinalizedRefundClaimCancellationEvidence {
    fn validate(&self) -> Result<(), ProviderError> {
        if self.signature.trim().is_empty()
            || self.signature.len() > 128
            || self.claim == [0; 32]
            || self.adjustment == [0; 32]
            || self.merchant == [0; 32]
            || self.mint == [0; 32]
            || self.nonce == 0
            || self.governance_authority == [0; 32]
            || self.reference_digest == [0; 32]
            || self.evidence_digest == [0; 32]
        {
            return Err(ProviderError::InvalidResponse(
                "refund claim cancellation evidence violates identity invariants".to_string(),
            ));
        }
        Ok(())
    }
}

pub fn decode_refund_claim_cancelled_event(
    signature: &str,
    slot: u64,
    event_index: u16,
    data: &[u8],
    evidence_digest: [u8; 32],
) -> Result<FinalizedRefundClaimCancellationEvidence, ProviderError> {
    const EVENT_BYTES: usize = 8 + (32 * 6) + 8;
    if data.len() != EVENT_BYTES
        || data[..8] != ADJUSTMENT_REFUND_CLAIM_CANCELLED_EVENT_DISCRIMINATOR
    {
        return Err(ProviderError::InvalidResponse(
            "refund claim cancellation event has an unexpected discriminator or length".to_string(),
        ));
    }
    let mut offset = 8;
    let event = FinalizedRefundClaimCancellationEvidence {
        signature: signature.to_string(),
        slot,
        event_index,
        claim: read_array::<32>(data, &mut offset)?,
        adjustment: read_array::<32>(data, &mut offset)?,
        merchant: read_array::<32>(data, &mut offset)?,
        mint: read_array::<32>(data, &mut offset)?,
        nonce: read_u64_le(data, &mut offset)?,
        governance_authority: read_array::<32>(data, &mut offset)?,
        reference_digest: read_array::<32>(data, &mut offset)?,
        evidence_digest,
    };
    if offset != data.len() {
        return Err(ProviderError::InvalidResponse(
            "refund claim cancellation event has trailing bytes".to_string(),
        ));
    }
    event.validate()?;
    Ok(event)
}

impl FinalizedRefundClaimEvidence {
    fn validate(&self) -> Result<(), ProviderError> {
        if self.signature.trim().is_empty()
            || self.signature.len() > 128
            || self.claim == [0; 32]
            || self.adjustment == [0; 32]
            || self.original_receipt == [0; 32]
            || self.merchant == [0; 32]
            || self.mint == [0; 32]
            || self.nonce == 0
            || self.gross_magnitude == 0
            || self.tax_magnitude.checked_add(self.net_magnitude) != Some(self.gross_magnitude)
            || self.refund_funder == [0; 32]
            || self.refund_destination == [0; 32]
            || self.proposer == [0; 32]
            || self.reference_digest == [0; 32]
            || self.evidence_digest == [0; 32]
            || self.refund_funder == self.refund_destination
            || self.proposer == self.refund_funder
            || self.execute_after_unix <= 0
            || self.execute_after_unix >= self.expires_at_unix
        {
            return Err(ProviderError::InvalidResponse(
                "refund claim evidence violates identity, conservation, authority, or window invariants"
                    .to_string(),
            ));
        }
        Ok(())
    }
}

/// Decode the immutable Anchor event retained by the indexer for a finalized
/// refund-claim creation. The caller must compare every field with the
/// durable claim intent before advancing its lifecycle.
pub fn decode_refund_claim_created_event(
    signature: &str,
    slot: u64,
    event_index: u16,
    data: &[u8],
    evidence_digest: [u8; 32],
) -> Result<FinalizedRefundClaimEvidence, ProviderError> {
    const EVENT_BYTES: usize = 8 + (32 * 9) + (8 * 6);
    if data.len() != EVENT_BYTES || data[..8] != ADJUSTMENT_REFUND_CLAIM_CREATED_EVENT_DISCRIMINATOR
    {
        return Err(ProviderError::InvalidResponse(
            "refund claim event has an unexpected discriminator or length".to_string(),
        ));
    }
    let mut offset = 8;
    let event = FinalizedRefundClaimEvidence {
        signature: signature.to_string(),
        slot,
        event_index,
        claim: read_array::<32>(data, &mut offset)?,
        adjustment: read_array::<32>(data, &mut offset)?,
        original_receipt: read_array::<32>(data, &mut offset)?,
        merchant: read_array::<32>(data, &mut offset)?,
        mint: read_array::<32>(data, &mut offset)?,
        nonce: read_u64_le(data, &mut offset)?,
        gross_magnitude: read_u64_le(data, &mut offset)?,
        tax_magnitude: read_u64_le(data, &mut offset)?,
        net_magnitude: read_u64_le(data, &mut offset)?,
        refund_funder: read_array::<32>(data, &mut offset)?,
        refund_destination: read_array::<32>(data, &mut offset)?,
        proposer: read_array::<32>(data, &mut offset)?,
        reference_digest: read_array::<32>(data, &mut offset)?,
        execute_after_unix: read_i64_le(data, &mut offset)?,
        expires_at_unix: read_i64_le(data, &mut offset)?,
        evidence_digest,
    };
    if offset != data.len() {
        return Err(ProviderError::InvalidResponse(
            "refund claim event has trailing bytes".to_string(),
        ));
    }
    event.validate()?;
    Ok(event)
}

impl AdjustmentRefundExecutedEvent {
    fn decode(data: &[u8]) -> Result<Self, ProviderError> {
        const EVENT_BYTES: usize = 8 + (32 * 9) + (8 * 4);
        if data.len() != EVENT_BYTES || data[..8] != ADJUSTMENT_REFUND_EXECUTED_EVENT_DISCRIMINATOR
        {
            return Err(ProviderError::InvalidResponse(
                "adjustment refund execution event has an unexpected discriminator or length"
                    .to_string(),
            ));
        }
        let mut offset = 8;
        let claim = read_array::<32>(data, &mut offset)?;
        let adjustment = read_array::<32>(data, &mut offset)?;
        let original_receipt = read_array::<32>(data, &mut offset)?;
        let merchant = read_array::<32>(data, &mut offset)?;
        let mint = read_array::<32>(data, &mut offset)?;
        let nonce = read_u64_le(data, &mut offset)?;
        let gross_magnitude = read_u64_le(data, &mut offset)?;
        let tax_magnitude = read_u64_le(data, &mut offset)?;
        let net_magnitude = read_u64_le(data, &mut offset)?;
        let refund_funder = read_array::<32>(data, &mut offset)?;
        let refund_destination = read_array::<32>(data, &mut offset)?;
        let approver = read_array::<32>(data, &mut offset)?;
        let reference_digest = read_array::<32>(data, &mut offset)?;
        let event = Self {
            claim,
            adjustment,
            original_receipt,
            merchant,
            mint,
            nonce,
            gross_magnitude,
            tax_magnitude,
            net_magnitude,
            refund_funder,
            refund_destination,
            approver,
            reference_digest,
        };
        let indexed = IndexedAdjustmentRefundExecution {
            signature: "refund-event-validation".to_string(),
            slot: 0,
            event_index: 0,
            claim: event.claim,
            adjustment: event.adjustment,
            original_receipt: event.original_receipt,
            merchant: event.merchant,
            mint: event.mint,
            nonce: event.nonce,
            gross_magnitude: event.gross_magnitude,
            tax_magnitude: event.tax_magnitude,
            net_magnitude: event.net_magnitude,
            refund_funder: event.refund_funder,
            refund_destination: event.refund_destination,
            approver: event.approver,
            reference_digest: event.reference_digest,
            evidence_digest: [1; 32],
        };
        indexed.validate_identity().map_err(|_| {
            ProviderError::InvalidResponse(
                "adjustment refund execution event violates identity or conservation invariants"
                    .to_string(),
            )
        })?;
        Ok(event)
    }
}

impl SettlementExecutedEvent {
    fn decode(data: &[u8]) -> Result<Self, ProviderError> {
        if data.len() != 8 + (32 * 4) + (8 * 4) + 32 || data[..8] != SETTLEMENT_EVENT_DISCRIMINATOR
        {
            return Err(ProviderError::InvalidResponse(
                "settlement event has an unexpected Anchor discriminator or length".to_string(),
            ));
        }
        let mut offset = 8;
        let receipt = read_array::<32>(data, &mut offset)?;
        let payer = read_array::<32>(data, &mut offset)?;
        let merchant = read_array::<32>(data, &mut offset)?;
        let mint = read_array::<32>(data, &mut offset)?;
        let nonce = read_u64_le(data, &mut offset)?;
        let gross_atomic = read_u64_le(data, &mut offset)?;
        let tax_atomic = read_u64_le(data, &mut offset)?;
        let net_atomic = read_u64_le(data, &mut offset)?;
        let policy_digest = read_array::<32>(data, &mut offset)?;
        if offset != data.len()
            || gross_atomic == 0
            || tax_atomic.checked_add(net_atomic) != Some(gross_atomic)
        {
            return Err(ProviderError::InvalidResponse(
                "settlement event violates amount conservation".to_string(),
            ));
        }
        Ok(Self {
            receipt,
            payer,
            merchant,
            mint,
            nonce,
            gross_atomic,
            tax_atomic,
            net_atomic,
            policy_digest,
        })
    }
}

/// Reconstruct the normalized receipt projection from one immutable raw event.
///
/// This is deliberately independent of checkpoints, providers, and storage so
/// a recovery worker cannot substitute caller-provided amounts or advance the
/// forward index while rebuilding a projection.
pub fn rebuild_receipt_from_raw_evidence(
    evidence: &RawChainEvidence,
) -> Result<IndexedReceipt, IndexerError> {
    evidence.validate(
        ScanRange {
            start_slot: evidence.slot,
            end_slot: evidence.slot,
        },
        evidence.slot,
    )?;
    let (
        receipt_key,
        payer,
        merchant,
        mint,
        nonce,
        gross_atomic,
        tax_atomic,
        net_atomic,
        policy_digest,
    ) = if evidence.event_discriminator == SETTLEMENT_EVENT_DISCRIMINATOR {
        let event = SettlementExecutedEvent::decode(&evidence.event_data)
            .map_err(|_| IndexerError::InvalidEvidence)?;
        (
            event.receipt,
            event.payer,
            event.merchant,
            event.mint,
            event.nonce,
            event.gross_atomic,
            event.tax_atomic,
            event.net_atomic,
            event.policy_digest,
        )
    } else if evidence.event_discriminator == DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR {
        let event = DocumentSettlementExecutedEvent::decode(&evidence.event_data)
            .map_err(|_| IndexerError::InvalidEvidence)?;
        (
            event.receipt,
            event.payer,
            event.merchant,
            event.mint,
            event.nonce,
            event.gross_atomic,
            event.tax_atomic,
            event.net_atomic,
            event.policy_digest,
        )
    } else if evidence.event_discriminator == APPLICABILITY_SETTLEMENT_EVENT_DISCRIMINATOR
        || evidence.event_discriminator == APPLICABILITY_DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR
    {
        let event = ApplicabilitySettlementExecutedEvent::decode(&evidence.event_data)
            .map_err(|_| IndexerError::InvalidEvidence)?;
        (
            event.receipt,
            event.payer,
            event.merchant,
            event.mint,
            event.nonce,
            event.gross_atomic,
            event.tax_atomic,
            event.net_atomic,
            event.policy_digest,
        )
    } else if evidence.event_discriminator == SPLIT_SETTLEMENT_EVENT_DISCRIMINATOR {
        let event = SplitSettlementExecutedEvent::decode(&evidence.event_data)
            .map_err(|_| IndexerError::InvalidEvidence)?;
        (
            event.receipt,
            event.payer,
            event.merchant,
            event.mint,
            event.nonce,
            event.gross_atomic,
            event.tax_atomic,
            event.net_atomic,
            event.policy_digest,
        )
    } else {
        return Err(IndexerError::InvalidEvidence);
    };
    let receipt = IndexedReceipt {
        signature: evidence.signature.clone(),
        slot: evidence.slot,
        receipt: receipt_key,
        payer,
        merchant,
        mint,
        nonce,
        gross_atomic,
        tax_atomic,
        net_atomic,
        policy_digest,
        evidence_digest: evidence.evidence_digest,
    };
    receipt.validate_identity()?;
    Ok(receipt)
}

/// Reconstruct the typed split projection from one immutable raw split event.
pub fn rebuild_split_settlement_from_raw_evidence(
    evidence: &RawChainEvidence,
) -> Result<IndexedSplitSettlement, IndexerError> {
    evidence.validate(
        ScanRange {
            start_slot: evidence.slot,
            end_slot: evidence.slot,
        },
        evidence.slot,
    )?;
    if evidence.event_discriminator != SPLIT_SETTLEMENT_EVENT_DISCRIMINATOR {
        return Err(IndexerError::InvalidEvidence);
    }
    let event = SplitSettlementExecutedEvent::decode(&evidence.event_data)
        .map_err(|_| IndexerError::InvalidEvidence)?;
    let split = IndexedSplitSettlement {
        signature: evidence.signature.clone(),
        slot: evidence.slot,
        receipt: event.receipt,
        payer: event.payer,
        merchant: event.merchant,
        mint: event.mint,
        nonce: event.nonce,
        gross_atomic: event.gross_atomic,
        tax_atomic: event.tax_atomic,
        net_atomic: event.net_atomic,
        split_plan_version: event.split_plan_version,
        split_plan_digest: event.split_plan_digest,
        policy_digest: event.policy_digest,
        evidence_digest: evidence.evidence_digest,
    };
    split.validate(
        ScanRange {
            start_slot: evidence.slot,
            end_slot: evidence.slot,
        },
        evidence.slot,
    )?;
    Ok(split)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SettlementExecutedDecoder;

#[async_trait]
impl ReceiptDecoder for SettlementExecutedDecoder {
    async fn decode_block(&self, slot: u64, block: Value) -> Result<DecodedBlock, ProviderError> {
        let transactions = block
            .get("transactions")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ProviderError::InvalidResponse("getBlock.transactions must be an array".to_string())
            })?;
        let mut decoded = DecodedBlock::default();
        for (transaction_index, transaction) in transactions.iter().enumerate() {
            let transaction_index = u32::try_from(transaction_index).map_err(|_| {
                ProviderError::InvalidResponse("block contains too many transactions".to_string())
            })?;
            let signature = transaction
                .get("transaction")
                .and_then(|value| value.get("signatures"))
                .and_then(Value::as_array)
                .and_then(|signatures| signatures.first())
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty() && value.len() <= 128)
                .ok_or_else(|| {
                    ProviderError::InvalidResponse("transaction signature is missing".to_string())
                })?;
            let payload = serde_json::to_vec(transaction).map_err(|error| {
                ProviderError::InvalidResponse(format!("transaction cannot be serialized: {error}"))
            })?;
            if payload.is_empty() || payload.len() > MAX_RAW_TRANSACTION_BYTES {
                return Err(ProviderError::InvalidResponse(
                    "transaction payload exceeds the bounded evidence size".to_string(),
                ));
            }
            decoded.raw_transactions.push(RawChainTransaction {
                signature: signature.to_string(),
                slot,
                transaction_index,
                payload_digest: raw_transaction_digest(
                    signature,
                    slot,
                    transaction_index,
                    &payload,
                ),
                payload,
            });
            let meta = transaction.get("meta").ok_or_else(|| {
                ProviderError::InvalidResponse("transaction.meta is required".to_string())
            })?;
            if !meta
                .get("err")
                .ok_or_else(|| {
                    ProviderError::InvalidResponse("transaction.meta.err is required".to_string())
                })?
                .is_null()
            {
                continue;
            }
            let logs = meta
                .get("logMessages")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    ProviderError::InvalidResponse(
                        "transaction.meta.logMessages must be an array".to_string(),
                    )
                })?;
            if !logs.iter().any(|log| {
                log.as_str().is_some_and(|line| {
                    line.starts_with("Program ")
                        && line.contains(SETTLEMENT_PROGRAM_ID)
                        && line.contains(" invoke")
                })
            }) {
                continue;
            }
            let mut event_index = 0_u16;
            let mut saw_movement_event = false;
            let mut saw_pause_event = false;
            for log in logs {
                let Some(line) = log.as_str() else { continue };
                let Some(encoded) = line.strip_prefix("Program data: ") else {
                    continue;
                };
                let event_data = BASE64.decode(encoded).map_err(|error| {
                    ProviderError::InvalidResponse(format!("invalid Anchor event base64: {error}"))
                })?;
                if event_data.len() < 8 {
                    continue;
                }
                let discriminator: [u8; 8] = event_data[..8].try_into().map_err(|_| {
                    ProviderError::InvalidResponse("event discriminator is malformed".to_string())
                })?;
                if discriminator != SETTLEMENT_EVENT_DISCRIMINATOR
                    && discriminator != DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR
                    && discriminator != SPLIT_SETTLEMENT_EVENT_DISCRIMINATOR
                    && discriminator != APPLICABILITY_SETTLEMENT_EVENT_DISCRIMINATOR
                    && discriminator != APPLICABILITY_DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR
                    && discriminator != SETTLEMENT_ADJUSTMENT_RECORDED_EVENT_DISCRIMINATOR
                    && discriminator != ADJUSTMENT_REFUND_EXECUTED_EVENT_DISCRIMINATOR
                    && discriminator != ADJUSTMENT_REFUND_CLAIM_CREATED_EVENT_DISCRIMINATOR
                    && discriminator != ADJUSTMENT_REFUND_CLAIM_CANCELLED_EVENT_DISCRIMINATOR
                    && discriminator != PROTOCOL_PAUSE_EVENT_DISCRIMINATOR
                {
                    continue;
                }
                if discriminator == ADJUSTMENT_REFUND_CLAIM_CREATED_EVENT_DISCRIMINATOR {
                    let evidence_digest =
                        settlement_evidence_digest(signature, slot, event_index, &event_data);
                    decode_refund_claim_created_event(
                        signature,
                        slot,
                        event_index,
                        &event_data,
                        evidence_digest,
                    )?;
                    decoded.raw_evidence.push(RawChainEvidence {
                        signature: signature.to_string(),
                        slot,
                        event_index,
                        event_discriminator: discriminator,
                        event_data,
                        evidence_digest,
                    });
                    event_index = event_index.checked_add(1).ok_or_else(|| {
                        ProviderError::InvalidResponse(
                            "too many supported events in one transaction".to_string(),
                        )
                    })?;
                    continue;
                }
                if discriminator == ADJUSTMENT_REFUND_CLAIM_CANCELLED_EVENT_DISCRIMINATOR {
                    let evidence_digest =
                        settlement_evidence_digest(signature, slot, event_index, &event_data);
                    decode_refund_claim_cancelled_event(
                        signature,
                        slot,
                        event_index,
                        &event_data,
                        evidence_digest,
                    )?;
                    decoded.raw_evidence.push(RawChainEvidence {
                        signature: signature.to_string(),
                        slot,
                        event_index,
                        event_discriminator: discriminator,
                        event_data,
                        evidence_digest,
                    });
                    event_index = event_index.checked_add(1).ok_or_else(|| {
                        ProviderError::InvalidResponse(
                            "too many supported events in one transaction".to_string(),
                        )
                    })?;
                    continue;
                }
                let is_movement_event = discriminator == SETTLEMENT_EVENT_DISCRIMINATOR
                    || discriminator == DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR
                    || discriminator == SPLIT_SETTLEMENT_EVENT_DISCRIMINATOR
                    || discriminator == APPLICABILITY_SETTLEMENT_EVENT_DISCRIMINATOR
                    || discriminator == APPLICABILITY_DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR
                    || discriminator == SETTLEMENT_ADJUSTMENT_RECORDED_EVENT_DISCRIMINATOR
                    || discriminator == ADJUSTMENT_REFUND_EXECUTED_EVENT_DISCRIMINATOR;
                if saw_movement_event && is_movement_event {
                    return Err(ProviderError::InvalidResponse(
                        "a transaction emitted more than one supported movement event".to_string(),
                    ));
                }
                if discriminator == PROTOCOL_PAUSE_EVENT_DISCRIMINATOR {
                    if saw_pause_event {
                        return Err(ProviderError::InvalidResponse(
                            "a transaction emitted more than one protocol pause event".to_string(),
                        ));
                    }
                    saw_pause_event = true;
                    decoded
                        .protocol_pause_changes
                        .push(ProtocolPauseChangedEvent::decode(
                            signature,
                            slot,
                            event_index,
                            &event_data,
                        )?);
                    event_index = event_index.checked_add(1).ok_or_else(|| {
                        ProviderError::InvalidResponse(
                            "too many supported events in one transaction".to_string(),
                        )
                    })?;
                    continue;
                }
                saw_movement_event = true;
                let evidence_digest =
                    settlement_evidence_digest(signature, slot, event_index, &event_data);
                if discriminator == SETTLEMENT_ADJUSTMENT_RECORDED_EVENT_DISCRIMINATOR {
                    let event = SettlementAdjustmentRecordedEvent::decode(&event_data)?;
                    decoded
                        .adjustment_records
                        .push(IndexedSettlementAdjustment {
                            signature: signature.to_string(),
                            slot,
                            event_index,
                            adjustment: event.adjustment,
                            original_receipt: event.original_receipt,
                            merchant: event.merchant,
                            mint: event.mint,
                            nonce: event.nonce,
                            kind: event.kind,
                            gross_sign: event.gross_sign,
                            gross_magnitude: event.gross_magnitude,
                            tax_sign: event.tax_sign,
                            tax_magnitude: event.tax_magnitude,
                            net_sign: event.net_sign,
                            net_magnitude: event.net_magnitude,
                            adjustment_digest: event.adjustment_digest,
                            adjustment_evidence_digest: event.evidence_digest,
                            evidence_digest,
                            approver: event.approver,
                        });
                    decoded.raw_evidence.push(RawChainEvidence {
                        signature: signature.to_string(),
                        slot,
                        event_index,
                        event_discriminator: discriminator,
                        event_data,
                        evidence_digest,
                    });
                    event_index = event_index.checked_add(1).ok_or_else(|| {
                        ProviderError::InvalidResponse(
                            "too many supported events in one transaction".to_string(),
                        )
                    })?;
                    continue;
                }
                if discriminator == ADJUSTMENT_REFUND_EXECUTED_EVENT_DISCRIMINATOR {
                    let event = AdjustmentRefundExecutedEvent::decode(&event_data)?;
                    decoded
                        .refund_executions
                        .push(IndexedAdjustmentRefundExecution {
                            signature: signature.to_string(),
                            slot,
                            event_index,
                            claim: event.claim,
                            adjustment: event.adjustment,
                            original_receipt: event.original_receipt,
                            merchant: event.merchant,
                            mint: event.mint,
                            nonce: event.nonce,
                            gross_magnitude: event.gross_magnitude,
                            tax_magnitude: event.tax_magnitude,
                            net_magnitude: event.net_magnitude,
                            refund_funder: event.refund_funder,
                            refund_destination: event.refund_destination,
                            approver: event.approver,
                            reference_digest: event.reference_digest,
                            evidence_digest,
                        });
                    decoded.raw_evidence.push(RawChainEvidence {
                        signature: signature.to_string(),
                        slot,
                        event_index,
                        event_discriminator: discriminator,
                        event_data,
                        evidence_digest,
                    });
                    event_index = event_index.checked_add(1).ok_or_else(|| {
                        ProviderError::InvalidResponse(
                            "too many supported events in one transaction".to_string(),
                        )
                    })?;
                    continue;
                }
                let (
                    receipt_key,
                    payer,
                    merchant,
                    mint,
                    nonce,
                    gross_atomic,
                    tax_atomic,
                    net_atomic,
                    policy_digest,
                ) = if discriminator == SETTLEMENT_EVENT_DISCRIMINATOR {
                    let event = SettlementExecutedEvent::decode(&event_data)?;
                    (
                        event.receipt,
                        event.payer,
                        event.merchant,
                        event.mint,
                        event.nonce,
                        event.gross_atomic,
                        event.tax_atomic,
                        event.net_atomic,
                        event.policy_digest,
                    )
                } else if discriminator == DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR {
                    let event = DocumentSettlementExecutedEvent::decode(&event_data)?;
                    (
                        event.receipt,
                        event.payer,
                        event.merchant,
                        event.mint,
                        event.nonce,
                        event.gross_atomic,
                        event.tax_atomic,
                        event.net_atomic,
                        event.policy_digest,
                    )
                } else if discriminator == APPLICABILITY_SETTLEMENT_EVENT_DISCRIMINATOR
                    || discriminator == APPLICABILITY_DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR
                {
                    let event = ApplicabilitySettlementExecutedEvent::decode(&event_data)?;
                    (
                        event.receipt,
                        event.payer,
                        event.merchant,
                        event.mint,
                        event.nonce,
                        event.gross_atomic,
                        event.tax_atomic,
                        event.net_atomic,
                        event.policy_digest,
                    )
                } else {
                    let event = SplitSettlementExecutedEvent::decode(&event_data)?;
                    decoded.split_settlements.push(IndexedSplitSettlement {
                        signature: signature.to_string(),
                        slot,
                        receipt: event.receipt,
                        payer: event.payer,
                        merchant: event.merchant,
                        mint: event.mint,
                        nonce: event.nonce,
                        gross_atomic: event.gross_atomic,
                        tax_atomic: event.tax_atomic,
                        net_atomic: event.net_atomic,
                        split_plan_version: event.split_plan_version,
                        split_plan_digest: event.split_plan_digest,
                        policy_digest: event.policy_digest,
                        evidence_digest,
                    });
                    (
                        event.receipt,
                        event.payer,
                        event.merchant,
                        event.mint,
                        event.nonce,
                        event.gross_atomic,
                        event.tax_atomic,
                        event.net_atomic,
                        event.policy_digest,
                    )
                };
                decoded.receipts.push(IndexedReceipt {
                    signature: signature.to_string(),
                    slot,
                    receipt: receipt_key,
                    payer,
                    merchant,
                    mint,
                    nonce,
                    gross_atomic,
                    tax_atomic,
                    net_atomic,
                    policy_digest,
                    evidence_digest,
                });
                decoded.raw_evidence.push(RawChainEvidence {
                    signature: signature.to_string(),
                    slot,
                    event_index,
                    event_discriminator: discriminator,
                    event_data,
                    evidence_digest,
                });
                event_index = event_index.checked_add(1).ok_or_else(|| {
                    ProviderError::InvalidResponse(
                        "too many settlement events in one transaction".to_string(),
                    )
                })?;
            }
        }
        Ok(decoded)
    }
}

pub fn settlement_evidence_digest(
    signature: &str,
    slot: u64,
    event_index: u16,
    event_data: &[u8],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"solana-tax-protocol/settlement-event/v1");
    hasher.update(signature.as_bytes());
    hasher.update(slot.to_le_bytes());
    hasher.update(event_index.to_le_bytes());
    hasher.update(event_data);
    hasher.finalize().into()
}

pub fn protocol_pause_evidence_digest(
    signature: &str,
    slot: u64,
    event_index: u16,
    authority: [u8; 32],
    paused: bool,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"solana-tax-protocol/protocol-pause-event/v1");
    hasher.update(signature.as_bytes());
    hasher.update(slot.to_le_bytes());
    hasher.update(event_index.to_le_bytes());
    hasher.update(authority);
    hasher.update([u8::from(paused)]);
    hasher.finalize().into()
}

pub fn raw_transaction_digest(
    signature: &str,
    slot: u64,
    transaction_index: u32,
    payload: &[u8],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"solana-tax-protocol/raw-transaction/v1");
    hasher.update(signature.as_bytes());
    hasher.update(slot.to_le_bytes());
    hasher.update(transaction_index.to_le_bytes());
    hasher.update(payload);
    hasher.finalize().into()
}

fn read_array<const N: usize>(data: &[u8], offset: &mut usize) -> Result<[u8; N], ProviderError> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| ProviderError::InvalidResponse("event offset overflowed".to_string()))?;
    let bytes = data
        .get(*offset..end)
        .ok_or_else(|| ProviderError::InvalidResponse("event payload is truncated".to_string()))?;
    *offset = end;
    bytes.try_into().map_err(|_| {
        ProviderError::InvalidResponse("event field has an invalid length".to_string())
    })
}

fn read_u64_le(data: &[u8], offset: &mut usize) -> Result<u64, ProviderError> {
    Ok(u64::from_le_bytes(read_array::<8>(data, offset)?))
}

fn read_i64_le(data: &[u8], offset: &mut usize) -> Result<i64, ProviderError> {
    Ok(i64::from_le_bytes(read_array::<8>(data, offset)?))
}

fn read_u8(data: &[u8], offset: &mut usize) -> Result<u8, ProviderError> {
    let value = *data
        .get(*offset)
        .ok_or_else(|| ProviderError::InvalidResponse("event payload is truncated".to_string()))?;
    *offset = (*offset)
        .checked_add(1)
        .ok_or_else(|| ProviderError::InvalidResponse("event offset overflowed".to_string()))?;
    Ok(value)
}

pub struct SolanaRpcPageSource<T, D> {
    transport: T,
    decoder: D,
}

impl<T, D> SolanaRpcPageSource<T, D> {
    pub const fn new(transport: T, decoder: D) -> Self {
        Self { transport, decoder }
    }
}

#[async_trait]
impl<T, D> FinalizedPageSource for SolanaRpcPageSource<T, D>
where
    T: JsonRpcTransport,
    D: ReceiptDecoder,
{
    async fn fetch_finalized_page(&self, range: ScanRange) -> Result<FinalizedPage, ProviderError> {
        range.validate()?;
        let length = range.end_slot - range.start_slot + 1;
        if length > MAX_RPC_PAGE_SLOTS {
            return Err(ProviderError::InvalidPage(IndexerError::RangeTooLarge));
        }
        let finalized_slot = rpc_u64(
            self.transport
                .call("getSlot", json!([{ "commitment": "finalized" }]))
                .await?,
            "getSlot",
        )?;
        if finalized_slot < range.end_slot {
            return Err(ProviderError::NonFinalized);
        }
        let blocks = self
            .transport
            .call(
                "getBlocks",
                json!([range.start_slot, range.end_slot, { "commitment": "finalized" }]),
            )
            .await?;
        let block_slots = blocks.as_array().ok_or_else(|| {
            ProviderError::InvalidResponse("getBlocks result must be an array".to_string())
        })?;
        let mut receipts = Vec::new();
        let mut split_settlements = Vec::new();
        let mut adjustment_records = Vec::new();
        let mut refund_executions = Vec::new();
        let mut raw_evidence = Vec::new();
        let mut raw_transactions = Vec::new();
        let mut protocol_pause_changes = Vec::new();
        let mut previous_slot = None;
        for value in block_slots {
            let slot = rpc_u64(value.clone(), "getBlocks slot")?;
            if slot < range.start_slot
                || slot > range.end_slot
                || previous_slot.is_some_and(|previous| slot <= previous)
            {
                return Err(ProviderError::InvalidResponse(
                    "getBlocks returned an unordered or out-of-range slot".to_string(),
                ));
            }
            previous_slot = Some(slot);
            let block = self
                .transport
                .call(
                    "getBlock",
                    json!([slot, {
                        "commitment": "finalized",
                        "encoding": "json",
                        "transactionDetails": "full",
                        "rewards": false,
                        "maxSupportedTransactionVersion": 0
                    }]),
                )
                .await?;
            if block.is_null() {
                return Err(ProviderError::InvalidResponse(
                    "finalized getBlock unexpectedly returned null".to_string(),
                ));
            }
            let decoded = self.decoder.decode_block(slot, block).await?;
            receipts.extend(decoded.receipts);
            split_settlements.extend(decoded.split_settlements);
            adjustment_records.extend(decoded.adjustment_records);
            refund_executions.extend(decoded.refund_executions);
            raw_evidence.extend(decoded.raw_evidence);
            raw_transactions.extend(decoded.raw_transactions);
            protocol_pause_changes.extend(decoded.protocol_pause_changes);
        }
        Ok(FinalizedPage {
            range,
            observed_finalized_slot: finalized_slot,
            receipts,
            split_settlements,
            adjustment_records,
            refund_executions,
            raw_evidence,
            raw_transactions,
            protocol_pause_changes,
            complete: true,
        })
    }
}

fn rpc_u64(value: Value, operation: &str) -> Result<u64, ProviderError> {
    value
        .as_u64()
        .ok_or_else(|| ProviderError::InvalidResponse(format!("{operation} result must be a u64")))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlannedPage {
    pub checkpoint: Checkpoint,
    pub range: ScanRange,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct IndexerCoordinator;

impl IndexerCoordinator {
    pub const fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        checkpoint: Checkpoint,
        max_slots: u64,
    ) -> Result<PlannedPage, IndexerError> {
        Ok(PlannedPage {
            checkpoint,
            range: checkpoint.plan_next_range(max_slots)?,
        })
    }

    pub async fn fetch<S: FinalizedPageSource + ?Sized>(
        &self,
        planned: PlannedPage,
        source: &S,
    ) -> Result<FinalizedPage, ProviderError> {
        let page = source.fetch_finalized_page(planned.range).await?;
        page.validate(planned.range)?;
        Ok(page)
    }

    pub fn advance(
        &self,
        planned: PlannedPage,
        page: FinalizedPage,
    ) -> Result<Checkpoint, IndexerError> {
        page.validate(planned.range)?;
        planned.checkpoint.apply_page(
            page.range,
            page.observed_finalized_slot,
            page.receipts,
            page.complete,
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScanRange {
    pub start_slot: u64,
    pub end_slot: u64,
}

impl ScanRange {
    pub fn validate(self) -> Result<(), IndexerError> {
        if self.end_slot < self.start_slot {
            return Err(IndexerError::InvalidRange);
        }
        let length = self
            .end_slot
            .checked_sub(self.start_slot)
            .and_then(|value| value.checked_add(1))
            .ok_or(IndexerError::InvalidRange)?;
        if length > MAX_SCAN_RANGE_SLOTS {
            return Err(IndexerError::RangeTooLarge);
        }
        Ok(())
    }
}

pub const MAX_REPLAY_GAPS: usize = 1024;
pub const MAX_REPLAY_ATTEMPTS: u32 = 8;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ReplayGapReason {
    IncompleteProviderPage,
    ProviderDisagreement,
    ProjectionCommitFailure,
    OperatorRequested,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReplayGap {
    pub range: ScanRange,
    pub observed_finalized_slot: u64,
    pub reason: ReplayGapReason,
    pub attempts: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReplayWork {
    pub range: ScanRange,
    pub observed_finalized_slot: u64,
    pub reason: ReplayGapReason,
    pub attempt: u32,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ReplayError {
    #[error("replay gap is invalid")]
    InvalidRange,
    #[error("replay gap queue is at capacity")]
    QueueFull,
    #[error("replay gap has exhausted its bounded retry budget")]
    AttemptsExhausted,
}

#[derive(Clone, Debug, Default)]
pub struct ReplayQueue {
    gaps: Vec<ReplayGap>,
}

impl ReplayQueue {
    pub fn enqueue(
        &mut self,
        range: ScanRange,
        observed_finalized_slot: u64,
        reason: ReplayGapReason,
    ) -> Result<(), ReplayError> {
        self.enqueue_gap(ReplayGap {
            range,
            observed_finalized_slot,
            reason,
            attempts: 0,
        })
    }

    pub fn claim_next(&mut self, max_slots: u64) -> Result<Option<ReplayWork>, ReplayError> {
        if max_slots == 0 || max_slots > MAX_SCAN_RANGE_SLOTS {
            return Err(ReplayError::InvalidRange);
        }
        self.gaps.sort_by_key(|gap| gap.range.start_slot);
        let Some(index) = (!self.gaps.is_empty()).then_some(0) else {
            return Ok(None);
        };
        if self.gaps[index].attempts >= MAX_REPLAY_ATTEMPTS {
            return Err(ReplayError::AttemptsExhausted);
        }
        let gap = self.gaps.remove(index);
        let work_end = gap
            .range
            .start_slot
            .checked_add(max_slots - 1)
            .map_or(gap.range.end_slot, |candidate| {
                candidate.min(gap.range.end_slot)
            });
        if work_end < gap.range.end_slot {
            self.gaps.push(ReplayGap {
                range: ScanRange {
                    start_slot: work_end.checked_add(1).ok_or(ReplayError::InvalidRange)?,
                    end_slot: gap.range.end_slot,
                },
                observed_finalized_slot: gap.observed_finalized_slot,
                reason: gap.reason,
                attempts: gap.attempts,
            });
        }
        Ok(Some(ReplayWork {
            range: ScanRange {
                start_slot: gap.range.start_slot,
                end_slot: work_end,
            },
            observed_finalized_slot: gap.observed_finalized_slot,
            reason: gap.reason,
            attempt: gap.attempts + 1,
        }))
    }

    pub fn requeue(&mut self, work: ReplayWork) -> Result<(), ReplayError> {
        if work.attempt >= MAX_REPLAY_ATTEMPTS {
            return Err(ReplayError::AttemptsExhausted);
        }
        self.enqueue_gap(ReplayGap {
            range: work.range,
            observed_finalized_slot: work.observed_finalized_slot,
            reason: work.reason,
            attempts: work.attempt,
        })
    }

    pub fn len(&self) -> usize {
        self.gaps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.gaps.is_empty()
    }

    fn enqueue_gap(&mut self, gap: ReplayGap) -> Result<(), ReplayError> {
        if gap.range.validate().is_err() || gap.attempts > MAX_REPLAY_ATTEMPTS {
            return Err(ReplayError::InvalidRange);
        }
        let mut merged = gap;
        let mut index = 0;
        while index < self.gaps.len() {
            let existing = self.gaps[index];
            if existing.reason == merged.reason && ranges_touch(existing.range, merged.range) {
                merged.range = ScanRange {
                    start_slot: existing.range.start_slot.min(merged.range.start_slot),
                    end_slot: existing.range.end_slot.max(merged.range.end_slot),
                };
                merged.observed_finalized_slot = merged
                    .observed_finalized_slot
                    .max(existing.observed_finalized_slot);
                merged.attempts = merged.attempts.max(existing.attempts);
                self.gaps.remove(index);
                index = 0;
                continue;
            }
            index += 1;
        }
        if self.gaps.len() >= MAX_REPLAY_GAPS {
            return Err(ReplayError::QueueFull);
        }
        self.gaps.push(merged);
        Ok(())
    }
}

fn ranges_touch(left: ScanRange, right: ScanRange) -> bool {
    left.start_slot <= right.end_slot.saturating_add(1)
        && right.start_slot <= left.end_slot.saturating_add(1)
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Checkpoint {
    pub next_slot: u64,
    pub finalized_slot: u64,
    pub generation: u64,
}

impl Checkpoint {
    pub fn new(next_slot: u64, finalized_slot: u64) -> Self {
        Self {
            next_slot,
            finalized_slot,
            generation: 0,
        }
    }

    pub fn plan_next_range(&self, max_slots: u64) -> Result<ScanRange, IndexerError> {
        if max_slots == 0 || max_slots > MAX_SCAN_RANGE_SLOTS {
            return Err(IndexerError::RangeTooLarge);
        }
        let end_slot = self
            .next_slot
            .checked_add(max_slots - 1)
            .ok_or(IndexerError::InvalidRange)?;
        let range = ScanRange {
            start_slot: self.next_slot,
            end_slot,
        };
        range.validate()?;
        Ok(range)
    }

    pub fn apply_page(
        self,
        range: ScanRange,
        observed_finalized_slot: u64,
        receipts: Vec<IndexedReceipt>,
        complete: bool,
    ) -> Result<Self, IndexerError> {
        range.validate()?;
        if range.start_slot != self.next_slot {
            return Err(IndexerError::CheckpointRangeMismatch);
        }
        if !complete {
            return Err(IndexerError::IncompleteRange);
        }
        if observed_finalized_slot < self.finalized_slot || observed_finalized_slot < range.end_slot
        {
            return Err(IndexerError::FinalityViolation);
        }

        let mut signatures = std::collections::HashSet::with_capacity(receipts.len());
        for receipt in receipts {
            receipt.validate(range, observed_finalized_slot)?;
            if !signatures.insert(receipt.signature) {
                return Err(IndexerError::ConflictingEvidence);
            }
        }
        Ok(Self {
            next_slot: range
                .end_slot
                .checked_add(1)
                .ok_or(IndexerError::InvalidRange)?,
            finalized_slot: observed_finalized_slot,
            generation: self
                .generation
                .checked_add(1)
                .ok_or(IndexerError::GenerationOverflow)?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IndexedReceipt {
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

impl IndexedReceipt {
    fn validate_identity(&self) -> Result<(), IndexerError> {
        if self.signature.trim().is_empty() || self.signature.len() > 128 {
            return Err(IndexerError::InvalidReceipt);
        }
        if self.receipt == [0; 32]
            || self.payer == [0; 32]
            || self.merchant == [0; 32]
            || self.mint == [0; 32]
            || self.nonce == 0
            || self.policy_digest == [0; 32]
            || self.gross_atomic == 0
            || self.tax_atomic.checked_add(self.net_atomic) != Some(self.gross_atomic)
            || self.evidence_digest == [0; 32]
        {
            return Err(IndexerError::InvalidReceipt);
        }
        Ok(())
    }

    fn validate(&self, range: ScanRange, observed_finalized_slot: u64) -> Result<(), IndexerError> {
        self.validate_identity()?;
        if self.slot < range.start_slot || self.slot > range.end_slot {
            return Err(IndexerError::ReceiptOutsideRange);
        }
        if self.slot > observed_finalized_slot {
            return Err(IndexerError::InvalidReceipt);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MergeOutcome {
    Inserted,
    Duplicate,
}

pub fn merge_receipt(
    existing: Option<IndexedReceipt>,
    incoming: IndexedReceipt,
) -> Result<MergeOutcome, IndexerError> {
    incoming.validate_identity()?;
    match existing {
        None => Ok(MergeOutcome::Inserted),
        Some(existing) if existing == incoming => Ok(MergeOutcome::Duplicate),
        Some(_) => Err(IndexerError::ConflictingEvidence),
    }
}

pub fn merge_raw_evidence(
    existing: Option<RawChainEvidence>,
    incoming: RawChainEvidence,
) -> Result<MergeOutcome, IndexerError> {
    incoming.validate(
        ScanRange {
            start_slot: incoming.slot,
            end_slot: incoming.slot,
        },
        incoming.slot,
    )?;
    match existing {
        None => Ok(MergeOutcome::Inserted),
        Some(existing) if existing == incoming => Ok(MergeOutcome::Duplicate),
        Some(_) => Err(IndexerError::ConflictingEvidence),
    }
}

pub fn merge_adjustment_refund_execution(
    existing: Option<IndexedAdjustmentRefundExecution>,
    incoming: IndexedAdjustmentRefundExecution,
) -> Result<MergeOutcome, IndexerError> {
    incoming.validate_identity()?;
    match existing {
        None => Ok(MergeOutcome::Inserted),
        Some(existing) if existing == incoming => Ok(MergeOutcome::Duplicate),
        Some(_) => Err(IndexerError::ConflictingEvidence),
    }
}

pub fn merge_settlement_adjustment(
    existing: Option<IndexedSettlementAdjustment>,
    incoming: IndexedSettlementAdjustment,
) -> Result<MergeOutcome, IndexerError> {
    incoming.validate_identity()?;
    match existing {
        None => Ok(MergeOutcome::Inserted),
        Some(existing) if existing == incoming => Ok(MergeOutcome::Duplicate),
        Some(_) => Err(IndexerError::ConflictingEvidence),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BalanceExpectation {
    pub gross_atomic: u64,
    pub tax_atomic: u64,
}

impl BalanceExpectation {
    pub const fn new(gross_atomic: u64, tax_atomic: u64) -> Self {
        Self {
            gross_atomic,
            tax_atomic,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BalanceSnapshot {
    pub gross_atomic: u64,
    pub tax_atomic: u64,
}

impl BalanceSnapshot {
    pub const fn new(gross_atomic: u64, tax_atomic: u64) -> Self {
        Self {
            gross_atomic,
            tax_atomic,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reconciliation {
    Matched,
    Mismatch {
        expected_gross: u64,
        observed_gross: u64,
        expected_tax: u64,
        observed_tax: u64,
    },
}

pub const fn reconcile(expected: BalanceExpectation, observed: BalanceSnapshot) -> Reconciliation {
    if expected.gross_atomic == observed.gross_atomic && expected.tax_atomic == observed.tax_atomic
    {
        Reconciliation::Matched
    } else {
        Reconciliation::Mismatch {
            expected_gross: expected.gross_atomic,
            observed_gross: observed.gross_atomic,
            expected_tax: expected.tax_atomic,
            observed_tax: observed.tax_atomic,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VaultBalanceExpectation {
    pub opening_balance_atomic: u64,
    pub receipt_tax_atomic: u64,
    pub release_atomic: u64,
}

impl VaultBalanceExpectation {
    pub const fn new(
        opening_balance_atomic: u64,
        receipt_tax_atomic: u64,
        release_atomic: u64,
    ) -> Self {
        Self {
            opening_balance_atomic,
            receipt_tax_atomic,
            release_atomic,
        }
    }

    pub fn expected_balance(self) -> Result<u64, VaultBalanceExpectationError> {
        self.opening_balance_atomic
            .checked_add(self.receipt_tax_atomic)
            .ok_or(VaultBalanceExpectationError::Overflow)
            .and_then(|balance| {
                balance
                    .checked_sub(self.release_atomic)
                    .ok_or(VaultBalanceExpectationError::Underflow)
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VaultBalanceExpectationError {
    Overflow,
    Underflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VaultBalanceReconciliation {
    Matched {
        expected_balance: u64,
        observed_balance: u64,
    },
    Mismatch {
        expected_balance: u64,
        observed_balance: u64,
    },
}

pub const fn reconcile_vault_balance(
    expected_balance: u64,
    observed_balance: u64,
) -> VaultBalanceReconciliation {
    if expected_balance == observed_balance {
        VaultBalanceReconciliation::Matched {
            expected_balance,
            observed_balance,
        }
    } else {
        VaultBalanceReconciliation::Mismatch {
            expected_balance,
            observed_balance,
        }
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum IndexerError {
    #[error("scan range is invalid")]
    InvalidRange,
    #[error("scan range exceeds the bounded slot limit")]
    RangeTooLarge,
    #[error("checkpoint page is incomplete")]
    IncompleteRange,
    #[error("scan page does not begin at the checkpoint cursor")]
    CheckpointRangeMismatch,
    #[error("provider returned a different range than requested")]
    ProviderRangeMismatch,
    #[error("scan page is not finalized or the finalized watermark regressed")]
    FinalityViolation,
    #[error("checkpoint generation overflowed")]
    GenerationOverflow,
    #[error("indexed receipt is invalid")]
    InvalidReceipt,
    #[error("indexed receipt lies outside the scanned range")]
    ReceiptOutsideRange,
    #[error("indexed receipt violates amount conservation")]
    AmountConservationViolation,
    #[error("indexed split settlement is invalid")]
    InvalidSplitSettlement,
    #[error("indexed split settlement lies outside the scanned range")]
    SplitSettlementOutsideRange,
    #[error("indexed adjustment refund execution is invalid")]
    InvalidAdjustmentRefundExecution,
    #[error("indexed adjustment refund execution lies outside the scanned range")]
    AdjustmentRefundExecutionOutsideRange,
    #[error("indexed settlement adjustment is invalid")]
    InvalidSettlementAdjustment,
    #[error("indexed settlement adjustment lies outside the scanned range")]
    SettlementAdjustmentOutsideRange,
    #[error("raw chain evidence is invalid")]
    InvalidEvidence,
    #[error("raw chain evidence lies outside the scanned range")]
    EvidenceOutsideRange,
    #[error("raw chain transaction is invalid")]
    InvalidRawTransaction,
    #[error("raw chain transaction lies outside the scanned range")]
    RawTransactionOutsideRange,
    #[error("conflicting evidence was received for an existing identity")]
    ConflictingEvidence,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Mutex, atomic::AtomicUsize};

    struct FakeSource {
        page: FinalizedPage,
    }

    #[async_trait]
    impl FinalizedPageSource for FakeSource {
        async fn fetch_finalized_page(
            &self,
            _range: ScanRange,
        ) -> Result<FinalizedPage, ProviderError> {
            Ok(self.page.clone())
        }
    }

    struct FailingSource;

    #[async_trait]
    impl FinalizedPageSource for FailingSource {
        async fn fetch_finalized_page(
            &self,
            _range: ScanRange,
        ) -> Result<FinalizedPage, ProviderError> {
            Err(ProviderError::Transport("synthetic outage".to_string()))
        }
    }

    struct NonFinalizedSource;

    #[async_trait]
    impl FinalizedPageSource for NonFinalizedSource {
        async fn fetch_finalized_page(
            &self,
            _range: ScanRange,
        ) -> Result<FinalizedPage, ProviderError> {
            Err(ProviderError::NonFinalized)
        }
    }

    struct StaticRefundSource {
        balance: FinalizedRefundBalances,
        failing: bool,
    }

    struct StaticApplicabilitySource {
        binding: FinalizedApplicabilityBinding,
        failing: bool,
    }

    #[async_trait]
    impl FinalizedApplicabilityBindingSource for StaticApplicabilitySource {
        async fn fetch_finalized_applicability_binding(
            &self,
            _target: &ApplicabilityBindingTarget,
        ) -> Result<FinalizedApplicabilityBinding, ProviderError> {
            if self.failing {
                Err(ProviderError::Transport(
                    "synthetic applicability outage".to_string(),
                ))
            } else {
                Ok(self.binding)
            }
        }
    }

    #[async_trait]
    impl FinalizedRefundBalanceSource for StaticRefundSource {
        async fn fetch_finalized_refund_balances(
            &self,
            _target: &RefundBalanceTarget,
        ) -> Result<FinalizedRefundBalances, ProviderError> {
            if self.failing {
                Err(ProviderError::Transport(
                    "synthetic refund outage".to_string(),
                ))
            } else {
                Ok(self.balance)
            }
        }
    }

    struct StaticRefundTransitionSource {
        transition: FinalizedRefundBalanceTransition,
        failing: bool,
    }

    #[async_trait]
    impl FinalizedRefundBalanceTransitionSource for StaticRefundTransitionSource {
        async fn fetch_finalized_refund_balance_transition(
            &self,
            _target: &RefundBalanceProofTarget,
        ) -> Result<FinalizedRefundBalanceTransition, ProviderError> {
            if self.failing {
                Err(ProviderError::Transport(
                    "synthetic refund transition outage".to_string(),
                ))
            } else {
                Ok(self.transition)
            }
        }
    }

    struct FakeRefundProofCandidates {
        candidate: Mutex<Option<RefundProofCandidate>>,
        failures: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl RefundProofCandidateSource for FakeRefundProofCandidates {
        async fn claim_next_refund_proof_candidate(
            &self,
        ) -> Result<Option<RefundProofCandidate>, RefundProofWorkerError> {
            self.candidate
                .lock()
                .map_err(|_| RefundProofWorkerError::Persistence("candidate mutex poisoned".into()))
                .map(|mut candidate| candidate.take())
        }

        async fn record_refund_proof_failure(
            &self,
            _candidate: &RefundProofCandidate,
            reason: &str,
        ) -> Result<(), RefundProofWorkerError> {
            self.failures
                .lock()
                .map_err(|_| RefundProofWorkerError::Persistence("failure mutex poisoned".into()))?
                .push(reason.to_string());
            Ok(())
        }
    }

    struct FakeRefundProofProvider {
        target: RefundBalanceProofTarget,
        transition: FinalizedRefundBalanceTransition,
    }

    #[async_trait]
    impl FinalizedRefundBalanceProofSource for FakeRefundProofProvider {
        async fn fetch_finalized_refund_balance_proof(
            &self,
            _request: &RefundClaimTopologyRequest,
            _signature: &str,
        ) -> Result<(RefundBalanceProofTarget, FinalizedRefundBalanceTransition), ProviderError>
        {
            Ok((self.target.clone(), self.transition))
        }
    }

    struct CapturingRefundProofSink {
        evidence: Mutex<Option<RefundProofEvidence>>,
    }

    #[async_trait]
    impl RefundProofSink for CapturingRefundProofSink {
        async fn persist_refund_proof(
            &self,
            evidence: RefundProofEvidence,
        ) -> Result<(), RefundProofWorkerError> {
            self.evidence
                .lock()
                .map_err(|_| RefundProofWorkerError::Persistence("sink mutex poisoned".into()))?
                .replace(evidence);
            Ok(())
        }
    }

    fn refund_proof_worker_fixture() -> (
        RefundProofCandidate,
        RefundBalanceProofTarget,
        FinalizedRefundBalanceTransition,
    ) {
        let execution = IndexedAdjustmentRefundExecution {
            signature: test_signature(42),
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
        let pubkey = |bytes: [u8; 32]| Pubkey::new_from_array(bytes).to_string();
        let topology_request = RefundClaimTopologyRequest {
            claim: execution.claim,
            adjustment: execution.adjustment,
            original_receipt: execution.original_receipt,
            merchant: execution.merchant,
            mint: execution.mint,
            claim_nonce: execution.nonce,
            gross_atomic: execution.gross_magnitude,
            tax_atomic: execution.tax_magnitude,
            net_atomic: execution.net_magnitude,
            refund_funder: execution.refund_funder,
            refund_destination: execution.refund_destination,
            reference_digest: execution.reference_digest,
        };
        let target = RefundBalanceProofTarget {
            signature: execution.signature.clone(),
            topology: RefundBalanceTarget {
                claim: pubkey(execution.claim),
                adjustment: pubkey(execution.adjustment),
                original_receipt: pubkey(execution.original_receipt),
                merchant: pubkey(execution.merchant),
                claim_nonce: execution.nonce,
                tax_vault: pubkey([11; 32]),
                refund_source: pubkey([12; 32]),
                payer_destination: pubkey(execution.refund_destination),
                mint: pubkey(execution.mint),
                token_program: settlement_client::SPL_TOKEN_PROGRAM_ID.to_string(),
                vault_authority: pubkey([14; 32]),
                refund_funder: pubkey(execution.refund_funder),
                payer: pubkey([15; 32]),
            },
            gross_atomic: execution.gross_magnitude,
            tax_atomic: execution.tax_magnitude,
            net_atomic: execution.net_magnitude,
            reference_digest: execution.reference_digest,
        };
        let transition = FinalizedRefundBalanceTransition {
            slot: execution.slot,
            tax_vault_before_atomic: 125,
            tax_vault_after_atomic: 100,
            refund_source_before_atomic: 175,
            refund_source_after_atomic: 100,
            payer_destination_before_atomic: 0,
            payer_destination_after_atomic: 100,
        };
        (
            RefundProofCandidate {
                stream_key: "mainnet/settlement/v1".to_string(),
                lease_token: Uuid::now_v7(),
                execution,
                topology_request,
            },
            target,
            transition,
        )
    }

    #[tokio::test]
    async fn refund_proof_worker_processes_one_bound_candidate_and_persists_exact_proof() {
        let (candidate, target, transition) = refund_proof_worker_fixture();
        let signature = candidate.execution.signature.clone();
        let sink = Arc::new(CapturingRefundProofSink {
            evidence: Mutex::new(None),
        });
        let worker = RefundProofWorker::new(
            Arc::new(FakeRefundProofCandidates {
                candidate: Mutex::new(Some(candidate)),
                failures: Mutex::new(Vec::new()),
            }),
            Arc::new(FakeRefundProofProvider { target, transition }),
            sink.clone(),
        );

        assert_eq!(
            worker.process_one().await.unwrap(),
            RefundProofCycleResult::Persisted {
                signature,
                slot: 777
            }
        );
        let evidence = sink.evidence.lock().unwrap().clone().unwrap();
        assert_eq!(evidence.stream_key, "mainnet/settlement/v1");
        assert_eq!(evidence.proof.gross_atomic, 100);
        assert_eq!(evidence.proof.tax_atomic, 25);
        assert_eq!(evidence.proof.net_atomic, 75);
        assert_eq!(evidence.proof.finalized_slot, 777);
    }

    #[tokio::test]
    async fn refund_proof_worker_has_one_candidate_bound_and_empty_cycle() {
        let (candidate, target, transition) = refund_proof_worker_fixture();
        let worker = RefundProofWorker::new(
            Arc::new(FakeRefundProofCandidates {
                candidate: Mutex::new(Some(candidate)),
                failures: Mutex::new(Vec::new()),
            }),
            Arc::new(FakeRefundProofProvider { target, transition }),
            Arc::new(CapturingRefundProofSink {
                evidence: Mutex::new(None),
            }),
        );

        worker.process_one().await.unwrap();
        assert_eq!(
            worker.process_one().await.unwrap(),
            RefundProofCycleResult::NoCandidate
        );
    }

    #[tokio::test]
    async fn refund_proof_worker_rejects_provider_topology_substitution_before_persistence() {
        let (candidate, mut target, transition) = refund_proof_worker_fixture();
        target.topology.payer_destination = Pubkey::new_from_array([16; 32]).to_string();
        let sink = Arc::new(CapturingRefundProofSink {
            evidence: Mutex::new(None),
        });
        let candidates = Arc::new(FakeRefundProofCandidates {
            candidate: Mutex::new(Some(candidate)),
            failures: Mutex::new(Vec::new()),
        });
        let worker = RefundProofWorker::new(
            candidates.clone(),
            Arc::new(FakeRefundProofProvider { target, transition }),
            sink.clone(),
        );

        assert!(matches!(
            worker.process_one().await,
            Err(RefundProofWorkerError::InvalidCandidate(message))
                if message.contains("refund destination")
        ));
        assert!(sink.evidence.lock().unwrap().is_none());
        assert_eq!(
            candidates.failures.lock().unwrap().as_slice(),
            ["invalid_candidate"]
        );
    }

    struct CountingSource {
        calls: Arc<AtomicUsize>,
        page: FinalizedPage,
        failing: bool,
    }

    #[async_trait]
    impl FinalizedPageSource for CountingSource {
        async fn fetch_finalized_page(
            &self,
            _range: ScanRange,
        ) -> Result<FinalizedPage, ProviderError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            if self.failing {
                Err(ProviderError::Transport("synthetic outage".to_string()))
            } else {
                Ok(self.page.clone())
            }
        }
    }

    struct FakeRpc {
        results: Mutex<VecDeque<Value>>,
        calls: Mutex<Vec<(String, Value)>>,
    }

    #[async_trait]
    impl JsonRpcTransport for FakeRpc {
        async fn call(&self, method: &str, params: Value) -> Result<Value, ProviderError> {
            self.calls
                .lock()
                .map_err(|_| ProviderError::Transport("test mutex poisoned".to_string()))?
                .push((method.to_string(), params));
            self.results
                .lock()
                .map_err(|_| ProviderError::Transport("test mutex poisoned".to_string()))?
                .pop_front()
                .ok_or_else(|| ProviderError::Transport("test response missing".to_string()))
        }
    }

    #[tokio::test]
    async fn applicability_binding_verifier_requires_finalized_typed_account_state() {
        let receipt = Pubkey::new_from_array([1; 32]);
        let merchant = Pubkey::new_from_array([2; 32]);
        let mint = Pubkey::new_from_array([3; 32]);
        let context_id = [4; 16];
        let attestation_digest = [5; 32];
        let (binding, bump) = Pubkey::find_program_address(
            &[b"applicability-binding", receipt.as_ref()],
            &settlement_client::settlement_program_id(),
        );
        let mut discriminator = Sha256::new();
        discriminator.update(b"account:ApplicabilitySettlementBinding");
        let mut account_data = discriminator.finalize()[..8].to_vec();
        account_data.extend_from_slice(receipt.as_ref());
        account_data.extend_from_slice(merchant.as_ref());
        account_data.extend_from_slice(mint.as_ref());
        account_data.extend_from_slice(&context_id);
        account_data.extend_from_slice(&attestation_digest);
        account_data.push(bump);
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([json!({
                "context": {"slot": 99},
                "value": [{
                    "owner": settlement_client::SETTLEMENT_PROGRAM_ID,
                    "executable": false,
                    "lamports": 1,
                    "data": [BASE64.encode(account_data), "base64"]
                }]
            })])),
            calls: Mutex::new(Vec::new()),
        };
        let source = SolanaRpcApplicabilityBindingSource::new(rpc);
        let verified = source
            .fetch_finalized_applicability_binding(&ApplicabilityBindingTarget {
                binding: binding.to_string(),
                receipt: receipt.to_string(),
                merchant: merchant.to_string(),
                mint: mint.to_string(),
                context_id,
                attestation_digest,
            })
            .await
            .unwrap();

        assert_eq!(verified.slot, 99);
        assert_eq!(verified.binding, binding.to_bytes());
        assert_eq!(verified.receipt, receipt.to_bytes());
        assert_eq!(verified.merchant, merchant.to_bytes());
        assert_eq!(verified.mint, mint.to_bytes());
        assert_eq!(verified.context_id, context_id);
        assert_eq!(verified.attestation_digest, attestation_digest);
    }

    fn test_signature(seed: u8) -> String {
        Signature::from([seed; 64]).to_string()
    }

    #[tokio::test]
    async fn signature_pages_are_finalized_bounded_and_cursored() {
        let first = test_signature(1);
        let second = test_signature(2);
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([json!([
                {
                    "signature": first,
                    "slot": 20,
                    "err": null,
                    "confirmationStatus": "finalized"
                },
                {
                    "signature": second,
                    "slot": 19,
                    "err": {"InstructionError": [0, "Custom"]},
                    "confirmationStatus": "finalized"
                }
            ])])),
            calls: Mutex::new(Vec::new()),
        };
        let source = SolanaRpcSignaturePageSource::new(rpc);
        let page = source
            .fetch_page(&Pubkey::new_from_array([9; 32]).to_string(), None, 2)
            .await
            .unwrap();

        assert_eq!(page.entries.len(), 2);
        assert!(page.entries[0].succeeded);
        assert!(!page.entries[1].succeeded);
        assert!(!page.complete);
        assert_eq!(page.next_before, Some(second.clone()));
        let calls = source.transport.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "getSignaturesForAddress");
        assert_eq!(calls[0].1[1]["commitment"], "finalized");
        assert_eq!(calls[0].1[1]["limit"], 2);
        assert!(calls[0].1[1].get("before").is_none());
    }

    #[tokio::test]
    async fn complete_initial_signature_page_does_not_report_cursor_stall() {
        let signature = test_signature(6);
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([json!([
                {
                    "signature": signature,
                    "slot": 20,
                    "err": null,
                    "confirmationStatus": "finalized"
                }
            ])])),
            calls: Mutex::new(Vec::new()),
        };
        let source = SolanaRpcSignaturePageSource::new(rpc);

        let page = source
            .fetch_page(&Pubkey::new_from_array([9; 32]).to_string(), None, 10)
            .await
            .unwrap();

        assert!(page.complete);
        assert!(page.next_before.is_none());
    }

    #[tokio::test]
    async fn signature_page_cursor_is_sent_only_after_canonical_validation() {
        let cursor = test_signature(3);
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([json!([])])),
            calls: Mutex::new(Vec::new()),
        };
        let source = SolanaRpcSignaturePageSource::new(rpc);
        let page = source
            .fetch_page(
                &Pubkey::new_from_array([9; 32]).to_string(),
                Some(&cursor),
                10,
            )
            .await
            .unwrap();
        assert_eq!(page.before, Some(cursor.clone()));
        assert!(page.complete);
        assert!(page.next_before.is_none());
        let calls = source.transport.calls.lock().unwrap();
        assert_eq!(calls[0].1[1]["before"], cursor);
        assert_eq!(calls[0].1[1]["limit"], 10);
    }

    #[tokio::test]
    async fn signature_pages_fail_closed_on_duplicates_non_finality_and_bad_limits() {
        let duplicate = test_signature(4);
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([
                json!([
                    {"signature": duplicate, "slot": 1, "err": null, "confirmationStatus": "finalized"},
                    {"signature": test_signature(4), "slot": 1, "err": null, "confirmationStatus": "finalized"}
                ]),
                json!([
                    {"signature": test_signature(5), "slot": 1, "err": null, "confirmationStatus": "confirmed"}
                ]),
            ])),
            calls: Mutex::new(Vec::new()),
        };
        let source = SolanaRpcSignaturePageSource::new(rpc);
        let address = Pubkey::new_from_array([9; 32]).to_string();
        assert!(
            matches!(source.fetch_page(&address, None, 2).await, Err(ProviderError::InvalidResponse(message)) if message.contains("duplicate"))
        );
        assert_eq!(
            source.fetch_page(&address, None, 1).await,
            Err(ProviderError::NonFinalized)
        );
        assert!(
            matches!(source.fetch_page(&address, None, 0).await, Err(ProviderError::InvalidResponse(message)) if message.contains("limit"))
        );
        assert!(
            matches!(source.fetch_page("not-a-key", None, 1).await, Err(ProviderError::InvalidResponse(message)) if message.contains("public key"))
        );
    }

    #[tokio::test]
    async fn finalized_transaction_source_binds_get_transaction_to_the_target() {
        let target = test_signature(6);
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([json!({
                "slot": 42,
                "transaction": {"signatures": [target], "message": {}},
                "meta": {"err": {"InstructionError": [0, "Custom"]}}
            })])),
            calls: Mutex::new(Vec::new()),
        };
        let source = SolanaRpcTransactionSource::new(rpc, SettlementExecutedDecoder);
        let decoded = source.fetch_finalized_transaction(&target).await.unwrap();
        assert_eq!(decoded.raw_transactions.len(), 1);
        assert_eq!(decoded.raw_transactions[0].signature, target);
        assert_eq!(decoded.raw_transactions[0].slot, 42);
        let calls = source.transport.calls.lock().unwrap();
        assert_eq!(calls[0].0, "getTransaction");
        assert_eq!(calls[0].1[1]["commitment"], "finalized");
        assert_eq!(calls[0].1[1]["maxSupportedTransactionVersion"], 0);
    }

    #[tokio::test]
    async fn finalized_transaction_source_rejects_signature_substitution() {
        let target = test_signature(7);
        let replacement = test_signature(8);
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([json!({
                "slot": 42,
                "transaction": {"signatures": [replacement], "message": {}},
                "meta": {"err": {"InstructionError": [0, "Custom"]}}
            })])),
            calls: Mutex::new(Vec::new()),
        };
        let source = SolanaRpcTransactionSource::new(rpc, SettlementExecutedDecoder);
        assert!(matches!(
            source.fetch_finalized_transaction(&target).await,
            Err(ProviderError::InvalidResponse(message)) if message.contains("does not match")
        ));
    }

    fn verified_route_fixture() -> (SettlementRouteConfig, Value) {
        let merchant = Pubkey::new_from_array([1; 32]);
        let mint = Pubkey::new_from_array([2; 32]);
        let tax_vault = Pubkey::new_from_array([3; 32]);
        let destination = Pubkey::new_from_array([4; 32]);
        let (asset_config, asset_config_bump) = Pubkey::find_program_address(
            &[b"asset-config", mint.as_ref()],
            &settlement_client::settlement_program_id(),
        );
        let (vault_authority, _) = Pubkey::find_program_address(
            &[b"vault-authority", merchant.as_ref(), mint.as_ref()],
            &settlement_client::settlement_program_id(),
        );
        let mut mint_data = vec![0; 82];
        mint_data[44] = 6;
        let mut tax_data = vec![0; 165];
        tax_data[..32].copy_from_slice(mint.as_ref());
        tax_data[32..64].copy_from_slice(vault_authority.as_ref());
        let mut destination_data = vec![0; 165];
        destination_data[..32].copy_from_slice(mint.as_ref());
        destination_data[32..64].copy_from_slice(Pubkey::new_from_array([9; 32]).as_ref());
        let mut asset_config_data = vec![0; ASSET_CONFIG_ACCOUNT_DATA_LEN];
        asset_config_data[..8].copy_from_slice(&ASSET_CONFIG_ACCOUNT_DISCRIMINATOR);
        asset_config_data[8..40].copy_from_slice(mint.as_ref());
        asset_config_data[40..72].copy_from_slice(
            settlement_client::SPL_TOKEN_PROGRAM_ID
                .parse::<Pubkey>()
                .expect("valid token program")
                .as_ref(),
        );
        asset_config_data[72] = 6;
        asset_config_data[73] = STABLECOIN_ASSET_KIND;
        asset_config_data[74..106].copy_from_slice(Pubkey::new_from_array([9; 32]).as_ref());
        asset_config_data[106] = 1;
        asset_config_data[107] = asset_config_bump;
        let account = |address: &Pubkey, owner: &str, data: Vec<u8>| {
            json!({
                "lamports": 1_000_000,
                "owner": owner,
                "executable": false,
                "data": [BASE64.encode(data), "base64"],
                "address": address.to_string()
            })
        };
        let route = SettlementRouteConfig {
            route_id: uuid::Uuid::from_u128(1),
            version: 1,
            merchant_account: merchant.to_string(),
            mint: mint.to_string(),
            tax_vault: tax_vault.to_string(),
            net_destination: destination.to_string(),
            token_program: settlement_client::SPL_TOKEN_PROGRAM_ID.to_string(),
        };
        let response = json!({
            "context": {"slot": 777},
            "value": [
                account(&merchant, settlement_client::SYSTEM_PROGRAM_ID, Vec::new()),
                account(&mint, settlement_client::SPL_TOKEN_PROGRAM_ID, mint_data),
                account(&tax_vault, settlement_client::SPL_TOKEN_PROGRAM_ID, tax_data),
                account(&destination, settlement_client::SPL_TOKEN_PROGRAM_ID, destination_data),
                account(
                    &asset_config,
                    settlement_client::settlement_program_id().to_string().as_str(),
                    asset_config_data,
                )
            ]
        });
        (route, response)
    }

    fn verified_refund_funding_fixture() -> (RefundFundingTopologyConfig, Value) {
        let merchant = Pubkey::new_from_array([11; 32]);
        let mint = Pubkey::new_from_array([12; 32]);
        let tax_vault = Pubkey::new_from_array([13; 32]);
        let refund_funder = Pubkey::new_from_array([14; 32]);
        let net_source = Pubkey::new_from_array([15; 32]);
        let (vault_authority, _) = Pubkey::find_program_address(
            &[b"vault-authority", merchant.as_ref(), mint.as_ref()],
            &settlement_client::settlement_program_id(),
        );
        let mut mint_data = vec![0; 82];
        mint_data[44] = 6;
        let mut tax_data = vec![0; 165];
        tax_data[..32].copy_from_slice(mint.as_ref());
        tax_data[32..64].copy_from_slice(vault_authority.as_ref());
        let mut source_data = vec![0; 165];
        source_data[..32].copy_from_slice(mint.as_ref());
        source_data[32..64].copy_from_slice(refund_funder.as_ref());
        let account = |owner: &str, data: Vec<u8>| {
            json!({
                "lamports": 1_000_000,
                "owner": owner,
                "executable": false,
                "data": [BASE64.encode(data), "base64"]
            })
        };
        let topology = RefundFundingTopologyConfig {
            configuration_id: uuid::Uuid::from_u128(2),
            version: 1,
            configuration_digest: [22; 32],
            merchant_account: merchant.to_string(),
            mint: mint.to_string(),
            tax_vault: tax_vault.to_string(),
            vault_authority: vault_authority.to_string(),
            refund_funder: refund_funder.to_string(),
            net_source: net_source.to_string(),
            token_program: settlement_client::SPL_TOKEN_PROGRAM_ID.to_string(),
        };
        let response = json!({
            "context": {"slot": 888},
            "value": [
                account(settlement_client::SYSTEM_PROGRAM_ID, Vec::new()),
                account(settlement_client::SPL_TOKEN_PROGRAM_ID, mint_data),
                account(settlement_client::SPL_TOKEN_PROGRAM_ID, tax_data),
                account(settlement_client::SYSTEM_PROGRAM_ID, Vec::new()),
                account(settlement_client::SPL_TOKEN_PROGRAM_ID, source_data)
            ]
        });
        (topology, response)
    }

    struct EmptyReceiptDecoder;

    #[async_trait]
    impl ReceiptDecoder for EmptyReceiptDecoder {
        async fn decode_block(
            &self,
            _slot: u64,
            _block: Value,
        ) -> Result<DecodedBlock, ProviderError> {
            Ok(DecodedBlock::default())
        }
    }

    #[test]
    fn refund_claim_event_decoder_binds_all_finality_fields() {
        let mut data = Vec::with_capacity(8 + (32 * 9) + (8 * 6));
        data.extend_from_slice(&ADJUSTMENT_REFUND_CLAIM_CREATED_EVENT_DISCRIMINATOR);
        for value in 1_u8..=5 {
            data.extend_from_slice(&[value; 32]);
        }
        for value in [3_u64, 100, 25, 75] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        for value in 6_u8..=9 {
            data.extend_from_slice(&[value; 32]);
        }
        data.extend_from_slice(&10_i64.to_le_bytes());
        data.extend_from_slice(&20_i64.to_le_bytes());

        let evidence = decode_refund_claim_created_event("claim-signature", 77, 2, &data, [1; 32])
            .expect("valid claim event");
        assert_eq!(evidence.signature, "claim-signature");
        assert_eq!(evidence.slot, 77);
        assert_eq!(evidence.event_index, 2);
        assert_eq!(evidence.claim, [1; 32]);
        assert_eq!(evidence.proposer, [8; 32]);
        assert_eq!(evidence.execute_after_unix, 10);
        assert_eq!(evidence.expires_at_unix, 20);

        data[8 + (32 * 5) + (8 * 4) + 32 + 32..8 + (32 * 5) + (8 * 4) + 32 + (32 * 2)].fill(6);
        assert!(
            decode_refund_claim_created_event("claim-signature", 77, 2, &data, [1; 32]).is_err()
        );
    }

    #[test]
    fn refund_claim_cancellation_decoder_binds_governance_and_reference() {
        let mut data = Vec::with_capacity(8 + (32 * 6) + 8);
        data.extend_from_slice(&ADJUSTMENT_REFUND_CLAIM_CANCELLED_EVENT_DISCRIMINATOR);
        for value in 1_u8..=4 {
            data.extend_from_slice(&[value; 32]);
        }
        data.extend_from_slice(&9_u64.to_le_bytes());
        data.extend_from_slice(&[5; 32]);
        data.extend_from_slice(&[6; 32]);

        let evidence =
            decode_refund_claim_cancelled_event("cancellation-signature", 88, 3, &data, [7; 32])
                .expect("valid cancellation event");
        assert_eq!(evidence.claim, [1; 32]);
        assert_eq!(evidence.governance_authority, [5; 32]);
        assert_eq!(evidence.reference_digest, [6; 32]);

        data[8..40].fill(0);
        assert!(
            decode_refund_claim_cancelled_event("cancellation-signature", 88, 3, &data, [7; 32],)
                .is_err()
        );
    }

    #[tokio::test]
    async fn route_verifier_requires_finalized_account_topology() {
        let (route, response) = verified_route_fixture();
        let expected_asset_config_bump = response["value"][4]["data"][0]
            .as_str()
            .and_then(|encoded| BASE64.decode(encoded).ok())
            .map(|data| data[107])
            .expect("fixture asset configuration has a PDA bump");
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([response])),
            calls: Mutex::new(Vec::new()),
        };
        let evidence = verify_settlement_route(&rpc, &route).await.unwrap();
        assert_eq!(evidence.verification_slot, 777);
        assert_eq!(evidence.mint_decimals, 6);
        assert_eq!(evidence.asset_kind, STABLECOIN_ASSET_KIND);
        assert!(evidence.asset_enabled);
        assert_eq!(evidence.asset_config_bump, expected_asset_config_bump);
        assert_eq!(evidence.accounts.len(), 5);
        assert_ne!(evidence.verification_digest, [0; 32]);
        let calls = rpc.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "getMultipleAccounts");
        assert_eq!(calls[0].1[1]["commitment"], "finalized");
        assert_eq!(calls[0].1[1]["encoding"], "base64");
        assert_eq!(calls[0].1[0].as_array().unwrap().len(), 5);
    }

    #[tokio::test]
    async fn route_verifier_rejects_disabled_asset_configuration() {
        let (route, mut response) = verified_route_fixture();
        let encoded = response["value"][4]["data"][0].as_str().unwrap();
        let mut data = BASE64.decode(encoded).unwrap();
        data[106] = 0;
        response["value"][4]["data"][0] = Value::String(BASE64.encode(data));
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([response])),
            calls: Mutex::new(Vec::new()),
        };
        assert_eq!(
            verify_settlement_route(&rpc, &route).await,
            Err(RouteVerificationError::AssetConfigDisabled)
        );
    }

    #[tokio::test]
    async fn revenue_destination_verifier_binds_owner_mint_and_finality() {
        let mint = Pubkey::new_from_array([21; 32]);
        let destination = Pubkey::new_from_array([22; 32]);
        let recipient_owner = Pubkey::new_from_array([23; 32]);
        let mut mint_data = vec![0; 82];
        mint_data[44] = 6;
        let mut destination_data = vec![0; 165];
        destination_data[..32].copy_from_slice(mint.as_ref());
        destination_data[32..64].copy_from_slice(recipient_owner.as_ref());
        let account = |owner: &str, data: Vec<u8>| {
            json!({
                "lamports": 1_000_000,
                "owner": owner,
                "executable": false,
                "data": [BASE64.encode(data), "base64"]
            })
        };
        let route = RevenueStreamDestinationRouteConfig {
            route_id: uuid::Uuid::from_u128(23),
            version: 1,
            recipient_owner: recipient_owner.to_string(),
            mint: mint.to_string(),
            destination_token_account: destination.to_string(),
            token_program: settlement_client::SPL_TOKEN_PROGRAM_ID.to_string(),
        };
        let response = json!({
            "context": {"slot": 901},
            "value": [
                account(settlement_client::SPL_TOKEN_PROGRAM_ID, mint_data),
                account(settlement_client::SPL_TOKEN_PROGRAM_ID, destination_data)
            ]
        });
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([response])),
            calls: Mutex::new(Vec::new()),
        };
        let evidence = verify_revenue_stream_destination_route(&rpc, &route)
            .await
            .unwrap();
        assert_eq!(evidence.verification_slot, 901);
        assert_eq!(evidence.mint_decimals, 6);
        assert_ne!(evidence.verification_digest, [0; 32]);
        let calls = rpc.calls.lock().unwrap();
        assert_eq!(calls[0].1[1]["commitment"], "finalized");
        assert_eq!(calls[0].1[0].as_array().unwrap().len(), 2);

        let mut tampered = evidence;
        tampered.destination_token_account = Pubkey::new_from_array([24; 32]).to_string();
        assert_ne!(
            tampered.destination_token_account,
            route.destination_token_account
        );
    }

    #[tokio::test]
    async fn revenue_destination_verifier_rejects_recipient_owner_substitution() {
        let mint = Pubkey::new_from_array([31; 32]);
        let destination = Pubkey::new_from_array([32; 32]);
        let recipient_owner = Pubkey::new_from_array([33; 32]);
        let mut mint_data = vec![0; 82];
        mint_data[44] = 6;
        let mut destination_data = vec![0; 165];
        destination_data[..32].copy_from_slice(mint.as_ref());
        destination_data[32..64].copy_from_slice(Pubkey::new_from_array([34; 32]).as_ref());
        let account = |data: Vec<u8>| {
            json!({
                "lamports": 1_000_000,
                "owner": settlement_client::SPL_TOKEN_PROGRAM_ID,
                "executable": false,
                "data": [BASE64.encode(data), "base64"]
            })
        };
        let route = RevenueStreamDestinationRouteConfig {
            route_id: uuid::Uuid::from_u128(24),
            version: 1,
            recipient_owner: recipient_owner.to_string(),
            mint: mint.to_string(),
            destination_token_account: destination.to_string(),
            token_program: settlement_client::SPL_TOKEN_PROGRAM_ID.to_string(),
        };
        let response = json!({
            "context": {"slot": 902},
            "value": [account(mint_data), account(destination_data)]
        });
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([response])),
            calls: Mutex::new(Vec::new()),
        };
        assert_eq!(
            verify_revenue_stream_destination_route(&rpc, &route).await,
            Err(RevenueStreamDestinationVerificationError::TokenAuthorityMismatch)
        );
    }

    #[tokio::test]
    async fn refund_funding_verifier_requires_complete_finalized_topology() {
        let (topology, response) = verified_refund_funding_fixture();
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([response])),
            calls: Mutex::new(Vec::new()),
        };
        let evidence = verify_refund_funding_topology(&rpc, &topology)
            .await
            .unwrap();
        assert_eq!(evidence.verification_slot, 888);
        assert_eq!(evidence.mint_decimals, 6);
        assert_eq!(evidence.accounts.len(), 5);
        assert_ne!(evidence.verification_digest, [0; 32]);
        let calls = rpc.calls.lock().unwrap();
        assert_eq!(calls[0].0, "getMultipleAccounts");
        assert_eq!(calls[0].1[1]["commitment"], "finalized");
        assert_eq!(calls[0].1[1]["encoding"], "base64");
        assert_eq!(calls[0].1[0].as_array().unwrap().len(), 5);
    }

    #[tokio::test]
    async fn refund_funding_verifier_rejects_source_authority_substitution() {
        let (topology, mut response) = verified_refund_funding_fixture();
        let encoded = response["value"][4]["data"][0].as_str().unwrap();
        let mut data = BASE64.decode(encoded).unwrap();
        data[32] ^= 1;
        response["value"][4]["data"][0] = Value::String(BASE64.encode(data));
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([response])),
            calls: Mutex::new(Vec::new()),
        };
        assert_eq!(
            verify_refund_funding_topology(&rpc, &topology).await,
            Err(RefundFundingVerificationError::TokenAuthorityMismatch(
                "net_source"
            ))
        );
    }

    #[tokio::test]
    async fn refund_funding_verifier_rejects_token_2022_before_rpc() {
        let (mut topology, _response) = verified_refund_funding_fixture();
        topology.token_program = settlement_client::TOKEN_2022_PROGRAM_ID.to_string();
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([_response])),
            calls: Mutex::new(Vec::new()),
        };
        assert!(matches!(
            verify_refund_funding_topology(&rpc, &topology).await,
            Err(RefundFundingVerificationError::InvalidTopology(message))
                if message.contains("legacy SPL Token")
        ));
        assert!(rpc.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn route_verifier_rejects_vault_authority_substitution() {
        let (route, mut response) = verified_route_fixture();
        let encoded = response["value"][2]["data"][0].as_str().unwrap();
        let mut data = BASE64.decode(encoded).unwrap();
        data[32] ^= 1;
        response["value"][2]["data"][0] = Value::String(BASE64.encode(data));
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([response])),
            calls: Mutex::new(Vec::new()),
        };
        assert_eq!(
            verify_settlement_route(&rpc, &route).await,
            Err(RouteVerificationError::VaultAuthorityMismatch)
        );
    }

    #[tokio::test]
    async fn route_verifier_rejects_missing_or_weak_account_responses() {
        let (route, mut response) = verified_route_fixture();
        response["value"][1] = Value::Null;
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([response])),
            calls: Mutex::new(Vec::new()),
        };
        assert_eq!(
            verify_settlement_route(&rpc, &route).await,
            Err(RouteVerificationError::MissingAccount(route.mint.clone()))
        );

        let (route, mut response) = verified_route_fixture();
        response["context"]["slot"] = Value::String("777".to_string());
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([response])),
            calls: Mutex::new(Vec::new()),
        };
        assert!(matches!(
            verify_settlement_route(&rpc, &route).await,
            Err(RouteVerificationError::InvalidResponse(_))
        ));
    }

    fn encoded_settlement_event(gross: u64, tax: u64, net: u64) -> String {
        let mut data = Vec::with_capacity(200);
        data.extend_from_slice(&SETTLEMENT_EVENT_DISCRIMINATOR);
        data.extend_from_slice(&[1; 32]);
        data.extend_from_slice(&[2; 32]);
        data.extend_from_slice(&[3; 32]);
        data.extend_from_slice(&[4; 32]);
        data.extend_from_slice(&9_u64.to_le_bytes());
        data.extend_from_slice(&gross.to_le_bytes());
        data.extend_from_slice(&tax.to_le_bytes());
        data.extend_from_slice(&net.to_le_bytes());
        data.extend_from_slice(&[5; 32]);
        BASE64.encode(data)
    }

    fn encoded_document_settlement_event(gross: u64, tax: u64, net: u64) -> String {
        let mut data = Vec::with_capacity(300);
        data.extend_from_slice(&DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR);
        data.extend_from_slice(&[41; 32]);
        data.extend_from_slice(&[42; 32]);
        data.extend_from_slice(&[43; 32]);
        data.extend_from_slice(&[44; 32]);
        data.extend_from_slice(&29_u64.to_le_bytes());
        data.extend_from_slice(&gross.to_le_bytes());
        data.extend_from_slice(&tax.to_le_bytes());
        data.extend_from_slice(&net.to_le_bytes());
        data.extend_from_slice(&[45; 32]);
        data.extend_from_slice(&[46; 32]);
        data.extend_from_slice(&[47; 32]);
        BASE64.encode(data)
    }

    fn encoded_applicability_settlement_event(
        gross: u64,
        tax: u64,
        net: u64,
        document: bool,
    ) -> String {
        let mut data = Vec::with_capacity(if document { 344 } else { 280 });
        data.extend_from_slice(if document {
            &APPLICABILITY_DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR
        } else {
            &APPLICABILITY_SETTLEMENT_EVENT_DISCRIMINATOR
        });
        data.extend_from_slice(&[51; 32]);
        data.extend_from_slice(&[52; 32]);
        data.extend_from_slice(&[53; 32]);
        data.extend_from_slice(&[54; 32]);
        data.extend_from_slice(&[55; 32]);
        data.extend_from_slice(&59_u64.to_le_bytes());
        data.extend_from_slice(&gross.to_le_bytes());
        data.extend_from_slice(&tax.to_le_bytes());
        data.extend_from_slice(&net.to_le_bytes());
        data.extend_from_slice(&[56; 32]);
        if document {
            data.extend_from_slice(&[57; 32]);
            data.extend_from_slice(&[58; 32]);
        }
        data.extend_from_slice(&[6; 16]);
        data.extend_from_slice(&[7; 32]);
        BASE64.encode(data)
    }

    fn encoded_split_settlement_event(gross: u64, tax: u64, net: u64) -> String {
        let mut data = Vec::with_capacity(240);
        data.extend_from_slice(&SPLIT_SETTLEMENT_EVENT_DISCRIMINATOR);
        data.extend_from_slice(&[11; 32]);
        data.extend_from_slice(&[12; 32]);
        data.extend_from_slice(&[13; 32]);
        data.extend_from_slice(&[14; 32]);
        data.extend_from_slice(&19_u64.to_le_bytes());
        data.extend_from_slice(&gross.to_le_bytes());
        data.extend_from_slice(&tax.to_le_bytes());
        data.extend_from_slice(&net.to_le_bytes());
        data.extend_from_slice(&2_u64.to_le_bytes());
        data.extend_from_slice(&[15; 32]);
        data.extend_from_slice(&[16; 32]);
        BASE64.encode(data)
    }

    fn encoded_refund_execution_event(gross: u64, tax: u64, net: u64) -> String {
        let mut data = Vec::with_capacity(328);
        data.extend_from_slice(&ADJUSTMENT_REFUND_EXECUTED_EVENT_DISCRIMINATOR);
        data.extend_from_slice(&[21; 32]);
        data.extend_from_slice(&[22; 32]);
        data.extend_from_slice(&[23; 32]);
        data.extend_from_slice(&[24; 32]);
        data.extend_from_slice(&[25; 32]);
        data.extend_from_slice(&29_u64.to_le_bytes());
        data.extend_from_slice(&gross.to_le_bytes());
        data.extend_from_slice(&tax.to_le_bytes());
        data.extend_from_slice(&net.to_le_bytes());
        data.extend_from_slice(&[26; 32]);
        data.extend_from_slice(&[27; 32]);
        data.extend_from_slice(&[28; 32]);
        data.extend_from_slice(&[29; 32]);
        BASE64.encode(data)
    }

    fn encoded_settlement_adjustment_event(gross: u64, tax: u64, net: u64) -> String {
        let mut data = Vec::with_capacity(299);
        data.extend_from_slice(&SETTLEMENT_ADJUSTMENT_RECORDED_EVENT_DISCRIMINATOR);
        data.extend_from_slice(&[31; 32]);
        data.extend_from_slice(&[32; 32]);
        data.extend_from_slice(&[33; 32]);
        data.extend_from_slice(&[34; 32]);
        data.extend_from_slice(&41_u64.to_le_bytes());
        data.push(1);
        data.push(1);
        data.extend_from_slice(&gross.to_le_bytes());
        data.push(1);
        data.extend_from_slice(&tax.to_le_bytes());
        data.push(1);
        data.extend_from_slice(&net.to_le_bytes());
        data.extend_from_slice(&[35; 32]);
        data.extend_from_slice(&[36; 32]);
        data.extend_from_slice(&[37; 32]);
        BASE64.encode(data)
    }

    fn encoded_protocol_pause_event(paused: bool) -> String {
        let mut data = Vec::with_capacity(41);
        data.extend_from_slice(&PROTOCOL_PAUSE_EVENT_DISCRIMINATOR);
        data.extend_from_slice(&[17; 32]);
        data.push(u8::from(paused));
        BASE64.encode(data)
    }

    #[tokio::test]
    async fn settlement_decoder_accepts_only_conserving_anchor_events() {
        let decoder = SettlementExecutedDecoder;
        let block = json!({
            "transactions": [{
                "transaction": {"signatures": ["settlement-signature"]},
                "meta": {
                    "err": null,
                    "logMessages": [
                        "Program 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi invoke [1]",
                        format!("Program data: {}", encoded_settlement_event(100, 25, 75)),
                        "Program 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi success"
                    ]
                }
            }]
        });
        let decoded = decoder.decode_block(42, block).await.unwrap();
        assert_eq!(decoded.receipts.len(), 1);
        assert_eq!(decoded.raw_evidence.len(), 1);
        assert_eq!(decoded.raw_transactions.len(), 1);
        assert_eq!(decoded.raw_transactions[0].slot, 42);
        assert_eq!(decoded.raw_transactions[0].transaction_index, 0);
        decoded.raw_transactions[0]
            .validate(
                ScanRange {
                    start_slot: 42,
                    end_slot: 42,
                },
                42,
            )
            .unwrap();
        assert_eq!(decoded.receipts[0].tax_atomic, 25);
        assert_eq!(
            decoded.receipts[0].evidence_digest,
            decoded.raw_evidence[0].evidence_digest
        );
        assert_eq!(decoded.raw_evidence[0].event_data.len(), 200);
    }

    #[tokio::test]
    async fn settlement_decoder_retains_applicability_bound_event_evidence() {
        let decoder = SettlementExecutedDecoder;
        let block = json!({
            "transactions": [{
                "transaction": {"signatures": ["applicability-signature"]},
                "meta": {
                    "err": null,
                    "logMessages": [
                        "Program 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi invoke [1]",
                        format!("Program data: {}", encoded_applicability_settlement_event(100, 25, 75, false))
                    ]
                }
            }]
        });
        let decoded = decoder.decode_block(42, block).await.unwrap();
        assert_eq!(decoded.receipts.len(), 1);
        assert_eq!(decoded.raw_evidence.len(), 1);
        assert_eq!(
            decoded.raw_evidence[0].event_discriminator,
            APPLICABILITY_SETTLEMENT_EVENT_DISCRIMINATOR
        );
        let event =
            ApplicabilitySettlementExecutedEvent::decode(&decoded.raw_evidence[0].event_data)
                .unwrap();
        assert_eq!(event.binding, [52; 32]);
        assert_eq!(event.context_id, [6; 16]);
        assert_eq!(event.applicability_attestation_digest, [7; 32]);
        assert!(event.document_attestation_digest.is_none());
    }

    #[tokio::test]
    async fn settlement_decoder_fails_closed_on_non_conserving_events() {
        let decoder = SettlementExecutedDecoder;
        let block = json!({
            "transactions": [{
                "transaction": {"signatures": ["settlement-signature"]},
                "meta": {
                    "err": null,
                    "logMessages": [
                        "Program 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi invoke [1]",
                        format!("Program data: {}", encoded_settlement_event(100, 25, 74))
                    ]
                }
            }]
        });
        assert!(matches!(
            decoder.decode_block(42, block).await,
            Err(ProviderError::InvalidResponse(_))
        ));
    }

    #[tokio::test]
    async fn settlement_decoder_projects_finalized_refund_execution_evidence() {
        let decoder = SettlementExecutedDecoder;
        let block = json!({
            "transactions": [{
                "transaction": {"signatures": ["refund-signature"]},
                "meta": {
                    "err": null,
                    "logMessages": [
                        "Program 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi invoke [1]",
                        format!("Program data: {}", encoded_refund_execution_event(100, 25, 75))
                    ]
                }
            }]
        });
        let decoded = decoder.decode_block(42, block).await.unwrap();
        assert!(decoded.receipts.is_empty());
        assert_eq!(decoded.refund_executions.len(), 1);
        let execution = &decoded.refund_executions[0];
        assert_eq!(execution.signature, "refund-signature");
        assert_eq!(execution.slot, 42);
        assert_eq!(execution.event_index, 0);
        assert_eq!(execution.gross_magnitude, 100);
        assert_eq!(execution.tax_magnitude, 25);
        assert_eq!(execution.net_magnitude, 75);
        assert_eq!(execution.refund_destination, [27; 32]);
        assert_eq!(decoded.raw_evidence.len(), 1);
        assert_eq!(
            decoded.raw_evidence[0].event_discriminator,
            ADJUSTMENT_REFUND_EXECUTED_EVENT_DISCRIMINATOR
        );
        assert_eq!(
            execution.evidence_digest,
            decoded.raw_evidence[0].evidence_digest
        );
        execution
            .validate(
                ScanRange {
                    start_slot: 42,
                    end_slot: 42,
                },
                42,
            )
            .unwrap();
    }

    #[tokio::test]
    async fn settlement_decoder_projects_finalized_adjustment_record_evidence() {
        let decoder = SettlementExecutedDecoder;
        let block = json!({
            "transactions": [{
                "transaction": {"signatures": ["adjustment-signature"]},
                "meta": {
                    "err": null,
                    "logMessages": [
                        "Program 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi invoke [1]",
                        format!("Program data: {}", encoded_settlement_adjustment_event(100, 25, 75))
                    ]
                }
            }]
        });
        let decoded = decoder.decode_block(42, block).await.unwrap();
        assert!(decoded.receipts.is_empty());
        assert_eq!(decoded.adjustment_records.len(), 1);
        let record = &decoded.adjustment_records[0];
        assert_eq!(record.adjustment, [31; 32]);
        assert_eq!(record.gross_magnitude, 100);
        assert_eq!(record.tax_magnitude, 25);
        assert_eq!(record.net_magnitude, 75);
        assert_eq!(record.approver, [37; 32]);
        assert_eq!(decoded.raw_evidence.len(), 1);
        assert_eq!(
            decoded.raw_evidence[0].event_discriminator,
            SETTLEMENT_ADJUSTMENT_RECORDED_EVENT_DISCRIMINATOR
        );
        assert_eq!(
            record.evidence_digest,
            decoded.raw_evidence[0].evidence_digest
        );
    }

    #[tokio::test]
    async fn settlement_decoder_rejects_non_conserving_adjustment_record() {
        let decoder = SettlementExecutedDecoder;
        let block = json!({
            "transactions": [{
                "transaction": {"signatures": ["adjustment-signature"]},
                "meta": {
                    "err": null,
                    "logMessages": [
                        "Program 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi invoke [1]",
                        format!("Program data: {}", encoded_settlement_adjustment_event(100, 25, 74))
                    ]
                }
            }]
        });
        assert!(matches!(
            decoder.decode_block(42, block).await,
            Err(ProviderError::InvalidResponse(_))
        ));
    }

    #[tokio::test]
    async fn settlement_decoder_rejects_non_conserving_refund_execution() {
        let decoder = SettlementExecutedDecoder;
        let block = json!({
            "transactions": [{
                "transaction": {"signatures": ["refund-signature"]},
                "meta": {
                    "err": null,
                    "logMessages": [
                        "Program 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi invoke [1]",
                        format!("Program data: {}", encoded_refund_execution_event(100, 25, 74))
                    ]
                }
            }]
        });
        assert!(matches!(
            decoder.decode_block(42, block).await,
            Err(ProviderError::InvalidResponse(_))
        ));
    }

    #[tokio::test]
    async fn settlement_decoder_retains_typed_split_plan_binding() {
        let decoder = SettlementExecutedDecoder;
        let block = json!({
            "transactions": [{
                "transaction": {"signatures": ["split-signature"]},
                "meta": {
                    "err": null,
                    "logMessages": [
                        "Program 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi invoke [1]",
                        format!("Program data: {}", encoded_split_settlement_event(100, 25, 75))
                    ]
                }
            }]
        });
        let decoded = decoder.decode_block(42, block).await.unwrap();
        assert_eq!(decoded.receipts.len(), 1);
        assert_eq!(decoded.split_settlements.len(), 1);
        assert_eq!(decoded.split_settlements[0].split_plan_version, 2);
        assert_eq!(decoded.split_settlements[0].split_plan_digest, [15; 32]);
        assert_eq!(decoded.split_settlements[0].policy_digest, [16; 32]);
        assert_eq!(
            decoded.split_settlements[0].evidence_digest,
            decoded.receipts[0].evidence_digest
        );
        assert_eq!(
            decoded.raw_evidence[0].event_discriminator,
            SPLIT_SETTLEMENT_EVENT_DISCRIMINATOR
        );
    }

    #[test]
    fn finalized_split_projection_maps_to_revenue_proof_only_with_finalized_observation() {
        let split = IndexedSplitSettlement {
            signature: Signature::from([60; 64]).to_string(),
            slot: 42,
            receipt: [1; 32],
            payer: [2; 32],
            merchant: [3; 32],
            mint: [4; 32],
            nonce: 9,
            gross_atomic: 100,
            tax_atomic: 25,
            net_atomic: 75,
            split_plan_version: 2,
            split_plan_digest: [15; 32],
            policy_digest: [16; 32],
            evidence_digest: [17; 32],
        };
        let proof = build_revenue_stream_allocation_proof(&split, [5; 32], 50, 42)
            .expect("finalized split proof");
        assert_eq!(proof.source_slot, 42);
        assert_eq!(proof.recipient_atomic, 50);
        assert_eq!(proof.commitment, FinalityCommitment::Finalized);

        assert_eq!(
            build_revenue_stream_allocation_proof(&split, [5; 32], 50, 41),
            Err(IndexerError::InvalidSplitSettlement)
        );
    }

    #[tokio::test]
    async fn settlement_decoder_projects_document_provenance_in_raw_evidence() {
        let decoder = SettlementExecutedDecoder;
        let block = json!({
            "transactions": [{
                "transaction": {"signatures": ["document-signature"]},
                "meta": {
                    "err": null,
                    "logMessages": [
                        "Program 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi invoke [1]",
                        format!("Program data: {}", encoded_document_settlement_event(100, 25, 75))
                    ]
                }
            }]
        });
        let decoded = decoder.decode_block(42, block).await.unwrap();
        assert_eq!(decoded.receipts.len(), 1);
        assert_eq!(decoded.receipts[0].receipt, [41; 32]);
        assert_eq!(decoded.receipts[0].gross_atomic, 100);
        assert_eq!(decoded.raw_evidence.len(), 1);
        assert_eq!(
            decoded.raw_evidence[0].event_discriminator,
            DOCUMENT_SETTLEMENT_EVENT_DISCRIMINATOR
        );
    }

    #[tokio::test]
    async fn settlement_decoder_projects_finalized_protocol_pause_evidence() {
        let decoder = SettlementExecutedDecoder;
        let block = json!({
            "transactions": [{
                "transaction": {"signatures": ["pause-signature"]},
                "meta": {
                    "err": null,
                    "logMessages": [
                        "Program 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi invoke [1]",
                        format!("Program data: {}", encoded_protocol_pause_event(true))
                    ]
                }
            }]
        });
        let decoded = decoder.decode_block(42, block).await.unwrap();
        assert!(decoded.receipts.is_empty());
        assert_eq!(decoded.protocol_pause_changes.len(), 1);
        let change = &decoded.protocol_pause_changes[0];
        assert_eq!(change.signature, "pause-signature");
        assert_eq!(change.authority, [17; 32]);
        assert!(change.paused);
        change
            .validate(
                ScanRange {
                    start_slot: 42,
                    end_slot: 42,
                },
                42,
            )
            .unwrap();
    }

    #[tokio::test]
    async fn settlement_decoder_rejects_multiple_supported_event_kinds() {
        let decoder = SettlementExecutedDecoder;
        let block = json!({
            "transactions": [{
                "transaction": {"signatures": ["mixed-signature"]},
                "meta": {
                    "err": null,
                    "logMessages": [
                        "Program 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi invoke [1]",
                        format!("Program data: {}", encoded_settlement_event(100, 25, 75)),
                        format!("Program data: {}", encoded_split_settlement_event(100, 25, 75))
                    ]
                }
            }]
        });
        assert!(matches!(
            decoder.decode_block(42, block).await,
            Err(ProviderError::InvalidResponse(_))
        ));
    }

    #[test]
    fn raw_evidence_rebuilds_the_same_receipt_without_trusting_projection_values() {
        let event_data = BASE64
            .decode(encoded_settlement_event(100, 25, 75))
            .unwrap();
        let evidence = RawChainEvidence {
            signature: "rebuild-signature".to_string(),
            slot: 42,
            event_index: 0,
            event_discriminator: SETTLEMENT_EVENT_DISCRIMINATOR,
            evidence_digest: settlement_evidence_digest("rebuild-signature", 42, 0, &event_data),
            event_data,
        };
        let rebuilt = rebuild_receipt_from_raw_evidence(&evidence).unwrap();
        assert_eq!(rebuilt.signature, evidence.signature);
        assert_eq!(rebuilt.slot, 42);
        assert_eq!(rebuilt.receipt, [1; 32]);
        assert_eq!(rebuilt.payer, [2; 32]);
        assert_eq!(rebuilt.merchant, [3; 32]);
        assert_eq!(rebuilt.mint, [4; 32]);
        assert_eq!(rebuilt.nonce, 9);
        assert_eq!(rebuilt.gross_atomic, 100);
        assert_eq!(rebuilt.tax_atomic, 25);
        assert_eq!(rebuilt.net_atomic, 75);
        assert_eq!(rebuilt.policy_digest, [5; 32]);
        assert_eq!(rebuilt.evidence_digest, evidence.evidence_digest);
    }

    #[test]
    fn raw_evidence_rebuild_rejects_default_receipt_identity() {
        let mut event_data = BASE64
            .decode(encoded_settlement_event(100, 25, 75))
            .unwrap();
        event_data[8..40].fill(0);
        let evidence = RawChainEvidence {
            signature: "default-receipt-signature".to_string(),
            slot: 42,
            event_index: 0,
            event_discriminator: SETTLEMENT_EVENT_DISCRIMINATOR,
            evidence_digest: settlement_evidence_digest(
                "default-receipt-signature",
                42,
                0,
                &event_data,
            ),
            event_data,
        };
        assert_eq!(
            rebuild_receipt_from_raw_evidence(&evidence),
            Err(IndexerError::InvalidReceipt)
        );
    }

    #[test]
    fn raw_evidence_rebuild_rejects_tampering_and_noncanonical_events() {
        let event_data = BASE64
            .decode(encoded_settlement_event(100, 25, 75))
            .unwrap();
        let digest = settlement_evidence_digest("rebuild-signature", 42, 0, &event_data);
        let mut tampered = RawChainEvidence {
            signature: "rebuild-signature".to_string(),
            slot: 42,
            event_index: 0,
            event_discriminator: SETTLEMENT_EVENT_DISCRIMINATOR,
            event_data: event_data.clone(),
            evidence_digest: digest,
        };
        tampered.event_data[80] ^= 1;
        assert_eq!(
            rebuild_receipt_from_raw_evidence(&tampered),
            Err(IndexerError::InvalidEvidence)
        );

        let mut unsupported = RawChainEvidence {
            signature: "rebuild-signature".to_string(),
            slot: 42,
            event_index: 0,
            event_discriminator: [9; 8],
            event_data,
            evidence_digest: digest,
        };
        unsupported.event_data[..8].copy_from_slice(&[9; 8]);
        unsupported.evidence_digest =
            settlement_evidence_digest("rebuild-signature", 42, 0, &unsupported.event_data);
        assert_eq!(
            rebuild_receipt_from_raw_evidence(&unsupported),
            Err(IndexerError::InvalidEvidence)
        );
    }

    #[test]
    fn split_raw_evidence_rebuilds_typed_projection() {
        let event_data = BASE64
            .decode(encoded_split_settlement_event(100, 25, 75))
            .unwrap();
        let evidence = RawChainEvidence {
            signature: "split-rebuild-signature".to_string(),
            slot: 42,
            event_index: 0,
            event_discriminator: SPLIT_SETTLEMENT_EVENT_DISCRIMINATOR,
            evidence_digest: settlement_evidence_digest(
                "split-rebuild-signature",
                42,
                0,
                &event_data,
            ),
            event_data,
        };
        let split = rebuild_split_settlement_from_raw_evidence(&evidence).unwrap();
        assert_eq!(split.split_plan_version, 2);
        assert_eq!(split.split_plan_digest, [15; 32]);
        assert_eq!(split.policy_digest, [16; 32]);
        assert_eq!(split.evidence_digest, evidence.evidence_digest);
    }

    #[test]
    fn plans_bounded_ranges_without_skipping_the_cursor() {
        let checkpoint = Checkpoint::new(100, 100);
        assert_eq!(
            checkpoint.plan_next_range(25).unwrap(),
            ScanRange {
                start_slot: 100,
                end_slot: 124,
            }
        );
        assert_eq!(
            checkpoint.plan_next_range(1).unwrap(),
            ScanRange {
                start_slot: 100,
                end_slot: 100,
            }
        );
    }

    #[test]
    fn applies_only_completed_matching_finalized_pages() {
        let checkpoint = Checkpoint::new(100, 100);
        let range = checkpoint.plan_next_range(25).unwrap();
        let next = checkpoint
            .apply_page(range, 124, vec![receipt("sig-1", 110, 100)], true)
            .unwrap();
        assert_eq!(next.next_slot, 125);
        assert_eq!(next.finalized_slot, 124);
        assert_eq!(next.generation, 1);

        assert!(matches!(
            checkpoint.apply_page(range, 124, Vec::new(), false),
            Err(IndexerError::IncompleteRange)
        ));
        assert!(matches!(
            checkpoint.apply_page(
                ScanRange {
                    start_slot: 99,
                    end_slot: 124,
                },
                124,
                Vec::new(),
                true,
            ),
            Err(IndexerError::CheckpointRangeMismatch)
        ));
    }

    #[test]
    fn duplicate_receipts_are_idempotent_but_conflicting_evidence_fails_closed() {
        let first = receipt("sig-1", 110, 100);
        assert_eq!(
            merge_receipt(None, first.clone()).unwrap(),
            MergeOutcome::Inserted
        );
        assert_eq!(
            merge_receipt(Some(first.clone()), first.clone()).unwrap(),
            MergeOutcome::Duplicate
        );
        let conflict = IndexedReceipt {
            signature: "sig-1".to_string(),
            slot: 110,
            receipt: [1; 32],
            payer: [2; 32],
            merchant: [3; 32],
            mint: [4; 32],
            nonce: 9,
            gross_atomic: 101,
            tax_atomic: 10,
            net_atomic: 91,
            policy_digest: [5; 32],
            evidence_digest: [7; 32],
        };
        assert!(matches!(
            merge_receipt(Some(first), conflict),
            Err(IndexerError::ConflictingEvidence)
        ));
    }

    #[test]
    fn reconciliation_is_exact_and_never_rounds_atomic_units() {
        assert_eq!(
            reconcile(
                BalanceExpectation::new(1_000, 250),
                BalanceSnapshot::new(1_000, 250)
            ),
            Reconciliation::Matched
        );
        assert_eq!(
            reconcile(
                BalanceExpectation::new(1_000, 250),
                BalanceSnapshot::new(999, 250)
            ),
            Reconciliation::Mismatch {
                expected_gross: 1_000,
                observed_gross: 999,
                expected_tax: 250,
                observed_tax: 250,
            }
        );
        assert_eq!(
            reconcile(
                BalanceExpectation::new(1_000, 250),
                BalanceSnapshot::new(1_000, 0)
            ),
            Reconciliation::Mismatch {
                expected_gross: 1_000,
                observed_gross: 1_000,
                expected_tax: 250,
                observed_tax: 0,
            }
        );
    }

    #[test]
    fn vault_reconciliation_uses_checked_delta_semantics() {
        let expectation = VaultBalanceExpectation::new(1_000, 250, 100);
        assert_eq!(expectation.expected_balance(), Ok(1_150));
        assert_eq!(
            VaultBalanceExpectation::new(0, 0, 1).expected_balance(),
            Err(VaultBalanceExpectationError::Underflow)
        );
        assert_eq!(
            VaultBalanceExpectation::new(u64::MAX, 1, 0).expected_balance(),
            Err(VaultBalanceExpectationError::Overflow)
        );
        assert_eq!(
            reconcile_vault_balance(1_150, 1_150),
            VaultBalanceReconciliation::Matched {
                expected_balance: 1_150,
                observed_balance: 1_150,
            }
        );
        assert_eq!(
            reconcile_vault_balance(1_150, 1_149),
            VaultBalanceReconciliation::Mismatch {
                expected_balance: 1_150,
                observed_balance: 1_149,
            }
        );
    }

    #[tokio::test]
    async fn vault_reader_proves_finalized_account_and_amount_binding() {
        let mint = Pubkey::new_from_array([7; 32]);
        let vault = Pubkey::new_from_array([8; 32]);
        let mut data = vec![0_u8; 165];
        data[..32].copy_from_slice(mint.as_ref());
        data[64..72].copy_from_slice(&1_150_u64.to_le_bytes());
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([json!({
                "context": {"slot": 777},
                "value": [{
                    "owner": settlement_client::SPL_TOKEN_PROGRAM_ID,
                    "executable": false,
                    "lamports": 1,
                    "data": [BASE64.encode(data), "base64"]
                }]
            })])),
            calls: Mutex::new(Vec::new()),
        };
        let source = SolanaRpcVaultBalanceSource::new(rpc);
        let result = source
            .fetch_finalized_vault_balance(&VaultBalanceTarget {
                tax_vault: vault.to_string(),
                mint: mint.to_string(),
                token_program: settlement_client::SPL_TOKEN_PROGRAM_ID.to_string(),
            })
            .await
            .unwrap();
        assert_eq!(
            result,
            FinalizedVaultBalance {
                slot: 777,
                amount_atomic: 1_150
            }
        );
        let calls = source.transport.calls.lock().unwrap();
        assert_eq!(calls[0].0, "getMultipleAccounts");
        assert_eq!(calls[0].1[1]["commitment"], "finalized");
        assert_eq!(calls[0].1[1]["encoding"], "base64");
    }

    #[tokio::test]
    async fn vault_reader_rejects_token_2022_until_extension_semantics_exist() {
        let source = SolanaRpcVaultBalanceSource::new(FakeRpc {
            results: Mutex::new(VecDeque::new()),
            calls: Mutex::new(Vec::new()),
        });
        let result = source
            .fetch_finalized_vault_balance(&VaultBalanceTarget {
                tax_vault: Pubkey::new_from_array([8; 32]).to_string(),
                mint: Pubkey::new_from_array([7; 32]).to_string(),
                token_program: settlement_client::TOKEN_2022_PROGRAM_ID.to_string(),
            })
            .await;
        assert!(
            matches!(result, Err(ProviderError::InvalidResponse(message)) if message.contains("Token-2022"))
        );
    }

    fn refund_balance_fixture() -> (RefundBalanceTarget, Value) {
        let claim_adjustment = Pubkey::new_from_array([8; 32]);
        let original_receipt = Pubkey::new_from_array([9; 32]);
        let merchant = Pubkey::new_from_array([10; 32]);
        let tax_vault = Pubkey::new_from_array([1; 32]);
        let refund_source = Pubkey::new_from_array([2; 32]);
        let payer_destination = Pubkey::new_from_array([3; 32]);
        let mint = Pubkey::new_from_array([4; 32]);
        let vault_authority = Pubkey::find_program_address(
            &[b"vault-authority", merchant.as_ref(), mint.as_ref()],
            &settlement_client::settlement_program_id(),
        )
        .0;
        let refund_funder = Pubkey::new_from_array([6; 32]);
        let payer = Pubkey::new_from_array([7; 32]);
        let claim_nonce: u64 = 1;
        let (claim, _) = Pubkey::find_program_address(
            &[
                b"refund-claim",
                claim_adjustment.as_ref(),
                &claim_nonce.to_le_bytes(),
            ],
            &settlement_client::settlement_program_id(),
        );
        let account = |address: &Pubkey, owner: &Pubkey, amount: u64| {
            let mut data = vec![0_u8; 165];
            data[..32].copy_from_slice(mint.as_ref());
            data[32..64].copy_from_slice(owner.as_ref());
            data[64..72].copy_from_slice(&amount.to_le_bytes());
            json!({
                "lamports": 1,
                "owner": settlement_client::SPL_TOKEN_PROGRAM_ID,
                "executable": false,
                "data": [BASE64.encode(data), "base64"],
                "address": address.to_string()
            })
        };
        let mut discriminator = Sha256::new();
        discriminator.update(b"account:AdjustmentRefundClaim");
        let mut claim_data = vec![0_u8; REFUND_CLAIM_ACCOUNT_DATA_LEN];
        claim_data[..8].copy_from_slice(&discriminator.finalize()[..8]);
        let set_key = |offset: usize, key: &Pubkey, data: &mut Vec<u8>| {
            data[offset..offset + 32].copy_from_slice(key.as_ref());
        };
        set_key(8, &claim_adjustment, &mut claim_data);
        set_key(40, &original_receipt, &mut claim_data);
        set_key(72, &merchant, &mut claim_data);
        set_key(104, &mint, &mut claim_data);
        set_key(136, &payer, &mut claim_data);
        set_key(168, &refund_funder, &mut claim_data);
        set_key(200, &tax_vault, &mut claim_data);
        set_key(232, &refund_source, &mut claim_data);
        set_key(264, &payer_destination, &mut claim_data);
        claim_data[296..304].copy_from_slice(&claim_nonce.to_le_bytes());
        claim_data[304..312].copy_from_slice(&100_u64.to_le_bytes());
        claim_data[312..320].copy_from_slice(&50_u64.to_le_bytes());
        claim_data[320..328].copy_from_slice(&50_u64.to_le_bytes());
        claim_data[328..360].copy_from_slice(&[31; 32]);
        claim_data[360..368].copy_from_slice(&0_i64.to_le_bytes());
        claim_data[368..376].copy_from_slice(&100_i64.to_le_bytes());
        claim_data[408] = 1;
        claim_data[409] = 0;
        let claim_account = json!({
            "lamports": 1,
            "owner": settlement_client::SETTLEMENT_PROGRAM_ID,
            "executable": false,
            "data": [BASE64.encode(claim_data), "base64"],
            "address": claim.to_string()
        });
        let target = RefundBalanceTarget {
            claim: claim.to_string(),
            adjustment: claim_adjustment.to_string(),
            original_receipt: original_receipt.to_string(),
            merchant: merchant.to_string(),
            claim_nonce,
            tax_vault: tax_vault.to_string(),
            refund_source: refund_source.to_string(),
            payer_destination: payer_destination.to_string(),
            mint: mint.to_string(),
            token_program: settlement_client::SPL_TOKEN_PROGRAM_ID.to_string(),
            vault_authority: vault_authority.to_string(),
            refund_funder: refund_funder.to_string(),
            payer: payer.to_string(),
        };
        let response = json!({
            "context": {"slot": 777},
            "value": [
                claim_account,
                account(&tax_vault, &vault_authority, 950),
                account(&refund_source, &refund_funder, 800),
                account(&payer_destination, &payer, 300)
            ]
        });
        (target, response)
    }

    fn refund_execution_event_for_target(target: &RefundBalanceProofTarget) -> String {
        let topology = &target.topology;
        let parse = |value: &str| Pubkey::from_str(value).unwrap().to_bytes();
        let mut data = Vec::with_capacity(328);
        data.extend_from_slice(&ADJUSTMENT_REFUND_EXECUTED_EVENT_DISCRIMINATOR);
        data.extend_from_slice(&parse(&topology.claim));
        data.extend_from_slice(&parse(&topology.adjustment));
        data.extend_from_slice(&parse(&topology.original_receipt));
        data.extend_from_slice(&parse(&topology.merchant));
        data.extend_from_slice(&parse(&topology.mint));
        data.extend_from_slice(&topology.claim_nonce.to_le_bytes());
        data.extend_from_slice(&target.gross_atomic.to_le_bytes());
        data.extend_from_slice(&target.tax_atomic.to_le_bytes());
        data.extend_from_slice(&target.net_atomic.to_le_bytes());
        data.extend_from_slice(&parse(&topology.refund_funder));
        data.extend_from_slice(&parse(&topology.payer_destination));
        data.extend_from_slice(&[11; 32]);
        data.extend_from_slice(&target.reference_digest);
        BASE64.encode(data)
    }

    fn refund_proof_fixture() -> (RefundBalanceProofTarget, Value, Value) {
        let (topology, claim_response) = refund_balance_fixture();
        let signature = test_signature(42);
        let target = RefundBalanceProofTarget {
            signature: signature.clone(),
            topology,
            gross_atomic: 100,
            tax_atomic: 50,
            net_atomic: 50,
            reference_digest: [31; 32],
        };
        let account_keys = json!([
            target.topology.tax_vault,
            target.topology.refund_source,
            target.topology.payer_destination
        ]);
        let balance = |account_index: u64, owner: &str, amount: u64| {
            json!({
                "accountIndex": account_index,
                "mint": target.topology.mint,
                "owner": owner,
                "programId": target.topology.token_program,
                "uiTokenAmount": {"amount": amount.to_string(), "decimals": 6, "uiAmountString": "0"}
            })
        };
        let transaction = json!({
            "slot": 777,
            "transaction": {
                "signatures": [signature],
                "message": {"accountKeys": account_keys}
            },
            "meta": {
                "err": null,
                "logMessages": [format!("Program data: {}", refund_execution_event_for_target(&target))],
                "preTokenBalances": [
                    balance(0, &target.topology.vault_authority, 1_000),
                    balance(1, &target.topology.refund_funder, 800),
                    balance(2, &target.topology.payer, 300)
                ],
                "postTokenBalances": [
                    balance(0, &target.topology.vault_authority, 950),
                    balance(1, &target.topology.refund_funder, 750),
                    balance(2, &target.topology.payer, 400)
                ]
            }
        });
        (target, transaction, claim_response)
    }

    #[tokio::test]
    async fn transaction_bound_refund_reader_proves_event_and_pre_post_balances() {
        let (target, transaction, claim_response) = refund_proof_fixture();
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([transaction, claim_response])),
            calls: Mutex::new(Vec::new()),
        };
        let source = SolanaRpcRefundBalanceTransitionSource::new(rpc);
        let result = source
            .fetch_finalized_refund_balance_transition(&target)
            .await
            .unwrap();
        assert_eq!(
            result,
            FinalizedRefundBalanceTransition {
                slot: 777,
                tax_vault_before_atomic: 1_000,
                tax_vault_after_atomic: 950,
                refund_source_before_atomic: 800,
                refund_source_after_atomic: 750,
                payer_destination_before_atomic: 300,
                payer_destination_after_atomic: 400,
            }
        );
        let calls = source.transport.calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, "getTransaction");
        assert_eq!(calls[1].0, "getMultipleAccounts");
    }

    #[tokio::test]
    async fn refund_proof_source_resolves_claim_topology_before_transaction() {
        let (target, transaction, full_claim_response) = refund_proof_fixture();
        let claim_response = json!({
            "context": {"slot": 900},
            "value": [full_claim_response["value"][0].clone()]
        });
        let parse = |value: &str| Pubkey::from_str(value).unwrap().to_bytes();
        let request = RefundClaimTopologyRequest {
            claim: parse(&target.topology.claim),
            adjustment: parse(&target.topology.adjustment),
            original_receipt: parse(&target.topology.original_receipt),
            merchant: parse(&target.topology.merchant),
            mint: parse(&target.topology.mint),
            claim_nonce: target.topology.claim_nonce,
            gross_atomic: target.gross_atomic,
            tax_atomic: target.tax_atomic,
            net_atomic: target.net_atomic,
            refund_funder: parse(&target.topology.refund_funder),
            refund_destination: parse(&target.topology.payer_destination),
            reference_digest: target.reference_digest,
        };
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([claim_response, transaction])),
            calls: Mutex::new(Vec::new()),
        };
        let source = SolanaRpcRefundBalanceProofSource::new(rpc);
        let (resolved, transition) = source
            .fetch_finalized_refund_balance_proof(&request, &target.signature)
            .await
            .unwrap();
        assert_eq!(resolved.topology.tax_vault, target.topology.tax_vault);
        assert_eq!(
            resolved.topology.refund_source,
            target.topology.refund_source
        );
        assert_eq!(resolved.topology.payer, target.topology.payer);
        assert_eq!(transition.slot, 777);
        let calls = source.transport.calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, "getMultipleAccounts");
        assert_eq!(calls[1].0, "getTransaction");
    }

    #[tokio::test]
    async fn transaction_bound_refund_reader_rejects_signature_and_event_substitution() {
        let (mut target, mut transaction, claim_response) = refund_proof_fixture();
        transaction["transaction"]["signatures"][0] = Value::String(test_signature(43));
        let source = SolanaRpcRefundBalanceTransitionSource::new(FakeRpc {
            results: Mutex::new(VecDeque::from([transaction, claim_response])),
            calls: Mutex::new(Vec::new()),
        });
        assert!(matches!(
            source.fetch_finalized_refund_balance_transition(&target).await,
            Err(ProviderError::InvalidResponse(message)) if message.contains("signature")
        ));

        let (target_fixture, mut transaction, claim_response) = refund_proof_fixture();
        target = target_fixture;
        transaction["meta"]["logMessages"] = json!([]);
        let source = SolanaRpcRefundBalanceTransitionSource::new(FakeRpc {
            results: Mutex::new(VecDeque::from([transaction, claim_response])),
            calls: Mutex::new(Vec::new()),
        });
        assert!(matches!(
            source.fetch_finalized_refund_balance_transition(&target).await,
            Err(ProviderError::InvalidResponse(message)) if message.contains("exactly one")
        ));
    }

    #[tokio::test]
    async fn transaction_bound_refund_failover_retries_the_complete_proof_request() {
        let (target, _, _) = refund_proof_fixture();
        let transition = FinalizedRefundBalanceTransition {
            slot: 777,
            tax_vault_before_atomic: 1_000,
            tax_vault_after_atomic: 950,
            refund_source_before_atomic: 800,
            refund_source_after_atomic: 750,
            payer_destination_before_atomic: 300,
            payer_destination_after_atomic: 400,
        };
        let source = FailoverRefundBalanceTransitionSource::new(
            vec![
                Arc::new(StaticRefundTransitionSource {
                    transition,
                    failing: true,
                }),
                Arc::new(StaticRefundTransitionSource {
                    transition,
                    failing: false,
                }),
            ],
            Duration::from_millis(100),
        )
        .unwrap();
        assert_eq!(
            source
                .fetch_finalized_refund_balance_transition(&target)
                .await,
            Ok(transition)
        );
    }

    #[tokio::test]
    async fn refund_reader_proves_one_finalized_context_and_account_bindings() {
        let (target, response) = refund_balance_fixture();
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([response])),
            calls: Mutex::new(Vec::new()),
        };
        let source = SolanaRpcRefundBalanceSource::new(rpc);
        let result = source
            .fetch_finalized_refund_balances(&target)
            .await
            .unwrap();
        assert_eq!(
            result,
            FinalizedRefundBalances {
                slot: 777,
                tax_vault_atomic: 950,
                refund_source_atomic: 800,
                payer_destination_atomic: 300,
            }
        );
        let calls = source.transport.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "getMultipleAccounts");
        assert_eq!(calls[0].1[1]["commitment"], "finalized");
        assert_eq!(calls[0].1[1]["encoding"], "base64");
        assert_eq!(calls[0].1[0].as_array().unwrap().len(), 4);
    }

    #[tokio::test]
    async fn refund_reader_rejects_authority_substitution_and_account_aliases() {
        let (target, mut response) = refund_balance_fixture();
        let encoded = response["value"][3]["data"][0].as_str().unwrap();
        let mut data = BASE64.decode(encoded).unwrap();
        data[32] ^= 1;
        response["value"][3]["data"][0] = Value::String(BASE64.encode(data));
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([response])),
            calls: Mutex::new(Vec::new()),
        };
        let source = SolanaRpcRefundBalanceSource::new(rpc);
        assert!(matches!(
            source.fetch_finalized_refund_balances(&target).await,
            Err(ProviderError::InvalidResponse(message)) if message.contains("payer_destination owner")
        ));

        let (mut target, _) = refund_balance_fixture();
        target.payer_destination = target.tax_vault.clone();
        let source = SolanaRpcRefundBalanceSource::new(FakeRpc {
            results: Mutex::new(VecDeque::new()),
            calls: Mutex::new(Vec::new()),
        });
        assert!(matches!(
            source.fetch_finalized_refund_balances(&target).await,
            Err(ProviderError::InvalidResponse(message)) if message.contains("pairwise distinct")
        ));
    }

    #[tokio::test]
    async fn refund_reader_rejects_token_2022_and_non_finalized_contexts() {
        let (mut target, _response) = refund_balance_fixture();
        target.token_program = settlement_client::TOKEN_2022_PROGRAM_ID.to_string();
        let source = SolanaRpcRefundBalanceSource::new(FakeRpc {
            results: Mutex::new(VecDeque::new()),
            calls: Mutex::new(Vec::new()),
        });
        assert!(matches!(
            source.fetch_finalized_refund_balances(&target).await,
            Err(ProviderError::InvalidResponse(message)) if message.contains("legacy SPL Token")
        ));

        let (target, mut response) = refund_balance_fixture();
        response["context"]["slot"] = Value::Number(0.into());
        let source = SolanaRpcRefundBalanceSource::new(FakeRpc {
            results: Mutex::new(VecDeque::from([response])),
            calls: Mutex::new(Vec::new()),
        });
        assert!(matches!(
            source.fetch_finalized_refund_balances(&target).await,
            Err(ProviderError::InvalidResponse(message)) if message.contains("finalized slot")
        ));
    }

    #[tokio::test]
    async fn refund_provider_failover_is_bounded_and_returns_one_complete_snapshot() {
        let (target, _) = refund_balance_fixture();
        let balance = FinalizedRefundBalances {
            slot: 777,
            tax_vault_atomic: 950,
            refund_source_atomic: 800,
            payer_destination_atomic: 300,
        };
        let source = FailoverRefundBalanceSource::new(
            vec![
                Arc::new(StaticRefundSource {
                    balance,
                    failing: true,
                }),
                Arc::new(StaticRefundSource {
                    balance,
                    failing: false,
                }),
            ],
            Duration::from_millis(100),
        )
        .unwrap();
        assert_eq!(
            source.fetch_finalized_refund_balances(&target).await,
            Ok(balance)
        );
        assert!(matches!(
            FailoverRefundBalanceSource::new(Vec::new(), Duration::from_millis(100)),
            Err(ProviderError::InvalidResponse(message)) if message.contains("between 1 and 3")
        ));
    }

    #[tokio::test]
    async fn applicability_provider_failover_returns_one_complete_finalized_binding() {
        let binding = FinalizedApplicabilityBinding {
            slot: 777,
            binding: [1; 32],
            receipt: [2; 32],
            merchant: [3; 32],
            mint: [4; 32],
            context_id: [5; 16],
            attestation_digest: [6; 32],
            bump: 7,
        };
        let target = ApplicabilityBindingTarget {
            binding: Pubkey::new_from_array(binding.binding).to_string(),
            receipt: Pubkey::new_from_array(binding.receipt).to_string(),
            merchant: Pubkey::new_from_array(binding.merchant).to_string(),
            mint: Pubkey::new_from_array(binding.mint).to_string(),
            context_id: binding.context_id,
            attestation_digest: binding.attestation_digest,
        };
        let source = FailoverApplicabilityBindingSource::new(
            vec![
                Arc::new(StaticApplicabilitySource {
                    binding,
                    failing: true,
                }),
                Arc::new(StaticApplicabilitySource {
                    binding,
                    failing: false,
                }),
            ],
            Duration::from_millis(100),
        )
        .unwrap();
        assert_eq!(
            source.fetch_finalized_applicability_binding(&target).await,
            Ok(binding)
        );
        assert!(matches!(
            FailoverApplicabilityBindingSource::new(Vec::new(), Duration::from_millis(100)),
            Err(ProviderError::InvalidResponse(message)) if message.contains("between 1 and 3")
        ));
    }

    #[tokio::test]
    async fn coordinator_rejects_provider_pages_that_are_not_exactly_planned() {
        let coordinator = IndexerCoordinator::new();
        let planned = coordinator.plan(Checkpoint::new(100, 100), 10).unwrap();
        let source = FakeSource {
            page: FinalizedPage {
                range: ScanRange {
                    start_slot: 101,
                    end_slot: 109,
                },
                observed_finalized_slot: 109,
                receipts: Vec::new(),
                split_settlements: Vec::new(),
                adjustment_records: Vec::new(),
                refund_executions: Vec::new(),
                raw_evidence: Vec::new(),
                raw_transactions: Vec::new(),
                protocol_pause_changes: Vec::new(),
                complete: true,
            },
        };
        assert_eq!(
            coordinator.fetch(planned, &source).await,
            Err(ProviderError::InvalidPage(
                IndexerError::ProviderRangeMismatch
            ))
        );
    }

    #[tokio::test]
    async fn coordinator_advances_only_after_a_valid_finalized_page() {
        let coordinator = IndexerCoordinator::new();
        let planned = coordinator.plan(Checkpoint::new(100, 100), 10).unwrap();
        let source = FakeSource {
            page: FinalizedPage {
                range: planned.range,
                observed_finalized_slot: 120,
                receipts: Vec::new(),
                split_settlements: Vec::new(),
                adjustment_records: Vec::new(),
                refund_executions: Vec::new(),
                raw_evidence: Vec::new(),
                raw_transactions: Vec::new(),
                protocol_pause_changes: Vec::new(),
                complete: true,
            },
        };
        let fetched = coordinator.fetch(planned, &source).await.unwrap();
        assert_eq!(
            coordinator.advance(planned, fetched).unwrap().next_slot,
            110
        );
    }

    #[test]
    fn replay_queue_coalesces_adjacent_ranges_without_merging_reasons() {
        let mut queue = ReplayQueue::default();
        queue
            .enqueue(
                ScanRange {
                    start_slot: 100,
                    end_slot: 109,
                },
                120,
                ReplayGapReason::IncompleteProviderPage,
            )
            .unwrap();
        queue
            .enqueue(
                ScanRange {
                    start_slot: 110,
                    end_slot: 119,
                },
                130,
                ReplayGapReason::IncompleteProviderPage,
            )
            .unwrap();
        queue
            .enqueue(
                ScanRange {
                    start_slot: 120,
                    end_slot: 129,
                },
                130,
                ReplayGapReason::ProviderDisagreement,
            )
            .unwrap();

        assert_eq!(queue.len(), 2);
        let first = queue.claim_next(100).unwrap().unwrap();
        assert_eq!(first.range.start_slot, 100);
        assert_eq!(first.range.end_slot, 119);
        assert_eq!(first.observed_finalized_slot, 130);
        assert_eq!(first.attempt, 1);
        assert_eq!(
            queue.claim_next(100).unwrap().unwrap().reason,
            ReplayGapReason::ProviderDisagreement
        );
    }

    #[test]
    fn replay_queue_splits_work_and_fails_closed_at_retry_exhaustion() {
        let mut queue = ReplayQueue::default();
        queue
            .enqueue(
                ScanRange {
                    start_slot: 200,
                    end_slot: 299,
                },
                400,
                ReplayGapReason::ProjectionCommitFailure,
            )
            .unwrap();
        let work = queue.claim_next(25).unwrap().unwrap();
        assert_eq!(work.range.end_slot, 224);
        assert_eq!(queue.len(), 1);
        queue.requeue(work).unwrap();
        assert_eq!(queue.len(), 1);

        let mut exhausted = ReplayWork {
            range: ScanRange {
                start_slot: 1,
                end_slot: 1,
            },
            observed_finalized_slot: 1,
            reason: ReplayGapReason::OperatorRequested,
            attempt: MAX_REPLAY_ATTEMPTS,
        };
        assert_eq!(
            queue.requeue(exhausted),
            Err(ReplayError::AttemptsExhausted)
        );
        exhausted.attempt = MAX_REPLAY_ATTEMPTS - 1;
        queue.requeue(exhausted).unwrap();
        assert_eq!(queue.len(), 2);
    }

    #[tokio::test]
    async fn solana_provider_requests_finalized_blocks_with_bounded_shape() {
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([
                json!(200),
                json!([100, 105]),
                json!({"block": 100}),
                json!({"block": 105}),
            ])),
            calls: Mutex::new(Vec::new()),
        };
        let source = SolanaRpcPageSource::new(rpc, EmptyReceiptDecoder);
        let page = source
            .fetch_finalized_page(ScanRange {
                start_slot: 100,
                end_slot: 105,
            })
            .await
            .unwrap();
        assert_eq!(page.observed_finalized_slot, 200);
        assert!(page.complete);
    }

    #[tokio::test]
    async fn solana_provider_rejects_weak_finality_before_scanning_blocks() {
        let rpc = FakeRpc {
            results: Mutex::new(VecDeque::from([json!(104)])),
            calls: Mutex::new(Vec::new()),
        };
        let source = SolanaRpcPageSource::new(rpc, EmptyReceiptDecoder);
        assert_eq!(
            source
                .fetch_finalized_page(ScanRange {
                    start_slot: 100,
                    end_slot: 105,
                })
                .await,
            Err(ProviderError::NonFinalized)
        );
    }

    #[test]
    fn reqwest_transport_rejects_insecure_remote_endpoints() {
        assert!(
            ReqwestJsonRpcTransport::new(
                "http://rpc.example.invalid",
                DEFAULT_RPC_TIMEOUT,
                MAX_RPC_RESPONSE_BYTES,
            )
            .is_err()
        );
        assert!(
            ReqwestJsonRpcTransport::new(
                "https://user:password@rpc.example.invalid",
                DEFAULT_RPC_TIMEOUT,
                MAX_RPC_RESPONSE_BYTES,
            )
            .is_err()
        );
        assert!(
            ReqwestJsonRpcTransport::new(
                "http://127.0.0.1:8899",
                DEFAULT_RPC_TIMEOUT,
                MAX_RPC_RESPONSE_BYTES,
            )
            .is_ok()
        );
        assert!(
            ReqwestJsonRpcTransport::new_for_local_development(
                "http://validator:8899",
                DEFAULT_RPC_TIMEOUT,
                MAX_RPC_RESPONSE_BYTES,
            )
            .is_ok()
        );
    }

    #[tokio::test]
    async fn provider_failover_is_bounded_and_preserves_the_finalized_contract() {
        let planned = IndexerCoordinator::new()
            .plan(Checkpoint::new(100, 100), 10)
            .unwrap();
        let healthy = Arc::new(FakeSource {
            page: FinalizedPage {
                range: planned.range,
                observed_finalized_slot: 120,
                receipts: Vec::new(),
                split_settlements: Vec::new(),
                adjustment_records: Vec::new(),
                refund_executions: Vec::new(),
                raw_evidence: Vec::new(),
                raw_transactions: Vec::new(),
                protocol_pause_changes: Vec::new(),
                complete: true,
            },
        });
        let pool = FailoverPageSource::new(
            vec![Arc::new(FailingSource), healthy],
            Duration::from_millis(100),
        )
        .unwrap();
        let page = pool.fetch_finalized_page(planned.range).await.unwrap();
        assert_eq!(page.range, planned.range);
        assert_eq!(page.observed_finalized_slot, 120);
    }

    #[tokio::test]
    async fn provider_probe_accepts_reachable_but_not_yet_finalized_start_slot() {
        let pool = FailoverPageSource::new(
            vec![Arc::new(NonFinalizedSource)],
            Duration::from_millis(100),
        )
        .unwrap();

        assert!(pool.probe(100).await.is_ok());
        assert_eq!(pool.health_snapshot()[0].consecutive_failures, 0);
    }

    #[tokio::test]
    async fn provider_health_moves_a_successful_endpoint_ahead_of_a_failed_one() {
        let planned = IndexerCoordinator::new()
            .plan(Checkpoint::new(100, 100), 10)
            .unwrap();
        let bad_calls = Arc::new(AtomicUsize::new(0));
        let healthy_calls = Arc::new(AtomicUsize::new(0));
        let page = FinalizedPage {
            range: planned.range,
            observed_finalized_slot: 120,
            receipts: Vec::new(),
            split_settlements: Vec::new(),
            adjustment_records: Vec::new(),
            refund_executions: Vec::new(),
            raw_evidence: Vec::new(),
            raw_transactions: Vec::new(),
            protocol_pause_changes: Vec::new(),
            complete: true,
        };
        let pool = FailoverPageSource::new(
            vec![
                Arc::new(CountingSource {
                    calls: bad_calls.clone(),
                    page: page.clone(),
                    failing: true,
                }),
                Arc::new(CountingSource {
                    calls: healthy_calls.clone(),
                    page,
                    failing: false,
                }),
            ],
            Duration::from_millis(100),
        )
        .unwrap();

        pool.fetch_finalized_page(planned.range).await.unwrap();
        pool.fetch_finalized_page(planned.range).await.unwrap();

        assert_eq!(bad_calls.load(Ordering::Relaxed), 1);
        assert_eq!(healthy_calls.load(Ordering::Relaxed), 2);
        let health = pool.health_snapshot();
        assert_eq!(health[0].score, -1);
        assert_eq!(health[0].consecutive_failures, 1);
        assert_eq!(health[1].score, 2);
        assert_eq!(health[1].consecutive_failures, 0);
    }

    #[test]
    fn provider_failover_rejects_unbounded_pools_and_deadlines() {
        assert!(FailoverPageSource::new(Vec::new(), Duration::from_secs(1)).is_err());
        assert!(
            FailoverPageSource::new(
                (0..=MAX_PROVIDER_ATTEMPTS)
                    .map(|_| Arc::new(FailingSource) as Arc<dyn FinalizedPageSource>)
                    .collect(),
                Duration::from_secs(1),
            )
            .is_err()
        );
        assert!(
            FailoverPageSource::new(vec![Arc::new(FailingSource)], Duration::from_millis(1),)
                .is_err()
        );
    }

    fn receipt(signature: &str, slot: u64, gross_atomic: u64) -> IndexedReceipt {
        IndexedReceipt {
            signature: signature.to_string(),
            slot,
            receipt: [1; 32],
            payer: [2; 32],
            merchant: [3; 32],
            mint: [4; 32],
            nonce: 9,
            gross_atomic,
            tax_atomic: 10,
            net_atomic: gross_atomic - 10,
            policy_digest: [5; 32],
            evidence_digest: [7; 32],
        }
    }
}
