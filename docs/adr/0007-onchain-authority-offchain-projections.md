# ADR-0007: Keep authority on-chain and projections off-chain

- Status: Accepted
- Date: 2026-09-25

## Context

Low-latency APIs and dashboards need indexed data, but database state can be stale or corrupted. Letting the API authorize money movement would introduce a custodial trust boundary.

## Decision

The Solana program and token programs are the authority for balances and successful settlement. Rust APIs compile and submit transactions but cannot sign for users or release funds without on-chain governance. The indexer stores rebuildable projections and reconciles them against finalized chain state.

## Consequences

- Read performance can be optimized independently.
- Indexer gaps are operational incidents, not accounting ambiguity.
- APIs must expose commitment level and reconciliation status to callers.

