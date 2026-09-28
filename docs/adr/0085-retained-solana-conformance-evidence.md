# ADR-0085: Retain validator-backed Solana conformance evidence

- Status: Accepted
- Date: 2026-09-26
- Owners: Protocol/SRE
- Scope: SBF build, Agave validator, settlement integration tests, and CI release evidence

## Context

ADR-0065 established a real Dockerized Agave validator gate. The existing
command printed an artifact hash and test output, but CI retained only the
job log. A production-grade protocol needs a reviewable evidence bundle that
answers which source revision, toolchain, container, program binary, validator
configuration, and transaction tests produced a result. Failure evidence is
equally important because the local macOS Docker runtime cannot currently
start the pinned Agave binary when `io_uring` is unavailable.

## Decision

`scripts/run_solana_conformance.sh` writes a bounded, public-safe evidence
bundle to `test-results/solana-conformance/`:

- execution metadata: commit, UTC timestamp, host/runtime, Compose versions,
  selected platform, and relevant pinned image digests;
- SHA-256 of `target/deploy/settlement.so`;
- Compose validator configuration and validator logs;
- the complete bounded browser/localnet test output;
- a machine-readable result file with `passed` or `failed` and the failure
  phase.

The trap collects logs before tearing down the isolated Compose project and
preserves the original exit code. The GitHub Actions conformance job uploads
the evidence bundle with a bounded retention period on both success and
failure. The bundle contains only synthetic localnet data; it must not contain
wallet secrets, credentials, or private jurisdiction evidence.

This evidence is necessary but not sufficient for production enablement. A
successful CI run proves the pinned validator and test cases executed in that
runner. It does not prove mainnet safety, an independent program audit,
production RPC behavior, or key governance.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Retain only the CI console log | Rejected | Omits deterministic metadata, artifact hash, and structured failure context. |
| Upload the complete workspace or ledger | Rejected | Excessive, potentially sensitive, non-reproducible, and may include generated secrets. |
| Store only a boolean status | Rejected | Cannot support forensic diagnosis or release review. |
| **Bounded public-safe evidence bundle with success/failure upload** | **Selected** | Reproducible, reviewable, minimal, and useful when the validator cannot start. |

## Invariants and failure behavior

1. The conformance command remains fail-closed; evidence capture never converts
   a failed build, validator, or test into success.
2. Cleanup is attempted after logs are captured and cannot mask the original
   failure code.
3. Evidence paths are bounded and ignored by Git.
4. Artifact hashes are computed locally from the exact binary loaded by the
   validator.
5. Upload retention is finite and documented; release records must preserve
   an approved artifact hash separately when required by the release policy.
6. No private key, token, raw sensitive tax evidence, or production endpoint is
   written to the bundle.

## Evidence

- `scripts/run_solana_conformance.sh`
- `.github/workflows/ci.yml`
- `docs/plans/production-readiness.md`
