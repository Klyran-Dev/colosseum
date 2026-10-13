# ADR-0517: Use bundled Plus Jakarta Sans and a restrained auth type scale

- Status: Accepted
- Date: 2026-10-11
- Owners: Web platform and security engineering

## Context

The new admin sign-in surface was visually legible but too display-heavy at
the first responsive breakpoint. The heading occupied too much vertical space
and the `Inter` declaration was only a fallback because the font was not
shipped with the SPA. That made the rendered hierarchy depend on the operating
system and reduced consistency across browsers.

## Decision

Bundle `@fontsource-variable/plus-jakarta-sans` and use it as the admin
surface's body and display stack, with the system sans stack as a resilient
fallback. Keep form controls at 1rem to preserve WCAG readability and prevent
mobile browser auto-zoom. Use a 2rem auth heading below the desktop
breakpoint and increase it to 2.75rem only when the two-column layout is active.

The auth supporting copy uses a 0.9375rem / 1.5rem rhythm, and the display
weight is reduced from `font-black` to `font-extrabold` so the hierarchy stays
strong without the previous heavy block shape. The card has no forced viewport
height below the desktop breakpoint; full-height split composition is reserved
for larger screens.

## Alternatives considered

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Keep unbundled `Inter` with the existing scale | No dependency or layout change | Rendering varies by host and the heading remains oversized | Rejected |
| Load a remote Google Font at runtime | Easy visual iteration | Adds a third-party request, privacy/performance dependency, and failure variance | Rejected |
| Use the system UI font only | Zero asset cost and good platform integration | Cross-platform metrics and brand character vary | Rejected |
| **Bundle Plus Jakarta Sans with a responsive auth scale** | Consistent modern geometry, deterministic delivery, compact mobile hierarchy, resilient fallback | Adds a small font asset and requires a typography contract | **Selected** |

## Acceptance criteria

- The font is packaged into the Vite output and does not require a runtime CDN.
- The admin body uses the semantic font token and preserves a 1rem base size.
- Auth headings are 2rem below 64rem and 2.75rem at or above 64rem.
- The auth shell uses content height below 64rem and a full-height split only
  at or above 64rem.
- The sign-in and sign-up flows retain the existing server-owned auth behavior.
- Typography contract, typecheck, production build, and browser smoke checks pass.
