# ADR-0111: Automated accessibility contract for the React SPA

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Product Design, and Frontend
- Scope: React 19/Vite SPA accessibility regression gates

## Context

The product surface is a real operational UI for quotes, tax explanations,
governance, replay operations, vault release, and wallet authorization. Source
inspection can confirm that labels and landmark attributes exist, but it cannot
detect regressions in the rendered accessibility tree. A Tier-1 product needs
an executable baseline that fails CI when the browser-visible DOM introduces a
WCAG violation.

The repository must remain a static React/Vite SPA served by NGINX. The first
accessibility gate must be deterministic, fast enough for every pull request,
compatible with the pinned Node 24 toolchain, and separate from financial or
wallet authority. Automated checks are evidence, not a claim that legal
compliance or every assistive technology has been certified.

## Decision

Use `@testing-library/react` to render the primary quote workspace and each
lazy operational workspace (governance, replay operations, vault release,
and checkout authorization) in the `happy-dom` Vitest environment. Use
`axe-core` to audit each rendered document for the WCAG 2.0 A and AA rulesets.
The test stubs only the health/request boundary needed for initial rendering;
it does not fake quote success, wallet signing, tax results, or settlement
state. The test runs through the normal frontend test command and has an
explicit CI step so it remains visible as a release gate.

The baseline asserts zero axe violations and supplements the existing unit
contracts for API errors, timeout/cancellation, wallet artifact validation,
governance parsing, and replay operations. Future workspace additions must add
rendered accessibility cases before the broad WCAG 2.2 AA readiness item can
be closed.

## Alternatives considered

### Source-level JSX assertions only

Rejected. They cannot evaluate the rendered accessibility tree, duplicate
labels, focusable controls, or landmark/name relationships.

### Browser Playwright audit as the only per-PR gate

Deferred as the sole first gate. It gives stronger CSS/layout and real-browser
coverage but adds browser binary lifecycle, startup cost, and environment
variance. It remains required for full keyboard, focus, responsive, reduced
motion, RTL, and live-provider journey evidence.

### Axe DevTools manual review only

Rejected. Manual review is valuable for judgment and assistive-technology
coverage but cannot provide deterministic merge protection.

### Disable rules until the visual design stabilizes

Rejected. Accessibility is a product invariant and must regress visibly in CI,
not become a late hardening phase.

## Consequences

- The primary and lazy operational surfaces have a repeatable rendered WCAG
  A/AA regression gate.
- Test-only DOM and network fixtures cannot become production authority paths.
- The dependency footprint grows by bounded, dev-only test packages.
- Axe does not prove WCAG 2.2 AA conformance, keyboard behavior, color contrast
  under every theme, RTL correctness, or screen-reader quality by itself.

## Verification and release impact

- `pnpm --dir apps/web exec vitest run src/accessibility.test.tsx` passes.
- The normal frontend suite and an explicit CI accessibility step pass.
- The broad readiness item remains open until live browser journeys have
  keyboard/focus/responsive/localization evidence.
