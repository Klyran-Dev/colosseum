# ADR-0159: Fixed-cardinality refund-proof operational observability

## Status

Accepted for implementation.

## Context

ADR-0158 adds a server-owned worker that proves finalized refund balance
transitions and persists immutable evidence. The worker is deliberately
bounded and fail-closed, but a pending candidate can otherwise remain visible
only in logs. Logs are useful forensic evidence, not a durable operational
signal: they are not restart-safe, difficult to alert on consistently, and
must not expose merchant, claim, signature, or account identifiers as metric
labels.

## Decision

Export two fixed-cardinality signals from the indexer service:

1. `stp_indexer_refund_proofs_pending{service="indexer"}` is a PostgreSQL
   snapshot gauge for the configured stream, counting indexed refund
   executions that have no immutable proof row.
2. The existing bounded `ServiceMetrics` operation histogram records the
   `refund_proof` worker cycle with request count, error count, and duration.

The worker records exactly one `refund_proof` observation per scheduled cycle,
including idle cycles and failures. The database gauge is refreshed after the
normal replay and vault cycles and after the proof attempt. No signature,
claim, tenant, merchant, address, provider URL, or error text is a metric
label. Logs retain redacted diagnostic context and the immutable database
evidence remains authoritative.

The initial alert contract is:

- warn when the pending gauge is non-zero for ten minutes;
- critical when the proof operation error ratio is above the bounded service
  threshold for ten minutes;
- investigate the indexed execution and finalized transaction before retrying
  or declaring a refund reconciled.

An alert never mutates evidence, marks a refund paid, or authorizes a release.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Logs only | No code or storage query | Not restart-safe, difficult to alert, and poor for backlog detection | Rejected |
| Per-claim/signature metric labels | Easy drill-down | Unbounded cardinality, privacy leakage, and memory/alerting hazards | Rejected |
| Redis-backed worker counters | Shared across replicas | Adds a non-authoritative dependency and can reset or diverge from durable evidence | Rejected |
| **PostgreSQL pending snapshot plus fixed operation metrics** | Durable backlog truth, bounded labels, low overhead, compatible with existing scrape contract | One bounded aggregate query and process-local request counters | **Selected** |

## Invariants and budgets

- The pending gauge is scoped by server configuration and never accepts a
  caller-provided stream or tenant label.
- The query is an aggregate over indexed executions and proof existence; it
  does not scan or return transaction payloads.
- Each worker cycle performs at most one candidate selection and one proof
  attempt; metrics add no RPC calls and no unbounded allocation.
- A metrics or scrape failure cannot alter quote, settlement, proof, or release
  behavior.
- The worker's p95 duration and error ratio are measured separately from API
  request SLOs; no production SLO claim is made until retained regional load
  evidence exists.

## Verification and rollout

- Unit-test fixed metric names, values, and absence of dynamic identity labels.
- Test pending-count SQL through the Docker PostgreSQL 18 integration path.
- Validate the alert rules with the pinned `promtool` contract.
- Exercise idle, success, provider failure, invalid-proof, and persistence
  failure cycles; prove that a failed cycle leaves the candidate pending.
- Keep the operational release gate open until scrape authorization,
  Alertmanager delivery, and retained production load evidence are complete.
