# ADR-0188: Refund-execute contention latency evidence

## Status

Accepted — bounded local evidence; not a production SLO.

## Context

The refund-execute API now has authenticated idempotency and cardinality
evidence, but a low-latency review also needs a repeatable latency distribution
for the same critical path. A single elapsed-time sample or an unconstrained
benchmark would not prove behavior under concurrent database contention.

## Decision

Extend the isolated Docker refund-execute contention drill to record one
end-to-end client-observed latency sample for each of 16 concurrent,
same-merchant, same-idempotency-key execute-preparation requests. The sample
includes the HTTP status and curl total time. The drill sorts the samples and
reports p50, p95, and p99 in milliseconds. After preparation, the same drill
also signs and submits the immutable claim and execute artifacts with
deterministic synthetic test-only wallets, proving one RPC send per movement
under concurrent duplicate submission.

The workload is explicitly bounded:

- one API container, PostgreSQL 18, NATS, and the migration runner;
- API database pool size 16;
- 16 concurrent authenticated callers with a 10–16 ms bounded stagger;
- synthetic server-owned finalized evidence and no external RPC;
- a bounded RPC fixture for signed claim/execute transport, with no finalized
  validator or provider evidence;
- Docker-local wall-clock measurements on the invoking host.

The result is diagnostic capacity evidence. It must not be converted into a
regional or production SLO claim because it excludes wallet transport, live
RPC/provider latency, validator finality, network variance, and production
tenant/data volume.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Measure only total script duration | Simple | Hides request distribution and setup/cleanup cost | Rejected |
| Benchmark a pure Rust function | Stable and cheap | Omits authentication, database, serialization, and contention | Rejected |
| Use an unbounded load tool | Richer traffic generation | Hard to reproduce, can overwhelm the local safety budget, and obscures the critical path | Rejected |
| **Bounded concurrent API samples with p50/p95/p99 and explicit metadata** | Reproducible, realistic route coverage, bounded resource use, interpretable tail latency | Local-only evidence and small sample size | **Selected** |

## Invariants and acceptance

- All 16 requests must return HTTP 200 and converge to one intent, one
  lifecycle event, one outbox row, and one response digest.
- Every successful response must have a positive finite latency sample.
- The output must state request count, concurrency, pool size, service topology,
  and percentile units.
- No percentile is used to authorize funds or mark a refund finalized.

## Verification

- make refund-execute-api-contention-drill prints the bounded p50/p95/p99
  latency distribution after the cardinality assertions pass.
- CI and the release-candidate workflow retain the drill output as evidence.
- Live wallet/provider, Linux validator, regional capacity, and production SLO
  evidence remain separate release gates.
