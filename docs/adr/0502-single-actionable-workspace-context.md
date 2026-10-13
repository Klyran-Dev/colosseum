# ADR-0502 — Keep workspace context actionable and diagnostics global

## Status

Accepted — 2026-10-11

## Context

The Metronic Demo 3 shell already provides a compact global system-status
control in the header. The workspace then repeated deployment, network, and API
diagnostics as three additional large cards beside the access card. This made
the primary work surface read like a status dashboard and duplicated the same
information in two places, especially for signed-out users.

## Decision

Keep one visible workspace status card for the actionable access state. Keep
deployment, Solana network, API, and session diagnostics in the existing
collapsed header disclosure. The workspace card remains server-derived and
does not grant access; it explains whether the current workspace is public,
requires sign-in, or is waiting for a server decision.

The card keeps the licensed Metronic Demo 3 card treatment and semantic
repository tokens, but uses a compact single-column footprint so the real
workflow begins sooner in the viewport.

## Alternatives considered

| Alternative | Decision | Rationale |
| --- | --- | --- |
| Keep four visible status cards | Rejected | Repeats header diagnostics and competes with the user's task. |
| Remove all status context | Rejected | Hides the access boundary and makes public/protected behavior ambiguous. |
| Add a second diagnostics accordion in the workspace | Rejected | Duplicates the global system-status disclosure. |
| One access card plus global diagnostics disclosure | Selected | Preserves verified context, reduces visual load, and matches Demo 3's task-first composition. |

## Invariants

- The access card is derived from the same server-owned authorization snapshot
  and never authorizes an action.
- Environment, network, API, and session status remain available through the
  header disclosure with their existing loading/degraded wording.
- No mock or client-inferred release context is introduced.
- The card remains keyboard accessible, responsive, localized-ready, and
  compatible with reduced-motion and forced-colors behavior.

## Verification

- Product-shell tests assert exactly one visible workspace status card.
- Stylesheet contract tests assert the single-column context layout and
  semantic-token usage.
- The full frontend test, typecheck, build, and Docker browser gates remain
  required after the visual simplification.
