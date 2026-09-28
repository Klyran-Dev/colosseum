# ADR-0154: Governed adjustment refund movement

## Status

Accepted for the first token-movement slice; implementation, validator
conformance, finalized reconciliation, and operational approval remain
required. This ADR deliberately does not generalize every adjustment kind.

## Context

ADR-0153 records an approved adjustment as immutable chain evidence but does
not move tokens. A production refund needs a separate authority boundary: the
tax portion may be released from the merchant/mint tax vault, while the net
portion must come from an explicit merchant-controlled funding account. Both
must arrive at the original payer's explicitly supplied token account in one
atomic Solana instruction.

The original settlement receipt intentionally does not store arbitrary source,
destination, or ATA topology. Reconstructing an account from a payer and mint,
or transferring from a vault to a caller-provided destination, would create
account-substitution and legal/beneficiary ambiguity. Tax, reversal, remittance,
chargeback, and correction semantics also do not share one universal funding
model.

## Decision

Add a separate governed `create_adjustment_refund_claim`,
`execute_adjustment_refund`, and governance cancellation instruction family for
the first supported movement kind: a negative `Refund` against an existing
immutable settlement receipt.

### Claim identity and roles

The claim PDA is:

```text
["refund-claim", adjustment_evidence_pda, claim_nonce]
```

The merchant/mint-scoped `SettlementAdjustmentRegistry` from ADR-0153 stores
the governance, proposer, and approver keys. The claim additionally binds an
explicit merchant refund-funder key. Creation requires the registered proposer
and refund-funder; execution requires the distinct registered approver and the
refund-funder. Governance can cancel a pending claim. The claim nonce and
executed/cancelled flags prevent replay.

### Account and transfer topology

The claim binds the exact accounts below; none may be substituted at
execution:

- original `SettlementReceipt` and its payer/merchant/mint scope;
- immutable `SettlementAdjustment` evidence PDA;
- tax-vault PDA token account and its deterministic vault-authority PDA;
- merchant-owned `net_source` token account and its explicit refund-funder
  signer;
- one `refund_destination` token account whose token-account owner is the
  original receipt payer; and
- the selected token program and extension-free mint/account layout.

Execution performs, in one instruction:

1. `tax_magnitude` from the vault-authority-controlled tax vault to the payer
   refund destination; and
2. `net_magnitude` from the refund-funder-controlled merchant source to the
   same payer refund destination.

The negative adjustment record already proves
`gross_magnitude = tax_magnitude + net_magnitude`. The instruction checks the
same invariant using unsigned magnitudes, and the two checked CPIs are atomic.
Zero legs are skipped after all account topology and authority checks. The
program supports only `Refund` kind in this slice; reversal, chargeback,
remittance, positive correction, mixed correction, and tax-authority-specific
flows require separate movement contracts.

### Finality and status boundary

The API/coordinator must persist a server-owned approved adjustment and claim
intent before preparing the transaction. A submitted transaction with unknown
status remains unknown. Only finalized reconciliation of the exact claim PDA,
adjustment PDA, transaction signature, slot, and token balance deltas may
transition off-chain evidence to `chain_executed`; this ADR does not authorize
an HTTP `paid` claim or legal/tax compliance statement.

## Invariants and failure policy

- Only kind `Refund` and negative, conserving signed deltas are executable.
- Registry scope and active proposer/approver/funder bindings are exact.
- Proposer and approver are distinct; approver and refund-funder are distinct
  in this first wire shape to avoid signer/account aliasing.
- The refund destination owner is exactly the original receipt payer.
- The net source owner is exactly the refund-funder signer.
- The tax vault owner is exactly the deterministic vault-authority PDA.
- Mint, token program, account owners, account lengths, and all token-account
  mints are exact; unsupported Token-2022 extensions fail closed.
- Tax vault, net source, and refund destination are pairwise distinct.
- Claim financial fields and topology are immutable after creation.
- A pending claim can be cancelled only by registry governance; an executed or
  cancelled claim cannot execute or cancel again.
- Insufficient funds, stale windows, paused protocol, account substitution,
  duplicate claim PDA, provider ambiguity, or either CPI failure leaves all
  claim state and token balances unchanged.
- No database, HTTP call, tax-rule graph, floating point, or unbounded loop is
  used on-chain.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`.

1. Reuse `execute_vault_release`: rejected because it cannot atomically bind
   the merchant-funded net leg and would make the tax vault the authority for
   the whole refund.
2. Transfer the full gross from the tax vault: rejected because the vault is
   not a universal refund funder and may not contain the net amount.
3. Transfer from the original payer source: rejected because the receipt does
   not retain that token account and a payer may not be the refund funder.
4. Browser/API direct transfer: rejected because it bypasses the program's
   account binding, atomicity, replay fencing, and non-custodial authority
   model.
5. **Immutable refund claim with two explicit sources and one payer-bound
   destination:** selected because it preserves atomic conservation while
   making every authority and account substitution checkable on-chain.

## Latency and resource budget

The claim creation is bounded and has no token CPI. Execution has exactly two
optional checked CPIs, a fixed account shape, and no dynamic fan-out. The
compiler target is p50 <= 1 ms and p99 <= 5 ms warm in-process; validator
evidence must report compute units, packet size, and processed/confirmed/
finalized latency separately. These are diagnostic budgets until target-cluster
measurements exist.

## Verification plan

1. Add failing domain/compiler tests for claim PDA, exact account order, refund
   kind restriction, signed-magnitude conservation, signer separation, and
   wire budget.
2. Add failing program tests for authority, mint/layout, owner, alias,
   insufficient-funds, window, pause, duplicate, cancellation, and CPI failure
   atomicity.
3. Add Linux validator conformance proving tax and net balances move exactly
   once, the payer receives gross, duplicate execution fails without drift, and
   wrong topology fails without movement.
4. Add finalized indexer/coordinator projection tests that retain unknown
   status and only advance after exact claim/signature/slot/balance evidence.
5. Add API/UI states for claim creation, wallet rejection, timeout, unknown,
   finalized execution, and reconciliation required; never show “paid” from
   submission alone.

## Consequences

Refunds become a real atomic, non-custodial product capability for one clearly
published scope. The protocol gains more accounts, a second governance claim
lifecycle, and operational key/funder setup. Other adjustment kinds remain
explicitly unavailable until their legal and authority semantics are modeled,
which keeps the Tier-1 product honest and extensible.
