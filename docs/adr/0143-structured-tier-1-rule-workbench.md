# ADR-0143: Structured Tier-1 rule workbench with canonical DSL escape hatch

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Tax Domain, Product Design, Frontend
- Scope: React rule-governance authoring experience and client-side rule document construction

## Context

The Tier-1 tax engine already defines a versioned canonical rule DSL for flat,
progressive, dependency-based, threshold, exemption, inclusive/exclusive,
credit, floor/cap, truncation, and explicit rounding semantics. The governance
workspace currently asks an operator to author that entire document in a raw
JSON textarea. Raw JSON is useful for expert interchange, but it is not a
gold-standard authoring surface: it increases syntax errors, hides dependency
relationships, makes bracket boundaries difficult to review, and weakens the
operator's ability to understand the eventual calculation trace.

The browser must not become a tax authority. It must construct a bounded,
explicit request; the Rust API remains the validation, canonicalization,
governance, and publication authority.

## Decision

Add a structured rule workbench to the governance bounded context. The
workbench will:

1. represent a bounded list of tax components and their dependency sources;
2. support flat and progressive rates with explicit numerator/denominator
   values and contiguous bracket boundaries;
3. expose price mode, credit, floor, cap, rounding mode, threshold semantics,
   and the explicit `GrossExcess` equation when an inclusive threshold is used,
   and exemption codes as named controls;
4. generate the exact canonical `RuleSet` JSON request without calculating tax
   locally;
5. show a read-only generated-document preview and client-side structural
   warnings before submission;
6. retain an explicit advanced JSON mode for importing/exporting future DSL
   fields, while labeling it as expert input and still requiring server
   validation; and
7. keep the existing idempotent authoring, authenticated role boundary, and
   four-eyes lifecycle unchanged.

The workbench has a hard client-side component and bracket bound matching the
server contract. It never silently repairs invalid values, infers a tax rate,
publishes a rule, or presents a local calculation as authoritative.

## Alternatives considered

### Keep only the raw JSON editor

Rejected as the primary experience. It preserves full expressiveness but has a
high operator-error rate, poor dependency review, and weak accessibility for
structured tax semantics.

### Build a jurisdiction-specific form per country

Rejected. It duplicates legal policy in the UI, creates an unbounded product
surface, and couples release cadence to jurisdiction-specific tax changes.

### Generate a visual graph editor as the sole authoring surface

Rejected for the first workbench. Graph canvases are expensive to make
keyboard-accessible, difficult to review deterministically, and can obscure
the canonical serialized rule document.

### Selected: schema-driven structured workbench plus advanced JSON mode

Selected because it makes the supported DSL semantics explicit and reviewable,
keeps the canonical document visible, remains extensible, and preserves an
expert escape hatch without weakening server authority.

## Invariants and evidence

- Every submitted document is generated from bounded explicit fields or is
  visibly marked advanced JSON; both paths are validated by the server.
- Financial values and rates remain decimal strings; no browser floating-point
  calculation is used.
- Progressive brackets are contiguous, ordered, and have only a final
  open-ended bracket; invalid states are submitted only if the operator
  explicitly chooses advanced mode and the server rejects them safely.
- Component dependencies are explicit and preserved in the generated DSL;
  cycles and missing references remain server-owned validation failures.
- The generated preview is a document preview, not a tax result or compliance
  assertion.
- Tests cover generated flat, progressive, tax-on-tax, thresholds, clamps,
  rounding, exemptions, invalid boundaries, duplicate IDs, and dependency
  references.
- Full browser keyboard, screen-reader, RTL, localization, and legal/tax
  review remain separate release gates.
