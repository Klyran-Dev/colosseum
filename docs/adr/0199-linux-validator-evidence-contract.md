# ADR-0199: Require an authoritative Linux validator evidence contract

## Status

Accepted — the Linux CI/release gate now verifies the retained evidence bundle
after the validator-backed conformance command succeeds.

## Context

ADR-0085 and ADR-0168 retain useful validator, SBF, Compose, and test output,
but a successful process exit alone does not prove that the release evidence
came from the required Linux runtime, the pinned Agave binary, the exact SBF
artifact, or the complete real-RPC test matrix. The macOS Docker Desktop
environment also intentionally fails the pinned Agave 4.3.0 validator when
`io_uring` is unavailable. Local failure classification must remain useful;
it must not be converted into a skip or a pass.

## Decision

Keep `make solana-conformance` as the fail-closed execution command and add
`scripts/verify_solana_conformance_evidence.sh` as a postcondition for the
authoritative Ubuntu CI and release-candidate jobs. The verifier requires:

- the versioned `result.env` success contract (`completed`, status `0`, and
  failure reason `none`);
- a full source revision, Linux host, amd64 Docker engine, validator container
  identity, and pinned Compose image metadata;
- the Agave 4.3.0 startup log without an `io_uring NOT supported` diagnostic;
- an SBF SHA-256 matching the artifact built in the same checkout; and
- retained output for every named real-RPC case: legacy SPL, v0/ALT,
  extension-free Token-2022, Split Plan, TransferFeeConfig rejection, and
  policy revocation.

The verifier is intentionally not part of the ordinary local command. A local
run can retain typed infrastructure failure evidence, while only a successful
Linux run can satisfy the release evidence contract. The release bundle calls
the same verifier before packaging.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Trust the process exit code | No additional script | Allows incomplete or non-authoritative evidence to pass release review | Rejected |
| Treat local `io_uring` failure as skipped | Faster developer feedback | Hides an unproven validator gate and can produce false release confidence | Rejected |
| Require a privileged Docker workaround | May bypass a constrained local kernel | Expands host/container privilege and changes the validator execution environment | Rejected |
| **Verify a bounded success bundle on Linux CI/release** | Preserves local diagnostics, proves runtime/artifact/test identity, small reviewable contract | Requires a second explicit gate and retained logs | **Selected** |

## Invariants and failure behavior

1. Missing, malformed, contradictory, or incomplete success evidence fails the
   verifier and therefore fails the CI/release job.
2. A validator `io_uring` failure remains non-zero and is recorded as
   `validator_io_uring_unsupported` by the runner.
3. The verifier never starts a validator, changes a ledger, or mutates
   application state.
4. The retained SBF hash is bound to the artifact in the checkout that ran the
   validator; a client-supplied or copied hash is insufficient.
5. Test names are checked as evidence labels, while the underlying assertions
   remain the TypeScript transactions executed against the Agave RPC endpoint.

## Verification

- `sh -n scripts/verify_solana_conformance_evidence.sh`
- `make compose-config`
- the Ubuntu `solana-conformance` job runs the verifier after the real gate;
- the release-candidate job verifies the copied bundle before packaging; and
- a local Docker Desktop `io_uring` failure remains classified and non-zero.

## Consequences

Release review gains a deterministic, public-safe validator evidence contract.
The contract intentionally keeps the Linux gate open until a successful
Ubuntu-run artifact is retained; local macOS execution cannot satisfy it.
