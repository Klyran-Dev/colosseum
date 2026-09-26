# ADR-0033: Deliver outbox events through bounded PostgreSQL leases and NATS JetStream

- Status: Accepted for the first asynchronous delivery boundary
- Date: 2026-09-25
- Owners: Application Platform / SRE
- Scope: `outbox_events` dispatcher

## Context

ADR-0032 makes the PostgreSQL outbox the atomic handoff from a quote transaction. A durable row still needs a bounded worker that can run concurrently, survive process failure, avoid duplicate claims, and only acknowledge delivery after the downstream transport confirms persistence.

The first workload needs low-latency fan-out without making Kafka or a broker round-trip part of quote creation. Consumers must be able to replay and deduplicate events.

## Decision

Implement a separate Rust/Tokio worker with:

- PostgreSQL `FOR UPDATE SKIP LOCKED` lease claims;
- bounded batch size, connection pool, polling interval, and lease duration;
- claim UUIDs that prevent stale workers from acknowledging another worker's row;
- bounded retry attempts and an explicit dead-letter timestamp/error code;
- NATS JetStream with a durable file-backed stream and `Nats-Msg-Id` equal to the outbox event UUID;
- `published_at` set only after the JetStream publish acknowledgement is received;
- at-least-once delivery semantics and idempotent consumers.

The worker does not delete outbox rows or treat JetStream as the authority for Solana funds. PostgreSQL remains the replayable event ledger. A failed publish releases/ages the claim and records a bounded error code; an event exceeding the retry budget becomes visible as dead-lettered and pages operations.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`: Kafka, Redis Streams, and direct consumer polling were rejected for the first transport boundary.

## Invariants

- A row is marked published only after broker acknowledgement.
- A stale worker cannot mark a row claimed by a newer worker.
- Concurrent workers do not process the same active lease.
- Publish retries use the stable outbox UUID as the broker deduplication key.
- Event payloads carry a schema version and merchant scope before downstream fan-out.
- Retry attempts and dead-letter state are bounded and observable.
- The quote request never waits on NATS.

## Consequences

Positive:

- Broker outages do not lose committed events or block quote creation.
- Multiple worker replicas can scale with bounded database contention.
- JetStream provides durable downstream replay and message-id deduplication.

Costs:

- The platform operates both PostgreSQL and NATS and must monitor lag, lease age, retry count, and dead letters.
- At-least-once consumers remain mandatory; broker deduplication is not a substitute for consumer idempotency.
- Stream retention, backup, credentials, and upgrade procedures are release gates.
