# ADR-0292: Durable revenue-accrual reconciliation jobs

- Status: Accepted
- Date: 2026-10-04
- Decision owners: Indexing, storage, and revenue-domain maintainers
- Scope: finalized split projection to revenue-stream accrual persistence

## Context

The finalized indexer checkpoint and indexed split projection are committed in
one serializable transaction. A revenue accrual can fail later because an
active stream plan is unavailable, PostgreSQL is temporarily unavailable, or
the plan cap/identity check rejects the evidence. Advancing the checkpoint
without a durable retry record could permanently lose a valid finalized
accrual; holding the checkpoint open would stall unrelated indexing.

## Decision

Add `revenue_stream_accrual_reconciliation_jobs` as a bounded, durable work
projection. Every finalized split settlement is inserted into this table in
the same transaction as its indexed projection and checkpoint update. The
insert is idempotent by `(stream_key, source_signature)`; the stored evidence
digest is checked against the indexed split before a worker can process it.

The indexer-service runs a one-job-at-a-time bounded reconciler:

1. Claim one pending or expired job with `FOR UPDATE SKIP LOCKED`, a UUIDv7
   lease token, bounded attempts, and a short lease expiry.
2. Load only server-owned active revenue plans and their digest-validated
   split plans.
3. Build a one-event finalized page and pass it through ADR-0291 and ADR-0290.
4. Persist each generated accrual through the existing serializable,
   tenant-bound, cap-checked, idempotent storage command.
5. Complete the exact lease only after persistence succeeds. A crash between
   accrual persistence and job completion is safe because the next attempt is
   a storage-level duplicate. Repeated invalid evidence reaches explicit
   dead-letter state and remains operator-visible.

Jobs are coordination records, not settlement authority. Finalized Solana
evidence remains authoritative, and claim/release movement remains separately
gated by signed-message verification and validator conformance.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Re-scan the chain after every accrual failure | Rejected | Expensive, duplicates provider work, and cannot guarantee recovery after checkpoint advancement. |
| Keep the checkpoint pending until accrual succeeds | Rejected | Couples unrelated indexing to a slow or unavailable revenue plan and can stall the chain cursor. |
| Publish only to an external broker | Rejected | Broker delivery is not atomic with the indexed evidence and introduces another availability dependency. |
| Add a job row in the same PostgreSQL transaction as the indexed projection | Selected | Durable, bounded, replayable, transactionally linked to finalized evidence, and compatible with existing lease patterns. |

## Invariants

- A finalized split projection and its reconciliation job commit together.
- A job references an existing indexed split settlement and cannot authorize
  token movement.
- Lease tokens are UUIDv7, ownership is fenced, and attempts are bounded.
- Accrual persistence precedes job completion.
- Duplicate source signatures are safe and return `Duplicate`.
- A stale worker cannot complete or fail a newer lease.
- Invalid evidence and exhausted retries are explicit, observable states.
- No browser/API field can create, claim, complete, or alter a job.

## Operational budget

Each cycle claims at most one job and loads at most 64 active reconciliation
contexts. Database statements use the existing lock and statement timeouts;
the worker has a bounded retry budget and does not run on the wallet-signing
or settlement-submit path. Metrics must expose pending, claimed, completed,
duplicate, rejected, and dead-letter counts without tenant or signature
cardinality.

## Consequences

The system becomes eventually consistent by an explicit, recoverable contract:
chain indexing can continue while revenue projection retries independently.
The job table adds durable operational data and a migration, but removes the
more dangerous implicit dependency between checkpoint advancement and a
secondary accounting projection.
