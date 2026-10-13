# ADR-0487: Use verified context for dashboard density

Status: Accepted

Date: 2026-10-10

## Context

The operational shell needs a more contemporary SaaS composition inspired by
compact dashboard layouts, but it must not invent financial metrics, activity,
charts, or records when the current account has not supplied them. The
previous expanded navigation rail also consumed too much of the work area and
made signed-out protected workspaces feel empty.

## Decision

Open the desktop shell with the existing compact navigation rail. Keep every
workspace name, access reason, and risk description available through semantic
labels, focusable controls, and native disclosure behavior; restore the full
navigation drawer on narrow screens.

Add a four-card status deck beneath the workspace heading. Its values are
limited to already verified shell context: access posture, deployment
environment, Solana network, and API service state. Each card uses existing
localized operational messages and explains the current state without implying
that access or settlement authority has been granted.

Do not add mock KPIs, charts, client names, recent activity, transaction
totals, or fabricated performance data. Real workspace children remain the
source of domain content after authentication and authorization are confirmed.

## Alternatives considered

- Keep the expanded rail and add no density: rejected because the desktop
  canvas remains visually empty and the navigation dominates the composition.
- Add dashboard metrics and charts with fixture values: rejected because it
  would present non-authoritative data as live operational state.
- Use a compact rail with unlabeled icon-only controls: rejected because it
  weakens discoverability and accessibility; labels remain in accessible names,
  descriptions, and the mobile drawer.

## Invariants

- No card in the status deck authorizes a protected action.
- Values are derived from runtime, service, authentication, or authorization
  inputs already held by the shell.
- The global sign-in control remains the only signed-out authentication action.
- The shell remains responsive, localized, RTL-safe, reduced-motion-safe, and
  compatible with forced-colors mode.

## Acceptance evidence

- Product-shell tests assert the compact default and four verified cards.
- The semantic stylesheet contract rejects literal colors and decorative
  gradients in the new visual layer.
- The Docker real-browser gate covers WCAG, 320px reflow, RTL, forced colors,
  reduced motion, and the signed-out protected state.
