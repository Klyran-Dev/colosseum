# ADR-0067: Isolate the SBF compiler compatibility boundary

- Status: Accepted
- Date: 2026-09-26
- Scope: `programs/settlement` compiler metadata and Dockerized SBF builds

## Context

The workspace baseline is Rust 1.98.1. The pinned Agave/Solana 4.3.0 build
toolchain uses platform-tools v1.57, whose SBF compiler reports
`rustc 1.95.0-dev`. Cargo therefore rejects the on-chain program when it
inherits the workspace `rust-version = "1.98.1"` declaration.

The compatibility issue is a toolchain boundary, not a reason to bypass Cargo
version checks or to lower the version contract of every off-chain service.

## Decision

Keep Rust 1.98.1 as the workspace and off-chain baseline. Declare
`rust-version = "1.95"` only in `programs/settlement/Cargo.toml`, matching the
minimum language level accepted by the pinned SBF compiler. The Docker build
must use the exact Agave-compatible platform-tools archive,
`cargo-build-sbf --skip-tools-install` only when that archive is already
present at the expected cache path, and the exact pinned lockfile.

The program may not use language or standard-library features unavailable to
the pinned SBF compiler. A platform-tools or Agave upgrade requires a
compatibility change, SBF build, localnet conformance run, and ADR review
before changing this declaration.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Pass `--ignore-rust-version` | Rejected | Bypasses an explicit compiler safety contract and can permit unsupported language or dependency assumptions. |
| Lower the workspace floor to Rust 1.95 | Rejected | Unnecessarily constrains API, tax-engine, storage, and worker crates and obscures the different compiler requirements. |
| Install a newer SBF compiler independently | Rejected | Breaks the pinned Agave/platform-tools compatibility contract and weakens reproducibility. |
| Keep the program at Rust 1.98.1 | Rejected | Cannot compile with the selected Agave 4.3.0 / platform-tools v1.57 release. |
| **Override only the on-chain package floor to Rust 1.95** | **Selected** | Preserves the modern off-chain baseline while making the on-chain compiler constraint explicit and reviewable. |

## Consequences

- The SBF program and off-chain services have separate compiler compatibility
  gates, both pinned and documented.
- New program dependencies must be checked against the SBF compiler before
  they are accepted.
- The on-chain package cannot silently consume Rust 1.98-only features.
- The localnet conformance gate remains required; a successful host compile is
  not evidence of validator-compatible execution.

## Verification

The required evidence is:

1. Dockerized `cargo-build-sbf` succeeds with the pinned platform-tools.
2. The produced artifact is loaded under the fixed program ID.
3. `make solana-conformance` passes real legacy SPL, extension-free
   Token-2022, duplicate-nonce, and unsupported-extension cases.
4. The artifact hash and toolchain versions are recorded by the release build.
