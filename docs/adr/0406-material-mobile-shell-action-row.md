# ADR-0406: Material 3 mobile shell action row

- Status: Accepted
- Date: 2026-10-06
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React SPA product shell at viewports up to 760 CSS pixels

## Context

The shell already uses one Material 3 semantic palette and progressive
disclosure for navigation and popovers. At mobile widths, a legacy flex-column
rule stacked system status, display settings, local sign-in, and account actions
vertically. This pushed the workspace context down the page and made routine
controls compete with the task heading.

The shell must remain fully operable for keyboard, screen-reader, RTL, reduced
motion, and authorization-recovery users. The change must affect presentation
only; it must not change access decisions, sign-in state, tax facts, wallet
state, or settlement behavior.

## Decision

At mobile widths, render the top bar as two semantic rows:

1. the breadcrumb/workspace identity row; and
2. a wrapped horizontal action row containing system status, display settings,
   session control, and account context.

The action row may wrap at narrow widths, but it must not revert to a vertical
control stack. Popover contents remain absolutely anchored and enter layout
only after explicit disclosure. Account context remains last in visual order
when space permits, while DOM order and accessible names remain unchanged.

The implementation uses existing M3 surface, outline, spacing, and state roles;
it introduces no new color, gradient, icon-only authorization affordance, or
business behavior.

## Alternatives considered

| Alternative | Scan cost | Accessibility and discoverability | Workspace stability | Decision |
| --- | --- | --- | --- | --- |
| Keep the vertical stack | High; controls dominate the mobile header | Pass | Fail; pushes task content down | Rejected |
| Hide session/status controls on mobile | Low | Fail; protected recovery and service state become undiscoverable | Pass | Rejected |
| Move all controls into one overflow menu | Low | Partial; adds interaction cost and hides important state | Pass | Rejected |
| **Two-row identity plus wrapped Material action row** | **Low** | **Pass; existing controls and names remain present** | **Pass** | **Selected** |

## Invariants and failure policy

- Existing control DOM order, accessible names, authorization semantics, and
  popover behavior remain unchanged.
- The mobile action row uses a horizontal flex direction with bounded wrapping;
  a regression test fails if it becomes a column or expands the header beyond
  the documented compact-height budget.
- No workspace becomes inaccessible when the action row wraps.
- If the layout cannot satisfy the compact geometry, it must preserve all
  controls and allow normal document flow; it must not hide or disable them.

## Verification

- The failing-first Playwright test in
  `apps/web/e2e/ui-journeys.spec.ts` asserts grid top-bar composition,
  horizontal wrapped actions, and bounded mobile geometry at 568 CSS pixels.
- The focused Docker/NGINX browser run passed after the CSS implementation.
- The complete `make web-browser-check` remains required after the next
  release-candidate change, along with manual screen-reader and operator review.

