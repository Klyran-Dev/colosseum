# ADR-0441 — Separate email-OTP authentication service and OIDC-compatible tokens

## Status

Accepted for the local vertical slice and the production provider adapter.
Provider account, tenant-provisioning, privacy-retention, and
identity-governance decisions remain deployment gates.

## Problem statement

The static React SPA already implements an OIDC Authorization Code + PKCE
client and the API already verifies bounded Ed25519 JWTs. Local Docker currently
uses a synthetic bearer-credential entry field, which is useful for fixtures
but is not a user sign-in or signup experience. The system needs a separately
bounded authentication service that verifies email OTP challenges, provisions a
tenant-scoped identity according to an explicit policy, and issues tokens that
the existing API accepts without moving identity authority into payment code.

## Users and scope

The first slice serves a local operator who signs up or signs in with
an email address, receives a short-lived one-time code, verifies it, and gets
an in-memory browser session. The service owns identity records, OTP challenge
state, Ed25519 signing, JWKS publication, and email-delivery ports.

The service does not own Solana keys, tax authority, settlement authority,
wallet signing, legal eligibility, or arbitrary tenant authorization. The API
continues to re-authorize every protected request from the verified token.

## Non-goals

- Storing passwords or refresh tokens.
- Enabling provider delivery before the approved adapter, sender domain, and
  delivery operations are configured.
- Treating a verified email address as legal, tax, merchant, or payout approval.
- Allowing self-signup in staging/production without an approved provisioning
  policy.
- Claiming that the direct local OTP JSON transport is a complete OIDC
  Authorization Code provider. Production OIDC PKCE remains the deployment
  target until a full authorization/token endpoint and provider review are
  completed.

## Selected design

Use a separate Rust/Tokio `auth-service` with a small pure domain layer and a
PostgreSQL adapter. It exposes:

1. `/jwks.json`, `/healthz`, and `/readyz`. JWKS is the compatibility boundary
   used by the existing API verifier; full OIDC discovery and authorization
   endpoints are intentionally deferred.
2. Bounded JSON email-OTP endpoints for the first SPA vertical slice:
   `POST /v1/auth/email/request` and `POST /v1/auth/email/verify`.
3. A signing-key adapter that accepts a mounted key file in non-local
   environments and a deterministic synthetic key only in local mode.
4. An `EmailSender` port. The local implementation writes a test-only inbox
   artifact containing the code to a project volume so the flow is reproducible;
   this mode is prohibited outside local development. Non-local startup uses
   the bounded Resend HTTPS adapter from ADR-0496 and fails closed unless its
   endpoint, secret, sender, and application origin are configured.

The service issues the same EdDSA JWT contract as the API verifier:
`iss`, `sub`, `aud`, `iat`, `nbf`, `exp`, `merchant_id`, `scope`, and `roles`.
Access tokens remain in browser memory. OTP codes are persisted only as
peppered digests in the database, except for the explicitly local Docker inbox
fixture; they are never logged, returned in non-local responses, or included in
telemetry.

First verified sign-in creates an `auth_user` and a tenant/merchant identity
only when an explicit self-signup policy is enabled. Otherwise it requires an
existing invitation/provisioning record. The initial local policy may enable
self-signup for synthetic development data; staging and production must use an
approved invitation or provisioning policy.

Challenge intent is explicit: `signin` requires an existing active account and
`signup` provisions only the base configured role. Elevated responsibilities
are stored in the server-owned `auth_user_roles` assignment table and are
unioned into signed role claims after verification; the browser cannot select
or submit administrator/moderator roles.

## End-to-end sequence

1. Browser submits a normalized email to the auth service.
2. Service applies bounded per-email and per-source rate limits, creates a
   UUIDv7 challenge, stores a salted/peppered digest with expiry and attempt
   budget, and invokes the email port.
3. Browser submits challenge ID, email, and code.
4. Service locks the challenge, rejects expired/consumed/exhausted challenges,
   compares digests in constant time, consumes the challenge exactly once, and
   creates or loads the identity in one transaction.
5. Service signs a short-lived EdDSA access token and ID token, returning only
   the token response required by the SPA.
6. Browser keeps the access token in memory and sends it to the existing API.
7. API validates issuer, audience, signature, time, tenant, scopes, and roles;
   it never trusts the auth service response outside the signed token.

## Invariants and failure modes

- A challenge is single-use and has a bounded lifetime, attempt count, and
  resend cooldown.
- Email normalization is canonical and case-insensitive; raw email is never
  used as an authorization selector.
- Request responses do not reveal whether an email already exists.
- OTP verification is atomic under concurrent requests; at most one succeeds.
- Issued tokens have bounded lifetime and contain a valid UUIDv7 merchant ID.
- Key material is never returned by the service; JWKS exposes public keys only.
- Missing provider, signing key, database, policy, or issuer configuration
  fails closed before the listener binds in non-local environments.
- Email delivery failure does not consume a prior valid challenge and is
  observable without exposing address or code contents.
- Auth-service outage leaves protected API actions unavailable; no local
  credential fallback is enabled outside local mode.

## Threat model

Threats include OTP brute force, replay, enumeration, resend abuse, email
account takeover, database disclosure, signing-key compromise, token theft,
cross-tenant claims, clock skew, concurrent verification, and provider outage.
Controls are bounded challenge state, salted/peppered digests, constant-time
comparison, generic responses, rate limits, short token lifetime, EdDSA key
rotation, strict claim validation, audit events, TLS/mTLS deployment policy,
and explicit identity/tenant provisioning.

## Budgets

- Request body: 8 KiB maximum.
- Email: normalized address maximum 320 bytes.
- OTP: six decimal digits, five attempts, five-minute expiry, one-minute resend
  cooldown.
- Per-email requests: three per fifteen minutes; per-source requests: twenty
  per fifteen minutes; limits are persisted and fail closed on ambiguity.
- Token lifetime: fifteen minutes for the first slice.
- Auth service concurrency: bounded by the database pool and a 32-request
  semaphore; no unbounded tasks or queues.
- Target local p95 after warm-up: request/verify service work under 100 ms
  excluding email-provider latency. This is local evidence, not a production
  SLO.

## Acceptance criteria

- Pure tests cover normalization, digesting, expiry, resend, attempt limits,
  constant-time mismatch behavior, concurrent consume, claim bounds, and JWT
  vectors.
- PostgreSQL integration tests cover idempotent user creation, one-winner OTP
  consumption, tenant binding, rollback on email/provider failure, and
  retention-safe audit records.
- API verification accepts issued tokens and rejects wrong issuer, audience,
  key, expiry, merchant, scope, or role.
- Browser tests cover sign-up, sign-in, wrong/expired/reused code, resend
  cooldown, service failure, sign-out, expiry, and no credential persistence.
- Docker Compose starts the auth service and a local-only inbox artifact with
  health checks and bounded resources.
- Production builds reject local inbox/self-signup modes and missing provider,
  key, issuer, or provisioning configuration.

## Alternatives

See the authentication section added to
`docs/architecture/alternative-analysis.md`.

## Revisit triggers

Revisit when a regulated identity provider, enterprise SSO, immediate token
revocation, account recovery, multi-merchant membership, or a production email
provider requires a BFF/session service, SCIM, external directory, or a new
provisioning authority.
