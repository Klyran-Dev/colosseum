# ADR-0439 — Cross-engine browser suite

- Status: Accepted
- Date: 2026-10-07
- Decision owners: Principal UI Architecture

## Context

The supported-browser baseline (Chrome/Edge 113+, Firefox 113+, Safari 17.2+,
ADR-0436) was enforced by the Vite `cssTarget` and a stylesheet test, but the product
had only ever been run in Chromium. The stylesheet depends on `color-mix()`, container
queries, `:is()`, and `linear()` easing, so engine behaviour is a real risk.

## Decision

`playwright.cross-browser.config.ts` and `e2e/cross-browser.spec.ts` run the product
in real Firefox and WebKit through `scripts/web_cross_browser_check.sh` (the existing
Playwright container ships all engines). The suite checks that the engine supports each
CSS feature used; the app renders with no page or console errors and resolves palette,
active-navigation and corner tokens in both themes; spring transitions run and hover
paints a state layer while reduced motion removes transitions; and four workspaces meet
WCAG 2.2 AA in both themes and fit 320px. It is skipped under Chromium, so the normal
gate is unchanged.

## Evidence

16 of 16 tests pass (8 per engine). The first run found one failure, caused by the
shared config's reduced-motion emulation correctly disabling transitions; the test now
opts into motion.

## Limits

- Playwright's Firefox and WebKit builds are current releases, not the oldest versions
  in the baseline, and not real iOS Safari; this proves engine behaviour, not every
  supported version.
- Four workspaces are sampled, not all sixteen; the full interaction, touch, and
  snapshot suites remain Chromium-based.
