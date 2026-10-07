# ADR-0427 — Canonical access-control, evidence, and empty-state surfaces

- Status: Accepted. Extends ADR-0421 (container queries) to the access-control layout.
- Date: 2026-10-07
- Decision owners: Product Design, Principal UI Architecture

## Context

The access-control surface carries the decisions users most need to read:
permission names, risk and decision chips, policy captions, and the current
authorization posture. It rendered at 9-11px (`.56rem`-`.73rem`), with
uppercase 9px chips, and used hard-coded radii. Two layout defects of the
ADR-0421 class remained: `.access-control-layout` was still sized by the
viewport, and the posture strip truncated its values with an ellipsis ("Not …",
"Confi…", "Waiting…"), hiding the authorization state itself.

## Decision

1. **Inner cards** (permission groups, policy dimensions, posture cells,
   technical policy rows) are borderless tonal containers with a medium corner.
2. **Type scale:** group and policy headings use title-small emphasized,
   permission names body-medium emphasized, captions body-small, chips and
   requirement text label-small (11px), posture labels label-small and values
   label-medium emphasized, disclosure summaries label-large emphasized, empty
   state copy body-medium and its icon headline-small.
3. **Chips** use a small corner and sentence case instead of 9px uppercase.
4. **Posture values wrap** instead of truncating, and the strip uses
   `repeat(auto-fit, minmax(min(100%, 9rem), 1fr))`.
5. `.access-control-layout` joins the workspace container query (stacks at
   56rem of workspace width).
6. A test rejects `rem` font sizes and radii on these surfaces.

## Evidence and its limits

- Before/after snapshots (16 routes, real dark and light, transitions
  disabled) show only typography, radius, border-colour, text-transform and
  resulting size/grid changes on these surfaces.
- 33 now-dead legacy declarations were removed by the ADR-0424 method.
- 382 unit tests, 4 web contracts, and 49 real-browser journeys with axe pass.
- **Coverage limit:** only surfaces that render without signed-in data can be
  snapshotted. Workspace tabs, the governance timeline, trace rows, applicability
  and release evidence summaries, vault and split-review rows, bracket rows,
  and metric cards did not render in any covered route and are **not migrated**.
  Migrating them needs style snapshots taken inside the Playwright journeys
  that already render them with mocked API data.

## Consequences

- Permission rows are taller at narrow widths because requirement text is larger
  (about 70% taller at 330px wide before stacking); the stacked single-column
  layout keeps them compact at narrow workspace widths.
- Chip text moves from about 9px to 11px, the Material label-small size.
- Un-migrated surfaces keep their `rem` literals until they are verifiable.
