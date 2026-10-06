# ADR-0326: Material 3 Expressive semantic state hierarchy for navigation

- Status: Accepted
- Date: 2026-10-04
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React SPA product shell navigation and state communication

## Context

The product shell must communicate route identity, current location, access
state, and action sensitivity without making every navigation row look like an
alert. Repeating policy marks and risk dots creates a dense, high-chroma rail;
it also encourages operators to treat presentation hints as authorization
decisions. The screenshot-driven review identified this as a Material 3
Expressive hierarchy problem, not a missing feature problem.

The server remains the authority for authorization. The UI may explain a
server decision, but it must never make a route appear usable because of a
color, icon, or local calculation.

## Decision

1. Navigation rows show only route identity and the current route. The current
   route is communicated with `aria-current="page"` and the existing tonal
   selected container; no policy badge or risk dot is rendered as a visual
   navigation affordance.
2. Every row retains an accessible description and native `title` containing
   the route description, server-owned access label/reason, and action-risk
   label. This keeps the information available to screen readers, keyboard
   users, and the Access & approvals workspace without adding persistent
   visual noise.
3. The palette remains semantic: primary roles identify selection and safe
   progression, warning roles identify attention, error roles identify a
   blocked or failed state, and tertiary roles provide supporting information.
   No component may introduce a new state color or raw color literal.
4. State detail belongs in the current page context, System status, and Access
   & approvals disclosures. Navigation is a locator, not an authorization
   dashboard.
5. Responsive layouts preserve the same information hierarchy. Collapsing the
   sidebar or opening the mobile navigation must not reintroduce per-row state
   badges.
6. Each workspace uses one fixed, monochrome semantic outline glyph from the
   product icon set. Glyph identity is part of the navigation contract, not a
   status channel; it never changes with RBAC/PBAC/ABAC decisions, service
   state, tenant, or risk. The active route supplies the only persistent
   visual emphasis, while accessible row context carries the task, access
   reason, and action sensitivity.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep a policy icon and risk dot on every row | Immediate visual signaling | Repeated alerts, palette noise, and false implication that local UI grants access | Rejected |
| Show “Available / Not available” text on every row | Explicit for sighted users | Dominates the navigation rail, becomes stale-looking during checks, and duplicates Access & approvals | Rejected |
| Hide access/risk information entirely | Cleanest rail | Violates accessible explanation and operator support requirements | Rejected |
| **Keep information in accessible row context and show only route/current state visually** | Calm scan path, complete semantics, deterministic rendering, low runtime cost | Detail requires focus, title inspection, or the dedicated access workspace | **Selected** |

## Invariants

- A route is never enabled, authorized, or settled by CSS or a client-side
  visual state.
- Every route remains in the DOM, keyboard reachable when its group is open,
  and labelled by its human task name.
- The active route keeps `aria-current="page"` and a role-based selected
  surface with sufficient contrast in light and dark themes.
- Access label, decision reason, and action-risk semantics remain available in
  the row's accessible description and `title`.
- No raw color, gradient, or one-off status token is added to the shell.
- Every workspace has a stable semantic glyph with no data-dependent color or
  status meaning.
- Theme, density, and navigation presentation cannot affect API requests,
  tax results, wallet artifacts, or settlement authorization.

## Verification

- Add a failing UI contract that asserts no policy/risk badge is rendered in
  the navigation rail, that each workspace has a stable semantic glyph, and
  that the accessible description retains access and risk semantics.
- Run the React UI suite, typecheck/build, `make web-color-contract`, and the
  Material palette contract.
- Review wide, narrow, light, dark, system, RTL, keyboard-only, screen-reader,
  and reduced-motion states before release evidence is accepted.

## References

- [ADR-0298: Material 3 Expressive visual system](0298-material-3-expressive-visual-system.md)
- [ADR-0311: Solana Aurora Material 3 role palette](0311-solana-aurora-material-palette.md)
- [ADR-0317: Material 3 Expressive surface hierarchy and navigation density](0317-material-expressive-surface-hierarchy-and-navigation-density.md)
- [ADR-0323: Material 3 Expressive operational workspace composition](0323-material-expressive-operational-workspace-composition.md)
