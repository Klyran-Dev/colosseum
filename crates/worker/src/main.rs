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
use observability::ServiceMetrics;
use std::{
    env,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use storage::PgStore;
use tokio::time::sleep;
use tracing::{info, warn};
use worker::{
    DEFAULT_STREAM_NAME, NatsJetStreamPublisher, default_lease, dispatch_once, parse_worker_limits,
};

#[derive(Clone)]
struct HealthState {
    ready: Arc<AtomicBool>,
    metrics: Arc<ServiceMetrics>,
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

async fn metrics(State(state): State<HealthState>) -> Response {
    (
        [(CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        state.metrics.render_prometheus(),
    )
        .into_response()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut telemetry = observability::init_tracing("worker")?;

    let database_url = env::var("WORKER_DATABASE_URL")?;
    let nats_url = env::var("WORKER_NATS_URL")?;
    let stream_name =
        env::var("WORKER_NATS_STREAM").unwrap_or_else(|_| DEFAULT_STREAM_NAME.to_string());
    let (batch_size, poll_interval) = parse_worker_limits(
        env::var("WORKER_OUTBOX_BATCH_SIZE").ok().as_deref(),
        env::var("WORKER_OUTBOX_POLL_MS").ok().as_deref(),
    )?;
    let health_bind_address =
        env::var("WORKER_HEALTH_BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8081".to_string());
    let health_address: SocketAddr = health_bind_address.parse()?;
    let health_state = HealthState {
        ready: Arc::new(AtomicBool::new(false)),
        metrics: Arc::new(ServiceMetrics::new("worker", &["dispatch"])?),
    };
    let health_router = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics))
        .with_state(health_state.clone());
    let health_listener = tokio::net::TcpListener::bind(health_address).await?;
    let health_server = async move { axum::serve(health_listener, health_router).await };
    tokio::pin!(health_server);
    let store = Arc::new(PgStore::from_url(
        &database_url,
        8,
        storage::DEFAULT_POOL_WAIT_TIMEOUT,
        tax_engine::ENGINE_VERSION,
    )?);
    store.ping().await?;
    let publisher = NatsJetStreamPublisher::connect(&nats_url, &stream_name).await?;
    health_state.ready.store(true, Ordering::Release);
    info!(%stream_name, batch_size, ?poll_interval, "outbox worker started");

    let mut shutdown = Box::pin(async {
        tokio::signal::ctrl_c().await.ok();
    });
    loop {
        tokio::select! {
            result = &mut health_server => {
                return match result {
                    Ok(()) => Err("worker health server stopped unexpectedly".into()),
                    Err(error) => Err(error.into()),
                };
            }
            _ = &mut shutdown => {
                info!("outbox worker shutting down");
                break;
            }
            _ = sleep(poll_interval) => {
                let started = Instant::now();
                let result = dispatch_once(&store, &publisher, batch_size, default_lease()).await;
                let _ = health_state.metrics.observe("dispatch", started.elapsed(), result.is_err());
                match result {
                    Ok(stats) if stats.claimed > 0 => info!(?stats, "outbox batch processed"),
                    Ok(_) => {}
                    Err(error) => warn!(error = %error, "outbox batch failed; retrying after bounded poll interval"),
                }
            }
        }
    }
    telemetry.shutdown();
    Ok(())
}
