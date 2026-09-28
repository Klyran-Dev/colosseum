# ADR-0110: Property-based adversarial account-meta gate

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Blockchain Architecture and Settlement
- Scope: signed-message verification, settlement-client, CI release gates

## Context

The settlement coordinator is the final off-chain authority before a wallet-
signed Solana message reaches an RPC provider. Example-based tests prove that
known substitutions are rejected, but account-meta attacks are a combinatorial
boundary: a message may substitute a public key, reorder an account, or change
signer/writable privileges while remaining a syntactically valid, correctly
signed transaction.

The repository's production contract requires adversarial account-meta tests
on every program change. A validator run is necessary for on-chain execution,
but it is not a substitute for deterministic pre-submit verification. The
test gate must run on stable Rust, remain bounded, produce reproducible failure
cases, and exercise actual signed transaction bytes rather than only mutating
the expected JSON artifact.

## Decision

Use bounded `proptest` cases in the settlement coordinator as the stable-Rust
fuzzing/property gate for signed account topology:

1. Compile one canonical settlement instruction.
2. Construct a real, payer-signed Solana transaction from that artifact.
3. Generate every instruction-account position and replace its public key with
   a distinct adversarial key, then sign the altered message with the payer.
4. Require `verify_settlement_instruction` to reject every generated message
   before any provider call or durable lifecycle transition.
5. Keep the existing deterministic tests for account order, program ID,
   instruction data, signatures, trailing bytes, and address-table rejection.

The property suite is bounded to the compiled account count and uses a fixed
case budget suitable for CI. `proptest` shrinking retains the smallest failing
position and transaction mutation in the test output. The Rust CI job already
runs all workspace targets for every pull request and push; a dedicated
`program-adversarial` job will make the release gate visible and ensure future
workflow changes cannot silently omit it.

This gate covers the off-chain signed-message boundary. It does not replace
validator-backed CPI, Token-2022 extension, rollback, compute-budget, or
program-audit evidence.

## Alternatives considered

### Example-only tests

Rejected. They provide useful regression fixtures but do not explore account
positions and mutation shapes systematically.

### `cargo-fuzz` with a nightly compiler

Deferred. Coverage-guided fuzzing remains valuable, but introducing nightly
toolchain and libFuzzer build requirements into the mandatory production gate
would violate the stable-toolchain baseline and reduce clean-checkout
reproducibility. It may be added as a non-blocking scheduled job after a
separate toolchain ADR.

### Mutate only the serialized compiler artifact

Rejected. That tests digest sensitivity, not whether the verifier rejects an
actual validly signed transaction whose executable account topology was
substituted.

### Trust Solana runtime account checks alone

Rejected. Runtime checks occur after submission and cannot prevent an API or
RPC boundary from accepting and broadcasting an intent-mismatched message.

## Invariants

- A valid signature does not authorize an account-meta substitution.
- Every executable instruction account must match the compiled public key,
  order, signer flag, and writable flag.
- No generated adversarial message reaches the RPC transport.
- The test case count and transaction size remain bounded.
- Production code remains free of private keys; test keypairs are synthetic.

## Verification and release impact

- `cargo test --locked -p settlement-coordinator --all-targets` passes the
  property suite.
- The CI `program-adversarial` job runs on every change and failures block
  merge.
- The readiness item for adversarial account-meta tests may be marked complete
  only after the CI workflow and local test evidence both pass.
- Localnet and staging still must prove the on-chain atomicity and Token-2022
  portions independently.

