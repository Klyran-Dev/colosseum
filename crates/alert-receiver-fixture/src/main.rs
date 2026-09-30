#![deny(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use axum::{
    Json, Router,
    body::Bytes,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{env, net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::RwLock};
use tower_http::limit::RequestBodyLimitLayer;

const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:8086";
const DEFAULT_MAX_EVENTS: usize = 128;
const MAX_EVENT_NAMES: usize = 32;
const MAX_NAME_BYTES: usize = 128;

#[derive(Clone)]
struct AppState {
    events: Arc<RwLock<Vec<AlertEventSummary>>>,
    max_events: usize,
}

#[derive(Debug, Deserialize)]
struct AlertmanagerWebhook {
    #[serde(default)]
    status: String,
    #[serde(default)]
    alerts: Vec<Alert>,
}

#[derive(Debug, Deserialize)]
struct Alert {
    #[serde(default)]
    status: String,
    #[serde(default)]
    labels: Value,
}

#[derive(Debug, Serialize, Clone)]
struct AlertEventSummary {
    sequence: u64,
    status: String,
    alert_count: usize,
    alert_names: Vec<String>,
    alert_jobs: Vec<String>,
    alert_statuses: Vec<String>,
}

#[derive(Debug, Serialize)]
struct EventsResponse {
    events: Vec<AlertEventSummary>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind_address = env::var("ALERT_RECEIVER_BIND_ADDRESS")
        .unwrap_or_else(|_| DEFAULT_BIND_ADDRESS.to_string())
        .parse::<SocketAddr>()?;
    let max_events = env::var("ALERT_RECEIVER_MAX_EVENTS")
        .unwrap_or_else(|_| DEFAULT_MAX_EVENTS.to_string())
        .parse::<usize>()?;
    if !(1..=DEFAULT_MAX_EVENTS).contains(&max_events) {
        return Err("ALERT_RECEIVER_MAX_EVENTS must be between 1 and 128".into());
    }

    let state = AppState {
        events: Arc::new(RwLock::new(Vec::with_capacity(max_events))),
        max_events,
    };
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/webhook", post(webhook))
        .route("/admin/events", get(events))
        .layer(RequestBodyLimitLayer::new(64 * 1024))
        .with_state(state);

    let listener = TcpListener::bind(bind_address).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healthz() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn webhook(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let payload = serde_json::from_slice::<AlertmanagerWebhook>(&body)
        .map_err(|_| (StatusCode::BAD_REQUEST, "webhook must be valid JSON"))?;
    let mut recorded = state.events.write().await;
    let sequence = recorded.last().map_or(1, |event| event.sequence + 1);
    let summary =
        summarize(sequence, payload).map_err(|message| (StatusCode::BAD_REQUEST, message))?;
    recorded.push(summary);
    if recorded.len() > state.max_events {
        let excess = recorded.len() - state.max_events;
        recorded.drain(..excess);
    }
    Ok(StatusCode::NO_CONTENT)
}

fn summarize(
    sequence: u64,
    payload: AlertmanagerWebhook,
) -> Result<AlertEventSummary, &'static str> {
    let mut alert_names = Vec::new();
    let mut alert_jobs = Vec::new();
    let mut alert_statuses = Vec::new();
    if payload.alerts.len() > MAX_EVENT_NAMES {
        return Err("alert group is out of bounds");
    }
    for alert in &payload.alerts {
        let name = alert
            .labels
            .get("alertname")
            .and_then(Value::as_str)
            .ok_or("alertname label is required")?;
        if name.is_empty() || name.len() > MAX_NAME_BYTES {
            return Err("alertname is out of bounds");
        }
        alert_names.push(name.to_string());
        let job = alert
            .labels
            .get("job")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        if job.len() > MAX_NAME_BYTES {
            return Err("job label is out of bounds");
        }
        alert_jobs.push(job.to_string());
        alert_statuses.push(if alert.status.is_empty() {
            "unknown".to_string()
        } else {
            alert.status.clone()
        });
    }
    Ok(AlertEventSummary {
        sequence,
        status: if payload.status.is_empty() {
            "unknown".to_string()
        } else {
            payload.status
        },
        alert_count: payload.alerts.len(),
        alert_names,
        alert_jobs,
        alert_statuses,
    })
}

async fn events(State(state): State<AppState>) -> Json<EventsResponse> {
    Json(EventsResponse {
        events: state.events.read().await.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::{AlertmanagerWebhook, summarize};

    #[test]
    fn summary_requires_alertname() {
        let payload = serde_json::from_str::<AlertmanagerWebhook>(
            r#"{"status":"firing","alerts":[{"status":"firing","labels":{}}]}"#,
        )
        .unwrap_or_else(|_| panic!("fixture JSON must parse"));

        assert!(matches!(
            summarize(1, payload),
            Err("alertname label is required")
        ));
    }

    #[test]
    fn summary_is_bounded_and_preserves_alert_identity() {
        let payload = serde_json::from_str::<AlertmanagerWebhook>(
            r#"{"status":"firing","alerts":[{"status":"firing","labels":{"alertname":"STPServiceScrapeUnavailable"}}]}"#,
        )
        .unwrap_or_else(|_| panic!("fixture JSON must parse"));

        let summary = summarize(7, payload).unwrap_or_else(|_| panic!("summary must be valid"));
        assert_eq!(summary.sequence, 7);
        assert_eq!(summary.status, "firing");
        assert_eq!(summary.alert_count, 1);
        assert_eq!(summary.alert_names, ["STPServiceScrapeUnavailable"]);
        assert_eq!(summary.alert_jobs, ["unknown"]);
        assert_eq!(summary.alert_statuses, ["firing"]);
    }
}
