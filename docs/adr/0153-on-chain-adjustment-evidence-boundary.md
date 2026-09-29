# ADR-0153: On-chain adjustment evidence boundary

## Status

Accepted for the first contract slice; implementation is complete and pinned
Linux validator execution remains required evidence. This ADR does not
authorize token movement, tax refunds, or a `posted` API state.

## Context

ADR-0150 defines immutable signed-delta adjustment arithmetic. ADR-0151 makes
the proposal and four-eyes approval durable, and ADR-0152 provides the
operator workspace. The remaining authority question is how an approved
adjustment becomes verifiable chain evidence without mutating the original
settlement receipt or allowing an API-calculated amount to move funds.

Adjustment semantics are not equivalent to a new checkout. A refund may need a
merchant-funded transfer, a tax remittance may require a tax-authority account,
and a chargeback may be an external dispute event. The program cannot infer
the correct funding account, beneficiary, tax jurisdiction, legal basis, or
refund eligibility from a negative delta. It must not turn those unresolved
facts into an implicit token transfer.

## Decision

Introduce a bounded Solana `record_adjustment` instruction as an immutable
chain-evidence primitive. The first slice records an approved adjustment
binding and emits an event; it does not transfer tokens and it does not change
the original `SettlementReceipt`.

The account is a deterministic PDA:

```text
["adjustment", original_receipt, adjustment_nonce]
```

The instruction requires:

- a merchant/mint-scoped on-chain adjustment registry initialized by a
  distinct governance authority, with active proposer and approver keys; the
  signer for this first slice is the registered approval authority and must
  differ from the registered proposer;
- the immutable original `SettlementReceipt` and the merchant/mint identities
  bound by its fields;
- a non-zero adjustment nonce, non-zero adjustment digest, and non-zero
  evidence digest;
- signed deltas represented on wire as `(sign, magnitude)` pairs, not signed
  arithmetic in the program: each magnitude is a checked `u64`, all three
  signs are equal for a refund/reversal/chargeback, and
  `gross_delta = tax_delta + net_delta` is verified with bounded `u128`
  intermediates before any account is created;
- the immutable source calculation and optional replacement calculation UUID
  digests as off-chain evidence, without treating UUIDs as secrets or chain
  ordering values; and
- a bounded reason/kind code and rule/attestation digest that are part of the
  PDA-bound canonical payload.

The on-chain record stores the original receipt key, merchant, mint, nonce,
kind, signed-delta components, adjustment digest, evidence digest, source and
replacement calculation digests, approval signer, and bump. A duplicate PDA
is rejected by account initialization. The original receipt and registry are
read-only during recording. The event is sufficient for indexer acceleration,
while the PDA is the replayable finalized source of adjustment evidence.

The API remains the authority for tenant scope, UUIDv7 lifecycle identity,
four-eyes approval, calculation replay, legal/tax evidence, and the later
choice of funding and beneficiary accounts. The settlement coordinator may
compile this instruction only after a server-owned approved record, a current
chain receipt, and a wallet/governance signature are all present. It must
withhold the transition to any `posted` state until finalized evidence is
reconciled against the exact PDA and event.

Actual token movement is a separate ADR and instruction family. It must bind
an explicit funding signer, beneficiary accounts, token program/mint/layout,
authority policy, and finalized adjustment record in one atomic transaction.
Tax-vault release may be used only where the jurisdiction policy and governed
vault workflow explicitly authorize it; this ADR does not generalize that
meaning. Until that movement contract exists, an adjustment remains approved
or chain-evidence-recorded, never paid.

## Invariants and failure policy

- no mutation of a settlement receipt;
- no token CPI and no implicit account creation besides the bounded evidence
  PDA;
- no zero digest, zero nonce, default identity, invalid kind, malformed
  signed amount, overflow, non-conserving delta, or duplicate PDA;
- original receipt merchant and mint must equal the supplied accounts;
- registry must be active and its merchant, mint, proposer, and approver must
  match the supplied accounts;
- governance, proposer, and approver keys must be non-default and pairwise
  distinct;
- the approval signer must be distinct from the proposer recorded by the
  server-side attestation/evidence envelope;
- all account metas are deterministic, unique, and packet-budget checked;
- a failed instruction creates no partial evidence because Solana execution is
  atomic;
- a submitted transaction with unknown status remains unknown and is never
  blindly retried; finalized reconciliation decides whether evidence exists;
- unsupported Token-2022 extensions are not relevant to the record-only slice
  and remain rejected by the later movement instruction.

## Alternatives considered

See `docs/architecture/alternative-analysis.md` for the scored comparison.

1. Mutate `SettlementReceipt` or reuse `settle`: rejected because it destroys
   original evidence, changes authority semantics, and allows an adjustment to
   masquerade as a new checkout.
2. Transfer tokens directly from the tax vault or original payer source:
   rejected because neither account is universally the lawful funder or
   beneficiary for every adjustment kind and jurisdiction.
3. Use only an off-chain `posted` row: rejected because a public product needs
   finalized, independently indexable chain evidence for any claimed chain
   state.
4. **Record a separate deterministic on-chain adjustment PDA first:** selected
   because it gives the chain an immutable, bounded, replayable commitment
   while keeping unresolved token-authority and legal semantics fail-closed.

## Latency and resource budget

The record-only instruction has a fixed account shape and no CPI. The compiler
budget is p50 <= 1 ms and p99 <= 5 ms in-process after warm-up; the serialized
instruction must remain below the existing safety margin. The validator-backed
gate must report compute units, transaction size, processed/confirmed/finalized
latency, and duplicate/invalid failure behavior on Linux with the pinned Agave
release. These are diagnostic budgets until target-cluster evidence exists.

## Verification plan

1. Add failing Rust compiler tests for canonical bytes, PDA/account order,
   amount conservation, signed-magnitude boundaries, duplicate protection,
   and packet budget.
2. Add failing program helper/property tests for all sign and overflow cases.
3. Add a cross-language vector for the adjustment instruction and PDA payload.
4. Extend the Linux validator conformance suite with successful record,
   duplicate rejection, wrong receipt binding, non-conserving delta, and
   unchanged token balances. The TypeScript conformance contract now includes
   the wrong-receipt and non-conserving negative cases; successful Linux
   execution is still required before this gate is checked.
5. Add coordinator/API gating tests proving an approved adjustment cannot be
   reported as paid from a non-final or mismatched transaction.
6. Add the separate token movement ADR only after the authority, funding,
   beneficiary, Token-2022, and tax-vault semantics are explicitly resolved.

## Consequences

The protocol gains a safe chain evidence boundary and replayable adjustment
identity without pretending that evidence is payment. The near-term operator
experience must show “approved” or “chain evidence recorded” distinctly from
“paid.” A later movement contract adds more accounts and governance complexity,
but that complexity is the necessary cost of not making an unsafe global
refund assumption.
