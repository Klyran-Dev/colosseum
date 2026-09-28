# ADR-0078: Reproducible quote-service load evidence

- Status: Accepted
- Date: 2026-09-26
- Owners: API / Tax Engine / SRE
- Scope: Local Docker service-level latency and bounded-concurrency evidence

## Context

Criterion proves that the pure tax domain hot path is fast, but it does not
measure authenticated HTTP, PostgreSQL connection-pool contention, idempotency
indexes, JSON serialization, or the persisted outbox transaction. A
low-latency architecture claim needs a repeatable workload with explicit
concurrency, request count, tenant, rule release, region/runtime, and resource
metadata. It must also prove that a benchmark does not silently create duplicate
financial records or report successful responses for failed requests.

## Decision

Add `scripts/benchmark_quote_service.sh` and the `make benchmark-quote-service`
target. The harness:

1. creates an isolated Compose project with pinned PostgreSQL/NATS/API images;
2. generates an ephemeral EdDSA JWT signing key and uses the synthetic local
   published rule only;
3. sends a bounded, configurable number of authenticated quote requests with
   unique idempotency keys at bounded concurrency;
4. records HTTP status and `curl` transfer time for every request;
5. fails if any request is not HTTP 200, if the measured sample is incomplete,
   or if PostgreSQL does not contain exactly one calculation per idempotency
   key; and
6. prints sample count, concurrency, endpoint, engine/rule fixture, status
   counts, error rate, and p50/p95/p99 latency in milliseconds.

The output is local evidence, not a production SLO claim. Production SLO
acceptance still requires the same workload across target regions, RPC/provider
profiles, representative data volumes, resource limits, and an approved error
budget. The harness never signs or submits a Solana transaction and never uses
real credentials or tax evidence.

The quote transaction retries only PostgreSQL serialization failure (`40001`)
and deadlock (`40P01`) SQLSTATEs, with at most three total attempts and a
bounded 2ms/4ms backoff. Unique-constraint conflicts, validation errors, pool
failures, timeouts, and evidence errors remain fail-closed. The retry is around
the complete transaction so a failed attempt cannot publish a partial
calculation or outbox event.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Domain-only Criterion benchmark | Rejected as the only evidence | Cannot expose HTTP, database, pool, serialization, or outbox contention. |
| Unpinned third-party load binary | Rejected | Adds toolchain drift and weakens clean-checkout reproducibility. |
| Unbounded shell flood | Rejected | Produces retry storms and measures overload rather than the declared service budget. |
| **Pinned Docker stack plus bounded `curl` workers** | **Selected** | Uses already required tooling, isolates state, records every response, and keeps the workload understandable and reproducible. |

## Budgets and invariants

- Default workload: 200 requests, concurrency 16, one authenticated tenant,
  one published synthetic rule, and one unique idempotency key per request.
- Every request has a bounded client timeout and is counted exactly once.
- The harness must report all non-200 responses and exit non-zero for any.
- Persisted calculation count must equal request count; no duplicate key may
  produce more than one calculation or outbox aggregate.
- The benchmark must not run against a non-local endpoint unless explicitly
  overridden with a documented operator action; default URLs are loopback.

## Consequences

The repository gains service-level evidence without claiming production
performance. The test consumes local Docker resources and is intentionally
separate from ordinary unit checks; the public CI workflow invokes it on a
dedicated runner with a bounded 15-minute job budget. Regional production SLO
acceptance remains a separate gate.
