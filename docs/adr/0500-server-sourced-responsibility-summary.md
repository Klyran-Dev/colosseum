# ADR-0500 — Server-sourced responsibility summary in the Metronic account menu

## Status

Accepted — 2026-10-11

## Context

The capability endpoint already returns the authenticated principal's active,
tenant-scoped roles. The Metronic account menu currently shows only a tenant
identifier and generic access state, so merchant operators, moderators, and
administrators have no immediate visual distinction after sign-in. Repeating
role choices in navigation would increase cognitive load and could imply that
the browser grants authority.

## Decision

Render a compact “Responsibilities” group inside the authenticated account
menu using only `authorizationSnapshot.roles` returned by the server. Map the
known role identifiers to localized operator labels and render an unknown
server role as a bounded, readable fallback. Do not decode JWTs, infer roles
from navigation, or make the chips interactive. Keep the account menu as the
single owner of authenticated identity details.

## Alternatives considered

- **Keep generic access text:** rejected because users cannot distinguish their
  current responsibility after sign-in.
- **Decode roles from the browser token:** rejected because the UI must not use
  unverified client-side claims as an authorization decision.
- **Add role badges to every navigation item:** rejected because it duplicates
  identity context and increases visual noise.
- **Fetch a second identity endpoint:** rejected because the already verified
  capability snapshot is the authoritative, bounded response available to the
  shell.

## Invariants

- Role chips are informational only; API authorization remains server-owned.
- No role is accepted from browser input or selected by the user.
- Unknown roles do not break rendering and are truncated by the existing
  bounded text treatment.
- The menu remains keyboard accessible, localized, and usable in RTL layouts.

## Verification

- Product-shell tests assert localized responsibility labels from a
  server-allowed snapshot and absence of the section for anonymous sessions.
- The full frontend and real-browser gates remain required after the change.

