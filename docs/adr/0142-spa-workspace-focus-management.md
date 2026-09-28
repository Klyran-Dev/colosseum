# ADR-0142: SPA workspace focus-management contract

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Product Design, Frontend
- Scope: React/Vite workspace navigation and keyboard/screen-reader behavior

## Context

The SPA switches between quote, preview, governance, replay, vault, and
checkout workspaces without a document navigation. A pointer user can see the
new panel, but a keyboard or screen-reader user can remain focused on the
navigation control with no reliable announcement or route landmark. WCAG
compliance requires a predictable focus path for this application-state change.

## Decision

Add a persistent skip link to the main workspace landmark, expose the workspace
navigation controls with active state and an explicit controlled target, and
move focus to the workspace content container after a user-triggered workspace
change. The initial page render does not steal focus. The focus target uses
`tabIndex=-1`, preserving normal tab order while giving assistive technology a
stable context boundary.

The behavior remains client-only presentation state. It does not alter API,
tax, wallet, settlement, or authorization authority.

## Alternatives considered

### Leave focus on the clicked navigation control

Rejected. It provides no deterministic context change for keyboard or screen-
reader users and makes the new workspace difficult to discover.

### Focus the first form control in each workspace

Rejected. Lazy workspaces have different loading/error/empty states, and
focusing a control can skip important heading and status context.

### Use a full client-side router solely for focus management

Rejected for this static SPA slice. It adds routing state and lifecycle cost
without improving the bounded workspace transition contract.

### Selected: skip link plus stable workspace focus boundary

Selected because it is deterministic across lazy loading and degraded states,
works with keyboard and assistive technology, and keeps navigation state
separate from financial authority.

## Invariants and evidence

- Initial render never steals focus from the browser or assistive technology.
- Every workspace navigation control exposes its active state and controls the
  same workspace landmark.
- A workspace change focuses the landmark exactly once after React commits the
  new state.
- The automated accessibility test proves the skip target and focus transition;
  real-browser, responsive, RTL, localization, and assistive-technology review
  remain separate readiness gates.
