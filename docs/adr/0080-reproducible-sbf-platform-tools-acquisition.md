# ADR-0080: Reproducible SBF platform-tools acquisition

- Status: Accepted
- Date: 2026-09-26
- Owners: Solana Runtime / Build / Release Engineering
- Scope: Dockerized `cargo-build-sbf` platform-tools acquisition

## Context

ADR-0067 pins the SBF compiler boundary to `cargo-build-sbf` 4.3.0 and
platform-tools v1.57. The default downloader uses a GitHub API/browser path
that can fail in restricted CI or Docker Desktop environments even when the
same release asset is reachable through a normal browser redirect. A build
must not work around that failure by accepting an unverified or floating
compiler artifact.

## Decision

`scripts/build_sbf.sh` keeps the pinned Dockerized builder and adds a bounded
fallback for the Docker path:

1. Download the exact `platform-tools-linux-x86_64.tar.bz2` v1.57 browser
   asset with `curl`, or accept an operator-supplied local archive through
   `SOLANA_PLATFORM_TOOLS_ARCHIVE`.
2. Verify the archive against the pinned SHA-256
   `b0f7af104adf726fff2a6a09ea2eb2f2d2965c92295f4d7388c08d140e0c2b00`.
3. Extract it into an ignored, reusable cache at
   `target/solana-platform-tools-cache/v1.57/platform-tools` (override with
   `SOLANA_PLATFORM_TOOLS_CACHE_DIR`).
4. Mount that exact cache read-only into the non-root builder and invoke
   `cargo-build-sbf --skip-tools-install`.

The cache is never committed, its digest is checked on every acquisition, and
the normal build still fails closed if the pinned asset cannot be obtained or
verified. The validator remains a separate gate; a successful SBF build does
not imply validator startup or transaction conformance.

CI sets `SOLANA_USE_DOCKER_BUILDER=true` so the release evidence uses the same
pinned builder and verified acquisition path as Docker-local development. The
conformance harness prints the resulting `settlement.so` SHA-256 before
starting the validator.

## Alternatives considered

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Default cargo-build-sbf downloader only | Minimal script and upstream behavior | Fails in environments where the API/asset client path is restricted; no deterministic recovery | Rejected as the only path |
| Floating latest platform-tools | Easy acquisition | Compiler drift and unreproducible program bytes | Rejected |
| Check the 500+ MB compiler archive into Git | Works offline | Unacceptable repository size, provenance and maintenance burden | Rejected |
| **Pinned browser asset with SHA-256 and ignored reusable cache** | Reproducible, auditable, works with local mirrors/archives, no runtime trust expansion | Requires initial download and periodic pin review | **Selected** |

## Verification

- archive metadata and SHA-256 are checked before extraction;
- the Docker builder consumes the cache read-only as the unprivileged
  `sbf-builder` user;
- the resulting `settlement.so` is hashed and recorded by the conformance
  workflow;
- validator startup and live transaction tests remain required evidence and
  are not marked complete by compiler success.
