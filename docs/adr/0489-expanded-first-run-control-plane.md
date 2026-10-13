# ADR-0489: Expanded first-run control plane

## Status

Accepted — 2026-10-10

## Decision

The desktop product shell opens with the navigation rail expanded. Compact mode remains available as an explicit operator preference, but it is not the first-run experience.

The shell uses a denser SaaS control-plane composition: an expanded labeled rail, a stable top toolbar, compact verified context cards, and task content below the context row. A signed-out protected workspace explains the verified access sequence in the gate itself so the user has a clear next step without introducing a second sign-in control.

## Why

An icon-only rail hid workspace names and made the first screen feel like an empty prototype. Oversized context cards consumed the page without advancing the operator's task. The protected state also explained the restriction but did not explain the path to usable content.

## Alternatives considered

- Keep compact navigation as the default: rejected because first-run discoverability and orientation were poor.
- Remove the context cards: rejected because deployment, network, API, and access state are real operational context and must remain visible.
- Add another sign-in button inside the gate: rejected because it duplicates the global authentication control and increases cognitive load.
- Add a fake dashboard with activity or financial KPIs: rejected because the shell has no authoritative records for those values and the repository forbids mock production state.

## Invariants

- Workspace labels remain visible by default on desktop and remain available to assistive technology in compact mode.
- Context cards contain only runtime or authorization facts already owned by the shell.
- Protected actions remain unmounted until server authorization is confirmed.
- The access guide describes the existing email/OIDC flow and does not claim successful authentication.
- All visual rules consume repository-owned semantic Material roles.

## Verification

- Component tests cover expanded first-run navigation and protected-workspace steps.
- Full browser, accessibility, and responsive gates remain required before release.
