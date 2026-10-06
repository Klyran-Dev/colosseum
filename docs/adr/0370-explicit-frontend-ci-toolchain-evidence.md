# ADR-0370: Explicit frontend CI toolchain evidence

## Status

Accepted; implemented 2026-10-05.

## Context

The frontend workflows already selected Node.js 24.21.0 and pnpm 12.3.4, but
selection alone does not prove which executables ran. Corepack and hosted
runners are mutable boundaries, so a release log must contain the resolved
Node.js and pnpm versions next to the locked install and build evidence.

## Decision

After the pinned Node setup and pnpm activation, the web CI and release-
candidate workflows must execute `node --version` and `pnpm --version` before
installing dependencies. The existing static `ci-toolchain-contract` verifies
the exact Node, pnpm, and version-evidence commands in both workflows.

This is evidence and drift detection; it does not make the browser runtime an
authority for tax, authorization, or settlement state. Docker image pins and
lockfiles remain authoritative for reproducible builds.

## Alternatives considered

| Alternative | Version visibility | Reproducibility | Decision |
| --- | --- | --- | --- |
| Trust setup-action and Corepack defaults | Fail | Partial | Rejected |
| Infer versions from install output | Partial | Partial | Rejected; output is incidental and easy to remove |
| Use a floating package-manager channel | Fail | Fail | Rejected |
| **Pin versions and print explicit executable evidence under contract** | **Pass** | **Pass** | **Selected** |

## Invariants and verification

- CI and release workflows select Node.js 24.21.0 and pnpm 12.3.4.
- Resolved `node --version` and `pnpm --version` output precedes dependency
  installation.
- `make ci-toolchain-contract` fails closed when selection or evidence drifts.
- The frontend remains a static React/Vite SPA served through NGINX.

## Consequences

The workflow logs gain two bounded, non-sensitive version lines. A future
upgrade must update the package manager, lockfile, image/runtime matrix, ADR,
and compatibility evidence together.
