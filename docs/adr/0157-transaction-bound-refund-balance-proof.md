# ADR-0157: Transaction-bound refund balance proof

## Status

Accepted for implementation. This ADR supersedes the provider-observation
portion of ADR-0156; ADR-0156's pure checked arithmetic contract remains
authoritative.

## Context

A finalized `getMultipleAccounts` response proves current token-account
topology and post-state, but it cannot provide the pre-state at the same slot
as an already executed transaction. Persisting a post-state snapshot as a
before/after proof would permit false reconciliation claims after unrelated
transfers or later transactions. A refund proof must bind the exact execution
signature, event identity, transaction slot, account indexes, token mint,
token program, account owners, and pre/post atomic amounts.

## Decision

Add a transaction-bound refund proof adapter with these rules:

1. Request the exact canonical execution signature with finalized
   `getTransaction`, bounded JSON encoding, and a supported transaction
   version.
2. Require the returned primary signature, non-zero slot, successful
   transaction metadata, and expected finalized refund event identity to match
   the indexed execution projection. Reject missing logs, duplicate event
   identity, signature substitution, failed metadata, and provider responses
   that exceed the response budget.
3. Resolve the legacy transaction account-key list and require the tax vault,
   refund source, and payer destination to appear exactly once. Read both
   `preTokenBalances` and `postTokenBalances` by account index, requiring one
   matching entry per account with the expected mint, legacy SPL token program,
   expected token authority, and canonical decimal atomic amount.
4. Use the transaction slot for both pure-domain snapshots. The
   `settlement-adjustments` proof then checks exact checked-u64 conservation,
   underflow, overflow, and all three deltas. No provider-derived amount is
   accepted from a caller.
5. Separately read the executed claim PDA at finalized commitment to validate
   its deterministic PDA, Anchor discriminator/layout, stored topology,
   nonce/window, and executed-not-cancelled state. A claim account read is
   topology evidence; it does not replace transaction pre/post balances.
6. Persist one immutable proof row transactionally with the matching indexed
   event and raw evidence. The proof row is evidence only and cannot advance a
   paid, posted, or `chain_executed` lifecycle state until a later ADR defines
   the complete signature/event/raw-evidence binding and operational state
   machine.

The existing `SolanaRpcRefundBalanceSource` remains a bounded post-state and
claim-topology reader for diagnostics. It must not be used as the source of a
before/after refund proof.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Persist a finalized `getMultipleAccounts` post-state | Simple and inexpensive | No transaction pre-state; unrelated later transfers can create false proof | Rejected |
| Read three accounts independently before and after | Familiar account API | Different slots and races cannot prove one atomic movement | Rejected |
| Trust event amounts only | Lowest latency | Events do not prove committed token-account deltas or account-index binding | Rejected |
| **Use finalized transaction `preTokenBalances`/`postTokenBalances` plus a separate finalized claim topology read** | Exact transaction slot, account-index binding, atomic pre/post observation, and explicit account semantics | Requires strict provider decoding and two bounded RPC calls | **Selected** |

## Invariants and budgets

- The execution signature is canonical and is the only transaction lookup key.
- Transaction slot is non-zero and finalized; metadata error must be null.
- Every required account occurs exactly once in the transaction key list and
  has exactly one valid pre and post balance entry.
- Token program, mint, authority, amount encoding, event identity, claim PDA,
  nonce, and transaction signature are all server-owned bindings.
- The aggregate provider response budget is bounded by the configured RPC
  response limit; account-key, balance-entry, log-message, and retry counts
  are bounded before allocation or iteration.
- Provider failover retries the complete proof request and never combines
  balances from different providers or transactions.

## Verification and rollout

- Add golden JSON fixtures for successful execution, absent zero balances,
  duplicate account indexes, mint/program/owner substitution, signature
  substitution, failed metadata, missing event, and malformed amounts.
- Property-test decimal `u64` parsing and exact delta conservation at all
  boundaries.
- Persist proofs only after the indexed event and raw evidence are present in
  the same serializable PostgreSQL transaction.
- Keep paid-state behavior unavailable until Linux validator execution,
  finalized provider fixtures, replay/idempotency, claim topology, and
  operator reconciliation evidence pass.
