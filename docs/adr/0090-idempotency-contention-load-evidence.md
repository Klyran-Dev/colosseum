# ADR-0090: Bounded idempotency-key contention load evidence

- Status: Accepted
- Date: 2026-09-27
- Owners: API / Platform
- Scope: Concurrent quote requests sharing one merchant-scoped idempotency key

## Context

The quote path uses a merchant-scoped idempotency record, serializable
transactions, and a transactional outbox. The existing local workload proved
independent concurrent requests and exact persistence counts, but it did not
prove the critical contention case where many callers retry the same request
at once. That case must return one durable calculation and one creation event,
without duplicate tax evidence or unbounded lock waits.

## Decision

Extend the bounded Docker quote-service workload with a second phase:

- Send a bounded default of 32 identical requests at the configured
  concurrency, using one exact merchant-scoped idempotency key and identical
  request bytes.
- Require every response to be HTTP 200 with the same calculation identity.
- Assert PostgreSQL contains exactly one idempotency record, one calculation,
  and one transactional outbox event for that key.
- Keep this phase outside the latency percentile sample for the independent
  request workload; report its request count, concurrency, and convergence
  counts separately.
- Keep the workload synthetic and local/CI. It is not a production regional
  SLO or a substitute for account-lock and fan-out tests at production data
  volume.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Independent idempotency keys only | Rejected | Does not exercise the serialization and replay boundary. |
| Unit-test the idempotency helper | Rejected | Cannot prove PostgreSQL row locking, transaction retries, or outbox uniqueness under concurrent HTTP requests. |
| Unbounded stress test | Rejected | Makes CI noisy and obscures the deterministic correctness invariant. |
| **Bounded same-key Docker workload with database convergence assertions** | **Selected** | Exercises the real API/database path with a reproducible, auditable invariant. |

## Invariants

1. Every exact duplicate request is safe and returns the original successful
   result.
2. One merchant-scoped idempotency key produces exactly one calculation and
   one `tax.calculation.created` outbox event.
3. A different request digest under the same key remains a conflict; the
   contention phase never relaxes that boundary.
4. The phase has finite request, concurrency, and timeout bounds.

## Evidence

- `scripts/benchmark_quote_service.sh`
- `docs/plans/production-readiness.md`
- `docs/plans/implementation-plan.md`

