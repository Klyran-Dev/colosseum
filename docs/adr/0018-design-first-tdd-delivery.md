# ADR-0018: Require design-first, TDD, and production-quality vertical slices

- Status: Accepted
- Date: 2026-09-25

## Context

This system moves value and calculates tax. Fast demo-driven implementation creates hidden assumptions, incomplete failure behavior, and misleading UI. Low latency also requires measurement rather than intuition.

## Decision

Every material change begins with design, alternative analysis, invariants, threat model, latency/resource budgets, and an ADR. Tests and executable specifications precede implementation. A vertical slice is complete only when correctness, security, observability, accessibility, error handling, performance, and Docker reproducibility are present. No demo or MVP path may be merged as production behavior.

## Consequences

- Early delivery is more deliberate, but rework and hidden risk decrease.
- CI must include unit, property, fuzz, integration, UI/accessibility, performance, security, and supply-chain checks.
- A feature may be narrow in scope while still being production-grade end to end.

