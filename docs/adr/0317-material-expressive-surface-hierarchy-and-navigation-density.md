# ADR-0317: Material 3 Expressive surface hierarchy and navigation density

- Status: Accepted
- Date: 2026-10-04
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React SPA shell, Material 3 Expressive theme tokens, primary navigation

## Context

The product shell is an operational SaaS workspace with high-consequence tax
and money-movement actions. The previous shell used semantic color roles, but
several surfaces were visually similar and the navigation presented too many
equal-weight choices. That increased scan time and made unavailable or
sensitive work appear more prominent than the current task.

Material 3 uses design tokens and tonal surface roles to create hierarchy;
expressiveness is not a reason to add arbitrary colors, gradients, or
decorative contrast. The shell also needs keyboard access, compact responsive
behavior, and server-owned authorization semantics.

## Decision

1. Keep the repository-owned Solana Aurora palette as the only production
   palette. Extend it with `surface-container-lowest`, `surface-dim`, and
   `surface-bright`, with explicit light and dark values.
2. Use semantic surface roles for shell layers: the sidebar uses the lowest
   container, the top bar uses the base container, form controls use the low
   container, and elevated menus use the high container. Components may not
   introduce a new color literal or tenant-provided state color.
3. Use one primary navigation item per workspace with a visible label and one
   active treatment. Descriptions, access explanations, and risk semantics
   remain available to assistive technology and in account/access surfaces;
   they are not repeated as visual badges beside every item.
4. Preserve the four task groups and existing workspace URLs. Density is
   reduced through spacing, tonal separation, and progressive disclosure—not
   by removing a production workflow or weakening authorization.
5. Keep motion token-based and optional. Reduced-motion users receive the same
   hierarchy and state meaning without transforms or animated decoration.

## Alternatives considered

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Add one-off colors per card and status | Fast visual variation | Random palette drift, state ambiguity, contrast regressions | Rejected |
| Use gradients and translucent glass layers | Expressive screenshot appeal | Weakens surface hierarchy and increases visual noise | Rejected |
| Hide lower-frequency workspaces permanently | Cleaner first view | Breaks discoverability and production coverage | Rejected |
| Add a separate micro-frontend navigation shell | Independent ownership | Duplicate tokens and inconsistent authorization context | Rejected |
| **Use the complete M3 surface hierarchy plus compact semantic navigation** | Deterministic, accessible, responsive, lower scan cost, preserves workflows | Requires token and browser-contract updates | **Selected** |

## Invariants and failure policy

- The palette contract contains the selected surface roles in both schemes and
  validates their foreground contrast.
- Production CSS references semantic roles; raw component color literals,
  decorative gradients, and unapproved status colors fail the contract.
- Navigation labels and workspace paths remain stable unless a new ADR changes
  the information architecture.
- Theme preference changes presentation only; it cannot change policy, tax,
  account, wallet, or settlement behavior.
- If a role or theme token is missing, the build fails rather than falling back
  to a browser default or arbitrary component color.

## Verification

- `make material-palette-contract` checks the fixed light/dark role matrix and
  contrast, including the surface hierarchy.
- `make web-color-contract` checks semantic component usage and rejects raw
  colors or gradients outside the token declarations.
- React and browser tests cover stable navigation labels, theme switching,
  responsive layout, RTL, reduced motion, focus visibility, and workspace
  routes.
- Manual release review still verifies operator scanability and assistive
  technology behavior; automated tests are not a substitute for that review.

## References

- [Material Design 3](https://m3.material.io/)
- [Material foundations and design tokens](https://m3.material.io/foundations/)
- [ADR-0311: Solana Aurora Material 3 role palette](0311-solana-aurora-material-palette.md)
- [ADR-0310: Tonal surfaces and no decorative gradients](0310-tonal-surfaces-no-decorative-gradients.md)
