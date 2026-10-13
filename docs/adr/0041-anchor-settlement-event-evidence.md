# ADR-0041: Decode and retain canonical Anchor settlement event evidence

- Status: Accepted for the first settlement indexer adapter
- Date: 2026-09-26
- Owners: Chain Data / Settlement
- Scope: Solana finalized block decoding and PostgreSQL chain evidence

## Context

Normalized receipts are useful for queries and reconciliation, but they are not
enough for a Tax Engine audit trail. A provider can re-encode a transaction, return
an event from a different program, or produce an ambiguous projection. The
indexer must therefore retain the exact event bytes used to derive a receipt and
must fail closed when the event does not match the deployed settlement ABI.

## Decision

The first settlement decoder is an injected `ReceiptDecoder` implementation in
the indexer bounded context. It accepts only successful transactions that
contain an invocation of the pinned settlement program and an Anchor
`SettlementExecuted` event with the exact discriminator and Borsh layout emitted
by the program. It validates `tax_atomic + net_atomic == gross_atomic` before
creating a normalized receipt. One settlement event per transaction is required
until the normalized receipt identity is explicitly widened beyond a Solana
signature.

For each accepted event, the adapter stores the complete base64-decoded event
bytes, discriminator, signature, finalized slot, event index, and a
domain-separated SHA-256 evidence digest. The raw evidence and normalized
receipt are inserted in the same serializable PostgreSQL page transaction. A
duplicate with byte-identical evidence is idempotent; any conflicting bytes,
digest, or normalized amount fails closed. Raw evidence is bounded to 1 MiB per
event and slots, signatures, and amount fields retain the existing u64/domain
limits.

## Alternatives considered

- Store only normalized receipts: rejected because it prevents independent
  forensic replay and makes provider-specific re-encoding authoritative.
- Decode arbitrary `Program data` logs: rejected because a log is not evidence
  of which program emitted it.
- Trust the Anchor IDL at runtime: rejected because the indexer must pin and
  review the ABI boundary; runtime network or mutable IDL fetches are not
  allowed on the hot path.
- Allow multiple settlement events per signature immediately: rejected because
  the current receipt table identity is `(stream_key, signature)`; widening the
  identity is a separate schema and API compatibility decision.

## Consequences

Positive:

- Receipt derivation is deterministic, bounded, and independently auditable.
- Raw evidence survives normalized projection rebuilds and provider failover.
- Invalid conservation, failed transactions, foreign programs, and malformed
  Anchor payloads cannot advance a finalized page.
- PostgreSQL can compare exact event bytes on replay instead of silently
  accepting a changed projection.

Costs and follow-on gates:

- The decoder must be versioned when the settlement event ABI changes.
- The current one-event-per-signature boundary must be revisited before a
  settlement instruction can emit multiple receipts in one transaction.
- Signature pagination, provider health scoring, gap/replay operator tooling,
  raw full-transaction retention, and localnet transaction tests remain
  production-readiness gates.
