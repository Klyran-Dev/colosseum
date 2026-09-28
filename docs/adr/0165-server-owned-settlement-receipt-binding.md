# ADR-0165: Server-owned settlement-intent receipt binding

## Status

Accepted for implementation; adjustment compiler exposure remains gated.

## Context

`SettlementAdjustment.original_settlement_id` is the UUIDv7 of the durable
settlement intent. A finalized chain receipt is indexed by transaction
signature, while the intent is the server-owned record that captures the
wallet-submitted transaction signature and finalized lifecycle. A future
adjustment evidence compiler must join these records without trusting a
receipt, payer, merchant, mint, or signature supplied by a browser or
operator.

The receipt projection can be present in more than one finalized indexer
stream. Identical duplicate projections are acceptable; conflicting identity,
amount, policy, or evidence rows are unsafe. An intent that is not finalized,
has no finalized commitment/signature/slot, has no complete receipt projection,
or has an ambiguous projection must not produce compiler input.

## Decision

Add a storage-port lookup named
`load_finalized_settlement_receipt_binding(merchant_id, settlement_id,
asset_id)`. It is the only supported server-side resolution path for the
original receipt used by a future adjustment compiler.

The lookup:

1. scopes the settlement intent by authenticated tenant and UUIDv7;
2. requires `settlement_intents.status = 'finalized'`, a finalized commitment,
   a non-null signature, and a non-null slot;
3. joins the intent signature and slot to the canonical `indexed_receipts`
   projection and requires the requested asset/mint;
4. collapses identical rows observed in multiple streams; and
5. returns exactly one complete binding or fails closed on not-found,
   incomplete, or conflicting evidence.

The returned binding includes the receipt PDA, payer, merchant, mint, nonce,
policy digest, gross/tax/net amounts, transaction signature, finalized slot,
and evidence digest. It is an immutable read model assembled from the intent
and finalized chain projection; it does not create a mutable UUID-to-receipt
authority and does not authorize token movement.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Accept a receipt PDA/signature in the adjustment request | Minimal API work | Caller can substitute a different finalized payment | Rejected |
| Add mutable receipt fields to `settlement_adjustments` | Simple later reads | Copies authority into a mutable proposal and can drift from chain evidence | Rejected |
| Join only by signature | Small query | Omits tenant, finalized lifecycle, slot, and asset binding | Rejected |
| **Join the tenant-scoped finalized intent to canonical receipt projection** | Server-owned, replayable, duplicate-tolerant, and fail-closed | Requires a bounded storage port and conflict tests | **Selected** |

## Invariants and failure policy

- the settlement UUID, tenant UUID, transaction signature, and finalized slot
  are server-derived or server-observed;
- the indexed receipt mint equals the requested adjustment asset;
- duplicate stream rows must be byte-for-byte equivalent across identity,
  amounts, policy, signature, slot, and evidence digest;
- an incomplete legacy projection never falls back to request data;
- zero, non-finalized, stale, or ambiguous evidence is unavailable to the
  compiler; and
- this lookup proves evidence binding only. A separate ADR must govern
  adjustment account compilation, wallet signing, finality, and any movement
  of funds.

## Verification

1. Add storage contract tests for not-finalized, missing-signature,
   missing-projection, incomplete-identity, duplicate-identical, duplicate-
   conflicting, wrong-asset, and successful binding cases.
2. Add query/index evidence for tenant, signature, slot, and mint lookup
   performance under the documented settlement-adjustment budget.
3. Use the returned binding in the future server-side adjustment compiler;
   reject all client-provided receipt/account overrides.
4. Retain Linux validator and finalized-indexer evidence before enabling any
   production adjustment-posting state.

## Consequences

The adjustment path has a single authority-safe identity resolution boundary.
It adds no chain writes or hot-path dependencies to ordinary checkout, but it
requires finalized indexer availability and makes evidence gaps visible as a
hard failure. Cross-stream duplicate handling is explicit rather than hidden
inside application code.
