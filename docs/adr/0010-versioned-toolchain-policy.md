# ADR-0010: Pin and govern the toolchain

- Status: Accepted
- Date: 2026-09-25

## Context

“Latest” dependencies can introduce incompatible Solana platform tools, compiler changes, frontend regressions, or security issues. Reproducibility is a production requirement.

## Decision

Pin Rust, Solana/Agave CLI, Anchor/AVM, Node, React, Solidity compiler, NGINX image, and all application dependencies. Commit lockfiles, use a CI image digest, generate an SBOM, and upgrade through a compatibility PR with full program, client, and integration tests. Track current versions in a dated document rather than floating a production tag.

## Consequences

- Upgrades are deliberate and reviewable.
- We must maintain compatibility matrices and periodically refresh dependencies.
- Security updates can bypass normal cadence through an emergency patch process with a recorded exception.

