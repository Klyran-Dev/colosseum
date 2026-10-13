# ADR-0498: Metronic branded authentication surface

## Status

Accepted — 2026-10-11

## Context

The licensed Metronic 9.5.0 package provides a branded authentication layout:
a focused card paired with a bounded product-context panel. The current
authentication route is functionally correct but visually reads as a solitary
legacy form and does not carry enough product context for an operator deciding
whether to continue.

## Decision

Adapt Metronic's branded auth composition into the existing React/Vite route.
Keep one email/identity action, one OTP verification step, the existing safe
return path, and server-owned role copy. Add a non-interactive context panel
that explains evidence, tenant access, and role assignment. Use semantic
repository tokens for all surfaces and retain a stacked mobile layout.

## Alternatives considered

- Keep the solitary card: rejected because it lacks the licensed branded
  composition and leaves large desktop space unused.
- Add a second sign-in form in the context panel: rejected because it increases
  cognitive load and duplicates the global identity action.
- Copy Metronic's background assets: rejected because the product does not need
  an external decorative asset to explain a money-moving workspace.

## Invariants

- The context panel never accepts credentials or chooses a role.
- Tokens remain memory-only and are never rendered as display data.
- The same email OTP/OIDC state machine and safe return path are preserved.
- The surface remains keyboard accessible, responsive, and reduced-motion safe.

## Verification

- Auth-page tests assert the branded context landmark and existing journey
  behavior.
- Stylesheet budget, TypeScript, accessibility, and browser auth journeys
  remain green.
