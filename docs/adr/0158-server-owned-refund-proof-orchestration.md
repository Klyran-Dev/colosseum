# ADR-0158: Server-owned refund-proof orchestration

## Status

Accepted for implementation.

## Context

ADR-0157 defines a transaction-bound refund balance proof, but the indexer
service must not accept the token accounts, payer, or signer topology from an
operator or browser request. The indexed `AdjustmentRefundExecuted` event
contains the execution identity and movement amounts, while the finalized
claim PDA contains the remaining server-owned account bindings (`tax_vault`,
`net_source`, `refund_destination`, `payer`, and the claim nonce). A proof
worker therefore needs a bounded orchestration port that resolves the claim,
fetches the exact transaction, proves the checked deltas, and persists the
immutable evidence.

## Decision

Add a server-owned proof worker with these boundaries:

1. PostgreSQL selects at most one indexed refund execution without an existing
   proof for the configured stream. The query is a projection cursor only; the
   finalized Solana transaction and raw evidence remain authoritative.
2. The worker derives an execution identity from the indexed event and asks a
   complete-request RPC provider to resolve the finalized claim PDA. The claim
   account is checked for deterministic PDA, program ownership, Anchor layout,
   event bindings, nonce, execution state, and valid window. Missing fields are
   never supplied by a caller.
3. The same provider attempt then reads the exact finalized transaction and
   obtains transaction-bound pre/post token balances as specified by
   ADR-0157. A provider failover retries the complete claim-plus-transaction
   request and never combines responses from different providers.
4. The worker constructs the negative `AdjustmentDelta` from the indexed event,
   calls the pure `settlement-adjustments` checked proof function, and persists
   the result through the serializable, idempotent storage boundary.
5. Proofs are evidence only. The worker does not create `paid`,
   `chain_executed`, or any other lifecycle state, and it does not retry an
   invalid proof indefinitely. A malformed or contradictory provider response
   is observable and requires later bounded retry/reconciliation policy.

The initial worker processes one candidate per bounded cycle. This keeps
provider fan-out, database contention, and memory bounded while preserving
room for future sharding by stream and merchant.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Accept a complete topology from API/operator input | Simple wiring | Client-controlled accounts can redirect evidence or hide substitution | Rejected |
| Persist topology in a mutable database table | Fast subsequent reads | Introduces another authority and lifecycle synchronization problem | Rejected |
| Use the indexed event alone | No extra claim read | Event omits the tax vault, net source, payer, and vault authority bindings | Rejected |
| **Resolve the finalized claim PDA inside a complete-request RPC provider attempt** | Server-owned authority, exact on-chain bindings, bounded failover, no mutable topology projection | Adds one bounded account read before transaction evidence | **Selected** |

## Invariants and budgets

- The candidate execution is loaded by `(stream_key, signature, event_index)`
  from the server-owned projection and must have no proof row at selection time.
- Claim identity, event identity, amount conservation, reference digest, mint,
  merchant, refund funder, and destination must match exactly.
- The claim resolver accepts only legacy SPL Token layouts until a separate
  Token-2022 extension contract is approved.
- One worker cycle loads at most one candidate, performs at most two bounded
  RPC calls per provider attempt, and persists at most one proof.
- Provider attempts are individually deadline-bounded and fail over as a whole
  request. Invalid evidence is never merged with a successful response.
- Storage remains idempotent under duplicate worker execution and serializable
  under contention. The immutable proof digest is the evidence identity.

## Verification and rollout

- Add domain-first tests for execution-to-delta conversion and overflow/
  underflow rejection.
- Add provider fixtures for claim field substitution, invalid PDA, missing or
  unexecuted claim, successful complete resolution, provider failover, and
  transaction-bound pre/post balance substitution.
- Add storage tests for pending-candidate selection, proof exclusion, and
  concurrent idempotent persistence.
- Add service tests proving one-candidate bounded processing and fail-closed
  behavior. Keep the production loop disabled until Linux validator-backed
  conformance and finalized-provider fixtures pass.
