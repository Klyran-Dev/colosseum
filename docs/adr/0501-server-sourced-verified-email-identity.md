# ADR-0501 — Server-sourced verified email identity in the workspace snapshot

## Status

Accepted — 2026-10-11

## Context

The authenticated Metronic account menu currently receives a server-owned
subject, tenant, roles, and capability decisions. The subject is intentionally
opaque, so a signed-in operator, moderator, or administrator is shown an
unhelpful identifier rather than the verified identity they used to sign in.
The browser must not decode access tokens or treat client-provided identity
claims as an authority boundary.

## Decision

The repository-issued EdDSA access token includes the normalized, verified
email used to complete authentication. The API JWT verifier validates the
bounded optional claim and exposes it only through the authenticated,
tenant-scoped `authorization-capabilities/v1` response. The SPA parses the
server response and uses the email as the account-menu identity label, with the
opaque subject as a bounded fallback for external OIDC tokens that do not
provide an email claim in their access token.

The email is display identity only. Authorization continues to use the server
capability decisions, and the browser still never decodes a JWT. The API does
not return email addresses to anonymous callers or to another tenant.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Keep displaying the opaque subject | Rejected | Safe but poor operator feedback and does not identify the signed-in account. |
| Decode the ID/access token in the browser | Rejected | Makes unverified client parsing part of the identity surface and duplicates token validation. |
| Add a separate profile round trip from the browser | Rejected | Adds a second identity authority and a new failure in the critical shell bootstrap. |
| Add a verified email claim to the access token and return it from the already-authenticated capability snapshot | Selected | One signed-token validation boundary, no extra request, and consistent identity for the API and SPA. |

## Invariants

- Custom auth-service tokens contain a normalized email claim after OTP
  verification; the browser never receives the raw OTP or signing material.
- API validation accepts missing email only for compatibility with configured
  external OIDC access tokens, but rejects malformed, oversized, or
  non-normalized email claims when present.
- Capability responses remain bounded, tenant-scoped, short-lived, and
  server-owned.
- Email is never used to authorize a role, tenant, settlement, or financial
  action.
- Existing tokens without the claim remain valid during migration and fall
  back to the opaque subject in the UI.

## Verification

- Rust auth tests prove the issued access token carries the normalized email.
- API auth tests prove optional email parsing, normalization, and rejection of
  malformed claims.
- API capability tests prove the response carries the authenticated email.
- SPA parser and shell tests prove the verified email is displayed and the
  opaque subject remains the fallback.
- Docker-backed administrator and moderator browser journeys continue to pass.
