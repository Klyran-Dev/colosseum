# ADR-0513: Restore Metronic Demo 3 workspace density

## Status

Accepted — 2026-10-11

## Context

The shell had the broad Demo 3 frame, but the workspace diverged from the
licensed composition in the most visible area: the status deck rendered as one
narrow, saturated card, while the protected-workspace explanation consumed the
remaining width and left a large empty stage. This made the application feel
like a custom dark dashboard instead of a Metronic Demo 3 product surface.

## Decision

Keep the existing repository-owned React shell and authorization semantics, but
restore the Demo 3 density model:

- retain the 58px icon rail, 58px topbar, 56px contextual navbar, rounded
  bordered content frame, and compact toolbar;
- render access, deployment, Solana network, and API service state as four
  equal status cards using server/runtime-owned values;
- keep the protected workspace gate as one full-width explanatory card below
  the status deck, with a compact right-side process list;
- reflow the status cards to two columns and then one column at narrow widths;
- use neutral tonal cards, restrained borders, and one primary access state
  accent instead of a large filled panel;
- keep the single global sign-in control and preserve all existing auth and
  authorization tests.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Keep the one-card status deck | Rejected | Leaves excessive dead space and does not match Demo 3's dashboard rhythm. |
| Copy the full Metronic CSS bundle | Rejected | Creates a second style authority and risks overriding repository-owned M3/accessibility contracts. |
| Add decorative illustrations or gradients | Rejected | Adds visual noise and does not communicate operational state. |
| **Use a repository-owned four-card Demo 3 status deck** | **Selected** | Matches the licensed composition while keeping state, security, and styling contracts local and auditable. |

## Invariants

- Status values are derived from runtime context, service state, and the
  authenticated authorization snapshot; the UI does not invent readiness.
- Protected actions remain unavailable until server authorization confirms them.
- The page has one actionable signed-out sign-in entry point.
- The layout remains keyboard accessible and reflows at 320px without
  horizontal page overflow.

## Verification

- Add a component regression asserting four status cards and three operational
  status kinds.
- Run the frontend shell tests, typecheck, stylesheet contract/budget tests,
  and a real-browser responsive review against the Docker-served SPA.
