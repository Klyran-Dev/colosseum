# ADR-0421 — Workspace container queries for responsive layout

- Status: Accepted
- Date: 2026-10-06
- Decision owners: Product Design, Principal UI Architecture

## Context

Workspace grids (`workspace-grid`, `checkout-layout`, `governance-layout`,
`replay-layout`, `vault-layout`, `adjustment-layout`) and `form-grid` changed
columns with viewport media queries. The navigation rail takes a variable share
of the viewport (full, compact, collapsed, mobile), so at tablet widths a
~600px workspace still rendered two columns and two-column forms. Inputs
truncated labels and placeholders ("Enter the taxab…"), which is an accessible
name/legibility defect for financial entry fields.

## Alternatives

| Option | Verdict |
| --- | --- |
| Tune more viewport breakpoints per rail state | Rejected: couples every layout to shell state and adds another cascade layer. |
| JS `ResizeObserver` layout classes | Rejected: runtime cost, flash of wrong layout, needless state. |
| CSS container queries on the workspace + intrinsic `auto-fit` form grid | **Selected**: layout follows actual space, no JS, baseline-supported in current browsers. |

## Decision

`.product-content` is a named inline-size container (`workspace`). Multi-pane
layouts stack at `max-width: 56rem` of the workspace; metric rows stack at
`40rem`. `form-grid` uses `repeat(auto-fit, minmax(min(100%, 15rem), 1fr))`.
Only layout is declared; all colour stays on the Material semantic roles.

## Consequences

- Container containment makes `<main>` a containing block for `position: fixed`
  descendants. No fixed element lives inside it today (the skip link is outside);
  new fixed overlays must render outside `<main>`.
- Enforced by `materialExpressiveShellContract.test.ts`.
- Existing viewport rules remain as fallbacks and are not removed in this slice.
