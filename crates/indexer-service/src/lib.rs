use async_trait::async_trait;
use indexer::{
    Checkpoint, FailoverPageSource, FailoverRefundBalanceProofSource, FailoverSignaturePageSource,
    FailoverTransactionSource, FailoverVaultBalanceSource, FinalizedPageSource,
    FinalizedRefundBalanceProofSource, FinalizedSignaturePageSource, FinalizedTransactionSource,
    FinalizedVaultBalanceSource, IndexerCoordinator, MAX_PROVIDER_ATTEMPTS, MAX_RPC_PAGE_SLOTS,
    MAX_RPC_RESPONSE_BYTES, ProviderError, ProviderHealthSnapshot, RefundClaimTopologyRequest,
    RefundProofCandidate, RefundProofCandidateSource, RefundProofCycleResult, RefundProofEvidence,
    RefundProofSink, RefundProofWorker, RefundProofWorkerError, ReplayGap, ReplayGapReason,
    ReqwestJsonRpcTransport, ScanRange, SettlementExecutedDecoder, SolanaRpcPageSource,
    SolanaRpcRefundBalanceProofSource, SolanaRpcSignaturePageSource, SolanaRpcTransactionSource,
    SolanaRpcVaultBalanceSource, VaultBalanceExpectation, VaultBalanceReconciliation,
    VaultBalanceTarget, canonical_signature,
};
use solana_pubkey::Pubkey;
use std::str::FromStr;
use std::{
    env,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use storage::{
    DEFAULT_REPLAY_LEASE, MAX_VAULT_RECONCILIATION_TARGETS, PgStore, ProtocolPauseMetricsSnapshot,
    RefundBalanceProofTopology, RefundClaimReconciliationMetricsSnapshot,
    RefundProofMetricsSnapshot, ReplayGapMetricsSnapshot, StorageError,
    TargetedRecoveryMetricsSnapshot, VaultReconciliationMetricsSnapshot, VaultReconciliationRun,
    refund_claim_evidence_expectation,
};
use thiserror::Error;

const DEFAULT_INDEXER_POLL: Duration = Duration::from_millis(250);
const DEFAULT_INDEXER_RPC_TIMEOUT: Duration = Duration::from_secs(2);
const DEFAULT_INDEXER_POOL_SIZE: usize = 8;
const DEFAULT_VAULT_RECONCILIATION_POLL: Duration = Duration::from_secs(30);
const REPLAY_CLAIM_BATCH_SIZE: usize = 1;
pub const MAX_TARGETED_RECOVERY_PAGES: usize = 64;
const TARGETED_RECOVERY_PAGE_SIZE: usize = 1_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeploymentEnvironment {
    Local,
    Staging,
    Production,
}

impl DeploymentEnvironment {
    fn allows_insecure_rpc(self, enabled: bool) -> bool {
        !enabled || matches!(self, Self::Local)
    }
}

fn parse_deployment_environment(value: &str) -> Result<DeploymentEnvironment, IndexerServiceError> {
    match value {
        "local" => Ok(DeploymentEnvironment::Local),
        "staging" => Ok(DeploymentEnvironment::Staging),
        "production" => Ok(DeploymentEnvironment::Production),
        _ => Err(IndexerServiceError::Configuration(
            "INDEXER_ENV must be exactly local, staging, or production".to_string(),
        )),
    }
}

#[derive(Clone, Debug)]
pub struct IndexerConfig {
    pub environment: DeploymentEnvironment,
    pub database_url: String,
    pub rpc_endpoints: Vec<String>,
    pub stream_key: String,
    pub start_slot: u64,
    pub start_finalized_slot: u64,
    pub page_slots: u64,
    pub poll_interval: Duration,
    pub rpc_timeout: Duration,
    pub database_pool_size: usize,
    pub allow_insecure_local_rpc: bool,
    pub replay_lease: Duration,
    pub vault_reconciliation_poll_interval: Duration,
    pub vault_reconciliation_target_limit: usize,
}

impl IndexerConfig {
    pub fn from_env() -> Result<Self, IndexerServiceError> {
        let environment = parse_deployment_environment(&required_env("INDEXER_ENV")?)?;
        let database_url = required_env("INDEXER_DATABASE_URL")?;
        let stream_key = required_env("INDEXER_STREAM_KEY")?;
        let rpc_endpoints = required_env("INDEXER_RPC_ENDPOINTS")?
            .split(',')
            .map(str::trim)
            .filter(|endpoint| !endpoint.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if rpc_endpoints.is_empty() || rpc_endpoints.len() > MAX_PROVIDER_ATTEMPTS {
            return Err(IndexerServiceError::Configuration(
                "INDEXER_RPC_ENDPOINTS must contain between 1 and 3 endpoints".to_string(),
            ));
        }
        let start_slot = parse_env_u64("INDEXER_START_SLOT", None)?;
        let start_finalized_slot = parse_env_u64("INDEXER_START_FINALIZED_SLOT", Some(start_slot))?;
        let page_slots = parse_env_u64("INDEXER_PAGE_SLOTS", Some(MAX_RPC_PAGE_SLOTS))?;
        if page_slots == 0 || page_slots > MAX_RPC_PAGE_SLOTS {
            return Err(IndexerServiceError::Configuration(format!(
                "INDEXER_PAGE_SLOTS must be between 1 and {MAX_RPC_PAGE_SLOTS}"
            )));
        }
        let poll_interval = parse_duration_ms("INDEXER_POLL_MS", DEFAULT_INDEXER_POLL, 10..=5_000)?;
        let rpc_timeout = parse_duration_ms(
            "INDEXER_RPC_TIMEOUT_MS",
            DEFAULT_INDEXER_RPC_TIMEOUT,
            50..=10_000,
        )?;
        let database_pool_size = parse_env_usize(
            "INDEXER_DATABASE_POOL_SIZE",
            Some(DEFAULT_INDEXER_POOL_SIZE),
        )?;
        if database_pool_size == 0 || database_pool_size > 64 {
            return Err(IndexerServiceError::Configuration(
                "INDEXER_DATABASE_POOL_SIZE must be between 1 and 64".to_string(),
            ));
        }
        let allow_insecure_local_rpc = parse_env_bool("INDEXER_ALLOW_INSECURE_LOCAL_RPC", false)?;
        if !environment.allows_insecure_rpc(allow_insecure_local_rpc) {
            return Err(IndexerServiceError::Configuration(
                "INDEXER_ALLOW_INSECURE_LOCAL_RPC requires INDEXER_ENV=local".to_string(),
            ));
        }
        let replay_lease = parse_duration_ms(
            "INDEXER_REPLAY_LEASE_MS",
            DEFAULT_REPLAY_LEASE,
            100..=300_000,
        )?;
        let vault_reconciliation_poll_interval = parse_duration_ms(
            "INDEXER_VAULT_RECONCILIATION_POLL_MS",
            DEFAULT_VAULT_RECONCILIATION_POLL,
            5_000..=900_000,
        )?;
        let vault_reconciliation_target_limit = parse_env_usize(
            "INDEXER_VAULT_RECONCILIATION_TARGET_LIMIT",
            Some(MAX_VAULT_RECONCILIATION_TARGETS),
        )?;
        if vault_reconciliation_target_limit == 0
            || vault_reconciliation_target_limit > MAX_VAULT_RECONCILIATION_TARGETS
        {
            return Err(IndexerServiceError::Configuration(
                "INDEXER_VAULT_RECONCILIATION_TARGET_LIMIT must be between 1 and 64".to_string(),
            ));
        }
        Ok(Self {
            environment,
            database_url,
            rpc_endpoints,
            stream_key,
            start_slot,
            start_finalized_slot,
            page_slots,
            poll_interval,
            rpc_timeout,
            database_pool_size,
            allow_insecure_local_rpc,
            replay_lease,
            vault_reconciliation_poll_interval,
            vault_reconciliation_target_limit,
        })
    }

    pub fn build_source(&self) -> Result<FailoverPageSource, IndexerServiceError> {
        let mut providers: Vec<Arc<dyn FinalizedPageSource>> =
            Vec::with_capacity(self.rpc_endpoints.len());
        for endpoint in &self.rpc_endpoints {
            let transport = if self.allow_insecure_local_rpc {
                ReqwestJsonRpcTransport::new_for_local_development(
                    endpoint,
                    self.rpc_timeout,
                    MAX_RPC_RESPONSE_BYTES,
                )?
            } else {
                ReqwestJsonRpcTransport::new(endpoint, self.rpc_timeout, MAX_RPC_RESPONSE_BYTES)?
            };
            providers.push(Arc::new(SolanaRpcPageSource::new(
                transport,
                SettlementExecutedDecoder,
            )));
        }
        Ok(FailoverPageSource::new(providers, self.rpc_timeout)?)
    }

    pub fn build_targeted_recovery_sources(
        &self,
    ) -> Result<(FailoverSignaturePageSource, FailoverTransactionSource), IndexerServiceError> {
        build_targeted_recovery_sources(
            &self.rpc_endpoints,
            self.rpc_timeout,
            self.allow_insecure_local_rpc,
        )
    }

    pub fn build_vault_balance_source(
        &self,
    ) -> Result<FailoverVaultBalanceSource, IndexerServiceError> {
        build_vault_balance_source(
            &self.rpc_endpoints,
            self.rpc_timeout,
            self.allow_insecure_local_rpc,
        )
    }

    pub fn build_refund_balance_proof_source(
        &self,
    ) -> Result<FailoverRefundBalanceProofSource, IndexerServiceError> {
        build_refund_balance_proof_source(
            &self.rpc_endpoints,
            self.rpc_timeout,
            self.allow_insecure_local_rpc,
        )
    }
}

/// Builds the two independent, failover-bounded ports required by targeted
/// recovery. The API uses this helper only when its explicit recovery RPC
/// configuration is present; the normal indexer process uses the method above.
pub fn build_targeted_recovery_sources(
    rpc_endpoints: &[String],
    rpc_timeout: Duration,
    allow_insecure_local_rpc: bool,
) -> Result<(FailoverSignaturePageSource, FailoverTransactionSource), IndexerServiceError> {
    if rpc_endpoints.is_empty() || rpc_endpoints.len() > MAX_PROVIDER_ATTEMPTS {
        return Err(IndexerServiceError::Configuration(
            "targeted recovery RPC endpoints must contain between 1 and 3 endpoints".to_string(),
        ));
    }
    let mut signature_providers: Vec<Arc<dyn FinalizedSignaturePageSource>> =
        Vec::with_capacity(rpc_endpoints.len());
    let mut transaction_providers: Vec<Arc<dyn FinalizedTransactionSource>> =
        Vec::with_capacity(rpc_endpoints.len());
    for endpoint in rpc_endpoints {
        let transport = if allow_insecure_local_rpc {
            ReqwestJsonRpcTransport::new_for_local_development(
                endpoint,
                rpc_timeout,
                MAX_RPC_RESPONSE_BYTES,
            )?
        } else {
            ReqwestJsonRpcTransport::new(endpoint, rpc_timeout, MAX_RPC_RESPONSE_BYTES)?
        };
        signature_providers.push(Arc::new(SolanaRpcSignaturePageSource::new(
            transport.clone(),
        )));
        transaction_providers.push(Arc::new(SolanaRpcTransactionSource::new(
            transport,
            SettlementExecutedDecoder,
        )));
    }
    Ok((
        FailoverSignaturePageSource::new(signature_providers, rpc_timeout)?,
        FailoverTransactionSource::new(transaction_providers, rpc_timeout)?,
    ))
}

pub fn build_vault_balance_source(
    rpc_endpoints: &[String],
    rpc_timeout: Duration,
    allow_insecure_local_rpc: bool,
) -> Result<FailoverVaultBalanceSource, IndexerServiceError> {
    if rpc_endpoints.is_empty() || rpc_endpoints.len() > MAX_PROVIDER_ATTEMPTS {
        return Err(IndexerServiceError::Configuration(
            "vault reconciliation RPC endpoints must contain between 1 and 3 endpoints".to_string(),
        ));
    }
    let mut providers: Vec<Arc<dyn FinalizedVaultBalanceSource>> =
        Vec::with_capacity(rpc_endpoints.len());
    for endpoint in rpc_endpoints {
        let transport = if allow_insecure_local_rpc {
            ReqwestJsonRpcTransport::new_for_local_development(
                endpoint,
                rpc_timeout,
                MAX_RPC_RESPONSE_BYTES,
            )?
        } else {
            ReqwestJsonRpcTransport::new(endpoint, rpc_timeout, MAX_RPC_RESPONSE_BYTES)?
        };
        providers.push(Arc::new(SolanaRpcVaultBalanceSource::new(transport)));
    }
    Ok(FailoverVaultBalanceSource::new(providers, rpc_timeout)?)
}

pub fn build_refund_balance_proof_source(
    rpc_endpoints: &[String],
    rpc_timeout: Duration,
    allow_insecure_local_rpc: bool,
) -> Result<FailoverRefundBalanceProofSource, IndexerServiceError> {
    if rpc_endpoints.is_empty() || rpc_endpoints.len() > MAX_PROVIDER_ATTEMPTS {
        return Err(IndexerServiceError::Configuration(
            "refund proof RPC endpoints must contain between 1 and 3 endpoints".to_string(),
        ));
    }
    let mut providers: Vec<Arc<dyn FinalizedRefundBalanceProofSource>> =
        Vec::with_capacity(rpc_endpoints.len());
    for endpoint in rpc_endpoints {
        let transport = if allow_insecure_local_rpc {
            ReqwestJsonRpcTransport::new_for_local_development(
                endpoint,
                rpc_timeout,
                MAX_RPC_RESPONSE_BYTES,
            )?
        } else {
            ReqwestJsonRpcTransport::new(endpoint, rpc_timeout, MAX_RPC_RESPONSE_BYTES)?
        };
        providers.push(Arc::new(SolanaRpcRefundBalanceProofSource::new(transport)));
    }
    Ok(FailoverRefundBalanceProofSource::new(
        providers,
        rpc_timeout,
    )?)
}

#[derive(Debug, Error)]
pub enum IndexerServiceError {
    #[error("invalid indexer configuration: {0}")]
    Configuration(String),
    #[error("provider failed: {0}")]
    Provider(#[from] ProviderError),
    #[error("indexer domain failed: {0}")]
    Domain(#[from] indexer::IndexerError),
    #[error("storage failed: {0}")]
    Storage(#[from] StorageError),
    #[error("could not persist replay gap after {primary}: {gap}")]
    GapRecordFailed { primary: String, gap: StorageError },
    #[error("replay claim was lost before completion")]
    ReplayLeaseLost,
    #[error("targeted recovery target {signature} was not found within {pages_scanned} pages")]
    TargetNotFoundWithinBudget {
        signature: String,
        pages_scanned: usize,
    },
    #[error("targeted recovery response is invalid: {0}")]
    TargetedRecoveryInvalid(String),
    #[error("refund balance proof domain validation failed: {0}")]
    RefundProofDomain(String),
    #[error("refund claim reconciliation domain validation failed: {0}")]
    RefundClaimDomain(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayOutcome {
    Idle,
    Completed,
    ChunkRequeued,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetedRecoveryOutcome {
    pub signature: String,
    pub pages_scanned: usize,
    pub slot: u64,
    pub raw_transaction_count: usize,
    pub raw_evidence_count: usize,
    pub receipt_count: usize,
}

#[derive(Clone, Debug, Default)]
pub struct ReplayMetrics {
    queued: Arc<AtomicU64>,
    leased: Arc<AtomicU64>,
    dead_lettered: Arc<AtomicU64>,
    leased_slots: Arc<AtomicU64>,
    refund_proofs_pending: Arc<AtomicU64>,
    refund_claims_pending: Arc<AtomicU64>,
    targeted_recovery_stale_requested: Arc<AtomicU64>,
    vault_reconciliation_mismatches: Arc<AtomicU64>,
    vault_reconciliation_errors: Arc<AtomicU64>,
    protocol_pause_known: Arc<std::sync::atomic::AtomicBool>,
    protocol_paused: Arc<std::sync::atomic::AtomicBool>,
    protocol_pause_slot: Arc<AtomicU64>,
}

impl ReplayMetrics {
    pub fn set_snapshot(
        &self,
        snapshot: ReplayGapMetricsSnapshot,
        targeted_recovery: TargetedRecoveryMetricsSnapshot,
        vault_reconciliation: VaultReconciliationMetricsSnapshot,
        refund_proofs: RefundProofMetricsSnapshot,
        refund_claims: RefundClaimReconciliationMetricsSnapshot,
        protocol_pause: ProtocolPauseMetricsSnapshot,
    ) {
        self.queued.store(snapshot.queued, Ordering::Release);
        self.leased.store(snapshot.leased, Ordering::Release);
        self.dead_lettered
            .store(snapshot.dead_lettered, Ordering::Release);
        self.leased_slots
            .store(snapshot.leased_slots, Ordering::Release);
        self.refund_proofs_pending
            .store(refund_proofs.pending, Ordering::Release);
        self.refund_claims_pending
            .store(refund_claims.pending, Ordering::Release);
        self.targeted_recovery_stale_requested
            .store(targeted_recovery.stale_requested, Ordering::Release);
        self.vault_reconciliation_mismatches
            .store(vault_reconciliation.mismatches, Ordering::Release);
        self.vault_reconciliation_errors
            .store(vault_reconciliation.errors, Ordering::Release);
        self.protocol_pause_known
            .store(protocol_pause.known, Ordering::Release);
        self.protocol_paused
            .store(protocol_pause.paused, Ordering::Release);
        self.protocol_pause_slot
            .store(protocol_pause.slot, Ordering::Release);
    }

    pub fn render_prometheus(&self) -> String {
        let mut output = String::from("# TYPE stp_indexer_replay_gaps gauge\n");
        output.push_str(&format!(
            "stp_indexer_replay_gaps{{service=\"indexer\",status=\"queued\"}} {}\n",
            self.queued.load(Ordering::Acquire)
        ));
        output.push_str(&format!(
            "stp_indexer_replay_gaps{{service=\"indexer\",status=\"leased\"}} {}\n",
            self.leased.load(Ordering::Acquire)
        ));
        output.push_str(&format!(
            "stp_indexer_replay_gaps{{service=\"indexer\",status=\"dead_lettered\"}} {}\n",
            self.dead_lettered.load(Ordering::Acquire)
        ));
        output.push_str("# TYPE stp_indexer_replay_leased_slots gauge\n");
        output.push_str(&format!(
            "stp_indexer_replay_leased_slots{{service=\"indexer\"}} {}\n",
            self.leased_slots.load(Ordering::Acquire)
        ));
        output.push_str("# TYPE stp_indexer_targeted_recovery_stale_requested gauge\n");
        output.push_str(&format!(
            "stp_indexer_targeted_recovery_stale_requested{{service=\"indexer\"}} {}\n",
            self.targeted_recovery_stale_requested
                .load(Ordering::Acquire)
        ));
        output.push_str("# TYPE stp_indexer_vault_reconciliation_mismatch gauge\n");
        output.push_str(&format!(
            "stp_indexer_vault_reconciliation_mismatch{{service=\"indexer\"}} {}\n",
            self.vault_reconciliation_mismatches.load(Ordering::Acquire)
        ));
        output.push_str("# TYPE stp_indexer_vault_reconciliation_error gauge\n");
        output.push_str(&format!(
            "stp_indexer_vault_reconciliation_error{{service=\"indexer\"}} {}\n",
            self.vault_reconciliation_errors.load(Ordering::Acquire)
        ));
        output.push_str("# TYPE stp_indexer_refund_proofs_pending gauge\n");
        output.push_str(&format!(
            "stp_indexer_refund_proofs_pending{{service=\"indexer\"}} {}\n",
            self.refund_proofs_pending.load(Ordering::Acquire)
        ));
        output.push_str("# TYPE stp_indexer_refund_claims_pending gauge\n");
        output.push_str(&format!(
            "stp_indexer_refund_claims_pending{{service=\"indexer\"}} {}\n",
            self.refund_claims_pending.load(Ordering::Acquire)
        ));
        output.push_str("# TYPE stp_settlement_protocol_pause_known gauge\n");
        output.push_str(&format!(
            "stp_settlement_protocol_pause_known{{service=\"indexer\"}} {}\n",
            u8::from(self.protocol_pause_known.load(Ordering::Acquire))
        ));
        output.push_str("# TYPE stp_settlement_protocol_paused gauge\n");
        output.push_str(&format!(
            "stp_settlement_protocol_paused{{service=\"indexer\"}} {}\n",
            u8::from(self.protocol_paused.load(Ordering::Acquire))
        ));
        output.push_str("# TYPE stp_settlement_protocol_pause_slot gauge\n");
        output.push_str(&format!(
            "stp_settlement_protocol_pause_slot{{service=\"indexer\"}} {}\n",
            self.protocol_pause_slot.load(Ordering::Acquire)
        ));
        output
    }
}

pub async fn refresh_replay_metrics(
    store: &PgStore,
    stream_key: &str,
    metrics: &ReplayMetrics,
) -> Result<(), StorageError> {
    let (
        replay,
        targeted_recovery,
        vault_reconciliation,
        refund_proofs,
        refund_claims,
        protocol_pause,
    ) = tokio::try_join!(
        store.load_replay_gap_metrics(stream_key),
        store.load_targeted_recovery_metrics(),
        store.load_vault_reconciliation_metrics(),
        store.load_refund_proof_metrics(stream_key),
        store.load_refund_claim_reconciliation_metrics(),
        store.load_protocol_pause_metrics(stream_key),
    )?;
    metrics.set_snapshot(
        replay,
        targeted_recovery,
        vault_reconciliation,
        refund_proofs,
        refund_claims,
        protocol_pause,
    );
    Ok(())
}

/// Runs one sequential, bounded reconciliation cycle. A provider or target
/// error is persisted as an alertable run and does not prevent other targets
/// from being inspected; no task is detached per target.
pub async fn reconcile_vaults_once(
    store: &PgStore,
    source: &FailoverVaultBalanceSource,
    target_limit: usize,
) -> Result<(), IndexerServiceError> {
    let targets = store
        .load_active_vault_reconciliation_targets(target_limit)
        .await?;
    for target in targets {
        let expectation = VaultBalanceExpectation::new(
            target.opening_balance_atomic,
            target.receipt_tax_atomic,
            target.release_atomic,
        );
        let Some(expected_balance) = expectation.expected_balance().ok() else {
            store
                .record_vault_reconciliation_run(VaultReconciliationRun {
                    target_id: target.id,
                    route_id: target.route_id,
                    merchant_id: target.merchant_id,
                    stream_key: target.stream_key,
                    mint: target.mint,
                    token_program: target.token_program,
                    tax_vault: target.tax_vault,
                    expectation,
                    expected_balance_atomic: None,
                    observed_balance_atomic: None,
                    observed_slot: None,
                    status: "error",
                    error_code: Some("projection_balance_out_of_range"),
                })
                .await?;
            continue;
        };
        let rpc_target = VaultBalanceTarget {
            tax_vault: target.tax_vault.clone(),
            mint: target.mint.clone(),
            token_program: target.token_program.clone(),
        };
        let observation = match source.fetch_finalized_vault_balance(&rpc_target).await {
            Ok(observation) => observation,
            Err(error) => {
                store
                    .record_vault_reconciliation_run(VaultReconciliationRun {
                        target_id: target.id,
                        route_id: target.route_id,
                        merchant_id: target.merchant_id,
                        stream_key: target.stream_key,
                        mint: target.mint,
                        token_program: target.token_program,
                        tax_vault: target.tax_vault,
                        expectation,
                        expected_balance_atomic: Some(expected_balance),
                        observed_balance_atomic: None,
                        observed_slot: None,
                        status: "error",
                        error_code: Some(vault_provider_error_code(&error)),
                    })
                    .await?;
                continue;
            }
        };
        let status =
            match indexer::reconcile_vault_balance(expected_balance, observation.amount_atomic) {
                VaultBalanceReconciliation::Matched { .. } => "matched",
                VaultBalanceReconciliation::Mismatch { .. } => "mismatch",
            };
        store
            .record_vault_reconciliation_run(VaultReconciliationRun {
                target_id: target.id,
                route_id: target.route_id,
                merchant_id: target.merchant_id,
                stream_key: target.stream_key,
                mint: target.mint,
                token_program: target.token_program,
                tax_vault: target.tax_vault,
                expectation,
                expected_balance_atomic: Some(expected_balance),
                observed_balance_atomic: Some(observation.amount_atomic),
                observed_slot: Some(observation.slot),
                status,
                error_code: None,
            })
            .await?;
    }
    Ok(())
}

struct PostgresRefundProofCandidateSource {
    store: Arc<PgStore>,
    stream_key: String,
}

#[async_trait]
impl RefundProofCandidateSource for PostgresRefundProofCandidateSource {
    async fn claim_next_refund_proof_candidate(
        &self,
    ) -> Result<Option<RefundProofCandidate>, RefundProofWorkerError> {
        let Some(candidate) = self
            .store
            .load_pending_refund_balance_proof_execution(&self.stream_key)
            .await
            .map_err(|error| RefundProofWorkerError::Persistence(error.to_string()))?
        else {
            return Ok(None);
        };
        let execution = candidate.execution;
        Ok(Some(RefundProofCandidate {
            stream_key: self.stream_key.clone(),
            lease_token: candidate.lease_token,
            topology_request: RefundClaimTopologyRequest {
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
            },
            execution,
        }))
    }

    async fn record_refund_proof_failure(
        &self,
        candidate: &RefundProofCandidate,
        reason: &str,
    ) -> Result<(), RefundProofWorkerError> {
        let owned = self
            .store
            .record_refund_proof_candidate_failure(
                &candidate.stream_key,
                &candidate.execution,
                candidate.lease_token,
                reason,
            )
            .await
            .map_err(|error| RefundProofWorkerError::Persistence(error.to_string()))?;
        if !owned {
            return Err(RefundProofWorkerError::Persistence(
                "refund proof failure lease was lost before recording".to_string(),
            ));
        }
        Ok(())
    }
}

struct PostgresRefundProofSink {
    store: Arc<PgStore>,
}

#[async_trait]
impl RefundProofSink for PostgresRefundProofSink {
    async fn persist_refund_proof(
        &self,
        evidence: RefundProofEvidence,
    ) -> Result<(), RefundProofWorkerError> {
        let target = evidence.topology;
        let topology = RefundBalanceProofTopology {
            claim: parse_refund_key(&target.claim, "claim")
                .map_err(|error| RefundProofWorkerError::Persistence(error.to_string()))?,
            adjustment: parse_refund_key(&target.adjustment, "adjustment")
                .map_err(|error| RefundProofWorkerError::Persistence(error.to_string()))?,
            original_receipt: parse_refund_key(&target.original_receipt, "original receipt")
                .map_err(|error| RefundProofWorkerError::Persistence(error.to_string()))?,
            merchant: parse_refund_key(&target.merchant, "merchant")
                .map_err(|error| RefundProofWorkerError::Persistence(error.to_string()))?,
            mint: parse_refund_key(&target.mint, "mint")
                .map_err(|error| RefundProofWorkerError::Persistence(error.to_string()))?,
            token_program: parse_refund_key(&target.token_program, "token program")
                .map_err(|error| RefundProofWorkerError::Persistence(error.to_string()))?,
            tax_vault: parse_refund_key(&target.tax_vault, "tax vault")
                .map_err(|error| RefundProofWorkerError::Persistence(error.to_string()))?,
            refund_source: parse_refund_key(&target.refund_source, "refund source")
                .map_err(|error| RefundProofWorkerError::Persistence(error.to_string()))?,
            payer_destination: parse_refund_key(&target.payer_destination, "payer destination")
                .map_err(|error| RefundProofWorkerError::Persistence(error.to_string()))?,
            vault_authority: parse_refund_key(&target.vault_authority, "vault authority")
                .map_err(|error| RefundProofWorkerError::Persistence(error.to_string()))?,
            refund_funder: parse_refund_key(&target.refund_funder, "refund funder")
                .map_err(|error| RefundProofWorkerError::Persistence(error.to_string()))?,
            payer: parse_refund_key(&target.payer, "payer")
                .map_err(|error| RefundProofWorkerError::Persistence(error.to_string()))?,
        };
        self.store
            .persist_refund_balance_proof(
                &evidence.stream_key,
                &evidence.execution,
                evidence.lease_token,
                topology,
                evidence.proof,
            )
            .await
            .map_err(|error| RefundProofWorkerError::Persistence(error.to_string()))?;
        Ok(())
    }
}

fn map_refund_proof_worker_error(error: RefundProofWorkerError) -> IndexerServiceError {
    match error {
        RefundProofWorkerError::Provider(error) => IndexerServiceError::Provider(error),
        RefundProofWorkerError::InvalidCandidate(message)
        | RefundProofWorkerError::Arithmetic(message)
        | RefundProofWorkerError::Persistence(message) => {
            IndexerServiceError::RefundProofDomain(message)
        }
    }
}

/// Proves and persists at most one refund balance transition. PostgreSQL
/// claims the candidate with a UUIDv7 lease before this bounded worker calls
/// external RPC; persistence is fenced by the same token.
pub async fn prove_one_refund_balance_once(
    store: &PgStore,
    source: Arc<FailoverRefundBalanceProofSource>,
    stream_key: &str,
) -> Result<bool, IndexerServiceError> {
    let worker = RefundProofWorker::new(
        Arc::new(PostgresRefundProofCandidateSource {
            store: Arc::new(store.clone()),
            stream_key: stream_key.to_string(),
        }),
        source,
        Arc::new(PostgresRefundProofSink {
            store: Arc::new(store.clone()),
        }),
    );
    match worker
        .process_one()
        .await
        .map_err(map_refund_proof_worker_error)?
    {
        RefundProofCycleResult::NoCandidate => Ok(false),
        RefundProofCycleResult::Persisted { .. } => Ok(true),
    }
}

/// Reconciles at most one server-owned refund claim. The candidate, target
/// signature, proposer address, event expectation, and lifecycle transition
/// all come from durable intent/artifact evidence. No browser or operator
/// parameter can redirect this worker to another transaction.
pub async fn reconcile_one_refund_claim_once<P, T>(
    store: &PgStore,
    signature_source: &P,
    transaction_source: &T,
    stream_key: &str,
    max_pages: usize,
) -> Result<bool, IndexerServiceError>
where
    P: FinalizedSignaturePageSource + ?Sized,
    T: FinalizedTransactionSource + ?Sized,
{
    let Some(candidate) = store.load_pending_refund_claim_reconciliation().await? else {
        return Ok(false);
    };
    let claim_token = candidate.claim_token;
    let intent = candidate.intent;
    let artifact = candidate.artifact;
    let signature = intent.signature.clone().ok_or_else(|| {
        IndexerServiceError::RefundClaimDomain(
            "reconciliation candidate has no persisted transaction signature".into(),
        )
    })?;
    let proposer = artifact
        .instruction
        .accounts
        .first()
        .map(|account| account.pubkey.as_str())
        .ok_or_else(|| {
            IndexerServiceError::RefundClaimDomain(
                "refund claim artifact has no proposer account".into(),
            )
        })?;
    let proposer = Pubkey::from_str(proposer).map_err(|_| {
        IndexerServiceError::RefundClaimDomain(
            "refund claim artifact proposer is not a canonical public key".into(),
        )
    })?;
    if proposer == Pubkey::default() {
        return Err(IndexerServiceError::RefundClaimDomain(
            "refund claim artifact proposer is the default public key".into(),
        ));
    }
    let recovery = recover_targeted_signature(
        store,
        signature_source,
        transaction_source,
        stream_key,
        &proposer.to_string(),
        &signature,
        max_pages,
    )
    .await?;
    let expected = refund_claim_evidence_expectation(&intent, &artifact)
        .map_err(|error| IndexerServiceError::RefundClaimDomain(error.to_string()))?;
    let evidence = store
        .load_finalized_refund_claim_evidence(stream_key, &signature, &expected)
        .await?;
    if evidence.signature != signature || evidence.slot != recovery.slot {
        return Err(IndexerServiceError::RefundClaimDomain(
            "finalized claim event and recovered transaction disagree".into(),
        ));
    }
    let original_status = intent.status.clone();
    if original_status == "submitting"
        && !store
            .transition_refund_claim_intent(storage::RefundClaimIntentTransition {
                intent_id: intent.id,
                merchant_id: intent.merchant_id,
                expected_status: "submitting".into(),
                next_status: "submitted".into(),
                signed_transaction_digest: intent.signed_transaction_digest,
                signature: Some(evidence.signature.clone()),
                slot: None,
                last_error_code: None,
                evidence: serde_json::json!({
                    "source": "automatic_refund_claim_recovery",
                    "stream_key": stream_key,
                    "event_index": evidence.event_index,
                    "event_digest": evidence.evidence_digest,
                    "recovery_pages_scanned": recovery.pages_scanned,
                }),
                reconciliation_claim_token: Some(claim_token),
            })
            .await?
    {
        return Ok(false);
    }
    let expected_status = if original_status == "submitting" {
        "submitted"
    } else {
        original_status.as_str()
    };
    let transitioned = store
        .transition_refund_claim_intent(storage::RefundClaimIntentTransition {
            intent_id: intent.id,
            merchant_id: intent.merchant_id,
            expected_status: expected_status.into(),
            next_status: "finalized".into(),
            signed_transaction_digest: intent.signed_transaction_digest,
            signature: Some(evidence.signature.clone()),
            slot: Some(evidence.slot),
            last_error_code: None,
            evidence: serde_json::json!({
                "source": "automatic_finalized_refund_claim_reconciler",
                "stream_key": stream_key,
                "event_index": evidence.event_index,
                "event_digest": evidence.evidence_digest,
                "recovery_pages_scanned": recovery.pages_scanned,
            }),
            reconciliation_claim_token: Some(claim_token),
        })
        .await?;
    Ok(transitioned)
}

fn parse_refund_key(value: &str, field: &str) -> Result<[u8; 32], IndexerServiceError> {
    Pubkey::from_str(value.trim())
        .map(|key| key.to_bytes())
        .map_err(|_| {
            IndexerServiceError::RefundProofDomain(format!("{field} is not a canonical public key"))
        })
}

pub async fn run_once(
    store: &PgStore,
    source: &FailoverPageSource,
    config: &IndexerConfig,
) -> Result<Checkpoint, IndexerServiceError> {
    let checkpoint = store
        .load_checkpoint(&config.stream_key)
        .await?
        .unwrap_or_else(|| Checkpoint::new(config.start_slot, config.start_finalized_slot));
    let coordinator = IndexerCoordinator::new();
    let planned = coordinator.plan(checkpoint, config.page_slots)?;
    let page = match coordinator.fetch(planned, source).await {
        Ok(page) => page,
        Err(error) => {
            return record_gap(
                store,
                &config.stream_key,
                ReplayGap {
                    range: planned.range,
                    observed_finalized_slot: checkpoint.finalized_slot,
                    reason: provider_gap_reason(&error),
                    attempts: 0,
                },
                IndexerServiceError::Provider(error),
            )
            .await;
        }
    };
    match store
        .persist_finalized_page(&config.stream_key, checkpoint, &page)
        .await
    {
        Ok(next_checkpoint) => Ok(next_checkpoint),
        Err(error) => {
            record_gap(
                store,
                &config.stream_key,
                ReplayGap {
                    range: page.range,
                    observed_finalized_slot: page.observed_finalized_slot,
                    reason: ReplayGapReason::ProjectionCommitFailure,
                    attempts: 0,
                },
                IndexerServiceError::Storage(error),
            )
            .await
        }
    }
}

/// Execute one bounded chunk of the oldest leased replay gap.
///
/// Replay never advances `chain_checkpoints`. A successful chunk is committed
/// to the receipt/evidence projections and then advances the lease-fenced gap
/// row; a provider or projection failure releases the row through the bounded
/// retry/dead-letter state machine.
pub async fn replay_once(
    store: &PgStore,
    source: &FailoverPageSource,
    config: &IndexerConfig,
) -> Result<ReplayOutcome, IndexerServiceError> {
    let mut claimed = store
        .claim_replay_gaps(
            &config.stream_key,
            REPLAY_CLAIM_BATCH_SIZE,
            config.replay_lease,
        )
        .await?;
    let Some(record) = claimed.pop() else {
        return Ok(ReplayOutcome::Idle);
    };

    let chunk_range = bounded_replay_chunk(record.work.range, config.page_slots)?;
    let chunk_end = chunk_range.end_slot;
    let coordinator = IndexerCoordinator::new();
    let checkpoint = Checkpoint::new(chunk_range.start_slot, record.work.observed_finalized_slot);
    let planned = coordinator.plan(
        checkpoint,
        chunk_range
            .end_slot
            .checked_sub(chunk_range.start_slot)
            .and_then(|length| length.checked_add(1))
            .ok_or(IndexerServiceError::Domain(
                indexer::IndexerError::InvalidRange,
            ))?,
    )?;
    let page = match coordinator.fetch(planned, source).await {
        Ok(page) => page,
        Err(error) => {
            fail_replay(store, &record, replay_error_code(&error)).await?;
            return Err(IndexerServiceError::Provider(error));
        }
    };
    if let Err(error) = store.persist_replayed_page(&config.stream_key, &page).await {
        fail_replay(store, &record, "projection_persist_failed").await?;
        return Err(IndexerServiceError::Storage(error));
    }

    let next_start = chunk_end.checked_add(1).ok_or(IndexerServiceError::Domain(
        indexer::IndexerError::InvalidRange,
    ))?;
    if !store
        .advance_replay_gap(record.id, record.claim_token, next_start)
        .await?
    {
        return Err(IndexerServiceError::ReplayLeaseLost);
    }
    if next_start <= record.work.range.end_slot {
        Ok(ReplayOutcome::ChunkRequeued)
    } else {
        Ok(ReplayOutcome::Completed)
    }
}

/// Discover and persist one operator-selected finalized transaction without
/// mutating the forward checkpoint. Authorization and audit evidence belong to
/// the caller/control-plane wrapper; this function is only the bounded domain
/// and persistence operation.
pub async fn recover_targeted_signature<P, T>(
    store: &PgStore,
    signature_source: &P,
    transaction_source: &T,
    stream_key: &str,
    address: &str,
    target_signature: &str,
    max_pages: usize,
) -> Result<TargetedRecoveryOutcome, IndexerServiceError>
where
    P: FinalizedSignaturePageSource + ?Sized,
    T: FinalizedTransactionSource + ?Sized,
{
    if max_pages == 0 || max_pages > MAX_TARGETED_RECOVERY_PAGES {
        return Err(IndexerServiceError::Configuration(format!(
            "targeted recovery page budget must be between 1 and {MAX_TARGETED_RECOVERY_PAGES}"
        )));
    }
    let target_signature = canonical_signature(target_signature)?;
    let mut before = None;
    let mut pages_scanned = 0_usize;
    let mut found = false;
    loop {
        pages_scanned = pages_scanned.saturating_add(1);
        let page = signature_source
            .fetch_finalized_signature_page(address, before.as_deref(), TARGETED_RECOVERY_PAGE_SIZE)
            .await?;
        if page
            .entries
            .iter()
            .any(|entry| entry.signature == target_signature)
        {
            found = true;
            break;
        }
        if page.complete || pages_scanned == max_pages {
            break;
        }
        before = page.next_before;
        if before.is_none() {
            return Err(IndexerServiceError::TargetedRecoveryInvalid(
                "full signature page did not provide a next cursor".to_string(),
            ));
        }
    }
    if !found {
        return Err(IndexerServiceError::TargetNotFoundWithinBudget {
            signature: target_signature,
            pages_scanned,
        });
    }

    let decoded = transaction_source
        .fetch_finalized_transaction(&target_signature)
        .await?;
    if decoded.raw_transactions.len() != 1 {
        return Err(IndexerServiceError::TargetedRecoveryInvalid(
            "finalized transaction response must decode to exactly one raw transaction".to_string(),
        ));
    }
    let raw_transaction = &decoded.raw_transactions[0];
    if raw_transaction.signature != target_signature {
        return Err(IndexerServiceError::TargetedRecoveryInvalid(
            "decoded raw transaction signature does not match the target".to_string(),
        ));
    }
    let slot = raw_transaction.slot;
    let page = indexer::FinalizedPage {
        range: ScanRange {
            start_slot: slot,
            end_slot: slot,
        },
        observed_finalized_slot: slot,
        receipts: decoded.receipts,
        split_settlements: decoded.split_settlements,
        adjustment_records: decoded.adjustment_records,
        refund_executions: decoded.refund_executions,
        raw_evidence: decoded.raw_evidence,
        raw_transactions: decoded.raw_transactions,
        protocol_pause_changes: decoded.protocol_pause_changes,
        complete: true,
    };
    let outcome = TargetedRecoveryOutcome {
        signature: target_signature,
        pages_scanned,
        slot,
        raw_transaction_count: page.raw_transactions.len(),
        raw_evidence_count: page.raw_evidence.len(),
        receipt_count: page.receipts.len(),
    };
    store.persist_replayed_page(stream_key, &page).await?;
    Ok(outcome)
}

async fn fail_replay(
    store: &PgStore,
    record: &storage::ReplayGapRecord,
    error_code: &str,
) -> Result<(), IndexerServiceError> {
    if store
        .fail_replay_gap(record.id, record.claim_token, error_code)
        .await?
    {
        Ok(())
    } else {
        Err(IndexerServiceError::ReplayLeaseLost)
    }
}

fn replay_error_code(error: &ProviderError) -> &'static str {
    match error {
        ProviderError::Transport(_) => "provider_transport_failed",
        ProviderError::ResponseTooLarge => "provider_response_too_large",
        ProviderError::InvalidResponse(_) => "provider_response_invalid",
        ProviderError::Rpc(_) => "provider_rpc_error",
        ProviderError::InvalidPage(_) => "provider_page_invalid",
        ProviderError::NonFinalized => "provider_not_finalized",
    }
}

fn vault_provider_error_code(error: &ProviderError) -> &'static str {
    match error {
        ProviderError::Transport(_) => "provider_transport_failed",
        ProviderError::ResponseTooLarge => "provider_response_too_large",
        ProviderError::InvalidResponse(_) => "provider_response_invalid",
        ProviderError::Rpc(_) => "provider_rpc_error",
        ProviderError::InvalidPage(_) => "provider_page_invalid",
        ProviderError::NonFinalized => "provider_not_finalized",
    }
}

fn bounded_replay_chunk(
    range: ScanRange,
    max_slots: u64,
) -> Result<ScanRange, IndexerServiceError> {
    if max_slots == 0 {
        return Err(IndexerServiceError::Domain(
            indexer::IndexerError::RangeTooLarge,
        ));
    }
    let end_slot = range
        .start_slot
        .checked_add(max_slots - 1)
        .ok_or(IndexerServiceError::Domain(
            indexer::IndexerError::InvalidRange,
        ))?
        .min(range.end_slot);
    let chunk = ScanRange {
        start_slot: range.start_slot,
        end_slot,
    };
    chunk.validate().map_err(IndexerServiceError::Domain)?;
    Ok(chunk)
}

async fn record_gap(
    store: &PgStore,
    stream_key: &str,
    gap: ReplayGap,
    primary: IndexerServiceError,
) -> Result<Checkpoint, IndexerServiceError> {
    match store.enqueue_replay_gap(stream_key, gap).await {
        Ok(_) => Err(primary),
        Err(gap) => Err(IndexerServiceError::GapRecordFailed {
            primary: primary.to_string(),
            gap,
        }),
    }
}

fn provider_gap_reason(error: &ProviderError) -> ReplayGapReason {
    match error {
        ProviderError::InvalidPage(_) | ProviderError::NonFinalized => {
            ReplayGapReason::ProviderDisagreement
        }
        ProviderError::Transport(_)
        | ProviderError::ResponseTooLarge
        | ProviderError::InvalidResponse(_)
        | ProviderError::Rpc(_) => ReplayGapReason::IncompleteProviderPage,
    }
}

pub fn provider_health(source: &FailoverPageSource) -> Vec<ProviderHealthSnapshot> {
    source.health_snapshot()
}

fn required_env(name: &str) -> Result<String, IndexerServiceError> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| IndexerServiceError::Configuration(format!("{name} is required")))
}

fn parse_env_u64(name: &str, default: Option<u64>) -> Result<u64, IndexerServiceError> {
    match env::var(name) {
        Ok(value) => value.parse().map_err(|_| {
            IndexerServiceError::Configuration(format!("{name} must be an unsigned integer"))
        }),
        Err(_) => {
            default.ok_or_else(|| IndexerServiceError::Configuration(format!("{name} is required")))
        }
    }
}

fn parse_env_usize(name: &str, default: Option<usize>) -> Result<usize, IndexerServiceError> {
    match env::var(name) {
        Ok(value) => value.parse().map_err(|_| {
            IndexerServiceError::Configuration(format!("{name} must be a positive integer"))
        }),
        Err(_) => {
            default.ok_or_else(|| IndexerServiceError::Configuration(format!("{name} is required")))
        }
    }
}

fn parse_env_bool(name: &str, default: bool) -> Result<bool, IndexerServiceError> {
    match env::var(name) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" => Ok(true),
            "0" | "false" | "no" => Ok(false),
            _ => Err(IndexerServiceError::Configuration(format!(
                "{name} must be a boolean"
            ))),
        },
        Err(_) => Ok(default),
    }
}

fn parse_duration_ms(
    name: &str,
    default: Duration,
    range: std::ops::RangeInclusive<u64>,
) -> Result<Duration, IndexerServiceError> {
    let value = match env::var(name) {
        Ok(value) => value.parse().map_err(|_| {
            IndexerServiceError::Configuration(format!("{name} must be an unsigned integer"))
        })?,
        Err(_) => default.as_millis() as u64,
    };
    if !range.contains(&value) {
        return Err(IndexerServiceError::Configuration(format!(
            "{name} is outside its bounded range"
        )));
    }
    Ok(Duration::from_millis(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deployment_environment_is_explicit_and_bounded() {
        assert_eq!(
            parse_deployment_environment("local").unwrap(),
            DeploymentEnvironment::Local
        );
        assert_eq!(
            parse_deployment_environment("staging").unwrap(),
            DeploymentEnvironment::Staging
        );
        assert_eq!(
            parse_deployment_environment("production").unwrap(),
            DeploymentEnvironment::Production
        );
        for value in ["", "mainnet", "LOCAL", " production "] {
            assert!(parse_deployment_environment(value).is_err());
        }
    }

    #[test]
    fn insecure_rpc_is_only_valid_for_local_environment() {
        assert!(DeploymentEnvironment::Local.allows_insecure_rpc(true));
        assert!(DeploymentEnvironment::Local.allows_insecure_rpc(false));
        assert!(DeploymentEnvironment::Staging.allows_insecure_rpc(false));
        assert!(!DeploymentEnvironment::Staging.allows_insecure_rpc(true));
        assert!(DeploymentEnvironment::Production.allows_insecure_rpc(false));
        assert!(!DeploymentEnvironment::Production.allows_insecure_rpc(true));
    }

    #[test]
    fn duration_limits_are_explicit_and_bounded() {
        assert_eq!(
            parse_duration_ms(
                "TEST_INDEXER_DURATION",
                Duration::from_millis(100),
                50..=200
            )
            .unwrap(),
            Duration::from_millis(100)
        );
    }

    #[test]
    fn provider_failures_map_to_replay_reasons_without_downgrading_finality() {
        assert_eq!(
            provider_gap_reason(&ProviderError::NonFinalized),
            ReplayGapReason::ProviderDisagreement
        );
        assert_eq!(
            provider_gap_reason(&ProviderError::Transport("timeout".to_string())),
            ReplayGapReason::IncompleteProviderPage
        );
    }

    #[test]
    fn replay_chunks_are_bounded_and_preserve_the_remaining_suffix() {
        let first = bounded_replay_chunk(
            ScanRange {
                start_slot: 300,
                end_slot: 500,
            },
            128,
        )
        .unwrap();
        assert_eq!(first.start_slot, 300);
        assert_eq!(first.end_slot, 427);
        assert_eq!(first.end_slot + 1, 428);

        let final_chunk = bounded_replay_chunk(
            ScanRange {
                start_slot: 428,
                end_slot: 500,
            },
            128,
        )
        .unwrap();
        assert_eq!(final_chunk.end_slot, 500);
    }

    #[test]
    fn replay_metrics_render_only_fixed_cardinality_series() {
        let metrics = ReplayMetrics::default();
        metrics.set_snapshot(
            ReplayGapMetricsSnapshot {
                queued: 2,
                leased: 1,
                dead_lettered: 3,
                leased_slots: 128,
            },
            TargetedRecoveryMetricsSnapshot { stale_requested: 4 },
            VaultReconciliationMetricsSnapshot {
                mismatches: 5,
                errors: 6,
            },
            RefundProofMetricsSnapshot { pending: 7 },
            RefundClaimReconciliationMetricsSnapshot { pending: 8 },
            ProtocolPauseMetricsSnapshot {
                known: true,
                paused: true,
                slot: 777,
            },
        );
        let rendered = metrics.render_prometheus();

        assert!(
            rendered.contains("stp_indexer_replay_gaps{service=\"indexer\",status=\"queued\"} 2")
        );
        assert!(
            rendered.contains("stp_indexer_replay_gaps{service=\"indexer\",status=\"leased\"} 1")
        );
        assert!(
            rendered.contains(
                "stp_indexer_replay_gaps{service=\"indexer\",status=\"dead_lettered\"} 3"
            )
        );
        assert!(rendered.contains("stp_indexer_replay_leased_slots{service=\"indexer\"} 128"));
        assert!(
            rendered
                .contains("stp_indexer_targeted_recovery_stale_requested{service=\"indexer\"} 4")
        );
        assert!(
            rendered.contains("stp_indexer_vault_reconciliation_mismatch{service=\"indexer\"} 5")
        );
        assert!(rendered.contains("stp_indexer_vault_reconciliation_error{service=\"indexer\"} 6"));
        assert!(rendered.contains("stp_indexer_refund_proofs_pending{service=\"indexer\"} 7"));
        assert!(rendered.contains("stp_indexer_refund_claims_pending{service=\"indexer\"} 8"));
        assert!(rendered.contains("stp_settlement_protocol_pause_known{service=\"indexer\"} 1"));
        assert!(rendered.contains("stp_settlement_protocol_paused{service=\"indexer\"} 1"));
        assert!(rendered.contains("stp_settlement_protocol_pause_slot{service=\"indexer\"} 777"));
        assert!(!rendered.contains("stream_key"));
    }
}
