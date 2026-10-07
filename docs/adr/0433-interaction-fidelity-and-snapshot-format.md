# ADR-0433 — Interaction fidelity, snapshot format, and honest capture

- Status: Accepted. Supersedes the "forced states approximate real interaction"
  limit in ADR-0431 and the settle-cap and format-boundary limits in ADR-0432.
- Date: 2026-10-07
- Decision owners: Principal UI Architecture

## Context

Forced pseudo-state snapshots (ADR-0431) were an approximation: nothing proved
they equal what a user gets from real pointer and keyboard interaction, or that
transitions use the spring tokens. Two capture weaknesses were also silent: a
page that never stopped changing was captured anyway at the settle cap, and
snapshots from different capture versions could be compared.

## Decision

1. **Prove the approximation.** `e2e/interaction-fidelity.spec.ts` is part of the
   normal real-browser gate. For six representative controls (navigation item,
   two topbar chips, account trigger, a text field, and a select) it performs real mouse hover, a real press
   (released off the control so it is not a click), and real keyboard focus with
   transitions running, waits for them to finish, and requires the computed style
   to equal the forced-state style, and the resting style to return after leaving.
2. **Fix what it found.** A real press also hovers the control, and real keyboard
   focus matches `:focus` as well as `:focus-visible`; forcing only the named
   pseudo-class under-reported both (the pressed state layer was missing). The
   snapshot helper now forces `hover+active` and `focus+focus-visible`
   (`forcedClassesFor`), and also records `:focus-within` for container groups.
3. **Prove the motion tokens.** The same spec asserts controls transition with the
   fast-effects and fast-spatial spring durations and `linear()` easing, and that
   reduced motion leaves no control transition.
4. **Version the capture.** Snapshots carry `format` (now 3). The diff tool exits
   with a clear message when asked to compare different formats instead of
   reporting noise, and reports any capture that hit the 4 s settle cap as
   unsettled instead of silently comparing it.

## Evidence

- 57 real-browser tests pass (49 journeys, 6 fidelity, 2 motion) with the
  corrected forced states; 408 unit tests, including the diff tool's format and
  unsettled cases.
- Before the fix the fidelity test failed on the pressed state for three
  controls with a missing 8% hover layer, which is how the gap was found.

## Limits

- Six controls are checked in real interaction; other controls share the same
  state-layer and field rules, which the snapshots then cover in forced form.
- Real touch input is covered by ADR-0437; multi-pointer and drag are not exercised.
- Snapshots from before format 3 cannot be compared; take a fresh baseline.
