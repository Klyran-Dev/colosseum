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

The project is currently documentation-first and is not claiming an audited or production-ready deployment. Security controls, audit status, supported versions, and release artifacts will be published as implementation progresses.

