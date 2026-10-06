# ADR-0371: Production-source Material color boundary

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React/TypeScript production source and Material 3 Expressive palette governance

## Context

The stylesheet contract already restricts component CSS to the versioned
Material 3 Expressive semantic role matrix. A React component could still
bypass that boundary with an inline style, CSS-in-JS string, SVG presentation
attribute, or generated class containing a raw color or gradient. Such a
bypass would create a second palette, make light/dark/forced-colors review
incomplete, and allow data or error state to acquire an accidental visual
meaning.

## Decision

Extend `scripts/verify_web_color_contract.sh` to scan production `.ts` and
`.tsx` files under `apps/web/src`. Test files remain outside this scan so they
can assert token values and contract failures. Production source must not
contain raw hex, RGB/HSL functions, or gradient functions; all presentation
must use CSS classes and repository-owned Material semantic roles.

The contract remains a build/review guard, not a runtime authorization or
financial invariant. The existing stylesheet token declarations remain the
only place for scheme-specific values, and the existing palette, contrast,
forced-colors, browser, and accessibility gates remain required.

## Alternatives considered

| Alternative | Coverage | Drift risk | Decision |
| --- | --- | --- | --- |
| Continue scanning CSS only | Misses inline/CSS-in-JS bypasses | High | Rejected |
| Ban all `style` props with a broad text rule | Noisy and incomplete; can reject layout values | Medium | Rejected |
| Add a component-library color dependency | More runtime and ownership surface | Medium | Rejected |
| **Scan production TypeScript/TSX for raw/decorative colors and keep tests exempt** | **Complete local source boundary with low runtime cost** | **Low** | **Selected** |

## Invariants

- Production React/TypeScript source cannot introduce a second color palette.
- Tests may contain expected token values but cannot weaken the production
  stylesheet or source contract.
- Theme, forced-colors, reduced-motion, authorization, tax, wallet, and
  settlement behavior remain independent of presentation.
- A new visual role requires a token update, contrast review, ADR update, and
  contract evidence; it cannot be introduced as a component-local literal.

## Verification

- `make web-color-contract`
- `make material-palette-contract`
- `make web-check`
- `make web-browser-check`
- temporary production-source probe is rejected by the contract with a
  non-zero result, and the probe is removed after verification.

