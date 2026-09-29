# ADR-0185: Lease-fenced refund-proof candidate ownership

## Status

Accepted — implement as the storage and service completion of ADR-0158.

## Context

ADR-0158 defines a one-candidate refund-proof worker, but the existing
projection cursor is read-only. With two indexer replicas, both replicas can
select the same unproved execution, perform duplicate finalized RPC work, and
race to persist evidence. A database row lock cannot be held across provider
calls without coupling database availability to RPC latency.

## Decision

Add a short, durable, UUIDv7 lease to
`indexed_adjustment_refund_executions`:

- `proof_claim_token` and `proof_claimed_at` are nullable as an expand step;
  the pair must be both null or both non-null.
- `proof_claim_attempts`, a bounded last-error field, and an optional
  dead-letter timestamp make provider/validation failures finite. A candidate
  is eligible for at most three attempts; a dead-lettered candidate remains
  visible as unproved evidence but is not automatically retried.
- A `ReadCommitted` transaction selects one execution with `FOR UPDATE
  SKIP LOCKED`, excludes executions with a proof row, and considers a lease
  eligible only when it is absent or older than the bounded 60-second TTL.
- The transaction writes a cryptographically unpredictable UUIDv7 token and
  commits before any provider call. It returns the execution and token as one
  candidate envelope.
- Persistence is `Serializable`, locks the execution row, and requires the
  exact token. It also rechecks the indexed execution, raw event discriminator
  and evidence digest, topology, proof arithmetic, and proof digest before
  inserting immutable evidence. It clears the lease in the same transaction.
- A stale worker can neither insert evidence nor clear a newer lease. A
  crashed owner is reclaimed after the TTL. Provider calls never occur while
  holding a database lock.
- The worker records a bounded failure through the same token-fenced storage
  port. Transient-looking failures release the lease for a later bounded
  attempt; the third failure clears ownership and marks the candidate for
  operator/reconciliation handling.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Read-only cursor with proof exclusion | Minimal schema change | Duplicate RPC work and no cross-replica ownership | Rejected |
| Hold `FOR UPDATE` across RPC | Strong exclusion while active | Blocks database connections, violates latency/backpressure budget, unsafe on provider stalls | Rejected |
| Redis/distributed lock | No SQL schema change | Disposable coordination is not evidence-bound and adds another failure authority | Rejected |
| **PostgreSQL UUIDv7 lease, `SKIP LOCKED`, TTL reclaim, and final token CAS** | Durable ownership, bounded crash recovery, no lock across RPC, stale-worker fencing | Additive migration and lease telemetry required | **Selected** |

## Invariants and budgets

- At most one live lease exists for an execution.
- A candidate is never returned when immutable proof evidence already exists.
- A persistence attempt must prove ownership with the exact UUIDv7 token.
- A stale worker cannot mutate a row after another worker reclaims it.
- Automatic retry attempts are bounded at three; dead-letter state is explicit
  and never authorizes payment.
- One cycle claims and processes at most one candidate; the database lease
  acquisition and persistence each use the repository lock and statement
  timeouts. The RPC attempt remains bounded by the provider timeout.
- Lease state is coordination only. It never authorizes a signer, changes
  refund lifecycle status, or acts as a source of funds truth.

## Verification and rollout

- Add migration and schema verification for the lease pair, check constraint,
  and pending-candidate index.
- Add storage tests for two concurrent claimers, stale reclaim, proof
  exclusion, token-fenced persistence, stale-owner rejection, bounded failure
  recording/dead-lettering, and idempotent duplicate persistence.
- Wire `RefundProofWorker` into the indexer service through PostgreSQL source
  and sink adapters; remove the legacy unleased proof path.
- Keep payment/lifecycle transitions unchanged and retain Linux validator,
  finalized-provider, and production SLO evidence as separate release gates.
