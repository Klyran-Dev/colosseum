# ADR-0428 — In-journey style snapshots, dead selectors, and token migration

- Status: Accepted. Supersedes the coverage limit recorded in ADR-0427.
- Date: 2026-10-07
- Decision owners: Principal UI Architecture, Product Design

## Context

ADR-0424..0427 proved CSS changes neutral by snapshotting computed styles on
the 16 signed-out routes. That cannot see surfaces that render only with API
data (timeline, evidence summaries, vault and replay cards, wallet panels), so
those rules stayed on hard-coded `rem` sizes. The Playwright journeys already
render most of those states with mocked API responses and run axe at each.

## Decision

1. **Snapshot inside the journeys.** `e2e/styleSnapshot.ts` (enabled only by
   `STYLE_SNAPSHOT_DIR`) records the computed style and geometry of every
   element, in real dark and light themes with transitions disabled, at the end
   of every passing journey and at each of the 61 axe checkpoints. 118 states
   are captured. `scripts/web_style_snapshot.sh <label>` runs it in the isolated
   Compose project; `scripts/diff_style_snapshots.mjs` compares two labels
   (`--props` gives a property histogram and flags unexpected properties;
   `--coverage` lists styled classes no journey renders). Identical CSS gives
   0 differences across two runs. Screenshot mode
   (`STYLE_SNAPSHOT_SCREENSHOTS=1`) saves full-page images for human review; it
   perturbs 1px sub-pixel sizes, so never diff a screenshot run.
2. **Delete dead selectors.** A test fails when any class in the stylesheet has
   no producer in production source (verbatim, or via a `prefix-${...}` template).
   Eight classes had none (`hero*`, `badge-label`, `metrics`, `workspace-tabs`,
   `tab-active`, `workbench-rule-identity`) and their 18 rules were removed.
   Snapshot diff: 0 differences.
3. **Tokens wherever verifiable.** 73 rules (59 font sizes, 25 radii) that
   target only classes the journeys render were mapped to the nearest type-scale
   and shape token. A test allows hard-coded font sizes and corner radii only on
   rules targeting the explicit `UNVERIFIED_SURFACES` list (54 classes that no
   journey renders yet). Shrinking that list is the remaining migration work.

## Mapping

Type: up to .70rem label-small, .78 label-medium, .90 body-medium, 1.05
body-large, 1.45 title-large, 1.6 headline-small, 1.85 headline-medium, 2.2
headline-large, larger display-medium. Corners: up to .35rem extra-small, .55
small, .85 medium, 1.1 large, 1.5 large-increased, 1.9 extra-large, 2.5
extra-large-increased, then extra-extra-large; `999px` becomes `full`.

## Evidence

- 385 unit tests, 4 web contracts, 49 real-browser journeys with axe pass.
- Token migration: 34,515 element-level differences over 118 states in two
  themes, all on font size, corner radius, and their derived geometry
  (line height, size, grid columns, em-based letter spacing, resolved `auto`
  margins). No colour, background, display, position, padding, or opacity change.

## Consequences

- No text on a migrated surface is below 11px. Surfaces are slightly larger.
- A time-dependent journey (expired-quote countdown) can change element count
  between identical runs; it is reported as a structural issue, not a diff.
- Final-state snapshots can miss states a journey never reaches. Coverage is
  measured, not assumed, and the unverified list is the explicit remainder.
