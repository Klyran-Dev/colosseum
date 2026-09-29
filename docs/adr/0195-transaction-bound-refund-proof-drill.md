# ADR-0195: Transaction-bound refund proof drill

- Status: Accepted
- Date: 2026-09-29
- Deciders: Principal Low-Latency Architecture, Finance/Tax, Security, and Operations
- Scope: local integration evidence for ADR-0157, ADR-0158, ADR-0179, and ADR-0185

## Context

The refund domain already rejects post-state-only evidence and requires a
transaction-bound finalized proof: one provider response must bind the exact
signature, finalized slot, successful execution, typed `AdjustmentRefundExecuted`
event, account-key positions, and pre/post legacy-SPL token balances. The
server-owned indexer worker also has bounded UUIDv7 candidate leases and an
immutable proof sink.

The repository did not yet exercise those boundaries together through the real
Docker indexer service. Unit tests and storage drills are necessary but do not
prove that the configured RPC adapter, claim-account topology resolver,
transaction parser, worker lease, PostgreSQL proof insert, and execute-intent
finalization are wired consistently.

## Decision

Add a deterministic local-only refund-proof integration drill. It will:

1. start PostgreSQL 18, the real RPC fixture, and the real indexer service;
2. seed one synthetic finalized execution projection and one durable execute
   intent with matching claim, adjustment, merchant, mint, funder, destination,
   amounts, signature, and reference digest;
3. configure the fixture with a finalized claim PDA account and a bounded
   `getTransaction` response containing exactly one matching typed event and
   three exact pre/post token-balance pairs;
4. let the real indexer worker claim the candidate, resolve the claim topology,
   fetch the transaction, prove conservation, and persist one immutable
   `refund_balance_proofs` row;
5. require the worker to advance the matching execute intent to `finalized`,
   emit one outbox event, and converge on replay without a duplicate proof;
6. attempt a conflicting proof mutation and require PostgreSQL's append-only
   guard to reject it.

The RPC fixture is synthetic and local-only. It models the provider contract;
it does not claim validator execution, live provider agreement, custody, or
legal/tax compliance. Those remain independent release gates.

## Invariants and budgets

- The candidate is selected only from the configured stream with no existing
  proof and an active bounded UUIDv7 lease.
- Claim-account fields, PDA derivation, nonce, amounts, reference digest, and
  topology must match the indexed execution before the transaction is read.
- The transaction signature, slot, success status, account-key positions,
  event fields, mint, token program, owners, and exact three-leg balance deltas
  must match the same target.
- `tax_after + tax_delta == tax_before`, `source_after + net_delta ==
  source_before`, and `destination_after == destination_before + gross` are
  checked with exact `u64` arithmetic.
- One successful worker cycle creates exactly one proof row, one finalized
  execute transition, and one finalized outbox event. Replays are idempotent.
- Provider reads, worker cycles, and polling are bounded; no user-controlled
  unbounded collection, retry, or task is introduced.
- The fixture exposes no private key or raw sensitive tax evidence.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Keep unit and storage tests only | Fast and already available | Does not prove service wiring, RPC JSON compatibility, leases, or finalization | Rejected as the sole gate |
| Seed `refund_balance_proofs` directly | Simple database assertion | Bypasses claim resolution, transaction parsing, arithmetic, and worker ownership | Rejected |
| Use a public RPC or mainnet transaction in CI | Highest environmental realism | Non-deterministic, credential/privacy risk, and unsafe for public repository CI | Rejected |
| **Real indexer + PostgreSQL + bounded synthetic finalized RPC contract** | Exercises the complete trust boundary reproducibly without credentials | Does not replace Linux validator or live-provider evidence | **Selected** |

## Verification and rollout

- Add explicit RPC-fixture proof mode with bounded configuration validation and
  unit tests for claim-account and transaction response construction.
- Run the drill through a dedicated Compose profile and `make` target so it
  cannot alter the normal development database or volumes.
- Retain only public-safe result metadata: proof digest, slot, cardinalities,
  worker state, and latency samples; never retain secrets or raw signed bytes.
- Keep the production-readiness checkboxes open until Linux validator,
  live-provider agreement, production load/SLO, key-ceremony, accessibility,
  and independent review evidence are complete.
