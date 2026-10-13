# ADR-0472: Use a responsive two-column rhythm for calculator fields

## Status

Accepted

## Context

The aggregate quote and simulator workflows rendered every short input in one
vertical column. On a normal desktop panel this created an unnecessarily long
form, pushed the result surface below the fold, and made related financial
inputs harder to scan as a group. The same panel must remain usable at tablet,
mobile, and localized widths.

## Decision

Use the calculator panel's inline-size as the layout authority. At 28rem or
wider, render the calculator fieldset as two equal, bounded columns. Keep
evidence, supporting-help, and other `.full-field` rows across the full width.
Below 28rem, retain one column so labels, placeholders, and numeric values do
not become cramped.

## Alternatives considered

- Keep one column at every width: rejected because it wastes desktop space and
  creates a long scanning path for a routine quote.
- Use viewport breakpoints: rejected because the persistent navigation rail and
  compact shell change the workspace width independently of the viewport.
- Use a fixed two-column layout: rejected because narrow panels and localized
  labels need a single-column fallback.

## Consequences

- Related quote inputs are visible together and the workflow reaches its action
  sooner on desktop.
- The layout remains container-aware and keeps evidence/help content readable.
- No financial semantics or DOM order change; this is a presentation-only
  improvement.

## Verification

- Material shell contract test for the container threshold, two-column grid,
  and full-width rows.
- Frontend unit suite, typecheck, `git diff --check`, and Docker browser reflow
  and accessibility gates.
