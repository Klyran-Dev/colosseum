# ADR-0405: Material theme-color synchronization

- Status: Accepted
- Date: 2026-10-06
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: Browser chrome for the React SPA

## Context

The application surface is governed by the Material 3 Expressive Aurora v2
role matrix in `styles.css`, but the document metadata still used an older
literal browser `theme-color`. That made the tab or mobile browser chrome
visually disagree with the selected light/dark surface and left a second,
unapproved palette boundary in production source.

## Decision

Use only the approved M3 background roles for browser `theme-color`:

- dark: `--md-sys-color-background` value `#0F1014`;
- light: `--md-sys-color-background` value `#FAF8FC`.

The SPA synchronizes the metadata after boot and whenever the explicit theme
preference changes. The two HTML metadata entries carry the approved values
and `prefers-color-scheme` media queries; the SPA changes only their bounded
media conditions. For `system`, the browser follows OS theme changes natively.
The initial dark entry is the approved pre-boot fallback; no arbitrary or
component color may be used. This metadata is presentation-only and has no
effect on authorization, tax calculation, signing, routing, or persistence.

## Alternatives considered

| Alternative | Palette consistency | First-paint behavior | Complexity | Decision |
| --- | --- | --- | --- | --- |
| Keep the legacy hard-coded color | Fail; not in the M3 role matrix | Pass | Low | Rejected |
| Remove `theme-color` metadata | Pass | Browser-defined and inconsistent | Low | Rejected |
| Reuse a component accent color | Fail; wrong semantic role | Pass | Low | Rejected |
| **Approved M3 background metadata plus bounded media synchronization** | **Pass** | **Pass with safe fallback** | **Low and testable** | **Selected** |

## Invariants and budgets

- Only the two approved M3 background values exist in the HTML metadata
  boundary; TypeScript cannot introduce another color literal.
- Theme resolution is an in-memory O(1) operation with no network, storage,
  RPC, or database dependency.
- System-theme changes are handled by the browser's media-query evaluation;
  no polling or application listener is required.
- Unsupported preference values resolve to the dark fallback and cannot create
  a new color.
- Theme metadata changes cannot mutate authorization, financial, wallet, or
  route state.

## Verification

- Unit tests cover explicit light/dark media conditions and system media-query
  resolution.
- The Material source contract rejects the legacy metadata color and checks
  both approved values.
- The Docker/NGINX browser gate remains required for rendered theme journeys.
