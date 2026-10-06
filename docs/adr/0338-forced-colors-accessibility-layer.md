# ADR-0338: Forced-colors accessibility layer for the Material shell

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React SPA presentation layer and browser accessibility evidence

## Context

The Solana Aurora Material 3 role palette provides governed dark, light, and
system themes, but operating-system high-contrast modes are an independent
accessibility boundary. Author-supplied tonal surfaces, subtle outlines, and
state aliases must not make navigation, focus, errors, approvals, or disabled
actions unreadable when a user enables forced colors.

Forced-colors support is presentation-only. It must not create a second
application palette, change authorization, alter tax or settlement behavior,
or persist a new product preference. The operating system remains responsible
for the final user-selected contrast mapping.

## Decision

Add a final `@media (forced-colors: active)` layer to the SPA stylesheet. It
maps the existing semantic application roles to system colors such as `Canvas`,
`CanvasText`, `ButtonFace`, `ButtonText`, `Highlight`, `HighlightText`,
`GrayText`, and `Mark`. The layer keeps `forced-color-adjust: auto`, removes
decorative elevation, preserves visible borders and focus outlines, and keeps
the existing semantic state distinctions without introducing literal hex/RGB
values.

The browser does not expose a high-contrast toggle in product settings. A
user's operating-system setting controls this mode; the application only
responds to the media query.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Rely on the Aurora palette only | No additional CSS | Does not guarantee system high-contrast readability | Rejected |
| Add a manually selected high-contrast product theme | Explicit user control | Duplicates OS behavior, adds persistence and another palette to audit | Rejected |
| Disable forced colors with `forced-color-adjust: none` | Preserves brand visuals | Can suppress user-selected accessibility contrast | Rejected |
| **Map semantic roles to OS system colors with `forced-color-adjust: auto`** | Honors user preference, preserves meaning, no second palette, low runtime cost | Brand tonal nuance is intentionally reduced in this mode | **Selected** |

## Invariants and failure behavior

- Forced colors never affect API requests, authorization, tax calculations,
  wallet signing, settlement state, or URL state.
- The layer contains no hex, RGB, gradient, or application-specific status
  color literals.
- Text, controls, panels, active navigation, error state, disabled state, and
  focus state retain visible system-color contrast.
- `prefers-reduced-motion`, RTL, narrow layouts, and native disclosure
  semantics remain unchanged.
- If the media query is unsupported, the governed Aurora light/dark/system
  palette remains the fallback.

## Latency and resource budget

- CSS-only media-query evaluation; no JavaScript listener, network request, or
  persisted state.
- No additional dependency or bundle chunk.
- Zero impact on the settlement, quote, tax, authorization, or wallet paths.

## Verification

- Add a failing-first static contract for the system-color layer and the
  absence of palette literals.
- Add a real-browser journey using Playwright forced-colors emulation to prove
  the semantic role remapping, focusability, and WCAG checks.
- Run the web unit suite, typecheck, production build, color contracts, and
  the full browser journey before release evidence is accepted.
- Retain manual screen-reader, OS accessibility, and operator usability review
  as production release gates.

## References

- [ADR-0298: Material 3 Expressive visual system](0298-material-3-expressive-visual-system.md)
- [ADR-0304: Material role contrast contract](0304-material-role-contrast-contract.md)
- [ADR-0335: Material 3 Expressive quiet state hierarchy](0335-material-expressive-quiet-state-hierarchy.md)
