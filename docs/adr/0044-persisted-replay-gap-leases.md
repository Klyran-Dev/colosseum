# ADR-0044: Persist replay gaps with crash-recoverable leases

- Status: Accepted for the first indexer operations adapter
- Date: 2026-09-26
- Owners: Chain Data / SRE
- Scope: PostgreSQL persistence of bounded replay gaps

## Context

The pure replay queue in ADR-0043 establishes safe scheduling semantics, but a
process-local queue cannot survive a crash or coordinate multiple indexer
replicas. Replay work must be durable, tenant/stream scoped, lease-fenced, and
observable without making PostgreSQL authoritative for Solana funds.

## Decision

Persist replay gaps in PostgreSQL with UUIDv7 row IDs, exact `numeric(20,0)`
slot/watermark fields, typed reasons, bounded attempts, and explicit `queued`,
`leased`, `completed`, and `dead_lettered` states. Claiming uses
`FOR UPDATE SKIP LOCKED`, a cryptographically random lease token, bounded lease
duration, and an attempt increment in one transaction. Completion and failure
updates require the current lease token; stale workers cannot acknowledge or
mutate another worker's claim. Expired leases become claimable again. Reaching
the retry limit moves a gap to `dead_lettered`, where operator acknowledgement
and a reviewed requeue action are required.

The persisted table is a recovery work ledger. A gap is never considered
repaired until a finalized page passes the indexer coordinator and its
serializable checkpoint/evidence transaction. A completed lease therefore does
not advance a checkpoint by itself.

## Alternatives considered

- In-memory queue only: rejected because crashes lose accounting recovery work.
- PostgreSQL advisory locks without a row ledger: rejected because ownership,
  attempts, failure reason, and operator visibility would be lost.
- Broker-only queue: rejected because durable recovery state must remain
  queryable and replayable with the evidence store.
- Redis lease state: rejected because Redis is explicitly disposable and must
  not be the source of accounting recovery truth.

## Consequences

Positive:

- Multiple indexer replicas coordinate without a global mutex.
- Lease fencing and bounded retries make crash/retry behavior explicit.
- Operators can inspect, alert, requeue, and audit unresolved gaps.

Costs and follow-on gates:

- The replay worker must connect claims to finalized provider pages and
  checkpoint persistence.
- Retention/partitioning and alert thresholds must be load-tested against the
  expected historical scan volume.
- Manual requeue requires authorization, audit logging, and an incident
  runbook.
