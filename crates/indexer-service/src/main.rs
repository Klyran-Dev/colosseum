#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use axum::{
    Router,
    extract::State,
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::get,
};
use indexer::{FailoverPageSource, ProviderError};
use indexer_service::{
    IndexerConfig, IndexerServiceError, MAX_TARGETED_RECOVERY_PAGES, ReplayMetrics, ReplayOutcome,
    prove_one_refund_balance_once, provider_health, reconcile_one_refund_claim_cancellation_once,
    reconcile_one_refund_claim_once, reconcile_one_revenue_stream_accrual_once,
    reconcile_vaults_once, refresh_replay_metrics, replay_once, run_once,
    verify_one_applicability_binding_once,
};
use observability::ServiceMetrics;
use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use storage::PgStore;
use tokio::time::interval;
use tracing::{info, warn};

const STARTUP_PROVIDER_ATTEMPTS: usize = 30;
const STARTUP_PROVIDER_RETRY: std::time::Duration = std::time::Duration::from_secs(1);

#[derive(Clone)]
struct HealthState {
    ready: Arc<AtomicBool>,
    metrics: Arc<ServiceMetrics>,
    replay_metrics: Arc<ReplayMetrics>,
}

async fn healthz() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn readyz(State(state): State<HealthState>) -> StatusCode {
    if state.ready.load(Ordering::Acquire) {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

async fn wait_for_provider(
    source: &FailoverPageSource,
    start_slot: u64,
) -> Result<(), ProviderError> {
    let mut last_error = None;
    for attempt in 1..=STARTUP_PROVIDER_ATTEMPTS {
        match source.probe(start_slot).await {
            Ok(()) => return Ok(()),
            Err(error) => {
                warn!(attempt, max_attempts = STARTUP_PROVIDER_ATTEMPTS, %error, "indexer RPC provider is not ready");
                last_error = Some(error);
                if attempt < STARTUP_PROVIDER_ATTEMPTS {
                    tokio::time::sleep(STARTUP_PROVIDER_RETRY).await;
                }
            }
        }
    }
    match last_error {
        Some(error) => Err(error),
        None => Err(ProviderError::Transport(
            "indexer RPC readiness probe produced no result".to_string(),
        )),
    }
}

async fn metrics(State(state): State<HealthState>) -> Response {
    let mut rendered = state.metrics.render_prometheus();
    rendered.push_str(&state.replay_metrics.render_prometheus());
    (
        [(CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        rendered,
    )
        .into_response()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut telemetry = observability::init_tracing("indexer")?;

    let config = IndexerConfig::from_env()?;
    let source = config.build_source()?;
    wait_for_provider(&source, config.start_slot).await?;
    let applicability_binding_source = config.build_applicability_binding_source()?;
    let (refund_claim_signature_source, refund_claim_transaction_source) =
        config.build_targeted_recovery_sources()?;
    let vault_source = config.build_vault_balance_source()?;
    let refund_proof_source = Arc::new(config.build_refund_balance_proof_source()?);
    let store = Arc::new(PgStore::from_url(
        &config.database_url,
        config.database_pool_size,
        storage::DEFAULT_POOL_WAIT_TIMEOUT,
        tax_engine::ENGINE_VERSION,
    )?);
    store.ping().await?;
    let replay_metrics = Arc::new(ReplayMetrics::default());
    refresh_replay_metrics(&store, &config.stream_key, &replay_metrics).await?;

    let health_state = HealthState {
        ready: Arc::new(AtomicBool::new(true)),
        metrics: Arc::new(ServiceMetrics::new(
            "indexer",
            &[
                "replay",
                "forward",
                "refund_claim_cancellation_reconcile",
                "refund_claim_reconcile",
                "refund_proof",
                "applicability_binding_verify",
            ],
        )?),
        replay_metrics,
    };
    let health_bind_address =
        std::env::var("INDEXER_HEALTH_BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8082".to_string());
    let health_address: SocketAddr = health_bind_address.parse()?;
    let health_router = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics))
        .with_state(health_state.clone());
    let health_listener = tokio::net::TcpListener::bind(health_address).await?;
    let health_server = async move { axum::serve(health_listener, health_router).await };
    tokio::pin!(health_server);

    info!(
        stream = %config.stream_key,
        providers = config.rpc_endpoints.len(),
        page_slots = config.page_slots,
        "finalized indexer started"
    );
    let mut ticks = interval(config.poll_interval);
    let mut vault_ticks = interval(config.vault_reconciliation_poll_interval);
    let mut shutdown = Box::pin(tokio::signal::ctrl_c());
    loop {
        tokio::select! {
            result = &mut health_server => {
                return match result {
                    Ok(()) => Err("indexer health server stopped unexpectedly".into()),
                    Err(error) => Err(error.into()),
                };
            }
            shutdown_result = &mut shutdown => {
                shutdown_result?;
                info!("finalized indexer shutting down");
                break;
            }
            _ = ticks.tick() => {
                let replay_started = Instant::now();
                let replay_result = replay_once(&store, &source, &config).await;
                health_state.metrics.observe_or_log("replay", replay_started.elapsed(), replay_result.is_err());
                match replay_result {
                    Ok(ReplayOutcome::Idle) => {}
                    Ok(ReplayOutcome::Completed) => info!("replay gap completed"),
                    Ok(ReplayOutcome::ChunkRequeued) => info!("replay chunk committed; gap remains queued"),
                    Err(error) => warn!(%error, "replay iteration failed; bounded lease state retained"),
                }
                let forward_started = Instant::now();
                let forward_result = run_once(&store, &source, &config).await;
                health_state.metrics.observe_or_log("forward", forward_started.elapsed(), forward_result.is_err());
                match forward_result {
                    Ok(checkpoint) => info!(next_slot = checkpoint.next_slot, finalized_slot = checkpoint.finalized_slot, "finalized page committed"),
                    Err(IndexerServiceError::Provider(error)) => warn!(%error, health = ?provider_health(&source), "provider page failed; replay gap recorded"),
                    Err(error) => warn!(%error, "indexer iteration failed; retrying on bounded interval"),
                }
                if let Err(error) = refresh_replay_metrics(&store, &config.stream_key, &health_state.replay_metrics).await {
                    warn!(%error, "replay metrics refresh failed; retaining the last bounded snapshot");
                }
            }
            _ = vault_ticks.tick() => {
                let applicability_started = Instant::now();
                let applicability_result = verify_one_applicability_binding_once(
                    &store,
                    &applicability_binding_source,
                    &config.stream_key,
                )
                .await;
                health_state.metrics.observe_or_log(
                    "applicability_binding_verify",
                    applicability_started.elapsed(),
                    applicability_result.is_err(),
                );
                match applicability_result {
                    Ok(true) => info!("one finalized applicability binding verified"),
                    Ok(false) => {}
                    Err(error) => warn!(%error, "applicability binding verification cycle failed; retaining the bounded lease state"),
                }
                let revenue_started = Instant::now();
                let revenue_result = reconcile_one_revenue_stream_accrual_once(
                    &store,
                    &config.stream_key,
                )
                .await;
                health_state.metrics.observe_or_log(
                    "revenue_stream_accrual_reconcile",
                    revenue_started.elapsed(),
                    revenue_result.is_err(),
                );
                match revenue_result {
                    Ok(true) => info!("one finalized revenue accrual reconciled"),
                    Ok(false) => {}
                    Err(error) => warn!(%error, "revenue accrual reconciliation cycle failed; retaining the leased job for bounded retry"),
                }
                if let Err(error) = reconcile_vaults_once(
                    &store,
                    &vault_source,
                    config.vault_reconciliation_target_limit,
                ).await {
                    warn!(%error, "vault reconciliation cycle failed; retaining the last bounded snapshot");
                }
                let refund_cancellation_started = Instant::now();
                let refund_cancellation_result = reconcile_one_refund_claim_cancellation_once(
                    &store,
                    &refund_claim_signature_source,
                    &refund_claim_transaction_source,
                    &config.stream_key,
                    MAX_TARGETED_RECOVERY_PAGES,
                )
                .await;
                health_state.metrics.observe_or_log(
                    "refund_claim_cancellation_reconcile",
                    refund_cancellation_started.elapsed(),
                    refund_cancellation_result.is_err(),
                );
                match refund_cancellation_result {
                    Ok(true) => info!("one finalized refund claim cancellation reconciled"),
                    Ok(false) => {}
                    Err(error) => warn!(%error, "refund claim cancellation reconciliation cycle failed; retaining the pending evidence candidate"),
                }
                let refund_claim_started = Instant::now();
                let refund_claim_result = reconcile_one_refund_claim_once(
                    &store,
                    &refund_claim_signature_source,
                    &refund_claim_transaction_source,
                    &config.stream_key,
                    MAX_TARGETED_RECOVERY_PAGES,
                )
                .await;
                health_state.metrics.observe_or_log(
                    "refund_claim_reconcile",
                    refund_claim_started.elapsed(),
                    refund_claim_result.is_err(),
                );
                match refund_claim_result {
                    Ok(true) => info!("one finalized refund claim reconciled"),
                    Ok(false) => {}
                    Err(error) => warn!(%error, "refund claim reconciliation cycle failed; retaining the pending evidence candidate"),
                }
                let refund_proof_started = Instant::now();
                let refund_proof_result = prove_one_refund_balance_once(
                    &store,
                    Arc::clone(&refund_proof_source),
                    &config.stream_key,
                )
                .await;
                health_state.metrics.observe_or_log(
                    "refund_proof",
                    refund_proof_started.elapsed(),
                    refund_proof_result.is_err(),
                );
                match refund_proof_result {
                    Ok(true) => info!("one finalized refund balance proof persisted"),
                    Ok(false) => {}
                    Err(error) => warn!(%error, "refund balance proof cycle failed; retaining the pending evidence candidate"),
                }
                if let Err(error) = refresh_replay_metrics(&store, &config.stream_key, &health_state.replay_metrics).await {
                    warn!(%error, "vault reconciliation metrics refresh failed; retaining the last bounded snapshot");
                }
            }
        }
    }
    telemetry.shutdown();
    Ok(())
}
