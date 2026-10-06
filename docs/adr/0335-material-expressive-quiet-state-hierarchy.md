# ADR-0335: Material 3 Expressive quiet state hierarchy

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React SPA product shell color roles, selected navigation, and session-status presentation

## Context

The attached visual review showed a common failure mode in operational SaaS
interfaces: several correct states become visually loud when they are all
rendered as equally strong badges. A selected route, unavailable sign-in, and
system attention state are different kinds of information and must not compete
for the same visual weight. The product also needs a recognizable Material 3
Expressive theme without allowing component authors to invent one-off colors.

Material 3 Expressive is a system of roles, tonal surfaces, shape, type, and
purposeful interaction—not a license for arbitrary accent colors. The public
Material guidance describes expressive products in terms of vibrant color,
adaptive components, flexible typography, contrasting shapes, and motion; this
decision applies those ideas within a high-consequence financial workflow.

## Decision

1. Keep Solana Aurora as the only fixed theme family. Light, dark, and system
   preferences resolve the same semantic role names; they never select a
   component-specific hue or tenant-provided color.
2. Add semantic state aliases (`success`, `attention`, `blocked`, and
   `supporting`) that resolve to the governed Material roles. Component CSS
   consumes these aliases instead of reaching into unrelated color values.
3. Represent the selected navigation item with a tonal surface, a thin
   primary leading rail, and a primary icon. The selected row must not be a
   large high-chroma fill that competes with the active workspace.
4. Represent an unavailable sign-in configuration as a compact attention
   disclosure. The summary says “Sign-in setup needed” and uses the warning
   role; the detailed configuration error remains an error alert inside the
   disclosure. This keeps the failure visible without presenting configuration
   debt as a user-caused destructive alarm.
5. Preserve all authorization, runtime, and failure information in the DOM,
   accessible names, and progressive disclosures. This is a presentation-only
   change and cannot enable a route, approve a decision, or change a request.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep every state as a strong filled badge | Immediate visibility | Competing alarms, high scan cost, and weak distinction between selection and failure | Rejected |
| Add a separate accent color for each workspace | Fast visual differentiation | Random palette drift, contrast review burden, and state ambiguity | Rejected |
| Hide unavailable sign-in configuration | Cleanest top bar | Conceals a release/configuration failure and harms supportability | Rejected |
| **Use governed semantic aliases, tonal selection, and a compact attention disclosure** | Calm hierarchy, accessible failure visibility, deterministic themes, no authority change | Requires role contract and focused UI regression tests | **Selected** |

## Invariants and failure policy

- No component outside the token declaration blocks may add a color literal,
  gradient, or unreviewed state token.
- Theme selection changes appearance only and cannot influence authorization,
  tax calculation, wallet artifacts, or settlement behavior.
- A selected route remains keyboard reachable and keeps `aria-current="page"`.
- An unconfigured sign-in state remains discoverable and its detail remains a
  semantic alert when opened.
- Warning and error roles are not interchangeable: warning communicates
  attention/configuration, while error communicates a concrete failed or
  blocked condition.

## Verification

- Failing-first UI coverage asserts the compact “Sign-in setup needed” label
  and preserves the expanded error alert.
- `make web-check`, `make web-color-contract`, and
  `make material-palette-contract` remain required.
- `make web-browser-check` verifies light/dark/system themes, selected-route
  hierarchy, reduced motion, responsive behavior, RTL, and axe accessibility.
- Manual review remains required for operator usability, screen readers, and
  high-contrast platform settings before production release.

## References

- [Material Design 3](https://m3.material.io/)
- [Material foundations and design tokens](https://m3.material.io/foundations/)
- [ADR-0311: Solana Aurora Material 3 role palette](0311-solana-aurora-material-palette.md)
- [ADR-0323: Material 3 Expressive operational workspace composition](0323-material-expressive-operational-workspace-composition.md)
