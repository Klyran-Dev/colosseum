# ADR-0453: Use responsive short-field grouping in operational forms

- Status: Accepted
- Date: 2026-10-09
- Owners: Product engineering, operations, and accessibility

## Context

The authenticated quote form rendered every short input in one vertical
column because its nested `fieldset` had a grid of its own without columns.
At a normal desktop width the quote workflow was about 1,220px tall, requiring
an operator to scan a long stack of independent controls while the result pane
remained empty beside it. This made a routine quote look like a prototype form
and increased review effort.

## Decision

Give workflow fieldsets that are direct children of a form a responsive grid
with 12rem minimum columns. Related short controls may sit side by side;
evidence blocks, explanatory notes, and primary actions keep their existing
full-width behavior through `full-field` and button rules. Container queries and
the `min(100%, …)` bound collapse the grid naturally when the workspace is
narrow.

## Alternatives considered

1. Keep one column. Rejected because it creates unnecessary scan length on
   desktop and makes the product feel like a raw data-entry screen.
2. Hard-code two columns at viewport breakpoints. Rejected because the shell
   has a collapsible rail and workspaces need to respond to their container.
3. **Use bounded auto-fit columns inside form-owned workflow fieldsets.**
   Selected because it improves desktop density while preserving narrow and
   assistive-technology layouts.

## Acceptance criteria

- The desktop quote form renders at least two columns for short controls.
- Evidence, guidance, and submit controls remain full width.
- 320px reflow and WCAG audits remain green.
- The Docker browser gate covers the responsive form behavior.
