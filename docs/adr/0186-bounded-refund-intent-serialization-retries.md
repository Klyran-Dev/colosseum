# ADR-0186: Bounded refund-intent serialization retries

- Status: Accepted
- Date: 2026-09-29
- Owners: Principal Low-Latency Architecture, Refund Movement, Data Platform
- Bounded contexts: Refund Movement, PostgreSQL Storage, API Idempotency

## Context

Refund claim and execute preparation use PostgreSQL `SERIALIZABLE` transactions
because an immutable artifact, its first lifecycle event, and its first outbox
publication must commit as one authority boundary. Under concurrent retries,
the database may abort a transaction with SQLSTATE `40001` (serialization
failure) or `40P01` (deadlock detected). Returning either transient conflict
directly to the API makes a correct request fail under ordinary replica
contention and encourages callers to retry without a clear budget.

The idempotency key remains the replay authority. Claim nonce gaps are safe and
intentional, so a retried nonce allocation cannot reuse a potentially committed
nonce. Integrity conflicts, malformed commands, pool failures, and business
state errors must not be retried as if they were transient serialization.

## Decision

The storage adapter owns a bounded retry policy for refund nonce allocation,
claim-intent creation, and execute-intent creation:

- at most three complete transaction attempts;
- retry only SQLSTATE `40001` and `40P01`;
- use a short linear 2 ms, 4 ms backoff between attempts;
- create a fresh UUIDv7 intent identity on each attempt, while preserving the
  caller's immutable request digest and idempotency key;
- re-read the durable idempotency row after a successful conflicting insert, so
  retries return the original artifact rather than create a second lifecycle;
- surface the final database error after the bounded budget; and
- never retry RPC, wallet signing, authorization, provider ambiguity, or a
  uniqueness/business conflict in this layer.

No database lock is held during the backoff. The transaction is fully rolled
back before sleeping, and the retry remains below the documented API deadline.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Propagate every serialization conflict | Rejected | Correct but creates avoidable tail failures and pushes unsafe retry policy to callers. |
| Retry in API handlers | Rejected | Duplicates persistence semantics across endpoints and can accidentally repeat authority-sensitive work. |
| Lower isolation to `ReadCommitted` | Rejected | Weakens the atomic idempotency/event/outbox boundary and changes the correctness proof. |
| Unbounded exponential retry | Rejected | Violates deadline, overload, and retry-storm mandates. |
| **Three-attempt storage-local retry for `40001`/`40P01`** | **Selected** | Keeps the authority boundary intact, bounds latency, and preserves idempotent replay semantics. |

## Invariants and budgets

- One `(merchant_id, idempotency_key)` maps to one immutable request digest and
  one durable artifact.
- Each successful creation has exactly one initial lifecycle event and one
  transactional outbox row.
- A retry cannot change financial inputs, account topology, rule/configuration
  digest, or the caller's request digest.
- A nonce may have gaps after crash or ambiguous commit, but no nonce is reused.
- The policy adds at most 6 ms of intentional backoff, excluding bounded
  database work and connection-pool wait.

## Verification

- Failing-first unit tests cover the exact retryable SQLSTATE set and budget.
- Storage methods use the policy for nonce, claim, and execute preparation.
- Existing idempotency, immutable-artifact, event, and outbox tests remain
  mandatory.
- The Docker PostgreSQL contention drill proves 16 concurrent nonce allocations
  remain unique and contiguous in aggregate, and that 16 same-key claim callers
  converge to one intent, one initial lifecycle event, and one outbox row. The
  authenticated API contention drill drives the real claim and execute
  preparation routes with 16 same-key callers and proves one execute intent,
  one lifecycle event, one outbox row, and stable response digests. Synthetic
  Docker evidence does not close retry telemetry, pool-pressure, live wallet or
  provider submission, Linux validator, or production load/SLO gates.
