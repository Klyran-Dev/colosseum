# ADR-0402: Single tax-trace disclosure in the result workspace

- Status: Accepted
- Date: 2026-10-05
- Owners: Product UI, Tax Engine, Accessibility

## Context

The aggregate quote and scenario result workspace rendered the same tax
component trace twice: once inside the progressive “Why this tax?” disclosure
and again below the evidence controls. The duplication increased scan cost,
made the result appear to contain more evidence than it did, and made the
operator switch between two visually identical representations of the same
server-owned values.

The trace is explanatory output, not an additional calculation or authority
boundary. It must remain available without expanding the primary result
surface, while technical fingerprints and replay controls remain progressively
disclosed.

## Decision

Render exactly one component trace per calculation result, inside the
progressive “Why this tax?” disclosure. Keep the total, jurisdiction, result
status, replay action, and technical details as separate bounded sections.
The trace continues to render only validated server response components; the
browser does not calculate, merge, or reorder tax components.

## Alternatives considered

| Alternative | Scan cost | Evidence clarity | Implementation risk | Decision |
| --- | --- | --- | --- | --- |
| Keep the duplicate trace | High | Misleading repetition | Low | Rejected |
| Remove the trace entirely | Low | Fails explainability and reviewability | Low | Rejected |
| Move the trace into technical details | Medium | Hides the operator-facing explanation | Medium | Rejected |
| **Keep one trace in “Why this tax?”** | **Low** | **Preserves plain-language explanation and exact values** | **Low** | **Selected** |

## Invariants

- For every accepted calculation response, each validated component is rendered
  exactly once in the result workspace.
- Component labels, explanations, and atomic tax values remain server-owned;
  no browser arithmetic or client-side component transformation is introduced.
- The result status, replay action, calculation reference, rule fingerprint,
  and facts fingerprint remain available in their existing bounded sections.
- The disclosure is keyboard-operable and retains the existing semantic
  `details`/`summary` relationship.
- A failed, unavailable, malformed, or stale response still renders no
  invented trace or financial total.

## Failure and security considerations

The change removes presentation duplication only. It does not weaken response
parsing, authorization, tenant scope, replay validation, evidence binding, or
settlement authority. A malformed component response continues to fail closed
at `parseCalculationResponse`. The regression test uses a synthetic response
only inside the test boundary and does not create a production fallback.

## Latency and resource budget

The change removes one bounded React subtree per component. It adds no network
request, state, serialization, or tax-engine work. The expected render and
memory cost is strictly no greater than the previous implementation; browser
journey and accessibility gates remain required.

## Verification

- The failing-first `apps/web/src/App.ui.test.tsx` regression proves the
  previous duplicate trace and then passes with exactly one `.trace-row` per
  returned component.
- `pnpm exec vitest run src/App.ui.test.tsx` passes.
- The full frontend, typecheck, production-build, browser, palette, and
  accessibility gates remain required before a production UI claim.

