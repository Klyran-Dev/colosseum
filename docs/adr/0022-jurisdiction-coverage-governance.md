# ADR-0022: Release tax coverage as governed jurisdictional capability

- Status: Accepted
- Date: 2026-09-25

## Context

“Global tax engine” cannot mean one unqualified universal rule set. Tax treatment depends on jurisdiction, regime, product, facts, time, evidence, and legal interpretation. Silent defaulting would create financial and legal risk.

## Decision

Publish coverage as jurisdictional capabilities with explicit regime, product categories, required facts, effective windows, rule provenance, owner, limitations, test vectors, and review date. Unsupported or ambiguous cases fail closed or require merchant review. General availability is limited to the published matrix; new jurisdictions require a governed release.

## Consequences

- Coverage expands deliberately and transparently.
- The UI/API must surface supported, unsupported, stale, and review-required states.
- Legal/tax review and engineering evidence are required for each published capability.

