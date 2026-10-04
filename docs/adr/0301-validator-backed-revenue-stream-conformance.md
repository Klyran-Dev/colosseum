# ADR-0301: Validator-backed revenue-stream movement conformance

## Status

Accepted — implement the real validator case before enabling revenue-stream
movement outside test environments.

## Context

The revenue-stream bounded context has domain, API, compiler, and on-chain
instruction coverage, but a release bundle must also prove the deployed SBF
artifact moves tokens through the revenue-stream plan, accrual, and claim
boundary. Unit tests and synthetic RPC fixtures cannot prove Anchor account
constraints, vault-authority CPI signing, PDA identity, token conservation, or
atomic replay behavior at the deployed program boundary.

## Decision

Extend the pinned Agave validator conformance suite with a real legacy-SPL
revenue-stream case. The case must:

1. settle two finalized source receipts into the configured tax vault;
2. initialize, approve, and activate one bounded stream plan;
3. record two evidence-bound accruals from those receipts;
4. create and execute one governed claim exactly once;
5. prove a duplicate execute fails without balance drift; and
6. cancel a second claim and prove execution after cancellation fails without
   balance drift; and
7. pause the plan and prove new claim creation is rejected without creating a
   claim account.

The conformance evidence must contain the stable case label
`revenue-stream single-use claim`. The test is opt-in through the existing
Dockerized validator runner and is a release evidence requirement, not a
substitute for Linux/amd64 execution, staging governance, independent audit,
or legal approval.

## Alternatives considered

| Alternative | Strengths | Risks | Decision |
| --- | --- | --- | --- |
| Rust/domain tests only | Fast and deterministic | Cannot prove deployed SBF account constraints or token CPI movement | Rejected |
| Synthetic RPC or program-test fixture | Easy to run in CI | Does not prove the pinned Agave runtime and real SPL token program boundary | Rejected |
| Manual operator walkthrough | Useful exploratory signal | Non-reproducible, incomplete, and not release-gated | Rejected |
| **Pinned Agave validator with real SPL transactions** | Proves the deployed artifact, PDA/account bindings, authority separation, conservation, replay, and cancellation behavior | Requires a real validator and retained evidence | **Selected** |

## Invariants and failure policy

- Accruals require an active plan, a matching finalized source receipt, a
  non-zero evidence digest, and a bounded cumulative amount.
- Claims bind to the plan, nonce, destination, amount, and claim evidence.
- A successful claim transfers only the reserved amount through the
  deterministic vault-authority PDA.
- Duplicate execution and execution after cancellation are rejected before
  token movement; source and destination balances remain unchanged.
- Plan pause rejects new accrual/claim work and the test verifies that a
  rejected claim does not create a PDA.
- The test fails if the validator, SBF artifact, or stable case evidence is
  unavailable. It must never downgrade to a mock or skipped release result.

## Performance and resource budget

The movement boundary remains outside the tax and quote hot path. Each claim
execution is one bounded legacy-SPL transaction with one vault-authority token
CPI, no provider or database call, and no user-controlled account collection.
The existing settlement wire-budget and compute-budget policies apply; the
release candidate must retain measured transaction size and compute-unit
evidence, with a hard packet ceiling of 1,232 bytes and a configured compute
limit no greater than 200,000 units. Accrual and claim preparation are
bounded asynchronous work with the existing deadline, queue, and retry
policies; unknown chain outcomes remain pending for reconciliation.

## Verification

- `apps/web/src/solana.localnet.test.ts` contains the real transaction case.
- `scripts/verify_solana_conformance_evidence.sh` requires its stable label.
- `scripts/release_bundle_integrity_contract.sh` protects the evidence schema.
- `make solana-conformance` remains the execution command; successful local
  execution is Docker evidence only, while the retained Linux/amd64 bundle is
  required for release.
