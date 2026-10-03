# ADR-0286: Bounded on-chain revenue-stream accruals and release claims

- Status: Accepted
- Date: 2026-10-04
- Owners: Settlement, Treasury, and Blockchain Architecture

## Context

The Revenue Streaming bounded context has a durable, tenant-scoped accrual
ledger and an API that reserves release claims. A database row must not become
authority to move tokens. The missing boundary is a small Solana state machine
that binds a governed plan, finalized-evidence references, a single-use claim,
the partner destination, and the vault CPI.

The program cannot query finality, inspect an off-chain tax rule graph, or call
an evidence provider. Those responsibilities remain in the finalized-proof
adapter. The program must nevertheless enforce the bindings and caps so a
compromised API cannot redirect a claim, change its amount, reuse a nonce, or
move more than the plan permits.

## Decision

Add bounded instructions to the existing settlement program:

1. `initialize_revenue_stream_plan` creates an immutable merchant/mint-scoped
   plan PDA with one recipient owner, one vault, one asset, four-eyes plan
   authorities, slot bounds, lifetime/single-claim caps, and policy/split
   digests.
2. `approve_revenue_stream_plan` and `activate_revenue_stream_plan` require
   distinct configured authorities and explicit state transitions.
3. `record_revenue_stream_accrual` accepts one finalized settlement receipt
   reference at a time, binds merchant/mint/policy/window/amount, and records
   the proof digest in a deterministic accrual PDA. The operator signature is
   the final authorization after the off-chain adapter has independently
   checked finalized evidence; the program does not claim to verify RPC
   finality by itself.
4. `create_revenue_stream_claim` reserves a bounded amount against the plan,
   binds the plan digest, claim digest, evidence reference, nonce, destination,
   and execution window, and requires the configured operator.
5. `execute_revenue_stream_claim` requires the distinct approver, validates
   the exact plan and destination, and transfers only from the plan's vault
   through the deterministic vault-authority PDA. It marks the claim executed
   atomically with the CPI.
6. `cancel_revenue_stream_claim` is governance-only and cannot cancel an
   executed claim. Plan pause stops new accruals and claims but allows a
   separately governed policy for pending releases; this first contract keeps
   execution available for pending claims while the plan is paused.

The on-chain program stores reserved and released totals and uses checked
arithmetic. It accepts only extension-free legacy SPL Token and extension-free
Token-2022 layouts until a separate compatibility ADR and validator matrix
approve additional extensions. It does not perform continuous time-based
streaming or unbounded recipient fan-out.

## Alternatives considered

| Alternative | Strengths | Risks | Decision |
| --- | --- | --- | --- |
| Reuse the generic vault release claim | Less program code | No plan, accrual, recipient, or cap binding; API could redirect a claim | Rejected |
| Keep release authority entirely in PostgreSQL | Fast to add | Database state could be mistaken for token authority | Rejected |
| Continuous mutable on-chain stream account | Familiar streaming model | Larger authority surface, clock/epoch complexity, correction ambiguity, more contention | Rejected |
| **Bounded plan + finalized accrual references + single-use claim** | Small deterministic state, explicit caps, replay protection, auditable CPI | Requires an off-chain proof adapter and explicit operator custody | **Selected** |

## Invariants and failure policy

- Plan, accrual, claim, mint, vault, destination, and merchant bindings are
  checked at every instruction boundary.
- `released + reserved + new_amount <= accrued` and total accrued cannot exceed
  the lifetime cap.
- Claim amount is positive and no greater than the single-claim cap.
- Plan and claim PDAs bind the merchant, mint, digest, and nonce; duplicates
  fail at account initialization or state checks.
- Operator and approver must be distinct. Governance is separate from the
  movement signer.
- Unknown RPC outcomes do not advance off-chain finalization; the proof adapter
  reconciles the exact claim PDA, signature, event, and finalized balances.
- Any unsupported token owner/layout, stale or malformed proof reference,
  mismatched digest, overflow, pause, or account alias fails closed.

## Consequences

The API can return a prepared claim without implying a transfer. The transaction
compiler can derive all PDAs from server-owned route data and prove the message
before wallet/HSM signing. The indexer can project `RevenueStreamAccrualRecorded`,
`RevenueStreamClaimCreated`, `RevenueStreamClaimExecuted`, and cancellation
events into the existing append-only evidence path.

This ADR does not claim Linux validator, live RPC, Token-2022 extension, HSM,
or regional production evidence. Those remain release gates.

## Verification

- Unit tests cover plan state, accrual conservation/window/cap, claim reserve,
  four-eyes, destination binding, pause, duplicate, and overdraw behavior.
- Client compiler vectors cover discriminator, PDA seeds, account order, exact
  decimal `u64` fields, and signed-message digest binding.
- `make solana-conformance` must execute the instruction matrix against the
  pinned Linux/amd64 validator before production enablement.
