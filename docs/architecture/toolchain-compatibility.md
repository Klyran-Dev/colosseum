# Toolchain compatibility matrix

This is the operational companion to [ADR-0163](../adr/0163-pinned-toolchain-compatibility-matrix.md).
It records the approved production baseline, not a claim that every component
is production-ready. Release candidates must use the exact lockfiles, image
digests, and compiler boundaries below.

## Approved baseline

| Component | Version | Source of truth | Evidence required before release |
| --- | --- | --- | --- |
| Off-chain Rust | 1.98.1 | `rust-toolchain.toml` | `cargo fmt`, clippy, tests, benchmarks, SBOM |
| Agave/Solana | 4.3.0 | validator and SBF Dockerfiles | Linux validator conformance |
| SBF platform-tools | v1.57 | SBF builder cache/archive | Rebuilt program hash and conformance |
| Anchor/AVM | 1.2.0 baseline | `Anchor.toml`, CI bootstrap | IDL and account-layout vectors |
| React | 19.3.0 | `apps/web/package.json` and lockfile | Typecheck, unit, axe, Playwright |
| Node.js | 24.21.0 LTS | Docker web images and CI | Clean Docker build |
| pnpm | 12.3.4 | `apps/web/package.json` `packageManager` and CI | Frozen-lockfile install |
| PostgreSQL | 18.6 | `compose.yaml` image digest | Migration, backup/restore, PITR, index drills |
| NGINX | 1.30.5 stable | `ops/docker/Dockerfile.web` | Config, image, header, and browser checks |
| Solidity | 0.8.37, optional | EVM adapter only | Separate audit and chain-specific conformance |

Node 26 is an upstream Current release, but Node 24 is intentionally selected
for production build reproducibility because the project policy prefers an
active or maintenance LTS runtime. A future Node LTS adoption requires the
same matrix and evidence-gated refresh process; it is not an automatic version
change.

## Upgrade checklist

Every version refresh must include:

- the upstream release and support/security rationale;
- exact tag, digest, lockfile, and SBOM updates;
- the compatibility impact on SBF, IDL, wire serialization, migrations, and
  browser behavior;
- clean-checkout Docker bootstrap and the complete relevant test flow;
- Linux validator evidence for program or Solana SDK changes; and
- a rollback reference and updated release notes.

The macOS Docker Desktop `io_uring` limitation is an environment limitation,
not conformance evidence. Linux CI or an equivalent Linux runner remains the
authoritative validator gate for Agave 4.3.0.
