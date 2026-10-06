# ADR-0353: Material 3 Expressive Aurora v2 shell

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React SPA theme roles, shell surfaces, navigation, state presentation

## Context

The current product shell already rejects raw component colors and decorative
gradients, but its hand-tuned blue surface values, repeated precedence layers,
and many low-opacity blends make unrelated surfaces look equally important.
Operators must scan high-consequence tax and settlement workflows quickly. A
production shell needs expression through a coherent role system, not through
one-off card colors or status decoration.

Material 3 Expressive is token-led: color, shape, typography, and motion are
system relationships. The current product also must retain dark, light, and
system themes, WCAG contrast, RTL, forced colors, reduced motion, and truthful
authorization states.

## Decision

1. Keep the Solana Aurora seed `#8DF0CE`, but replace the v1 role matrix with a
   v2 matrix whose surfaces are neutral-tonal and whose primary, secondary,
   tertiary, warning, and error roles are the only approved semantic accents.
2. Add explicit success, attention, blocked, and supporting container aliases.
   Components use these aliases instead of inventing opacity blends for state.
3. Use the M3 surface hierarchy for the shell: the sidebar and canvas use
   low-tonal surfaces, the top bar uses the base surface, panels use the low
   container, and menus use the high container. Outlines are reserved for
   boundaries and focus, not decorative card stacking.
4. Keep the current information architecture and authorization semantics. The
   active workspace is the only filled navigation treatment; access and risk
   explanations remain progressive-disclosure or assistive context.
5. Use expressive shape contrast sparingly: large workspace panels, compact
   navigation pills, and bounded control groups. No gradients, arbitrary
   workspace hues, tenant-supplied colors, or visual state inferred from API
   failure.

## Alternatives considered

| Alternative | Benefit | Risk | Decision |
| --- | --- | --- | --- |
| Keep v1 blue-heavy surfaces and tune individual cards | Small diff | Preserves competing emphasis and palette drift | Rejected |
| Add a color per workspace/status | Immediate visual differentiation | Random hues, state confusion, contrast and localization burden | Rejected |
| Adopt a third-party component theme | Faster component coverage | Adds dependency and token ownership drift; weakens SPA control | Rejected |
| Use runtime dynamic color from the browser or tenant | Personalized appearance | Non-deterministic evidence/support screenshots and unsafe status semantics | Rejected |
| **Use one seed-derived M3 role matrix with neutral tonal surfaces and semantic state containers** | **Deterministic, accessible, expressive, lower scan cost, preserves all workflows** | **Requires a versioned palette contract and visual review** | **Selected** |

## Invariants and failure policy

- Theme preference affects presentation only; it cannot change authorization,
  tax, wallet, settlement, evidence, or route behavior.
- Every foreground/background pair used by the role matrix meets the repository
  contrast contract.
- Raw component color literals, gradients, and data-dependent hues fail the
  stylesheet contract.
- A missing role fails the build instead of falling back to an arbitrary
  browser default.
- Forced-colors and reduced-motion modes retain state meaning and focus order.
- The browser never interprets a color as proof of access, finality, legal
  compliance, or settlement success.

## Verification

- `make material-palette-contract`
- `make web-color-contract`
- `pnpm --dir apps/web test -- materialExpressiveShellContract productShell-ui`
- `make web-check`
- `make web-browser-check`
- Manual review at desktop, narrow, light, dark, RTL, reduced-motion, and
  forced-colors settings remains required before a release claim.

## References

- [Material Design 3](https://m3.material.io/)
- [Material foundations and design tokens](https://m3.material.io/foundations/)
- [ADR-0298: Material 3 Expressive visual system](0298-material-3-expressive-visual-system.md)
- [ADR-0317: surface hierarchy and navigation density](0317-material-expressive-surface-hierarchy-and-navigation-density.md)
