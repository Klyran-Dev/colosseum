# ADR-0155: Finalized refund execution evidence projection

## Status

Accepted for implementation. This ADR adds typed finalized evidence only; it
does not authorize a refund, mark an adjustment paid, or replace the Solana
program as the source of truth.

## Context

ADR-0154 defines an atomic `execute_adjustment_refund` instruction that emits
`AdjustmentRefundExecuted` after the tax-vault and merchant-funder checked
transfers commit. The current indexer retains unknown program events as raw
evidence but cannot yet project this event into a typed, replay-safe record.
Without a typed projection, a reconciler cannot prove that a refund claim's
exact claim PDA, adjustment, payer-bound destination, funder, approver, mint,
and signed magnitudes match the durable adjustment workflow.

The durable database remains a projection and workflow fence. A transaction
signature, event, or API request alone is insufficient to state that money
moved. Finalized transaction evidence and independently verified finalized
token-account balances are required before a future lifecycle can expose
`chain_executed` or `paid` semantics.

## Decision

Add `IndexedAdjustmentRefundExecution` to the finalized page contract. The
indexer will:

1. recognize only the canonical Anchor discriminator for
   `AdjustmentRefundExecuted` from the fixed settlement program;
2. decode the bounded fixed-width event and reject malformed lengths,
   zero identities, zero nonce, zero reference digest, and non-conserving
   magnitudes;
3. retain the event in `raw_chain_evidence` and emit exactly one typed
   execution projection per signature/event index;
4. require the event's signature and slot to satisfy the existing finalized
   page contract; and
5. merge duplicate evidence only when every field, digest, signature, slot,
   and event index is identical. Conflicting evidence fails closed.

The first database projection stores the typed event with UUIDv7 identity,
merchant scope, native signature, finalized slot, event index, claim and
adjustment PDAs, original receipt, mint, funder, destination, approver,
unsigned magnitudes, reference digest, and evidence digest. It has an
immutable unique key on `(stream_key, signature, event_index)` and a separate
claim PDA uniqueness constraint. It is not a balance ledger and cannot be
used to skip finalized token-account reconciliation.

The API and UI may expose this as evidence with a reconciliation-required
state. No `posted`, `executed`, `chain_executed`, or `paid` adjustment status is
added by this ADR. A subsequent ADR must define the exact durable transition,
balance proof, API contract, and operational approval.

## Alternatives considered

| Alternative | Benefit | Risk | Decision |
| --- | --- | --- | --- |
| Keep only opaque raw event bytes | Small schema change | Every consumer reimplements parsing and may disagree on field binding | Rejected |
| Trust API-submitted signature and amounts | Simple workflow | Caller-controlled evidence can claim movement without finalized chain proof | Rejected |
| Derive movement from token balances only | Direct financial observation | Cannot bind a balance delta to the exact claim, event, or beneficiary | Rejected |
| **Typed finalized event projection plus independent balance reconciliation** | Deterministic parsing, replay, account binding, and auditable evidence | Requires a new projection and later reconciliation work | **Selected** |

## Invariants and budgets

- Only the fixed settlement program event discriminator is accepted.
- Event length and all collection sizes are bounded before allocation.
- Gross magnitude equals tax magnitude plus net magnitude and is non-zero.
- Claim, adjustment, receipt, merchant, mint, funder, destination, and
  approver are non-default and distinct where the contract requires it.
- Event evidence is finalized and immutable; provider disagreement remains
  unknown and never advances a workflow.
- Decoder target is p50 <= 10 microseconds and p99 <= 50 microseconds warm;
  persistence uses the existing bounded PostgreSQL transaction and timeout
  policy.

## Verification

- Unit tests cover discriminator/length decoding, conservation, zero-field
  rejection, duplicate/conflict merging, and finality/range validation.
- Indexer fixture tests cover a finalized transaction containing the event and
  ensure other events in the same transaction remain bounded.
- PostgreSQL integration tests cover idempotent insert, conflicting evidence,
  merchant scoping, and append-only projection behavior.
- A Linux validator conformance test from ADR-0154 must produce the event and
  retain its signature, slot, raw evidence digest, and typed projection.
- A later reconciliation ADR must independently prove both token-account
  deltas before exposing any paid-state behavior.

## Consequences

The system gains a reusable, deterministic evidence contract for refund
execution without weakening the authority boundary. It increases indexer and
schema surface area and intentionally leaves the financial lifecycle open
until finalized account reconciliation is implemented.
