# ADR-0088: Deterministic indexer projection rebuild from raw evidence

- Status: Accepted
- Date: 2026-09-26
- Owners: Chain Data / SRE
- Scope: Rebuilding normalized settlement receipt projections from retained finalized event evidence

## Context

The indexer retains bounded, digest-verified Anchor `SettlementExecuted` event
bytes beside the normalized receipt projection. Replay repairs currently merge
known pages, but a Tax Engine product also needs a controlled rebuild path for
operator recovery, schema migrations, and disaster-recovery verification. A
rebuild must not trust a normalized row, a provider re-encoding, a checkpoint,
or client-supplied amounts.

The rebuild path is projection maintenance. Solana finalized evidence remains
the authority for settlement facts; raw evidence is immutable and normalized
tables may be deleted and recreated. The operation must be bounded so it cannot
turn an incident response request into an unbounded database transaction or
memory allocation.

## Decision

Add a projection-rebuild capability to the Indexer bounded context:

- Decode only the canonical, versioned `SettlementExecuted` Anchor event from
  `raw_chain_evidence.event_data`.
- Recompute and verify the evidence digest, event discriminator, event length,
  finalized slot range, and `gross = tax + net` conservation before producing a
  receipt.
- Use the raw row's signature, slot, and evidence digest as the receipt
  identity. Never accept amounts, signatures, or digests from a rebuild caller.
- Process rows with keyset pagination ordered by `(slot, signature,
  event_index)` and a bounded batch size. Each batch uses a short transaction
  with lock and statement timeouts; the checkpoint is never read for authority
  and never mutated.
- Insert a missing normalized receipt idempotently. An identical existing row
  is a duplicate. Any conflicting row fails closed and rolls back that batch.
- Return bounded counters and a deterministic projection digest for the
  processed scope. The digest is evidence for the drill, not a settlement
  authority or a substitute for chain finality.
- Keep rebuild authorization operational: production invocation requires an
  authenticated operator workflow and a runbook; this storage primitive does
  not become a public merchant API.

The first implementation covers the `indexed_receipts` projection. Merchant,
vault, and reporting projections require their own raw-evidence contracts and
ADRs; this decision does not imply that receipt reconstruction alone rebuilds
every product read model.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Rebuild from `indexed_receipts` or a database backup | Rejected | Rebuilds the projection from itself and cannot detect normalized corruption. |
| Re-query live RPC during the rebuild | Rejected | Adds provider availability and re-encoding variance to a recovery operation; finalized raw evidence is already retained. |
| SQL-only byte slicing of Anchor data | Rejected | Duplicates protocol decoding in SQL and weakens typed conservation/discriminator validation. |
| One unbounded transaction or in-memory export | Rejected | Creates lock, memory, timeout, and operational blast radius at scale. |
| **Rust decoder plus keyset-paginated, batch-committed storage adapter** | **Selected** | Reuses canonical domain validation, bounds resource usage, preserves idempotency, and is independently testable. |

## Invariants and failure behavior

1. Raw evidence must be immutable, digest-valid, canonical, and within the
   requested slot range before it can create a receipt.
2. A receipt cannot be created if event amount conservation fails, even when a
   caller supplies a replacement amount.
3. A rebuild never advances or repairs `chain_checkpoints`; forward indexing and
   replay remain separate state machines.
4. Duplicate identical projections are safe; signature conflicts fail closed.
5. Batch failure rolls back its projection writes and does not advance the
   keyset cursor, so a retry cannot skip evidence.
6. Pagination, batch size, slot range, lock timeout, and statement timeout are
   bounded by configuration and validated at the boundary.
7. The output digest is calculated from canonical database values in stable
   order and is reproducible across repeated rebuilds.

## Latency and capacity budget

The default drill target is at most 1,000 raw evidence rows per batch, a
900-ms statement timeout, a 250-ms lock timeout, and no more than one batch in
flight per rebuild worker. Production capacity must be measured with a
representative row size and database hardware; this ADR makes no unmeasured
regional throughput claim. The synchronous operator request must enqueue or
start a bounded job rather than hold an HTTP request open for a full history.

## Evidence

- `crates/indexer/src/lib.rs`
- `crates/storage/src/lib.rs`
- `scripts/projection_rebuild_drill.sh`
- `.github/workflows/ci.yml`
- `docs/plans/production-readiness.md`
