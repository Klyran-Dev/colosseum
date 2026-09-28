# ADR-0047: Bounded raw transaction evidence for projection rebuilds

- Status: Accepted for the finalized indexer evidence model
- Date: 2026-09-26
- Owners: Chain Data / SRE
- Scope: Solana finalized block ingestion and PostgreSQL evidence retention

## Context

The provider adapter requested full Solana transactions but retained only the
decoded `SettlementExecuted` event bytes and normalized receipt. That was
insufficient to prove a clean projection rebuild from the chain payload or to
perform forensic inspection when decoding rules evolve. Provider output must
remain evidence rather than becoming the authority for settlement invariants.

## Decision

For every transaction in a finalized `getBlock` response, the pinned decoder
stores a bounded serialized transaction object with its slot, block position,
signature, and domain-separated SHA-256 digest. The PostgreSQL identity is
`(stream_key, signature, transaction_index)`, and payload size is capped at
512 KiB. Event bytes and normalized receipts remain separate records with
their existing validation and conservation rules.

Raw transaction payloads are immutable evidence. Re-delivery of identical
payload and digest is idempotent; any conflicting payload for the same
identity fails closed. Raw payload retention never authorizes settlement and
never bypasses finalized commitment or decoder validation.

## Alternatives considered

- Retain only decoded event bytes: rejected because it cannot support full
  transaction forensic review or future decoder rebuilds.
- Store provider JSON as unbounded JSONB: rejected because it creates an
  unbounded database/RPC amplification path and does not preserve a bounded
  resource budget.
- Store only a payload digest: rejected because a digest cannot reconstruct a
  projection or answer forensic questions without the original payload.
- Treat raw payload fields as settlement truth: rejected because raw chain
  evidence must still pass finality, conservation, and program-specific rules.

## Consequences

Positive:

- Finalized projection rebuilds have access to bounded transaction payloads.
- Evidence is immutable, content-addressed, and conflict-detectable.
- Storage and memory budgets remain explicit for high-throughput blocks.

Costs and follow-on gates:

- Retention, partitioning, encryption, and privacy review are required before
  production volume is enabled.
- A clean-database rebuild drill and signature-pagination strategy remain
  release gates.
