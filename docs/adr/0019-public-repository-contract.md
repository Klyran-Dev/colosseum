# ADR-0019: Treat public repository documentation as a product contract

- Status: Accepted
- Date: 2026-09-25

## Context

The repository is public-facing documentation for financial infrastructure. Contributors, integrators, auditors, and users must be able to determine what exists, what is supported, what is not claimed, how to contribute, and how to report security issues.

## Decision

Maintain a truthful root README, architecture index, AGENTS instructions, contribution guide, security policy, code of conduct, changelog, ADR index, production gates, coverage matrix, and release notes. Documentation must be versioned with code and reviewed as part of the product. License selection is an explicit release gate.

## Consequences

- Documentation quality is part of release quality.
- Public claims require current evidence.
- The repository cannot be treated as an internal scratchpad after publication.

