# ADR-0156: Finalized refund balance proof

## Status

Accepted for the pure domain contract and diagnostic finalized post-state
adapter. ADR-0157 supersedes its provider-observation portion for real
before/after proof persistence. It does not add a paid-state transition.

## Context

ADR-0155 provides typed finalized `AdjustmentRefundExecuted` evidence, but an
event alone does not prove that the three token-account deltas committed. The
refund movement contract has two sources and one destination: the tax vault
loses the tax leg, the explicit merchant refund-funder source loses the net
leg, and the original payer-owned destination gains gross. A provider may
return different account slots or stale balances, and a single-account read
cannot establish the complete atomic topology.

## Decision

Add a pure `settlement-adjustments` balance-proof function. It accepts a
negative conserving refund delta and two server/provider-derived snapshots:

- `finalized_slot`;
- tax-vault atomic balance;
- merchant refund-source atomic balance; and
- payer-destination atomic balance.

The proof succeeds only when both snapshots have the same non-zero finalized
slot and the exact checked-u64 deltas are:

```text
tax_vault_after        = tax_vault_before        - tax_atomic
refund_source_after    = refund_source_before    - net_atomic
payer_destination_after= payer_destination_before+ gross_atomic
```

The result preserves the slot, all three expected amounts, and before/after
balances for audit and later durable persistence. It fails closed on positive
or non-conserving deltas, slot disagreement, underflow, overflow, or any
observed mismatch. It does not accept floating point, caller-provided
balances, confirmed commitment, or a future timestamp as proof.

The implemented infrastructure adapter obtains the executed claim account and
all three token-account states from one bounded finalized
`getMultipleAccounts` response. It verifies the deterministic claim PDA,
Anchor discriminator/layout, adjustment/receipt/merchant/mint/payer/source/
destination bindings, claim nonce/window/finalized flags, token owner/mint/
layout and expected authority, rejects Token-2022 until its extension contract
exists, and supports bounded deadline-aware provider failover. It does not yet
bind the snapshot to the exact execution signature/event digest or persist a
composite proof. Only that future composite evidence may advance a
`chain_executed` state.

## Alternatives considered

| Alternative | Strength | Risk | Decision |
| --- | --- | --- | --- |
| Trust the execution event | Simple and low latency | Events can be malformed, replayed, or detached from observed balances | Rejected |
| Read only the payer destination | Proves an inflow | Cannot prove tax-vault and merchant-funder outflows or source substitution | Rejected |
| Read each account independently | Easy to call | Slots can differ and mixed snapshots can create a false proof | Rejected |
| **One finalized multi-account snapshot plus exact pure-domain delta proof** | Atomic observation boundary, checked arithmetic, deterministic replay | Requires bounded account topology verification and later persistence | **Selected** |

## Verification and rollout

- Property-test all bounded `u64` amounts, including zero-leg refunds,
  destination overflow, source underflow, and conservation.
- [x] Add a provider adapter that requests one finalized multi-account response
  and rejects missing, weak-owner, wrong-mint, extension, aliased, malformed,
  invalid-claim, or zero-slot accounts; add bounded provider failover without
  combining snapshots.
- Persist the proof with the typed event only after signature, claim account,
  and account topology verification.
- Keep `posted`, `chain_executed`, and `paid` unavailable until Linux
  validator, finalized RPC-fixture, replay, and operator-reconciliation
  evidence pass.
