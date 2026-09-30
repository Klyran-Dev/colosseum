#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

//! Bounded transactional-outbox delivery worker.
//!
//! PostgreSQL is the durable event ledger. NATS JetStream is a downstream,
//! acknowledged transport. The worker never marks an event published before
//! the broker acknowledgement and never deletes the source evidence.

use async_nats::jetstream::{self, message::PublishMessage, stream::Config as StreamConfig};
use async_trait::async_trait;
use serde::Serialize;
use std::time::Duration;
use storage::{
    DEFAULT_OUTBOX_BATCH_SIZE, DEFAULT_OUTBOX_LEASE, MAX_OUTBOX_BATCH_SIZE, OutboxEvent, PgStore,
    StorageError,
};
use thiserror::Error;
use uuid::Uuid;

pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(250);
pub const DEFAULT_STREAM_NAME: &str = "TAX_EVENTS";
pub const DEFAULT_SUBJECT_PATTERN: &str = "tax.>";
pub const EVENT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Error)]
pub enum WorkerError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("event serialization failed")]
    Serialization(#[from] serde_json::Error),
    #[error("NATS connection or publish failed")]
    Nats(String),
    #[error("invalid worker configuration: {0}")]
    InvalidConfiguration(String),
}

impl WorkerError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "STORAGE_FAILURE",
            Self::Serialization(_) => "EVENT_SERIALIZATION_FAILURE",
            Self::Nats(_) => "NATS_PUBLISH_FAILURE",
            Self::InvalidConfiguration(_) => "WORKER_CONFIGURATION_FAILURE",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DispatchStats {
    pub claimed: usize,
    pub published: usize,
    pub failed: usize,
    pub stale_claims: usize,
}

#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: &OutboxEvent) -> Result<(), WorkerError>;
}

#[derive(Clone, Debug, Serialize)]
struct EventEnvelope<'a> {
    schema_version: u16,
    event_id: Uuid,
    aggregate_type: &'a str,
    aggregate_id: Uuid,
    event_type: &'a str,
    attempt_count: i32,
    payload: &'a serde_json::Value,
}

pub fn event_subject(event: &OutboxEvent) -> Result<&str, WorkerError> {
    let subject = event.event_type.as_str();
    if subject.is_empty()
        || subject.len() > 255
        || subject.starts_with('.')
        || subject.ends_with('.')
        || subject.contains(' ')
        || subject.contains('*')
        || subject.contains('>')
        || !subject.starts_with("tax.")
    {
        return Err(WorkerError::InvalidConfiguration(
            "outbox event type is not a valid tax subject".to_string(),
        ));
    }
    Ok(subject)
}

pub fn encode_event(event: &OutboxEvent) -> Result<Vec<u8>, WorkerError> {
    Ok(serde_json::to_vec(&EventEnvelope {
        schema_version: EVENT_SCHEMA_VERSION,
        event_id: event.id,
        aggregate_type: &event.aggregate_type,
        aggregate_id: event.aggregate_id,
        event_type: &event.event_type,
        attempt_count: event.attempt_count,
        payload: &event.payload,
    })?)
}

pub async fn dispatch_once<P: EventPublisher>(
    store: &PgStore,
    publisher: &P,
    batch_size: usize,
    lease: Duration,
) -> Result<DispatchStats, WorkerError> {
    let events = store.claim_outbox(batch_size, lease).await?;
    let mut stats = DispatchStats {
        claimed: events.len(),
        ..DispatchStats::default()
    };
    for event in events {
        match publisher.publish(&event).await {
            Ok(()) => {
                if store
                    .mark_outbox_published(event.id, event.claim_token)
                    .await?
                {
                    stats.published += 1;
                } else {
                    stats.stale_claims += 1;
                }
            }
            Err(error) => {
                store
                    .record_outbox_failure(event.id, event.claim_token, error.code())
                    .await?;
                stats.failed += 1;
            }
        }
    }
    Ok(stats)
}

#[derive(Clone)]
pub struct NatsJetStreamPublisher {
    context: jetstream::Context,
}

impl NatsJetStreamPublisher {
    pub async fn connect(url: &str, stream_name: &str) -> Result<Self, WorkerError> {
        if url.trim().is_empty() || stream_name.trim().is_empty() {
            return Err(WorkerError::InvalidConfiguration(
                "NATS URL and stream name are required".to_string(),
            ));
        }
        let client = async_nats::connect(url)
            .await
            .map_err(|error| WorkerError::Nats(error.to_string()))?;
        let context = jetstream::new(client);
        context
            .get_or_create_stream(StreamConfig {
                name: stream_name.to_string(),
                subjects: vec![DEFAULT_SUBJECT_PATTERN.to_string()],
                max_messages: 1_000_000,
                max_age: Duration::from_secs(7 * 24 * 60 * 60),
                duplicate_window: Duration::from_secs(10 * 60),
                description: Some("Solana Tax Protocol durable domain events".to_string()),
                ..StreamConfig::default()
            })
            .await
            .map_err(|error| WorkerError::Nats(error.to_string()))?;
        Ok(Self { context })
    }
}

#[async_trait]
impl EventPublisher for NatsJetStreamPublisher {
    async fn publish(&self, event: &OutboxEvent) -> Result<(), WorkerError> {
        let subject = event_subject(event)?.to_string();
        let payload = encode_event(event)?;
        let message = PublishMessage::build()
            .payload(payload.into())
            .message_id(event.id.to_string());
        let acknowledgement = self
            .context
            .send_publish(subject, message)
            .await
            .map_err(|error| WorkerError::Nats(error.to_string()))?;
        acknowledgement
            .await
            .map_err(|error| WorkerError::Nats(error.to_string()))?;
        Ok(())
    }
}

pub fn parse_worker_limits(
    batch_size: Option<&str>,
    poll_interval_ms: Option<&str>,
) -> Result<(usize, Duration), WorkerError> {
    let batch_size = batch_size
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|_| {
            WorkerError::InvalidConfiguration("batch size must be an integer".to_string())
        })?
        .unwrap_or(DEFAULT_OUTBOX_BATCH_SIZE);
    if batch_size == 0 || batch_size > MAX_OUTBOX_BATCH_SIZE {
        return Err(WorkerError::InvalidConfiguration(format!(
            "batch size must be between 1 and {MAX_OUTBOX_BATCH_SIZE}"
        )));
    }
    let poll_interval_ms = poll_interval_ms
        .map(|value| value.parse::<u64>())
        .transpose()
        .map_err(|_| {
            WorkerError::InvalidConfiguration("poll interval must be an integer".to_string())
        })?
        .unwrap_or(DEFAULT_POLL_INTERVAL.as_millis() as u64);
    if !(10..=5_000).contains(&poll_interval_ms) {
        return Err(WorkerError::InvalidConfiguration(
            "poll interval must be between 10 and 5000 milliseconds".to_string(),
        ));
    }
    Ok((batch_size, Duration::from_millis(poll_interval_ms)))
}

pub fn default_lease() -> Duration {
    DEFAULT_OUTBOX_LEASE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(event_type: &str) -> OutboxEvent {
        OutboxEvent {
            id: Uuid::now_v7(),
            aggregate_type: "tax_calculation".to_string(),
            aggregate_id: Uuid::now_v7(),
            event_type: event_type.to_string(),
            payload: serde_json::json!({"status": "calculated_unattested"}),
            attempt_count: 1,
            claim_token: Uuid::new_v4(),
        }
    }

    #[test]
    fn subjects_are_bounded_and_tax_scoped() {
        assert_eq!(
            event_subject(&event("tax.calculation.created")).unwrap(),
            "tax.calculation.created"
        );
        assert!(event_subject(&event("other.event")).is_err());
        assert!(event_subject(&event("tax.*")).is_err());
    }

    #[test]
    fn envelope_contains_stable_event_identity_and_schema() {
        let event = event("tax.calculation.created");
        let encoded = encode_event(&event).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(value["schema_version"], EVENT_SCHEMA_VERSION);
        assert_eq!(value["event_id"], event.id.to_string());
        assert_eq!(value["event_type"], event.event_type);
    }

    #[test]
    fn worker_limits_are_bounded() {
        assert!(parse_worker_limits(Some("8"), Some("100")).is_ok());
        assert!(parse_worker_limits(Some("0"), Some("100")).is_err());
        assert!(parse_worker_limits(Some("101"), Some("100")).is_err());
        assert!(parse_worker_limits(Some("8"), Some("9")).is_err());
    }
}
