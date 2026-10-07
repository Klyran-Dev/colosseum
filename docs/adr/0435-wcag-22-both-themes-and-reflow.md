# ADR-0435 — WCAG 2.2 AA in both themes, and reflow at 320px

- Status: Accepted
- Date: 2026-10-07
- Decision owners: Principal UI Architecture, Accessibility

## Context

`AGENTS.md` commits the product to WCAG 2.2 AA. The real-browser gate ran axe
with the WCAG 2.0 rule tags, only in the dark theme, so the light theme's contrast
and the 2.1/2.2 rules (target size and others) had never been machine-verified, and
nothing checked WCAG 1.4.10 Reflow (no horizontal scrolling at 320 CSS px).

## Decision

1. **One axe runner, both themes, 2.2 rules.** `e2e/axeAudit.ts` runs axe with the
   `wcag2a/2aa/21a/21aa/22aa` tags in the dark and light themes at every journey
   checkpoint (about 60), restoring the page's theme afterwards.
2. **Forced-colors contrast is the OS's.** In forced-colors mode axe pairs an
   element's authored text colour with the OS-forced background (for example
   `#4d3d75` on black), which passes in dark only by coincidence and fails in light,
   while users see the browser's high-contrast pair (measured yellow on black,
   about 19:1). Only `color-contrast` is disabled for that one journey, with this
   evidence in the code; every other rule still runs in both themes.
3. **Reflow test.** `e2e/reflow.spec.ts` opens every disclosure and requires, for
   all 16 workspaces in both themes at a 320px viewport, that no element outside an
   intentionally scrollable region leaves the viewport and the page does not scroll
   horizontally.
4. **Fix what it found.**
   - Topbar popovers anchored to their chip ran off the screen edge on phones (the
     status panel spanned -101px to 187px at 320px). Below 760px the topbar is
     their containing block and they fit the viewport, scrolling internally if tall.
   - In the rule workbench a nested grid's column was sized to the widest `<select>`
     option (290px inside 254px). Nested workbench grids now have a `minmax(0, 1fr)`
     track, their items may shrink, and selects are `max-width: 100%`.

## Evidence

- All 73 real-browser tests pass: 49 journeys with WCAG 2.2 AA in both themes, 6
  interaction-fidelity, 2 motion, and 16 reflow tests.
- Before the fixes, the reflow test failed on every route (popover) and then on the
  tax-rule workbench (grid track). The light-theme/2.2 audit found no other
  violations.
- 416 unit tests and the 4 web contracts pass. A before/after snapshot diff shows
  the fixes change only grid track values, resulting sizes, and the intended
  small-viewport `position` of the popover hosts.

## Limits

- Reflow is checked with signed-out workspaces; content that appears only with API
  data is not measured at 320px by this test.
- Axe cannot assess everything in WCAG (for example focus order meaning, content
  clarity); manual review and screen-reader testing remain necessary.
- The `.full-field` grid rule also sets an inert `grid-template-columns` on
  non-grid elements with that class.
