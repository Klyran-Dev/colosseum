# Public repository release contract

This repository is currently an implementation foundation, not a production
settlement release. The public documentation must remain truthful while the
remaining Solana, security, operations, legal, performance, and accessibility
gates in the [production-readiness plan](plans/production-readiness.md) are
open.

## Required before public source release

- An explicit license decision is approved by the repository owner and the
  selected `LICENSE` or `LICENSE.md` file is committed. Until then, the source
  must not be represented as licensed for general reuse.
- The root [README](../README.md), [AGENTS.md](../AGENTS.md),
  [contribution guide](../CONTRIBUTING.md), [security policy](../SECURITY.md),
  [code of conduct](../CODE_OF_CONDUCT.md), and [changelog](../CHANGELOG.md)
  describe the same implementation status and limitations.
- The [ADR index](adr/README.md), [API contract](api-contract.md),
  [tax-coverage matrix](product/tax-coverage-matrix.md),
  [implementation plan](plans/implementation-plan.md), and
  [production-readiness gates](plans/production-readiness.md) are linked from
  the public entry points and contain no unsupported production claims.
- The public runbook index includes the Tax Engine mTLS certificate-rotation
  procedure, including fail-closed rollback and evidence-redaction rules.
- The public runbook index includes the Tax Engine service-token rotation
  procedure with atomic file delivery, overlap, revocation, and secret-redaction
  requirements.
- The deployment preflight contract accepts only the API's documented
  structured or compatibility JWKS and jurisdiction-provider representations
  and rejects local identity/provider material; provider rotation and HSM/KMS
  ceremony evidence remain deployment gates.
- The PostgreSQL schema contract is catalog-driven for all `uuidv7()` defaults;
  migrations `0058`/`0059` and `db/verification/uuidv7_contract.sql` remain
  public so clean-bootstrap, upgraded-schema, and lease-trigger checks cannot
  drift from the UUIDv7 and reconciliation mandates.
- CI records the exact toolchain, dependency lockfiles, container image
  digests, security-scan result, test result, and validator-conformance result
  for every release candidate. ADR-0129 provides the tag/manual workflow,
  grouped dependency automation, checksums, SBOM, retained evidence bundle, and
  ADR-0218's explicit final-program and resolved-configuration hashes;
  the workflow does not publish images or deploy to production.
- A maintainer has reviewed public examples, fixtures, logs, benchmark output,
  and test artifacts for credentials, private keys, PII, and sensitive
  jurisdiction evidence.

## Required before production settlement enablement

The repository must not be described as production-ready until every relevant
unchecked item in the readiness plan has attached evidence. In particular,
architecture prose, unit tests, simulated API responses, and a successful
Docker build do not substitute for validator-backed token-conservation tests,
program/security review, key ceremonies, regional latency measurements,
multi-provider failover evidence, backup/key-recovery drills, accessibility
automation, legal/tax ownership, or approved operational on-call coverage.

## Automated contract

Run `make public-docs-check` from a clean checkout. The check verifies the
required public files, root documentation links, ADR-index coverage, explicit
status language, tracked-secret filename hygiene, and presence of the
repository's current executable safety contracts and reconciliation drill. The
separate `make rust-panic-safety-contract`, `make container-pinning-contract`,
and `make refund-cancellation-reconciliation-drill` gates verify runtime lint
coverage, immutable container/action inputs, and validator-backed cancellation
evidence respectively. The public-docs check intentionally does not approve a
license or production readiness; those are human governance decisions and
remain release blockers until evidenced in the readiness plan.
