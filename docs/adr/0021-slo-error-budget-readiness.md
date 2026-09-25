# ADR-0021: Use SLOs and error budgets as Tier-1 release controls

- Status: Accepted
- Date: 2026-09-25

## Context

Low latency, availability, correctness, and reconciliation compete for engineering attention. A product handling money needs measurable reliability and a disciplined response to degradation.

## Decision

Define SLOs for API availability, quote/calculation/submit/finalization latency, projection freshness, reconciliation accuracy, and unauthorized vault outflows. Track error budgets, page on burn-rate and financial anomalies, and block scope expansion or GA when critical budgets are exhausted without an approved exception.

## Consequences

- Product and engineering decisions use evidence instead of intuition.
- The system needs complete telemetry and ownership for every SLO.
- A fast system that loses correctness still fails the release gate.

