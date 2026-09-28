# ADR-0093: Finalized split-settlement evidence projection

- Status: Accepted
- Date: 2026-09-27
- Owners: Chain Data / Settlement
- Scope: Finalized decoding and durable projection of `SplitSettlementExecuted`

## Context

The settlement program emits a dedicated `SplitSettlementExecuted` Anchor event
for the additive Split Plan instruction. A generic receipt projection proves
amount conservation, but it does not prove which immutable split-plan version
or digest was used. Treating the split event as a generic receipt would make
replay, support, and reconciliation unable to distinguish legacy settlement
from split settlement or detect a plan-binding mismatch.

## Decision

Extend the finalized indexer contract with a bounded, canonical split-event
projection:

- Decode exactly one supported `SplitSettlementExecuted` event per successful
  transaction, using a versioned discriminator and fixed Anchor field layout.
- Retain the event bytes in the existing immutable raw-evidence table and
  retain a typed `indexed_split_settlements` projection containing receipt,
  payer, merchant, mint, nonce, amounts, split-plan version/digest, and policy
  digest.
- Validate `gross = tax + net`, non-zero gross, non-zero digests, canonical
  public-key fields, finalized slot range, and digest identity before storage.
- Keep legacy `SettlementExecuted` and split events mutually exclusive for one
  transaction. A transaction with both event kinds, multiple supported events,
  malformed event bytes, or conflicting projection rows fails closed.
- Store one split projection per `(stream_key, signature)`. The receipt
  projection remains the common amount/conservation read model; the split
  projection is the authoritative evidence for split-plan binding.
- Include the split projection in forward indexing, replay merge, and the
  bounded projection-rebuild path. Rebuild decodes the raw event discriminator
  rather than trusting normalized split rows.

This ADR does not infer recipient-level transfers from token balances. Recipient
allocation and account-level conservation require a separate finalized balance
evidence contract and validator-backed conformance gate.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Treat split events as generic receipts | Rejected | Loses split-plan version/digest and weakens replay evidence. |
| Decode recipients from transaction account metas | Rejected | Account ordering is not a durable event contract and cannot prove executed allocation. |
| Store only raw bytes | Rejected | Forensic recovery remains possible, but operational queries and reconciliation become expensive and error-prone. |
| **Typed projection plus immutable raw evidence** | **Selected** | Preserves protocol identity, bounded queries, idempotent replay, and independent forensic recovery. |

## Invariants and failure behavior

1. A split projection is accepted only from finalized, digest-verified raw event
   bytes with the exact canonical discriminator and length.
2. Amount conservation and non-zero identity fields are checked before both
   projection and checkpoint advancement.
3. Identical replay is idempotent; any conflicting row for the same signature
   fails closed and rolls back the page transaction.
4. The legacy and split event discriminators cannot both be accepted for the
   same transaction.
5. Projection persistence never becomes Solana authority; finalized chain
   state and retained raw evidence remain authoritative.

## Budgets and evidence

The decoder is fixed-width and O(1) per event. The persistence path uses the
existing 250 ms lock and 900 ms statement timeouts and the same bounded page
and retry limits as legacy indexing. Evidence now includes Rust decoder and
rebuild tests, malformed/conservation/conflict tests, Docker migration
validation, and a replay-safe integration assertion. Validator-backed account
balance and recipient-transfer evidence remains a separate release gate.
