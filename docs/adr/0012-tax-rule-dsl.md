# ADR-0012: Use a canonical tax-rule DSL compiled to immutable rule graphs

- Status: Accepted
- Date: 2026-09-25

## Context

Global tax behavior is a composition of bases, components, brackets, dependencies, thresholds, clamps, exemptions, and rounding semantics. Hand-coded jurisdiction branches become opaque and difficult to audit.

## Decision

Represent rules in a versioned canonical DSL with explicit schemas for facts, jurisdiction selectors, tax components, bases, rates, brackets, dependencies, thresholds, credits, min/max operations, and rounding. Validate and compile the DSL into an immutable directed acyclic rule graph. Reject cycles, ambiguous precedence, overlapping brackets, missing precision, and unbounded graph complexity.

## Consequences

- Rules become reviewable, diffable, replayable, and portable to Rust/WASM.
- The DSL is a critical security boundary and requires schema compatibility and adversarial testing.
- Legal/tax reviewers need a human-readable rendering alongside the machine representation.

