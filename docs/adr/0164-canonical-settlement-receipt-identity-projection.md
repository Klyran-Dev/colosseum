# ADR-0164: Canonical settlement-receipt identity projection

## Status

Accepted for implementation; release evidence remains open.

## Context

The finalized `SettlementExecuted` event contains the chain identity needed
to bind a later adjustment: receipt PDA, payer, merchant, mint, nonce, and
policy digest. The current generic `indexed_receipts` projection stores only
amounts and an evidence digest. An adjustment compiler therefore cannot
resolve its original receipt from server-owned evidence and would be forced
to accept caller-supplied chain accounts, violating the authority boundary.

Split settlements already have a richer typed projection, but single and
split settlement evidence must share the same identity contract for audit,
replay, and correction workflows.

## Decision

Extend `IndexedReceipt` and `indexed_receipts` with the immutable identity
fields decoded from the finalized Anchor event:

- `receipt`, `payer`, `merchant`, and `mint` public keys;
- `nonce` as a bounded `u64`; and
- `policy_digest` as a 32-byte digest.

The raw event remains the evidence source. The normalized row is a projection
that must be byte-for-byte consistent with the event and evidence digest. The
projection validator rejects default public keys, zero nonce, zero policy
digest, invalid amount conservation, identity substitution, and conflicting
replay. No API or browser may override these fields.

The migration uses expand/contract safety: new identity columns are added in
an additive migration with a not-valid completeness constraint, the existing
raw-evidence rebuild backfills them, and a later release validates and makes
the constraint strict after retained rebuild evidence. New writes are always
complete. Readers fail closed when a legacy row is incomplete instead of
silently using caller data.

Adjustment compilation may resolve the original receipt only through this
server-owned projection and a finalized evidence boundary. This ADR does not
authorize token movement, a `posted` state, or a browser-provided receipt PDA.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Accept original receipt and accounts in the adjustment request | Minimal schema work | Caller can substitute receipt, payer, merchant, or mint | Rejected |
| Store only a UUID-to-receipt mapping in the API database | Easy lookup | Creates a second mutable authority disconnected from raw chain evidence | Rejected |
| Reuse split-settlement projection for every payment | Rich identity already exists | Omits ordinary settlement rows and conflates bounded contexts | Rejected |
| **Enrich the canonical indexed receipt from finalized event bytes** | Server-owned, replayable, immutable, and shared by reconciliation/adjustments | Requires additive migration and projection rebuild | **Selected** |

## Invariants and failure policy

- all six identity fields are exactly 32 bytes and are not the default key;
- nonce is positive and policy digest is non-zero;
- amounts conserve `gross = tax + net` in checked `u64` arithmetic;
- the signature, slot, event index, event discriminator, and evidence digest
  remain bound through raw evidence;
- duplicate identical projections are idempotent; conflicting projections fail;
- incomplete legacy rows cannot authorize adjustment compilation; and
- no receipt projection mutation can alter the original on-chain receipt.

## Verification

1. Add failing decoder tests for identity extraction and default/zero-field
   rejection.
2. Add shared Rust fixtures proving single and split events produce complete
   receipt identity projections.
3. Add PostgreSQL migration and projection-rebuild tests for additive legacy
   rows, complete new rows, digest equality, and conflict rejection.
4. Add an adjustment-bound lookup contract before exposing any chain-evidence
   compiler endpoint.
5. Retain Linux validator evidence for receipt PDA binding before enabling
   production adjustment posting.

## Consequences

Adjustments can be compiled from authenticated, finalized, server-owned chain
evidence without accepting arbitrary receipt accounts. Projection rebuilds
become richer and the migration rollout requires an explicit legacy backfill
and validation phase. The chain remains authoritative; PostgreSQL remains a
replayable projection, never a source of token authority.
