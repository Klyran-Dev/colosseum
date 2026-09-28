# ADR-0129: Release artifact provenance and dependency automation

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Security, Release Engineering
- Scope: public repository release candidates and dependency maintenance

## Context

The repository has pinned toolchains, lockfiles, Docker image digests, CI
checks, and a public-release contract. A production-grade public repository
also needs a repeatable release-candidate boundary that records exactly what
was built, produces checksums and an SBOM, and preserves validator and security
evidence. Dependency maintenance must be automated without allowing an update
bot to bypass the design-first and security review mandates.

The repository is not licensed for general reuse and is not yet a production
settlement release. Release automation must therefore validate and retain
evidence; it must not silently publish a public package, deploy to mainnet, or
choose a license on behalf of the repository owner.

## Decision

Add two release-integrity controls:

1. Dependabot monitors Cargo, pnpm, Docker, and GitHub Actions dependencies
   with weekly grouped maintenance updates. Updates still pass the normal CI,
   security, compatibility, and ADR review gates.
2. A tag/manual GitHub Actions workflow runs the repository checks on a pinned
   Ubuntu runner, builds the locked Rust release binaries, web bundle, and SBF
   artifact, then creates a bounded release-candidate directory containing:
   source revision, tag/ref, toolchain versions, build timestamp policy,
   artifact paths, SHA-256 checksums, SBOM, Compose image metadata, security
   scan output, and validator-conformance evidence. The bundle is uploaded as
   a retained workflow artifact and is not a production deployment.

The local `scripts/create_release_bundle.sh` script owns manifest creation and
rejects missing or unexpected artifact paths. It uses `SOURCE_DATE_EPOCH`
derived from the source revision, records the actual build environment, and
never includes credentials, private keys, `.env` files, raw jurisdiction
evidence, or unredacted runtime logs. A release candidate is not considered
provenance-complete unless the workflow has successfully run the validator,
security, test, and public-documentation gates.

## Alternatives considered

### Manual release notes and ad-hoc binaries

Rejected. They are not reproducible, do not bind artifacts to a source
revision, and create an unsafe gap between the tested and distributed bytes.

### Publish Docker images or deploy from the tag workflow

Rejected. The repository has no approved registry, deployment identity, key
ceremony, or license decision. Release evidence must be separated from
deployment authority.

### Floating “latest” actions, images, or dependencies

Rejected. Floating inputs undermine repeatability and violate the repository
pinning policy.

### Dependabot without grouped review policy

Rejected. Unbounded update PRs create review noise and can obscure correlated
toolchain changes. Grouped updates remain subject to CI and explicit
compatibility review.

### Selected: evidence-producing release candidate workflow

Selected because it binds source, exact build inputs, artifacts, checksums,
SBOM, security results, and validator evidence while preserving the separation
between public source release, production enablement, and deployment authority.

## Invariants and failure behavior

- A release bundle names one immutable source revision and never derives its
  identity from a caller-provided version string alone.
- Missing SBF, web, Rust, checksum, SBOM, security, or validator evidence fails
  the workflow; no partial bundle is uploaded as complete.
- Artifact checksums are generated after the final copy and are verified before
  manifest publication.
- Release automation never handles wallet keys, signing keys, credentials, or
  production deployment tokens.
- Dependency updates cannot merge without the same required CI and security
  checks as ordinary changes.
- The workflow does not create a license, alter readiness claims, or imply
  legal/tax compliance.

## Evidence and remaining gates

The workflow YAML, Dependabot configuration, release-bundle script, and local
contract tests are repository evidence. A successful tag-run artifact,
independent release review, license decision, validator evidence, program
audit, and production deployment ceremony remain separate release gates.
