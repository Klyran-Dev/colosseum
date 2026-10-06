# ADR-0426 — Canonical form controls, supporting text, and notices

- Status: Accepted
- Date: 2026-10-06
- Decision owners: Product Design, Principal UI Architecture

## Context

Form surfaces were styled by hard-coded `rem` radii and font sizes across
several layers. Supporting text and local sign-in fields rendered at 9-11px
(`.55rem`-`.7rem`), below the Material label-small size. There was no styling for
invalid, disabled, or hovered fields, and field styling existed twice: once for
the whole shell and once only for `.product-content`, so fields in the topbar
menus differed from fields in workspaces.

## Decision

One canonical block replaces them, covering the whole `.product-shell`:

1. **Outlined text fields** (M3): transparent container, extra-small (4px)
   corner, 1px `outline`, hover to `on-surface`, 2px-equivalent `primary` outline on
   focus, body-large text.
2. **Invalid** (`aria-invalid="true"`): `error` outline with a matching 1px ring
   so it is not colour-only (the control's own text/message still carries the
   meaning). **Disabled**: 12% outline and 38% text of `on-surface`, with a
   not-allowed cursor.
3. **Type roles:** field labels use label-medium; supporting text uses
   body-small; notices (`inline-*`, `route-evidence`) use body-medium with a
   medium corner; legends use title-small emphasized; local sign-in fields use
   label-small and body-small, replacing 9-10px text.
4. A test fails if any form-surface rule reintroduces a `rem` font size or radius.

## Evidence

- Before/after computed-style and geometry snapshots (16 routes, real dark and
  light themes, transitions disabled) show only fontSize, lineHeight,
  letterSpacing, backgroundColor, radius, size, and the disabled-state colours
  changing. No display, position, margin, padding or opacity changes.
- 40 now-dead legacy declarations were removed by the ADR-0424 method.
- 376 unit tests, 4 web contracts, and 49 real-browser journeys with axe pass.

## Consequences

- Fields are taller (41px to about 50px) and text is larger; forms are a little
  longer. Smallest form text is now 11px (label-small) instead of about 9px.
- The disabled style follows the Material specification (38% content), which
  the specification exempts from the 4.5:1 text requirement. Blocked actions are
  still explained in adjacent text.
- Hover, focus, and open-menu states are not part of the static snapshot; they
  are covered by tests and by the earlier manual check.
- Remaining legacy surfaces (tables, evidence views, workspace tabs, vault and
  revenue panels) still use `rem` literals and are the next migration targets.
