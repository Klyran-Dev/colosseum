# ADR-0013: Make tax arithmetic and evaluation order explicit

- Status: Accepted
- Date: 2026-09-25

## Context

Tax outcomes can differ materially based on whether a tax is inclusive, whether one tax is included in another tax base, whether a cap applies before a credit, and whether rounding happens per line or at invoice level.

## Decision

Use fixed-point integer or exact rational intermediates; never binary floating point. Every rule declares dependency order, base selection, bracket boundaries, clamp order, decimal scale, rounding/truncation mode, rounding scope, and residual allocation. These semantics are included in the rule digest and golden test vectors.

## Consequences

- Results are deterministic across platforms and languages.
- Rule authoring is more detailed but far more auditable.
- Changes to arithmetic semantics require a new rule/engine version and compatibility review.

