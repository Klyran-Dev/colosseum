# Toolchain compatibility matrix

This is the operational companion to [ADR-0163](../adr/0163-pinned-toolchain-compatibility-matrix.md).
It records the approved production baseline, not a claim that every component
is production-ready. Release candidates must use the exact lockfiles, image
digests, and compiler boundaries below.

The baseline was revalidated on 2026-10-06 against the upstream release
channels. “Latest” is interpreted as the newest security-supported stable or
LTS release compatible with the Solana/SBF boundary, not an unreleased,
experimental, or merely newer Current channel. This is a compatibility
decision, not a claim that every upstream release is automatically suitable
for production.

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

## 2026-10-06 revalidation outcome

- Rust 1.98.1 remains the selected stable release and remains compatible with
  the off-chain workspace boundary; the newer beta channel is not a production
  candidate.
- React 19.3 remains the latest React line; the exact `19.3.0` package and
  lockfile pins remain unchanged.
- Node 24 remains the selected LTS build runtime. Node 26 is newer, but is the
  Current line rather than the approved LTS boundary.
- PostgreSQL 18.6 remains the current minor release for the 18 major and is
  retained for UUIDv7, migration, and evidence-store compatibility.
- Solidity 0.8.37 remains the optional EVM adapter compiler boundary.
- Anchor 1.2.0 remains the selected compatible baseline; the official
  changelog records its release and explicit platform-tools selection controls.
- Agave 4.3.0 remains the stable channel. Agave 4.4 and 4.5 entries are
  prerelease channels and are not eligible for this production baseline.
- NGINX 1.30.5 remains the repository’s exact stable image pin. NGINX 1.31 is
  the newer mainline line, but is not selected for the stable production
  baseline. The image digest and configuration checks remain mandatory release
  evidence.

No version change is authorized by this refresh. Any upgrade must follow the
compatibility-change process in ADR-0163 and retain rollback evidence.

The official release checks used for this refresh are [Rust stable
releases](https://blog.rust-lang.org/releases/), [React 19.3 release
notes](https://react.dev/blog/2026/09/09/react-19-3) and [React
versions](https://react.dev/versions), [Node.js release
schedule](https://nodejs.org/en/about/previous-releases), [PostgreSQL 18.6
release notes](https://www.postgresql.org/docs/release/18.6/), [Agave release
channels](https://github.com/anza-xyz/agave-runtime/blob/master/CHANGELOG.md),
[Anchor changelog](https://www.anchor-lang.com/docs/updates/changelog),
[Solidity 0.8.37 release notes](https://www.soliditylang.org/blog/2026/09/10/solidity-0.8.37-release-announcement/),
and [NGINX stable downloads](https://nginx.org/en/download.html).

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
