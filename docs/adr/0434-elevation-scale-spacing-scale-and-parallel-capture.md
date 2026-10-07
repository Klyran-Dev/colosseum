# ADR-0434 — Elevation scale, spacing scale, and fully parallel capture

- Status: Accepted. Supersedes the "spacing and shadows are not tokenized" limit.
- Date: 2026-10-07
- Decision owners: Principal UI Architecture, Product Design

## Context

Shadows and spacing were the last hard-coded visual values. Material 3 does define
an elevation scale (levels 0 to 5) but the stylesheet had two ad-hoc shadow
tokens and a dead 4px ring left behind by an earlier tint cleanup. Material
defines no spacing tokens, yet 344 padding, margin, and gap declarations used 37
distinct `rem` values from `.1rem` to `2rem`, so the layout rhythm was
accidental.

## Decision

1. **Elevation:** declare the M3 level 0-5 shadows as `--md-sys-elevation-level*`.
   Menus and popovers use level 2. A test allows only a level token, `none`, or a
   one-pixel state ring in any `box-shadow`. The dead ring was removed.
2. **Spacing:** declare a repository-owned scale on the 4px layout grid with a 2px
   half step (`--app-space-half`, `-1` ... `-10`: 2, 4, 8, 12, 16, 20, 24, 32, 40px).
   Every `padding`, `margin`, and `gap` now uses the scale; each legacy value moved
   to the nearest step (ties up). `calc`, `clamp`, `min`, and `max` expressions,
   `0`, `auto`, and percentages stay, and the visually-hidden utility keeps its 1px
   clip. A test rejects raw lengths in spacing declarations, and the existing
   unresolved-`var()` test covers the tokens.
3. **Capture throughput:** with a 30 s per-test default, the many-checkpoint
   journey timed out under parallel load, so snapshot runs extend the test timeout
   to 120 s. `STYLE_SNAPSHOT_WORKERS=N` now also runs journeys in parallel
   (`--fully-parallel`): every API call is mocked per test. A full run, all
   variants, takes about 2.9 minutes (about 8 serial).

## Evidence

- Two parallel full runs (1,427 files each) differ in 0 elements, none unsettled.
- Spacing step: about 199,000 element-level differences over 1,427 snapshot files,
  all on padding, margin, gap, resulting size, and grid columns; no colour, radius,
  font, display, position, or opacity change. Padding and gap shifts are at most
  1.9px; the only larger margin change is the sidebar footer's `margin-top: auto`,
  a resolved auto margin that absorbs height.
- 413 unit tests, 4 web contracts, and 55 real-browser tests with axe pass.

## Consequences and limits

- Layout is slightly tighter or looser by up to about 2px in places; the spacing
  scale is a repository extension, not part of the Material specification.
- Component sizes (widths, heights, min/max sizes) and positioning offsets are not
  spacing and remain authored lengths.
