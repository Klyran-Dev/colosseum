# ADR-0016: Make tax calculation explainable and accessible by default

- Status: Accepted
- Date: 2026-09-25

## Context

Users need to understand what will be withheld before signing, while tax operators and auditors need full calculation traces. A dense technical rule editor alone is not an acceptable financial UX.

## Decision

Use a React SPA design system with progressive disclosure: clear checkout totals first, an accessible “Why this tax?” explanation second, and full inputs/rule graph/evidence third. Provide guided rule authoring, scenario simulation, approval workflows, explicit operational states, localization, WCAG 2.2 AA, and clear separation of calculated, attested, settled, and filed statuses.

## Consequences

- Trust and supportability improve without exposing private evidence unnecessarily.
- The frontend needs a stable calculation-trace contract and shared numeric formatting.
- Accessibility and localization are release gates, not later polish.

