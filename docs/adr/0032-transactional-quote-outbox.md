# ADR-0032: Commit quote domain events through a PostgreSQL outbox

- Status: Accepted
- Date: 2026-09-25
- Owners: Application Platform / Indexing
- Scope: Persisted quote transaction

## Context

The quote path creates calculation evidence and an idempotency response that downstream indexer, audit, notification, and reconciliation workers need to observe. Publishing directly to a broker or webhook inside the request creates partial-commit and retry-storm failure modes. Publishing after the transaction can lose the event if the process exits.

## Decision

Insert one `tax.calculation.created` row into PostgreSQL `outbox_events` in the same serializable transaction that inserts `tax_calculations` and completes `idempotency_records`. The event payload is a versioned envelope containing the merchant UUID and persisted `StoredQuote` evidence, so downstream consumers cannot accidentally process a quote without tenant scope. ADR-0033 implements the bounded dispatcher: it claims rows with `FOR UPDATE SKIP LOCKED`, fences leases with a claim token, publishes to NATS JetStream, and marks `published_at` only after an acknowledged delivery. Consumers must remain idempotent.

An exact idempotent quote replay reads the stored response and does not create a second outbox event. Failed quote transactions roll back both evidence and event rows.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`: broker-before-commit and request-time fan-out were rejected because they make external delivery part of the settlement-critical path and cannot provide the same atomicity.

## Invariants

- A newly committed calculation has exactly one creation event.
- A replay has no new creation event.
- No event is visible for a calculation whose idempotency transaction failed.
- Event payloads contain versioned, replayable evidence and no bearer tokens or private credentials.
- Delivery retries are bounded and idempotent; the outbox is not the authority for Solana funds.

## Consequences

Positive:

- Database commit is the atomic handoff from synchronous quote work to asynchronous processing.
- Broker outages do not make the quote path depend on broker availability.
- Events can be replayed or rebuilt from durable evidence.

Costs:

- The database requires retention/partitioning and dispatcher operations at scale.
- At-least-once delivery requires idempotent consumers and observability for age/backlog.
- Broker delivery remains outside the quote hot path; the worker is a downstream reliability boundary and must not be moved into the request transaction.
