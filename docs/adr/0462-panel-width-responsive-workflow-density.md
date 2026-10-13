# ADR-0462: Make workflow density responsive to panel width

- Status: Accepted
- Date: 2026-10-10
- Owners: Product engineering, accessibility, and operations

## Context

The primary quote workspace is a two-panel layout. At ordinary laptop and
small-desktop widths, the calculator panel can be narrow enough that its
two-column field grid compresses labels and placeholders, while the empty
result panel stretches to the height of the entire form. The result is a
visually unbalanced page and a form that is harder to scan and complete even
though the overall viewport still has usable width.

## Decision

Treat the calculator panel as its own inline-size container. When that panel is
narrow, place workflow fields in one full-width column so labels, supporting
copy, and input affordances remain readable. Keep the multi-column grouping
when the panel itself is wide enough. Align the workspace grid to the start so
an empty result surface keeps its intrinsic empty-state height instead of
stretching to match a longer input form.

## Alternatives considered

1. **Keep viewport-only breakpoints.** Rejected because the persistent
   navigation rail changes the width available to the workspace without
   changing the viewport.
2. **Always use one form column.** Rejected because it creates unnecessary
   vertical travel on wide desktop panels.
3. **Shorten placeholders only.** Rejected because it hides the layout defect
   and does not protect labels, help text, or translated strings.
4. **Use panel-width queries and start-aligned result surfaces.** Selected
   because it preserves desktop scanability while preventing narrow-panel
   clipping and empty-space inflation.

## Acceptance criteria

- A calculator panel at or below the narrow-panel threshold renders workflow
  fields as one readable column.
- A wide calculator panel retains responsive multi-column grouping.
- Empty result panels do not stretch to the height of the input form.
- The change uses repository-owned layout tokens and does not add decorative
  colour or shadow layers.
- Existing unit, accessibility, responsive, and browser gates remain green.
