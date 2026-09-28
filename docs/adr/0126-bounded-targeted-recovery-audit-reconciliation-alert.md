# ADR-0126: Bounded targeted-recovery audit reconciliation alert

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture and SRE
- Scope: operational detection of incomplete targeted-recovery attempts

## Context

ADR-0125 intentionally writes a `requested` audit row before projection
recovery and a terminal `recovered` or `failed` row after the attempt. The two
transactions are separate so a process crash cannot erase operator intent.
That safety property creates an operational obligation: a requested row that
has no terminal row after a bounded grace period must be detected and
reconciled. Without a stable metric, the immutable ledger can silently hide an
incomplete recovery attempt.

## Decision

Add a fixed-cardinality indexer operational gauge,
`stp_indexer_targeted_recovery_stale_requested`, refreshed with the existing
bounded replay telemetry cycle. It counts requested audit rows older than five
minutes that have no terminal audit row for the same UUIDv7 request ID.

The storage query is an aggregate with a bounded result, uses the request and
outcome/time indexes, and returns only a count. The metric has no stream,
address, signature, operator, merchant, or request labels. A refresh failure
retains the last known snapshot and is logged by the existing structured
service loop; it never blocks chain settlement or advances a checkpoint.

Prometheus alerts when the gauge remains non-zero for two minutes. The runbook
requires operators to inspect the append-only audit ledger, determine whether
the projection was actually persisted, and append the missing terminal outcome
through a controlled reconciliation procedure. The alert is evidence of an
incomplete control-plane workflow, not evidence that funds moved or that a
recovery succeeded.

## Alternatives considered

### Query the audit table from every API metrics scrape

Rejected. It adds a database round trip to the API metrics endpoint and couples
operator telemetry to the request-serving process, while still requiring a
separate reconciliation worker.

### Export one metric label per target or request

Rejected. Addresses, signatures, and request IDs are unbounded sensitive
cardinality and would create memory, privacy, and alerting hazards.

### Log stale rows only

Rejected. Logs are useful forensic evidence but do not provide a stable alert
expression or bounded dashboard signal.

### **Fixed-cardinality aggregate refreshed by the indexer service**

Selected. It reuses the existing bounded operational refresh loop, keeps
settlement and API hot paths unchanged, and makes crash evidence alertable
without exposing target identity.

## Invariants and failure behavior

- The count is derived only from immutable audit rows and never from chain
  projections or settlement state.
- A request is stale only when it is older than five minutes and has no
  terminal row with the same UUIDv7 request ID.
- The metric cannot identify a target through labels or metric values beyond
  the aggregate count.
- Database or scrape failure never becomes a success signal; the last snapshot
  is retained and the service logs the refresh failure.
- The alert does not mutate audit rows, projections, checkpoints, or funds.

## Evidence and remaining gates

Unit tests cover fixed-cardinality rendering and snapshot semantics. The
PostgreSQL integration path must exercise the aggregate against requested,
recovered, failed, and stale rows. Production scrape authorization,
Alertmanager delivery, on-call response, and a live crash-reconciliation drill
remain separate readiness gates.
