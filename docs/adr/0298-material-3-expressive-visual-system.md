# ADR-0298: Material 3 Expressive visual system for the operational SaaS shell

## Status

Accepted — implemented and verified in the product shell and pinned browser
gate.

## Context

The product shell has grown into a high-information operations surface. Its
current dark treatment uses many unrelated literal colors, repeats the same
status in several places, and gives navigation, environment, authorization,
and financial work equal visual weight. This makes a real failure state look
like decoration and makes the primary operator task harder to scan.

The UI must remain production-grade and accessible while adopting Material 3
Expressive principles: role-based color tokens, tonal surfaces, expressive but
bounded shape, clear typography hierarchy, adaptive layout, and meaningful
motion. Material 3 Expressive is a visual system, not permission to add
arbitrary gradients, neon colors, or decorative animation.

## Decision

Adopt a repository-owned Material 3 Expressive token layer for the React/Vite
SPA. The system uses one Solana dusk seed family, with light and dark schemes
derived into explicit Material-style roles. Components consume semantic roles;
the complete stylesheet must not introduce ad-hoc hex colors outside the token
declaration blocks. ADR-0300 closes the legacy declaration migration and keeps
the final Material role cascade as a defense-in-depth boundary.

The initial role contract is:

- `primary`: the trusted action and selected-navigation accent;
- `secondary`: supporting controls and non-critical emphasis;
- `tertiary`: expressive accent for workspace identity and progress;
- `error`: blocked, unavailable, or failed state;
- `warning`: review, pending, or not-yet-verified state;
- `surface`, `surface-container`, and `surface-container-high`: page, card,
  and raised-card hierarchy;
- `on-*`, `outline`, and `outline-variant`: readable content and boundaries.

The shell follows these product rules:

1. The workspace title and next action are the strongest visual signals.
2. Deployment, network, service, access, and session state are grouped into a
   compact context area; repeated status text is removed from navigation rows.
3. Navigation shows the selected state and a quiet lock/availability affordance;
   detailed policy reasons remain in accessible names, tooltips, and the Access
   & approvals workspace.
4. Cards use tonal elevation and shape hierarchy instead of multiple colored
   borders. Color communicates state or action, never decoration.
5. Display settings expose `system`, `light`, and `dark` themes. The selected
   preference is safe presentation state and never affects authorization.
6. Motion is short, purposeful, and disabled under `prefers-reduced-motion`.
7. Financial and security copy remains plain language; protocol detail stays
   behind progressive disclosure.

The first implementation keeps the static NGINX delivery model and does not
introduce a component framework whose defaults would obscure the existing
accessibility and wallet boundaries. The token layer is CSS custom properties
with React-owned theme preference state, so the browser can switch schemes
without a network request or a second rendering runtime.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Continue literal per-component colors | Smallest code change | Drift, poor contrast review, duplicated meaning, hard-to-audit states | Rejected |
| Adopt an arbitrary third-party theme | Fast visual change | Theme defaults conflict with financial state semantics and add dependency surface | Rejected |
| Use Material 3 Expressive roles and a repository token contract | Consistent semantics, light/dark support, accessible contrast review, small runtime cost | Requires migration of existing styles and token governance | **Selected** |
| Introduce a full UI component framework | Broad component coverage | Large migration, CSS ownership ambiguity, wallet/failure behavior hidden behind abstractions | Deferred |

## Invariants and failure policy

- A color role must have one documented meaning across the shell.
- Error and warning states must remain distinguishable without color alone.
- Theme selection never changes capabilities, API requests, wallet artifacts, or
  settlement behavior.
- Missing runtime context remains visibly unconfigured in every theme.
- `prefers-reduced-motion` disables non-essential transitions and shape motion.
- Light and dark schemes must pass the rendered WCAG A/AA browser audit.

## Verification

- Component tests verify the theme control and `data-theme` boundary.
- The real-browser suite switches between light and dark schemes, proves their
  rendered shell surfaces differ, and verifies the selected scheme persists
  across every navigation workspace.
- Navigation rows render quiet availability marks instead of repeating access
  sentences; the description, decision, and policy reason remain bound through
  `aria-describedby` and the row tooltip, while the page context card carries
  the prominent access state.
- The local-development credential is a closed progressive disclosure in the
  top bar; opening it exposes the in-memory-only field and its environment
  warning without allowing that local affordance to appear in staging or
  production authentication modes.
- The web color contract rejects raw color literals in the final Material
  precedence layer and requires semantic role consumption; this makes the
  rendered shell deterministic while legacy selector migration continues.
- Full web tests, typecheck, production build, and the pinned real-browser
  accessibility journeys are required.
- Pull-request CI must run the CSP, semantic color-role, and plain-language
  contracts before the web test/build steps.
- Visual review covers wide, narrow, RTL, reduced-motion, signed-out, access
  error, and server-allowed states before production UX claims.

## References

- [Material 3](https://m3.material.io/)
- [Material 3 foundations](https://m3.material.io/foundations/)
