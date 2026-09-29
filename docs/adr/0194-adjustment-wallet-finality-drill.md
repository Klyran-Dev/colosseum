# ADR-0194: Wallet-bound adjustment finality drill

- Status: Accepted
- Date: 2026-09-29
- Deciders: Principal Low-Latency Architecture, Finance/Tax, Security, and UX
- Scope: local integration evidence for ADR-0170 and ADR-0171

## Context

ADR-0170 and ADR-0171 define the wallet-bound, record-only adjustment flow:
the server compiles an immutable `record_adjustment` instruction, the wallet
signs the exact message, the API submits it through the bounded RPC
coordinator, and finalized status is granted only when the typed
`SettlementAdjustmentRecorded` projection matches the durable intent.

The repository had unit coverage for the transaction verifier and a Docker
integration check for intent creation, but no executable proof crossed all of
the following boundaries in one scenario: server-owned compile response,
real Solana transaction serialization and wallet signature, API signed-byte
binding, exactly-once submission fencing, finalized provider observation, and
exact typed-event proof-envelope response.

## Decision

Add a deterministic local integration drill using the real built API,
PostgreSQL 18 schema, RPC coordinator, and a small Node wallet harness using
`@solana/web3.js` only for synthetic test keys and transaction serialization.

The drill will:

- create and approve one tenant-scoped adjustment through the authenticated
  API;
- compile the server-owned record-only instruction and verify retry stability;
- construct a legacy transaction with exactly the compiled instruction, sign
  it with a synthetic wallet key, and submit it through the production API;
- run two concurrent identical submissions and require one durable RPC send;
- configure the fixture to report finalized status, insert the deterministically
  derived synthetic typed-event projection into local indexer tables, and call
  the real reconciliation endpoint;
- require `finalized` plus the complete server-owned proof envelope, then
  replay read/reconcile calls with no second submission or duplicate event.

The fixture and SQL projection are explicitly synthetic evidence. They model
provider and indexer contracts without claiming validator execution, live
RPC-provider agreement, production wallet custody, or legal/tax compliance.
Those remain separate release gates.

## Invariants and budgets

- The transaction contains exactly one compiled `record_adjustment`
  instruction, with no compute-budget injection, lookup table, account
  substitution, or extra static account.
- The API verifies the signature, recent blockhash, program ID, account order,
  account privileges, instruction data, and intent digest before submission.
- Concurrent identical submissions produce at most one `sendTransaction` call
  and one submission lifecycle.
- Finalization requires the exact signature, slot, event index, event digest,
  adjustment PDA, receipt, merchant, mint, nonce, signed deltas, adjustment
  digest, evidence digest, and approver.
- The drill has bounded retries and a local 2-second provider deadline; it
  performs no unbounded polling or task creation.
- No private key, signed transaction bytes, raw tax evidence, or credential is
  persisted or emitted in logs.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Unit tests for transaction verification only | Fast and precise | Misses API persistence, RPC submission, concurrency, and finalized proof binding | Rejected as the sole gate |
| Manually seed a finalized intent and event row | Simple and repeatable | Bypasses wallet serialization, API submit, provider observation, and lifecycle fencing | Rejected |
| Full public-RPC and wallet-provider test in every CI run | Production-like | Requires credentials and external state; unsafe for public CI | Rejected for the deterministic gate |
| **Real API + PostgreSQL + synthetic finalized RPC/indexer contracts + real wallet transaction serialization** | Complete local authority-boundary evidence, reproducible and credential-free | Does not replace Linux validator, live-provider, KMS, or production wallet evidence | **Selected** |

## Verification and rollout

- Add the Node wallet harness and invoke it from `scripts/verify_local_stack.sh`.
- Extend the RPC fixture with an explicit finalized-status mode used only by
  the integration profile.
- Require the drill from `make integration-check` and retain its bounded output
  in CI logs without exposing secrets.
- Keep ADR-0170/0171 open until Linux validator execution, live wallet and
  provider evidence, load/SLO evidence, and production approval are complete.
