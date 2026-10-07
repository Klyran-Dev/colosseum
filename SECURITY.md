# Security policy

This project handles designs for financial and tax infrastructure. Do not disclose exploitable vulnerabilities, private keys, seed phrases, personal data, or sensitive tax evidence in public issues, pull requests, or chat.

## Reporting a vulnerability

Use a private GitHub Security Advisory for this repository. If private advisories are unavailable, contact the repository maintainers through a private GitHub channel and provide only the minimum reproduction details needed for triage.

Please include:

- affected component and version/commit;
- impact and attack prerequisites;
- minimal reproduction or proof of concept;
- whether funds, keys, tax evidence, or user data may be affected;
- any suggested mitigation.

Do not exploit a vulnerability beyond what is necessary to demonstrate impact. Do not publish the issue until maintainers coordinate disclosure.

## Scope

In scope:

- Solana program authority, account validation, CPI, replay, and arithmetic flaws;
- tax-rule compilation, calculation, attestation, provenance, and evidence-integrity flaws;
- API authorization, key handling, serialization, injection, and denial-of-service flaws;
- frontend wallet/signing, transaction presentation, XSS, CSRF, and supply-chain flaws;
- Docker image, build, dependency, and secret-management flaws.

Out of scope:

- unsupported deployments or modified binaries;
- social engineering or attacks requiring compromised user devices;
- legal/tax opinions without a software vulnerability;
- denial-of-service reports without a credible reproducible impact analysis.

## Security posture

The repository contains an executable implementation foundation, but it is not
an audited or production-ready deployment. `make security-scan` runs the
pinned vulnerability, secret, and Docker-misconfiguration scan; a passing scan
is evidence for that revision only and is not a security audit. Security
controls, audit status, supported versions, release artifacts, and remaining
production gates are tracked in
[`docs/plans/production-readiness.md`](docs/plans/production-readiness.md).

Do not include credentials, seed phrases, private keys, raw jurisdiction
evidence, or production data in bug reports. Use synthetic fixtures and redact
logs before sharing them.

## Dependency findings and exceptions

`node scripts/verify_web_audit_contract.mjs` runs `pnpm audit` for `apps/web` and fails on
any high or critical finding that lacks an approved, time-bounded exception in
[`docs/security/dependency-exceptions.json`](docs/security/dependency-exceptions.json).
An exception names an accountable approver and owner, expires within 180 days, and may
cover only packages absent from the shipped browser bundle; stale or expired entries fail
the check (see ADR-0440). Patched versions are applied as pinned overrides in
`apps/web/pnpm-workspace.yaml`.
