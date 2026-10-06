# ADR-0369: Explicit CI toolchain bootstrap

## Status

Accepted; implemented 2026-10-05.

## Context

The repository pins the off-chain Rust toolchain in `rust-toolchain.toml`, but
GitHub-hosted runners are mutable environments. Relying on runner preinstalls
or implicit `rustup` discovery makes the first Cargo command responsible for
installing the compiler and does not make the selected version visible in every
workflow log. That weakens reproducibility and can cause formatting, lint, or
release artifacts to differ from the approved matrix.

## Decision

Before any host-side Cargo command, the Rust CI and release-candidate workflows
must explicitly install Rust `1.98.1` with the minimal profile and the pinned
`rustfmt` and `clippy` components, export `RUSTUP_TOOLCHAIN=1.98.1` through
`GITHUB_ENV`, and print the selected `rustc` and Cargo versions.

The contract applies to the Rust workspace, the adversarial settlement job, and
the release-candidate workflow. Docker-only jobs remain responsible for their
own pinned image/toolchain contracts and do not inherit the host runner's Rust
installation.

`scripts/verify_ci_toolchain_contract.sh` is the failing-closed static guard;
`make ci-toolchain-contract` is included in `make check` and the public CI
workflow. The guard validates the exact install command and environment export
rather than trusting a human-readable workflow label.

## Alternatives considered

| Alternative | Reproducibility | Operational cost | Decision |
| --- | --- | --- | --- |
| Trust the runner's preinstalled Rust | Fail | Low | Rejected |
| Rely only on implicit `rust-toolchain.toml` discovery | Partial | Low | Rejected; selection is less visible and less explicit |
| Use a floating `stable` toolchain install | Fail | Low | Rejected |
| Use a third-party setup action without an additional repository contract | Partial | Medium | Rejected; adds supply-chain and review surface |
| **Explicit pinned `rustup` install plus executable workflow contract** | **Pass** | **Bounded install time and cache cost** | **Selected** |

## Invariants and verification

- Host-side CI and release Cargo commands run with `RUSTUP_TOOLCHAIN=1.98.1`.
- `rustfmt` and `clippy` are installed from the same exact toolchain.
- Workflow logs contain the selected compiler and Cargo versions.
- A missing or changed bootstrap command fails `make check` before a release
  artifact can be treated as reproducible.
- Docker toolchains remain independently pinned and are not replaced by host
  toolchain state.

## Consequences

CI spends a bounded amount of time installing the exact compiler on fresh
runners, but release evidence is no longer dependent on mutable runner state.
The workflow is intentionally explicit and can later add a cache only if the
cache key includes the exact toolchain, lockfiles, and runner architecture.
