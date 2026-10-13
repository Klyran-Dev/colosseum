# ADR-0362: Explicit inclusive gross-excess threshold equation

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal Low-Latency Blockchain Architecture
- Scope: Tax Engine inclusive pricing and thresholds

## Context

Inclusive pricing and thresholds cannot be combined safely by inference. A
threshold may be a gross-price threshold, a derived taxable-base threshold, a
registration threshold, or another jurisdiction-specific concept. Treating
those meanings as interchangeable would produce plausible but legally
incorrect amounts.

The engine previously rejected every inclusive threshold. The Tax Engine contract
needs an executable equation while continuing to reject undeclared semantics.

## Decision

Add the versioned `InclusiveThresholdEquation::GrossExcess` rule value. When an
inclusive component declares this equation:

- `ExemptUpTo` computes `gross_excess = max(gross - threshold, 0)` and applies
  the component's ordinary inclusive tax equation to that gross excess;
- `TriggerAt` applies the ordinary inclusive tax equation to the full gross
  amount only when the declared threshold is reached;
- `CumulativePeriod` interprets the evidence snapshot and threshold in gross
  atomic units;
- the same bounded inclusive calculation path is used for flat and
  progressive rates, with declared rounding applied afterward.

The rule compiler rejects an inclusive threshold without this explicit
equation, rejects the equation on an exclusive component, and preserves the
legacy digest bytes for rules that have no threshold equation. The equation is
not a claim that any jurisdiction uses this treatment; jurisdiction publication
still requires legal/tax ownership, provenance, effective dating, and approved
vectors.

Taxable-net thresholds, registration thresholds, and other equations remain
unsupported until each receives its own named semantics and evidence.

## Alternatives considered

### Infer the equation from inclusive price mode

Rejected. Gross and derived-net thresholds are not interchangeable and a
plausible default would silently encode a legal decision.

### Reject all inclusive thresholds

Rejected as the permanent Tax Engine boundary. It prevents valid explicitly
specified product rules from being represented even when the equation is
bounded and deterministic.

### Implement every jurisdiction-specific threshold equation in one enum

Rejected. It would turn the domain crate into an ungoverned legal catalog.
Each new equation must be versioned, bounded, vectorized, and approved.

### Convert the threshold to a derived net amount implicitly

Rejected. This changes the meaning of the rule and requires a separate
rounding and evidence contract.

## Consequences

- An explicit inclusive threshold can now execute deterministically without
  weakening fail-closed behavior.
- The calculation trace continues to expose the original gross base and the
  effective gross-excess base.
- Existing rule digests without the new field remain stable; rules using the
  equation receive a distinct canonical digest.
- UI and governance authoring must present the equation as a required choice
  and must not label it as jurisdictional compliance without approved release
  evidence.

## Verification

- `calculates_inclusive_gross_threshold_with_an_explicit_equation` proves the
  exact `1200` gross / `1100` threshold / `10%` inclusive-excess case.
- `test-vectors/tax-engine-v1.json` contains the `inclusive-gross-threshold`
  golden vector consumed by the Rust vector suite.
- `rejects_inclusive_threshold_without_an_explicit_equation` preserves the
  fail-closed boundary for ambiguous rules.
