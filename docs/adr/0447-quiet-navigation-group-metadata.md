# ADR-0447 — Quiet navigation group metadata

- Status: Accepted
- Date: 2026-10-09
- Decision owners: Principal UI Architecture

## Context

The navigation rail displayed a workspace count and a literal `+` or `−` for
every group. The count is useful context for assistive-technology users, but
the repeated symbols and numbers compete with the task labels in a dense
operator shell.

## Decision

Keep the group count in the disclosure's accessible name and retain the native
`details` semantics. Hide the duplicate visual count and use a small semantic
chevron as the visual disclosure affordance. This changes presentation only;
group membership, single-open behavior, keyboard access, and navigation routes
remain unchanged.

## Alternatives considered

- **Keep the count and text symbols:** Rejected. The metadata is repeated
  visual noise in the primary task-navigation rail.
- **Remove counts entirely:** Rejected. Counts help communicate scope to
  assistive-technology users and remain useful in a collapsed or mobile menu.
- **Replace native disclosures with custom state buttons:** Rejected. It adds
  keyboard and state-management complexity without improving task discovery.

## Evidence

- `productShell-ui.test.tsx` proves the count remains in the accessible group
  name while the rendered count is visually hidden.
- The Docker browser gate covers keyboard navigation, touch navigation, RTL,
  forced colors, reflow, and interaction fidelity for the shell.
