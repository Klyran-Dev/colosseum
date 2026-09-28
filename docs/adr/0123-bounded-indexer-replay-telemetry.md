# ADR-0123: Bounded indexer replay telemetry and alert inputs

- Status: Accepted
- Date: 2026-09-27
- Scope: Indexer operations, PostgreSQL replay-gap projection, Prometheus metrics

## Context

The indexer already persists replay gaps with crash-recoverable leases,
bounded retries, dead-letter state, and authenticated operator actions. That
state was not exposed by the indexer metrics endpoint, so operators could not
distinguish a growing repair backlog from normal forward-ingestion latency or
write alerts for retry exhaustion.

## Decision

The indexer loads a bounded operational snapshot for its configured stream and
exports only fixed-cardinality Prometheus series:

- `stp_indexer_replay_gaps{status="queued|leased|dead_lettered"}`;
- `stp_indexer_replay_leased_slots` for the total slot span held by active
  leases.

The stream key remains service configuration and is never emitted as a metric
label. Metrics are refreshed after each bounded replay/forward iteration and
retain the last successful snapshot if the telemetry query fails. A failed
telemetry query must not advance or alter indexing state.

Prometheus rules alert on a sustained queued backlog and any dead-lettered
work. Alert delivery, on-call ownership, lease-expiry counters, claim-token
loss counters, and production scrape authentication remain separate
operational gates.

## Invariants

1. The snapshot is scoped to exactly one configured stream.
2. Counts and leased slot spans are parsed as bounded unsigned integers.
3. No user-controlled stream key or error text becomes a metric label.
4. Metrics are observational and cannot authorize settlement or mutate chain
   checkpoints.
5. A database/metrics failure preserves the last known bounded snapshot and
   does not make the indexer claim that replay completed.

## Alternatives considered

### Emit one metric label per stream key

Rejected. Stream keys are configuration data and can grow metric cardinality;
the indexer fleet should be partitioned operationally, not through unbounded
labels.

### Query replay state from Prometheus exporters only

Rejected. An independent exporter can lag the indexer's database transaction
and creates another failure boundary for the operational truth needed by the
indexer process.

### Log-only replay state

Rejected. Logs are useful audit context but do not provide bounded, stable
alert expressions for backlog and dead-letter growth.

### **Fixed gauges from the indexer process**

Selected. The query is scoped, the output shape is deterministic, the last
known value is safe during a telemetry outage, and Prometheus rules can be
validated in the same pinned local/CI configuration.

## Verification

- Unit tests assert all fixed series and reject stream-key labels.
- `promtool check rules` validates the alert expressions.
- Integration/production evidence must additionally prove scrape policy,
  notification delivery, lease-expiry/claim-loss accounting, and alert
  response through the replay runbook.

