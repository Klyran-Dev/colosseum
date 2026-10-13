# ADR-0504: Render operator-facing responsibility and invitation labels

- Status: Accepted
- Date: 2026-10-11
- Decision owners: Web platform and identity bounded contexts

## Context

The administrator role-management directory receives durable role and invitation
status codes from the authentication service. Those codes are intentionally
machine-readable (`merchant_operator`, `moderator`, `pending`, and so on), but
rendering them verbatim makes the control plane feel like an internal API
debugger and increases the reading cost for administrators.

## Decision

Keep the server contract and raw values unchanged, and add a bounded
presentation mapping in the role-management surface. Known roles and statuses
render as operator-facing labels such as “Merchant operator”, “Moderator”,
“Pending”, and “Revoked”. Unknown values remain visible through a conservative
humanized fallback so a newly introduced server value is not silently hidden.

The raw values continue to drive request payloads, parsing, authorization
checks, mutation routes, and test selectors where protocol identity matters.
The presentation mapping is isolated so it can later consume the repository's
localization catalog without changing the identity-service API.

## Alternatives considered

1. Render raw protocol values. Rejected because underscore-delimited role codes
   are implementation vocabulary, not an administrator-facing interface.
2. Let the browser define role semantics independently. Rejected because it
   risks drift from the auth-service contract and can hide newly introduced
   values.
3. Hide unknown roles or statuses. Rejected because omission would reduce
   auditability and make server/client version skew harder to diagnose.

## Invariants and failure behavior

- No API request or response schema changes.
- No change to tenant scoping, authorization, or mutation idempotency.
- Only bounded, already validated values reach the display formatter.
- Unknown values remain visible as readable text rather than disappearing.
- Role and invitation status semantics remain server-owned.

## Verification

- Focused role-management UI tests assert the presentation labels while still
  asserting the raw mutation role in the request body.
- The Docker-backed administrator invitation and moderator-assignment browser
  journey verifies the rendered labels against the real auth service.
- Typecheck and the repository frontend test suite remain required gates.
