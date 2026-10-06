# ADR-0424 — Remove provably dead stylesheet declarations

- Status: Accepted
- Date: 2026-10-06
- Decision owners: Principal UI Architecture

## Context

Eight-plus cascade layers repeated the same selectors, so many declarations
could never win. They made every colour or shape change a multi-layer edit and
let text-based contracts pass on code that had no effect.

## Decision

Remove a declaration only when another top-level rule **always** overrides it
for every element the first rule matches:

1. the same selector later in the file (same specificity, later wins), or
2. the same selector prefixed with `.product-shell` (higher specificity, wins
   regardless of order). All application content renders inside the
   `.product-shell` root and nothing is portalled, so the premise holds. Rules
   on the root element, `:root`, `html`, `body`, `*` and `@`-blocks are never
   touched.

Declarations carrying `!important`, custom properties, and anything inside
`@media`/`@container` are left alone. A removal is accepted only if the
selector list's every component is dominated.

## Evidence

- 341 declarations removed across 218 rules (1,572 to 1,473 lines).
- A computed-style and geometry snapshot of every element on all 16 routes in
  dark and light themes, taken before and after, shows **0 differences**
  (about 11,000 elements). Hover, focus, open-menu and media-query states are
  not part of that snapshot; the removal argument is order-independent for
  them, but they are not empirically covered.
- 365 unit tests, the four web contracts, and 49 real-browser journeys pass.

## Consequence for the colour contract

`--app-supporting` was only referenced by a dead `.risk-governance` border.
ADR-0419 made risk chips neutral, so the role has no live consumer. The web
colour contract no longer requires it in the final layer; it instead requires
the supporting aliases to remain defined in the token block. Narrowing a
contract to match reality is deliberate and recorded here.

## Alternatives

| Option | Verdict |
| --- | --- |
| Hand-edit duplicates | Rejected: unverifiable at this size. |
| Rewrite the stylesheet | Rejected: large risk; see ADR-0423. |
| **Mechanical removal of provably dead declarations plus snapshot proof** | **Selected.** |
