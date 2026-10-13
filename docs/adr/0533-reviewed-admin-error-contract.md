# ADR-0533: Preserve reviewed admin service errors in mutation surfaces

- Status: Accepted
- Date: 2026-10-13
- Owners: Web platform and identity service

## Context

The identity service returns a reviewed, redacted error envelope for protected
administrator mutations. The admin data adapter represented that envelope as a
plain object, while mutation surfaces rendered only `Error` instances. As a
result, actionable responses such as missing administrator authorization,
duplicate pending invitation, or temporary email unavailability were replaced
with the generic “The invitation service rejected the request.” message.

## Decision

Represent HTTP failures from the admin data adapter as an `AdminRequestError`
that extends `Error` and carries the typed error code and optional correlation
identifier. Preserve the server-provided message because the auth service is
the boundary that redacts internal details and emits only reviewed product
copy. Unknown client or parsing failures continue to use safe generic copy.

## Invariants

- The browser never constructs an authorization decision from the error.
- The server remains the authority for role, tenant, invitation, and email
  delivery outcomes.
- Error messages shown to users are server-reviewed or local safe fallbacks;
  database, SMTP, token, and stack details are not exposed.
- Correlation identifiers remain diagnostic metadata and are not bearer data.

## Alternatives

- Keep the generic message: rejected because it prevents operators from
  distinguishing authorization, conflict, and availability failures.
- Render raw response bodies: rejected because it can expose internal details
  and bypass the auth-service error contract.
- Add per-component error-shape checks: rejected because it duplicates the
  boundary contract and will regress on future mutation surfaces.

## Verification

- Unit coverage verifies reviewed server messages and typed `Error` behavior.
- Typecheck and production build are required before deployment.
- The deployed source revision and health state are recorded in the release
  evidence for the production VM.
