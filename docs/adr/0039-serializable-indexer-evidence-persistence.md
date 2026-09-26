# ADR-0039: Persist finalized indexer pages with checkpoint fencing

- Status: Accepted for the first durable indexer adapter
- Date: 2026-09-25
- Owners: Chain Data / Storage / SRE
- Scope: finalized scan pages, receipt evidence, checkpoints, reconciliation runs

## Context

The indexer domain rules are pure and testable, but a production indexer also
needs crash recovery and concurrent-worker protection. A page must not advance
the cursor before all of its evidence is durable, and two workers must not
silently apply overlapping pages. Receipt notifications can be repeated after
provider reconnects, while conflicting evidence must remain visible.

## Decision

Persist `chain_checkpoints`, `indexed_receipts`, and append-only
`reconciliation_runs` in PostgreSQL 18. A page write runs in a serializable
transaction. The adapter locks the stream checkpoint, verifies the caller's
expected checkpoint, inserts or exact-replays each receipt, and advances the
checkpoint only after the page is complete and finalized. A retry whose
checkpoint already equals the calculated next checkpoint is idempotent;
different checkpoint state fails closed.

Slot and atomic amount columns use `numeric(20,0)` with explicit Solana `u64`
bounds. Receipt evidence is keyed by `(stream_key, signature)` and its 32-byte
digest and conservation relation are database constrained. The database stores
evidence and operational status; finalized Solana state remains the authority.

## Alternatives considered

See `docs/architecture/alternative-analysis.md` under “Chain indexing and
reconciliation”. In-memory cursors were rejected because crashes lose progress;
broker-only offsets were rejected because they do not atomically commit the
projection; an upsert that overwrites receipts was rejected because it hides
provider disagreement.

## Consequences

Positive:

- A process crash cannot publish a cursor for an uncommitted page.
- Reconnect duplicates are safe and conflicting evidence is retained as a
  failed operation rather than silently replaced.
- Numeric storage preserves the complete `u64` slot and atomic-unit domain.

Costs and remaining gates:

- Serializable contention and database latency require bounded page sizes and
  measured retry budgets.
- Raw transaction/event evidence, provider failover, gap queues, RPC adapters,
  replay tooling, partition/retention policy, and production alerts remain
  follow-on work.
