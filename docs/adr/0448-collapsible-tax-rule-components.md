# ADR-0448: Collapsible tax-rule components

- Status: Accepted
- Date: 2026-10-09
- Owners: Product engineering and tax domain owners

## Context

The structured tax-rule authoring surface exposes the complete deterministic
rule model: taxable-base sources, shares, rates, brackets, credits, bounds,
rounding, thresholds, and exemptions. Rendering every control for every
component at once makes the authoring surface difficult to scan and increases
the chance that an operator edits the wrong component. The complete model must
remain available; hiding controls must not change the serialized rule or the
validation boundary.

## Decision

Render each tax component as an accessible disclosure section. The first
component is open when the builder starts, additional components are closed,
and a newly added component opens automatically. The summary exposes the
component number, customer-facing label, and rate mode so the operator can
identify a component without opening it. All existing controls, validation,
and canonical rule serialization remain inside the disclosure content.

The disclosure state is local UI state only. It is not persisted, sent to the
API, or used as an authorization or validation signal.

## Alternatives considered

1. Keep every component permanently expanded. Rejected because large rules
   become visually dense and make component-level review error-prone.
2. Show only a fixed subset of fields and remove expert controls behind a
   separate editor. Rejected because it would split the canonical authoring
   model and risk hiding fields that materially affect tax semantics.
3. Use a separate route or wizard step for each component. Rejected because it
   makes cross-component dependencies and cascading bases harder to review.

## Invariants and acceptance criteria

- Opening or closing a component does not change `RuleWorkbenchDraft`, its
  validation result, or the canonical rule document.
- The first component is discoverable and open by default.
- New components are discoverable and open by default so the operator receives
  immediate feedback that the add action succeeded.
- Closed component content remains keyboard and screen-reader accessible after
  activation of its summary.
- Summary text is localized with the existing workbench message catalog.
- Existing structured-authoring, serialization, accessibility, and browser
  gates remain green.

