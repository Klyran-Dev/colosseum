# ADR-0466: Top-align independent operational panels

- Status: Accepted
- Date: 2026-10-10
- Owners: Product engineering, accessibility, and operations

## Context

The shell uses two-column layouts for checkout, governance, replay recovery,
vault operations, adjustments, and access control. CSS Grid's default
`align-items: stretch` made a shorter empty evidence/detail panel expand to the
height of a long form beside it. This created large blank canvases that looked
unfinished and increased scan distance without adding information.

## Decision

Top-align each multi-panel operational layout. Panels keep their own intrinsic
height; existing workspace container queries still collapse the layout to one
column at narrow widths. No panel content, authority boundary, or empty-state
semantics change.

## Alternatives considered

1. **Keep the default stretch behavior.** Rejected because empty or loading
   siblings visually imply missing content and waste vertical scan space.
2. **Give every panel a fixed shared height.** Rejected because it preserves
   blank space and creates overflow risk for localized or evidence-heavy data.
3. **Top-align the layout and let each panel size intrinsically.** Selected
   because it reflects independent content lifecycles and reflows safely.

## Invariants and acceptance criteria

- Checkout, governance, replay, vault, adjustment, and access-control panels
  align to the top at multi-column widths.
- Narrow container layouts continue to collapse to one column.
- No page-level horizontal overflow is introduced at 320px.
- Panel content, actions, authorization, and evidence semantics are unchanged.

## Verification

- The stylesheet contract requires top alignment for every affected layout.
- The full frontend suite and Docker browser gate cover panel markup, themes,
  responsive reflow, and accessibility.
