# ADR-0311: Solana Aurora Material 3 role palette

## Status

Superseded by [ADR-0422](0422-material-baseline-palette-and-expressive-tokens.md) (palette only).


Accepted — the fixed light and dark role matrix is implemented in the SPA
tokens and guarded by the palette contract. Rendered visual review and manual
assistive-technology review remain release gates.

## Context

The shell had correct Material role names and contrast checks, but the role
values were a generic lavender/pink family. That made the product identity
inconsistent with a Solana operational control plane and left the source of
the palette implicit. “No random colors” requires a named theme family, a
declared seed, explicit role ownership, and a deterministic review boundary.

The application is a static React/Vite SPA served by NGINX. It must not fetch
theme state, calculate colors in the browser, or let a tenant-provided color
change the meaning of financial, authorization, warning, or error states.

## Decision

Adopt the repository-owned **Solana Aurora** theme family:

- palette seed: `#8DF0CE` (Aurora Mint);
- dark operational surfaces: Solana navy tones with mint primary actions and
  blue tertiary emphasis;
- light operational surfaces: cool white/blue-gray tones with deep mint
  primary actions and accessible blue tertiary emphasis;
- warning and error remain dedicated semantic roles and are never repurposed
  as decoration;
- every role has an explicit light/dark value in `styles.css`, and the
  dependency-free palette contract checks the exact role matrix plus WCAG
  contrast;
- theme selection remains presentation-only and cannot influence authorization,
  tax calculation, wallet artifacts, or settlement behavior.

The role matrix is intentionally fixed at build time. Material 3 Expressive
comes from semantic color, tonal surfaces, typography, shape, adaptive layout,
and purposeful motion; runtime randomization is not appropriate for a
high-consequence financial workspace.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep the existing lavender/pink family | No migration cost | Generic identity and no documented palette source | Rejected |
| Generate dynamic colors in the browser | Highly personalized and flexible | Runtime dependency, non-deterministic evidence, and tenant colors can weaken state semantics | Rejected |
| Accept tenant-provided arbitrary theme colors | Strong branding flexibility | Accessibility, state meaning, support, and auditability cannot be guaranteed | Rejected |
| **Use a fixed Solana Aurora role matrix with an explicit seed** | Deterministic, brand-consistent, accessible, low latency, easy to audit | Requires coordinated palette changes and contract maintenance | **Selected** |

## Invariants and failure policy

- Only the documented Solana Aurora role matrix may be used in the production
  token blocks.
- Every required role pair passes the repository contrast threshold.
- No component can introduce a color literal or override a state role locally.
- Light/dark/system selection changes presentation only.
- A malformed or unexpected palette fails the build contract; it is never
  silently substituted with a browser or tenant default.

## Verification

- The palette contract is updated failing-first to require the named theme and
  exact role matrix.
- `make material-palette-contract` verifies both role values and contrast.
- `make web-color-contract` verifies component role usage and no gradients or
  raw component colors.
- `make web-check` and `make web-browser-check` verify theme switching,
  responsive layout, RTL, reduced motion, keyboard behavior, and rendered
  accessibility states.

## References

- [Material Design 3](https://m3.material.io/)
- [Material foundations and design tokens](https://m3.material.io/foundations/)
- [ADR-0298: Material 3 Expressive visual system](0298-material-3-expressive-visual-system.md)
- [ADR-0304: Static contrast contract](0304-material-role-contrast-contract.md)
- [ADR-0310: Tonal surfaces and no decorative gradients](0310-tonal-surfaces-no-decorative-gradients.md)
