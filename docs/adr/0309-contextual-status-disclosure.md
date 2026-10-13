# ADR-0309: Contextual status disclosure for a calm Material 3 shell

## Status

Accepted — implemented as a shell-density and progressive-disclosure change.

## Context

The operational shell must make deployment, Solana-network, service, session,
and authorization state honest. The first Material 3 Expressive migration
displayed each state as a separate top-bar badge or persistent warning. In a
signed-out or locally unconfigured build this produced several adjacent error
surfaces before the operator reached the task: two configuration badges, a
service warning, and a sign-in error. The information was correct, but the
hierarchy was poor and the primary action became visually subordinate.

This is a product-quality problem, not a reason to hide state. A Tax Engine
operator surface must preserve fail-closed behavior, expose the exact reason
and next step, remain keyboard and screen-reader accessible, and keep status
outside the settlement hot path.

## Decision

Use one compact `System status` disclosure in the top bar. Its summary shows
one semantic state (`Ready`, `Checking`, or `Needs attention`) and a bounded
count when attention is required. The closed summary is the only persistent
system-status surface. Opening it exposes a labelled list containing:

- deployment environment;
- Solana network;
- API services; and
- the safe next step for each unavailable or unconfigured item.

Keep session sign-in as a separate compact control because identity is an
operator action rather than infrastructure health. In an unconfigured build,
`Sign-in unavailable` is a closed disclosure whose explanation remains an
assertive, accessible message when opened. OIDC and explicitly local
credential flows retain their existing behavior.

Retire the former persistent-badge and shell-service selectors from the
stylesheet rather than leaving dormant styling that could silently recreate
the old visual hierarchy. The Material color contract checks this absence.

The shell continues to render all navigation items and server-owned access
decisions. No state is inferred, downgraded, or used as authorization. The
disclosures are presentation-only and do not add a request, storage access,
or work to the wallet-signing or settlement path.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep every badge and warning persistent | Maximum immediate visibility | Repeated information overwhelms the task, especially when several states fail together | Rejected |
| Hide non-ready states until an error page | Cleanest chrome | Conceals deployment and service conditions and weakens supportability | Rejected |
| Use one generic “offline” label | Very compact | Loses the distinction between configuration, service health, and sign-in next steps | Rejected |
| **One system-status disclosure plus a separate session disclosure** | Calm hierarchy, complete diagnostics, low runtime cost, and accessible progressive detail | Requires one deliberate interaction to inspect details | **Selected** |

## Invariants

- Every deployment, network, service, and sign-in failure remains visible in
  the accessibility tree when its disclosure is opened.
- The summary never claims readiness when a required context is missing or the
  service is unavailable.
- The browser does not use presentation state to authorize a request,
  calculate tax, sign a wallet transaction, or release funds.
- Status labels use semantic Material color roles only; no component adds a
  one-off color token.
- The disclosure is native keyboard-operable, has a stable accessible name,
  and remains usable in narrow and RTL layouts.

## Verification

- Product-shell tests assert the compact summary, complete opened diagnostics,
  and the absence of repeated persistent configuration badges.
- Authentication-control tests assert that an unconfigured build keeps its
  explanation behind an accessible disclosure.
- The real-browser suite focuses the status summary and verifies keyboard
  open/close behavior without changing the diagnostic content.
- Existing Material palette, axe, focus, RTL, browser-journey, and build gates
  remain mandatory.
- Visual review covers signed-out/unconfigured, signed-in/ready, service
  degraded, narrow, dark, and light states.
