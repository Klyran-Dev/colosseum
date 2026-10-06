# ADR-0410: Material state emphasis and border budget

- Status: Accepted
- Date: 2026-10-06
- Decision owners: Principal UI Architect, Product Design, Accessibility
- Scope: React/Vite operations shell and shared workspace panels

## Context

The operations console contains many legitimate status labels: workflow type,
approval state, evidence state, risk classification, and service health. Giving
each label a filled accent and giving every panel an outline makes the interface
look like a dashboard of alerts even when the user is completing one ordinary
task. The attached review image shows this failure mode: navigation, system
health, account state, panel boundaries, and action controls compete for the
same visual attention.

Material 3 color roles are semantic. Primary is for the highest-emphasis action
and active state; secondary and tertiary provide lower-emphasis support; error
is reserved for an actual error; and surface-container roles establish hierarchy
without inventing colors. Container roles must be paired with their `on-*`
roles. The official M3 color guidance also recommends surface roles for neutral
backgrounds and cautions against using outline variants as a generic visual
hierarchy tool.

## Decision

1. Neutral `status-pill` is the default for informational, workflow, count, and
   capability labels. It uses `surface-container` and `on-surface-variant`.
2. Filled state roles are opt-in and explicit: `status-pill-success`,
   `status-pill-warning`, and `status-pill-danger`. They map only to the
   corresponding semantic state aliases and paired `on-*` roles.
3. Shared workspace panels and metric cards use tonal surface layering without
   a visible border or elevation shadow. Outlines remain available for inputs,
   focus, dialogs, and controls where a target boundary is needed.
4. Navigation icons do not receive an outline by default. The active item uses
   the primary container and its paired content role; the icon receives primary
   only in the active state.
5. No workspace may introduce a literal color, gradient, or one-off accent to
   signal a business domain. New visual states require a semantic role mapping,
   accessible text, and a contract test.

## Alternatives considered

| Option | Decision | Reason |
| --- | --- | --- |
| Keep every badge filled and every card outlined | Rejected | Preserves the clutter and makes routine information look urgent. |
| Remove all status styling | Rejected | Loses scannability for approval, failure, and evidence states. |
| Assign one accent per workspace | Rejected | Encodes taxonomy as decoration and creates an unbounded palette. |
| Semantic state fills plus quiet tonal surfaces | Selected | Preserves operational meaning while restoring a clear primary task hierarchy. |

## Invariants and verification

- Default badges remain neutral; success, warning, and danger are explicit
  classes.
- Filled roles use paired `on-*` tokens.
- Shared panels have no stacked border/shadow treatment in the final shell
  precedence layer.
- `materialExpressiveShellContract.test.ts` proves the token contract.
- `make web-check`, `make web-color-contract`, and browser checks remain
  required before merge.

