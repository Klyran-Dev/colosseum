# ADR-0437 — Hover only where hover exists, and real touch tests

- Status: Accepted. Supersedes the "real touch input is not exercised" limit in
  ADR-0433.
- Date: 2026-10-07
- Decision owners: Principal UI Architecture, Accessibility

## Context

Hover styling applied unconditionally. On a touch device the browser keeps `:hover`
on the element that was last tapped, so a tapped control stays visually "hovered"
until another element is touched. Nothing in the real-browser gate used touch input
or the mobile layout's accessibility.

## Decision

1. **Guard every `:hover` rule** with `@media (hover: hover)`, so state layers and
   hover outlines exist only on devices whose primary input can hover. The one rule
   that grouped `:hover` with `:focus-visible` was split so keyboard focus styling is
   never hover-gated. A contract test requires every `:hover` rule to sit inside that
   media query and rejects mixed selector lists.
2. **Test real touch.** `e2e/touch.spec.ts` uses touch and mobile emulation at
   375px: the device reports `hover: none` and a coarse pointer; taps open the mobile
   navigation and change workspace; a tap leaves no hover or pressed style on the
   topbar controls; and the mobile layout (navigation and popovers open) passes axe
   with the WCAG 2.2 AA rules in both themes.

## Evidence

- Negative control: with the unguarded hover CSS restored, the "tap does not leave
  hover styling behind" test fails; with the guard it passes.
- A full snapshot set (204 tests, all states including forced `:hover`) differs in 0
  elements from the previous run, so hover-capable devices are unchanged.
- 425 unit tests, 4 web contracts, and 77 real-browser tests pass.

## Limits

- Chromium touch emulation approximates device behaviour; real iOS and Android
  browsers, multi-touch gestures, and drag are not exercised.
- Pressed (`:active`) feedback remains on touch devices by design; only hover is
  gated.
