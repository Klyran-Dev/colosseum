# ADR-0414: Bounded applicability-binding verifier worker

- Status: Accepted
- Date: 2026-10-06
- Owners: Indexing and settlement evidence
- Related: ADR-0411, ADR-0412, ADR-0413

## Context

ADR-0412 defines the finalized Solana account proof for an applicability
binding, and ADR-0413 defines the append-only PostgreSQL projection. Those
boundaries are not operationally complete until a worker can discover every
eligible raw event, execute the bounded finalized RPC read, and commit the
typed result exactly once. The worker must tolerate concurrent instances,
provider failures, process termination, and malformed or permanently
conflicting evidence without holding a database transaction across RPC.

The workflow is intentionally asynchronous. Quote, transaction compilation,
wallet signing, and settlement submission must not wait for this verification.
Only the finalized raw event and the verified projection can be used by
operator-facing evidence reads.

## Decision

Add a PostgreSQL-backed, lease-fenced verification job table. A finalized page
or replay page inserts an idempotent job for each applicability settlement event
in the same transaction that inserts its raw bytes. A worker claims at most one
job per bounded cycle, commits the lease before RPC, and then completes the
job in the same serializable transaction as the ADR-0413 projection.

The lifecycle is:

1. `pending`: raw applicability event exists and no verification projection is
   present.
2. `leased`: one worker owns a UUIDv7 claim token; the lease is bounded and
   recoverable after expiry.
3. `completed`: finalized account evidence and the projection are committed;
   the row is immutable for read purposes.
4. `dead_lettered`: the bounded retry budget is exhausted. This state is
   visible to operations and cannot silently authorize settlement.

The worker uses the dedicated `FinalizedApplicabilityBindingSource` through a
bounded provider failover pool. It never combines fields from providers. A
provider result is accepted only after the source validates the fixed PDA,
owner, executable flag, discriminator, exact layout, finalized commitment,
and all server-owned binding fields. Any provider or storage error releases
the lease with a bounded error code; a conflict or invalid account response
eventually becomes explicit dead-letter evidence.

The service invokes one bounded verification cycle from the existing
reconciliation interval. This keeps concurrency and database pressure bounded
without adding RPC work to the indexer's page scan or settlement hot path.

## Alternatives considered

| Alternative | Concurrency safety | Recovery | Hot-path impact | Decision |
| --- | --- | --- | --- | --- |
| Scan all raw events every tick | Poor; duplicate reads grow with history | Partial | Low | Rejected |
| In-memory queue | Fail across restarts and replicas | Fail | Low | Rejected |
| Hold `FOR UPDATE` across RPC | Locks database during provider latency | Poor under crash | Medium/high | Rejected |
| Unleased bounded candidate query | Duplicate RPC work across workers | Partial | Low | Rejected |
| **Lease-fenced PostgreSQL job table** | **Bounded and replica-safe** | **Explicit retry/dead-letter** | **None** | **Selected** |

## Invariants

- A job is created only for an immutable raw applicability event.
- A claim token is UUIDv7 and is required for completion or failure.
- At most `MAX_APPLICABILITY_BINDING_ATTEMPTS` claims are permitted.
- A stale worker cannot complete or fail a newer lease.
- Completion and typed projection insertion are one transaction.
- No job status, database row, or API response authorizes token movement.
- The raw event and finalized account proof remain the evidence authorities.
- RPC calls are bounded by provider count, response size, timeout, and one
  candidate per cycle.

## Operational budgets

- One candidate per worker cycle by default.
- At most three RPC providers and one attempt timeout per provider.
- Lease duration is 60 seconds; attempts are bounded at eight.
- No database transaction remains open during RPC.
- Metrics distinguish success, provider failure, invalid evidence, lease loss,
  and dead-letter state.

## Consequences

The system gains restart-safe and horizontally safe verification scheduling,
but requires one additive migration, worker metrics, and operator handling for
dead-lettered evidence. The verifier is eventually consistent by design;
the UI must show `pending`, `verified`, `retrying`, and `dead-lettered` states
and must never imply that a pending verification is finalized compliance.
