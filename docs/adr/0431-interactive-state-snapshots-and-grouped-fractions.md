# ADR-0431 — Interactive-state snapshots and grouped fractions

- Status: Accepted. Supersedes the "hover, focus, and open-menu not covered" limit
  in ADR-0428 and ADR-0429.
- Date: 2026-10-07
- Decision owners: Principal UI Architecture, Product Design

## Context

Style snapshots proved resting appearance only. Hover, keyboard focus, pressed,
and open-disclosure styling (state layers, focus rings, menus) were checked by
tests and by hand. The workbench source row, once wrapping (ADR-0430), could also
leave its "/" separator alone at the start of a line.

## Decision

1. For every captured state, `e2e/styleSnapshot.ts` records three forced
   interactive variants and one disclosure variant:
   - `:hover`, `:focus-visible`, and `:active` are forced onto every interactive
     element (`a[href]`, `button`, `input`, `select`, `textarea`, `summary`,
     focusable `[tabindex]`) at once through the Chrome DevTools Protocol
     (`CSS.forcePseudoState`), and only those elements are recorded;
   - every closed `<details>` is opened (and restored) and the shell is recorded.
   All variants are recorded in both themes. `STYLE_SNAPSHOT_NO_STATES=1` skips
   them for fast resting-state comparisons.
2. Group the numerator, divider, and denominator of a base-source share in one
   `.base-share-fraction` wrapper that wraps as a unit, so the fraction cannot
   be split across lines.

## Evidence

- 1,182 snapshot files per run: 245 resting, 245 each hover, focus-visible and
  pressed, and 202 with disclosures opened. Two identical runs differ in 0
  elements.
- The forced states take effect: for example hover changes backgrounds and
  borders, focus-visible applies the outline to all interactive elements, and
  pressed changes backgrounds.
- The fraction change affects 26 elements in eight governance snapshots (divider
  padding and size) plus the one added wrapper element.
- 401 unit tests, 4 web contracts, and 49 real-browser journeys with axe pass.

## Limits

- Forced pseudo-states approximate real interaction; they do not exercise
  transitions or pointer sequences (transitions are disabled during capture).
- `:focus-within`, `:target`, `:checked`, and `:invalid` combinations are not
  forced; invalid and disabled fields are covered by the unit tests and fixtures
  that render them.
- A run is larger and slower (about 8 minutes); use the resting-state mode for
  quick comparisons.
