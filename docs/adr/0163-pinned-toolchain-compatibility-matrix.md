# ADR-0163: Pinned toolchain compatibility matrix and refresh policy

## Status

Accepted; baseline verified 2026-09-28.

## Context

The system spans a Rust/Anchor SBF program, Rust asynchronous services,
PostgreSQL 18, a React/Vite SPA, and an NGINX edge. “Latest” is not a
reproducible dependency constraint: a current release can be incompatible
with Agave platform-tools, the SBF compiler, wallet SDKs, browser tooling, or
the production support policy. Node 26 is currently Current, for example,
but Node 24 remains the selected LTS line for the build toolchain.

The repository already pins versions and image digests. It needs one explicit
matrix that explains why each version is selected, which compatibility edge
is authoritative, and how upgrades become reviewable changes.

## Decision

Maintain the following compatibility matrix as the repository baseline:

| Surface | Selected baseline | Compatibility boundary | Upgrade gate |
| --- | --- | --- | --- |
| Off-chain Rust | 1.98.1 stable | `rust-toolchain.toml`, workspace lockfile | Full Rust workspace format, clippy, tests, load, SBOM |
| Solana/Agave | 4.3.0 stable | Validator, RPC wire behavior, feature set | Linux validator conformance and retained evidence |
| SBF compiler | Agave platform-tools v1.57 | `programs/settlement` declares Rust 1.95 floor | Docker SBF build, fixed program hash, validator conformance |
| Anchor | CLI/AVM 1.2.0 baseline | Generated IDL, discriminators, account layout | IDL/vector compatibility and on-chain tests |
| Frontend runtime | React 19.3.0 | React + TypeScript + Vite SPA; no SSR | Typecheck, unit, axe, Playwright, bundle review |
| Frontend build runtime | Node.js 24.21.0 LTS | `packageManager` and committed `pnpm-lock.yaml` | Clean Docker build and browser journey suite |
| Optional EVM adapter | Solidity 0.8.37 | Separate package, chain, authority, and audit boundary | Adapter-specific audit and cross-chain threat review |
| Edge | NGINX 1.30.5 stable | Static SPA, security headers, reverse proxy | Image scan, config test, browser journey suite |
| Durable store | PostgreSQL 18.6 | Native `uuid`, `uuidv7()`, migration contract | Restore/PITR, migration drift, schema and load drills |

Exact container tags and immutable digests remain authoritative for Docker;
this ADR does not authorize a floating `latest`, `stable`, `current`, or
unbounded semver range in production or CI.

The selected baseline is refreshed by a compatibility change that must:

1. record the upstream release date and security/support rationale;
2. update the matrix, lockfiles, image digests, SBOM inputs, and compatibility
   notes together;
3. run format, lint, unit, property, integration, browser, security,
   reproducible-build, and relevant load gates;
4. rebuild the SBF artifact and compare its program/configuration hashes;
5. execute Linux validator conformance before release approval; and
6. retain the evidence bundle and a rollback reference to the prior matrix.

Security-only patches may use an expedited change, but they must preserve the
same pinned major/runtime boundary and receive a follow-up compatibility
review if the patch changes behavior.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Floating `latest` tags and semver ranges | Minimal maintenance | Non-reproducible builds, silent compiler/ABI drift, unsafe rollback | Rejected |
| Always select the newest Current runtime | Earlier features | Current releases may not have the required support horizon or ecosystem compatibility | Rejected for production runtimes |
| Let each service upgrade independently | Smaller local changes | Breaks shared IDL, serialization, SBF, browser, and migration compatibility | Rejected |
| **Versioned matrix with exact pins, digests, and evidence-gated refreshes** | Reproducible, reviewable, supportable, and rollback-safe | Requires scheduled maintenance work and retained evidence | **Selected** |

## Consequences

- The repository can distinguish “newest upstream release” from “approved
  production baseline.”
- A toolchain upgrade is a compatibility event, not a dependency-only edit.
- The SBF compiler’s older Rust floor remains explicit and isolated from the
  modern off-chain toolchain.
- Docker image refreshes require digest changes, vulnerability scans, and
  reproducibility evidence.
- The baseline must be revalidated before every release candidate because
  upstream support and security status can change.

## References

- [Rust Forge release versions](https://forge.rust-lang.org/)
- [React versions](https://react.dev/versions)
- [Node.js release schedule](https://nodejs.org/en/about/previous-releases)
- [NGINX stable downloads](https://nginx.org/en/download.html)
- [PostgreSQL 18.6 release notes](https://www.postgresql.org/docs/release/18.6/)
- [Agave v4.3 release schedule](https://github.com/anza-xyz/agave/wiki/v4.3-Release-Schedule)
- [Solana compute budget and fee rules](https://solana.com/docs/core/fees/compute-budget)
