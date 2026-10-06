# ADR-0377: Dedicated Solana conformance evidence workflow

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal Low-latency Blockchain Architecture
- Scope: Linux/amd64 validator evidence production and release review

## Context

The settlement program has a broad validator-backed conformance suite, but
authoritative evidence is an external execution artifact. The normal CI job
is useful for regression detection and the release-candidate workflow combines
conformance with a large evidence bundle. Neither path is an efficient,
explicit handoff for the blockchain owner who needs to run the exact commit on
a native Linux/amd64 runner and retain the validator artifact.

The repository must make the evidence path reproducible without weakening the
release gates, substituting mocked RPC behavior, or treating a workflow
definition as proof that a run succeeded.

## Decision

Add `.github/workflows/solana-conformance.yml`, triggered manually or by a
version tag. It is a narrow evidence-producing workflow with a pinned Ubuntu
24.04 runner, pinned checkout and artifact-upload actions, explicit Linux and
amd64 Docker assertions, the shared host preflight, the Dockerized SBF build,
the real local-validator suite, postcondition verification, and 30-day
artifact retention.

The artifact name includes both the workflow run ID and source SHA. The
retained bundle must include the versioned host preflight, exact source
revision, pinned validator metadata, SBF hash, container identity, validator
log, result state, Compose configuration, and all named real-RPC test cases.
The workflow can fail or produce no artifact; neither a green YAML contract
nor a manually uploaded file may be used as evidence without the verifier's
successful result.

The existing CI and release-candidate conformance jobs remain in place. This
workflow is an additional explicit operator path, not a replacement or a
second settlement authority.

## Alternatives considered

1. Keep conformance only in the broad CI workflow. Rejected: exact-commit
   evidence remains coupled to every unrelated PR gate and is harder to rerun
   for an evidence owner.
2. Run conformance only inside the release-candidate bundle. Rejected: it
   couples validator evidence to long-running provider, backup, and UI gates
   and slows targeted diagnosis.
3. Add an unpinned third-party validator action or hosted RPC. Rejected:
   execution provenance, image policy, and source-of-truth boundaries become
   unclear.
4. **Use a dedicated pinned workflow that invokes the existing Dockerized
   suite and evidence verifier.** Selected: fast operational handoff while
   preserving exactly the same validator, account, finality, and evidence
   contracts.

## Invariants and verification

- Only native Linux/amd64 execution can pass the host preflight.
- The workflow runs the repository's `make solana-conformance` target and
  verifies its retained evidence before artifact upload.
- The verifier must prove the exact source revision, Agave version, SBF hash,
  validator identity, no `io_uring` failure, and every named real-RPC case.
- The workflow has read-only repository permissions and no wallet, secret,
  signer, provider, or mainnet authority.
- A workflow contract checks runner shape, preflight ordering, command,
  postcondition, and fail-closed artifact retention.

## Consequences

Blockchain and release owners can run the conformance suite directly for a
tagged or reviewed commit and attach the resulting evidence to a release
review. The repository now has a clear path to close its Linux validator gate,
but the gate remains open until a successful artifact is actually produced and
independently reviewed.
