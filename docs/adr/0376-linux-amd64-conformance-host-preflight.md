# ADR-0376: Linux/amd64 conformance host preflight

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal Low-latency Blockchain Architecture
- Scope: Validator-backed Solana conformance and release evidence

## Context

Agave 4.3.0 validator evidence is authoritative only when the validator runs
on a Linux/amd64 execution environment with the required kernel capabilities.
Docker Desktop on an ARM developer host can build or select an amd64 image but
still expose an ARM or incompatible kernel to the validator. The observed
`io_uring NOT supported` failure is therefore a real environment limitation,
not evidence that the settlement program or conformance cases passed.

The existing CI and release workflows assert the runner shape inline, but a
local run discovers the same incompatibility only after image build and
validator startup. Evidence also needs a machine-readable record of the
preflight decision so a retained bundle can distinguish an authoritative run
from a diagnostic attempt.

## Decision

Add `scripts/verify_solana_conformance_host.sh` as the shared, fail-closed
preflight for local, CI, and release-candidate execution. It records
`preflight.txt` using the versioned
`solana-tax-protocol/conformance-host/v1` schema, including host OS and
architecture, Docker OS and server architecture, Docker version, status, and
failure reason.

Authoritative execution requires all of the following before the expensive SBF
build and validator startup:

- native Linux host;
- x86_64/amd64 host architecture;
- Linux Docker engine;
- amd64/x86_64 Docker server architecture.

The preflight is a capability gate, not a substitute for validator evidence.
Validator startup remains the authority for kernel capability such as
`io_uring`; `validator.log`, the completed result, exact SBF hash, container
identity, and named real-RPC cases remain mandatory. The evidence verifier
requires a successful preflight and rejects ARM emulation, failed preflight,
or incomplete metadata. Local ARM failures remain useful diagnostics but can
never close the release gate.

## Alternatives considered

1. Keep inline YAML checks only. Rejected: local and CI paths would have
   different failure timing and no shared retained preflight evidence.
2. Permit Docker amd64 emulation on ARM hosts. Rejected: image architecture
   does not prove kernel capability or validator correctness; it conflicts
   with the observed `io_uring` failure.
3. Probe `io_uring` with an ad-hoc host command. Rejected: a partial probe can
   disagree with the actual validator container and would create a second
   capability authority.
4. Use a native Linux/amd64 preflight plus the existing validator startup and
   evidence verifier. Selected: early, actionable feedback with no weakened
   validator or settlement proof.

## Invariants and verification

- A conformance attempt always records one versioned `preflight.txt`.
- A successful evidence bundle has `status=ready` and `failure_reason=none`.
- Host and Docker architecture must both be amd64-compatible; emulation is not
  authoritative.
- The conformance result remains incomplete until validator startup and all
  named real-RPC cases finish successfully.
- CI and release workflows run the same preflight before `make
  solana-conformance`; the workflow contract tests this ordering.
- The preflight has no network, wallet, database, or settlement authority and
  adds negligible work to the conformance path.

## Consequences

Developers on the current ARM Docker host receive a fast, explicit failure and
can hand the recorded diagnostic to the Linux runner owner. CI and release
artifacts become easier to review because the execution topology is explicit.
The Linux/amd64 validator gate remains open until a successful artifact is
retained for the exact release commit; this ADR does not claim that external
evidence has been produced.
