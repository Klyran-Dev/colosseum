# ADR-0490: Licensed Metronic Demo 3 shell and dedicated authentication routes

## Status

Accepted — 2026-10-10

## Context

The React/Vite application has a production-oriented operational shell and a
separate Rust email-OTP service, but authentication is currently exposed as a
compact header control. The licensed Metronic 9.5.0 Demo 3 package provides a
different, concrete layout contract: a 58px icon rail, a 58px header, a 56px
secondary workspace navbar, a rounded bordered content canvas, and a compact
toolbar. The package also contains demo-only Supabase authentication and demo
data that are not authority for this application.

The product needs a real sign-in/sign-up journey for users, while
administrator and moderator privileges must remain server-owned and must not be
selected by an untrusted browser.

## Decision

Adapt the licensed Metronic Demo 3 structure into the repository-owned React
19/Vite SPA. Keep the existing semantic Material role system and domain
workspace content; use Metronic's layout hierarchy, spacing relationships,
card treatment, compact rail, and toolbar rhythm rather than copying its demo
application or its authentication provider.

Add dedicated `/auth/signin` and `/auth/signup` routes rendered by the SPA.
Both routes use the existing Rust email-OTP service and receive the same
OIDC-compatible EdDSA access/ID-token response. The requested auth intent is
bound to the OTP challenge: sign-in requires an existing active account;
sign-up provisions only the base user/merchant-operator account. Moderator and
administrator roles are loaded from durable server-side assignments and are
never accepted from browser input or self-sign-up claims.

After verification, the browser keeps the access token in memory and returns to
the originally requested workspace. Server capability evaluation remains the
only UI authorization authority and every protected API repeats authorization.

## Alternatives considered

- Keep the current header-only OTP disclosure: rejected because it is not a
  discoverable sign-in/sign-up journey and does not match the licensed Demo 3
  information architecture.
- Copy the complete Metronic demo application: rejected because it brings demo
  data, Supabase auth, unrelated routes, and a different component/runtime
  contract into a production Solana SPA.
- Allow the browser to choose administrator/moderator during sign-up: rejected
  because role elevation would cross the authority boundary.
- Replace the service with a hosted auth SDK: rejected because the repository
  already owns a bounded Rust OIDC-compatible issuer and must preserve its
  audit, tenant, and Docker contracts.

## Invariants

- Metronic layout adaptation does not introduce raw color literals outside the
  repository's scheme token declarations.
- Auth intent is validated server-side and cannot elevate privileges.
- OTP challenges are single-use, bounded, rate-limited, and persisted in
  PostgreSQL; tokens are signed with Ed25519 and verifiable through JWKS.
- Self-sign-up receives only the configured base role; privileged roles come
  from durable assignments and are included in tokens only after verification.
- Access tokens are never rendered, persisted in browser storage, or sent to
  the UI as display data.
- Protected workspace data is not mounted until server capability evaluation
  succeeds.

## Verification

- Auth-page component tests cover sign-in, sign-up, OTP request/verify, retry,
  and role-neutral copy.
- Rust domain and service tests cover intent validation, existing-account
  requirements, self-sign-up provisioning, and server-owned role claims.
- Docker integration verifies PostgreSQL migration, auth health/readiness,
  request/verify, JWKS, and API capability evaluation with the issued token.
- Browser tests verify the Demo 3 shell landmarks, auth route reflow, and the
  return-to-workspace flow at desktop and mobile widths.
- The stylesheet remains within the revised 170,000-byte/1,050-rule source
  ceiling and the existing 26,000-byte gzip ceiling; the current measured gzip
  size is 25,048 bytes.

