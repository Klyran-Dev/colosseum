use axum::{Router, extract::State, http::StatusCode, routing::get};
use observability::ServiceMetrics;
use route_verifier::{RouteVerifierConfig, build_store, verify_once};
use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tokio::time::interval;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

async fn metrics(State(state): State<HealthState>) -> impl axum::response::IntoResponse {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        state.metrics.render_prometheus(),
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().json())
        .init();
    let config = RouteVerifierConfig::from_env()?;
    let transports = config.build_transports()?;
    let store = Arc::new(build_store(&config).await?);
    let health_state = HealthState {
        ready: Arc::new(AtomicBool::new(true)),
        metrics: Arc::new(ServiceMetrics::new("route_verifier", &["verify"])?),
    };
    let bind_address = std::env::var("ROUTE_VERIFIER_HEALTH_BIND_ADDRESS")
        .unwrap_or_else(|_| "0.0.0.0:8083".to_string());
    let address: SocketAddr = bind_address.parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    let health_router = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics))
        .with_state(health_state.clone());
    let health_server = async move { axum::serve(listener, health_router).await };
    tokio::pin!(health_server);
    info!(
        providers = transports.len(),
        "settlement route verifier started"
    );
    let mut ticks = interval(config.poll_interval);
    let mut shutdown = Box::pin(async {
        tokio::signal::ctrl_c().await.ok();
    });
    loop {
        tokio::select! {
            result = &mut health_server => {
                return match result {
                    Ok(()) => Err("route verifier health server stopped unexpectedly".into()),
                    Err(error) => Err(error.into()),
                };
            }
            _ = &mut shutdown => { info!("settlement route verifier shutting down"); break; }
            _ = ticks.tick() => {
                let started = Instant::now();
                match verify_once(&store, &transports, config.batch_size, config.rpc_timeout).await {
                    Ok(summary) => {
                        info!(examined = summary.examined, activated = summary.activated, failed = summary.failed, "route verification pass completed");
                        let _ = health_state.metrics.observe("verify", started.elapsed(), summary.failed > 0);
                    }
                    Err(error) => {
                        warn!(%error, "route verification pass failed; routes remain inactive");
                        let _ = health_state.metrics.observe("verify", started.elapsed(), true);
                    }
                }
            }
        }
    }
    Ok(())
}
