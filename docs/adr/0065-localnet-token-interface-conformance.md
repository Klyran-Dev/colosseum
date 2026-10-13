# ADR-0065: Validator-backed token-interface conformance

- Status: Accepted
- Date: 2026-09-26
- Owners: Settlement and protocol engineering

## Context

The settlement program accepts legacy SPL Token and extension-free Token-2022
through Anchor's token interface, but rejects unsupported account layouts and
extensions. Host unit tests prove the pure layout predicates; they do not prove
that the deployed program, CPI path, token program owners, account authorities,
receipt initialization, or final token balances agree on a live Solana runtime.

A Tax Engine settlement boundary needs executable evidence for both supported token
programs and for a real unsupported Token-2022 mint. The evidence must also
prove that duplicate receipt initialization cannot move funds a second time.

## Decision

Use an opt-in validator-backed conformance suite implemented in the existing
TypeScript test toolchain. The suite connects to the pinned Agave validator,
constructs real mints and token accounts, registers a governed policy
attestation, settles the exact gross/tax/net tuple, and reads finalized account
state. It runs only when `SOLANA_LOCALNET_TESTS=true`; the default unit and UI
test commands do not silently skip it as a release gate.

The suite covers:

1. legacy SPL Token settlement with exact source, tax-vault, and destination
   deltas;
2. extension-free Token-2022 settlement through the same instruction envelope;
3. duplicate nonce/receipt rejection with unchanged token balances; and
4. a real Token-2022 TransferFeeConfig mint rejected before route activation.

The validator is started from the pinned Compose profile and loads the SBF
artifact under the fixed program ID. The SBF artifact is built with the pinned
`cargo-build-sbf` version. Test keys and mints are generated per run and are
never reused outside localnet.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Pure helper/unit tests | Rejected as the only gate | Cannot prove CPI, account ownership, runtime serialization, or actual token deltas. |
| In-process `solana-program-test` only | Rejected as the release evidence | Useful for fast native tests, but does not validate the Dockerized Agave validator and external RPC boundary required by the local development contract. |
| External validator plus TypeScript client | Selected | Exercises the deployed artifact, exact client serialization, SPL/Token-2022 programs, PDA accounts, transaction confirmation, and observable RPC behavior. |
| Mocked token programs or fabricated account data | Rejected | Would permit false-positive compatibility claims and violate the no-demo-path mandate. |

## Invariants and failure behavior

- A successful supported-token case must conserve `gross = tax + net` and show
  exactly `tax` atomic units in the vault and `net` atomic units at the
  destination.
- A duplicate nonce must fail at receipt initialization and must not change any
  token balance.
- Unsupported Token-2022 extension state must fail closed before a route or
  policy becomes usable.
- Missing validator, missing SBF artifact, wrong program ID, RPC timeout, or
  incomplete account evidence fails the test command; it is never reported as
  a passing compatibility result.

## Consequences

This adds a Docker/validator dependency and increases the conformance runtime,
but it supplies evidence that unit tests cannot. The suite remains separate
from the low-latency production path and can run in CI after the pinned SBF
toolchain and validator are available. Until this suite and independent audit
evidence pass, the repository must not claim mainnet settlement readiness.
