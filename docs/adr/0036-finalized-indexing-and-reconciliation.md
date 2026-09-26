# ADR-0036: Index finalized chain state with checkpointed backfill and exact reconciliation

- Status: Accepted for the first indexer/reconciliation boundary
- Date: 2026-09-25
- Owners: Chain Data / SRE
- Scope: settlement receipts, chain events, checkpoints, and balance evidence

## Context

Settlement state in PostgreSQL and NATS is a projection. Solana finalized
account and transaction state is the authority for funds and receipts. RPC
subscriptions are useful for low latency but can be dropped, duplicated,
reordered, or delivered at a weaker commitment than the product requires.

The indexer must repair gaps without unbounded work, survive provider failure,
and make reconciliation discrepancies explicit rather than “fixing” a balance
from an off-chain expectation.

## Decision

Use a Rust/Tokio indexer with two inputs:

1. finalized subscription notifications as an acceleration signal; and
2. bounded finalized slot/signature range backfill driven by a durable
   checkpoint.

The domain checkpoint records the next scan slot, finalized watermark, and
generation. A backfill page may advance the checkpoint only when it exactly
matches the planned range and reports completion. Event identity is the
finalized transaction signature plus event discriminator; duplicates are
idempotent, while the same identity with different canonical evidence is a
conflict requiring operator attention.

The reconciliation domain compares exact atomic-unit expectations with
finalized chain snapshots. It emits `Matched` or `Mismatch`; missing evidence
remains an explicit operational state at the adapter/orchestration boundary.
It never mutates authoritative chain balances or silently rounds a delta.
Persistence stores finalized receipt evidence, checkpoints, and reconciliation
runs with append-only semantics; future raw transaction/event and gap tables
must preserve explicit supersession rather than overwrite evidence.

## Invariants

- Only finalized observations can advance the authoritative checkpoint.
- A checkpoint cannot move backwards or skip an incomplete planned range.
- Backfill ranges and page sizes are bounded.
- Event identity is stable and duplicate delivery is idempotent.
- Conflicting evidence for the same identity is fail-closed.
- Reconciliation uses integer atomic units and exact equality.
- Unknown, missing, or provider-disagreeing evidence becomes an explicit
  operational state, not a successful settlement.

## Alternatives considered

See `docs/architecture/alternative-analysis.md` under “Chain indexing and
reconciliation”. WebSocket-only ingestion was rejected because notification
delivery is not durable; Geyser-only ingestion was deferred to a scale adapter;
database status cannot be the source of funds truth.

## Consequences

Positive:

- Dropped notifications are repairable from a durable cursor.
- Read models can be deleted and rebuilt from raw finalized evidence.
- Reconciliation is explainable and exact at atomic-unit precision.
- Provider failover is isolated behind ports without changing domain rules.

Costs:

- The platform must operate checkpoint, backfill, gap, and provider-health
  telemetry.
- Finalized confirmation adds latency versus processed notifications.
- The current executable adapter persists bounded finalized pages, checkpoints,
  duplicate-safe receipts, and reconciliation runs. A production release still
  requires RPC adapters, raw transaction/event evidence, multi-provider
  failover, gap queues, replay tooling, and load/chaos evidence.
