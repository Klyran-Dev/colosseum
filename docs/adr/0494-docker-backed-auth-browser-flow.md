# ADR-0494 — Docker-backed browser authentication flow

## Status

Accepted

## Context

The application now has dedicated Metronic-style sign-in and sign-up routes,
but the general browser gate intentionally builds the SPA with a synthetic
credential harness so it can test protected workspace interaction without
coupling every UI journey to identity-service state. That gate cannot prove
that the real browser flow requests an email OTP, verifies it through the Rust
auth service, receives an OIDC-compatible token, presents that token to the
API, and receives the correct server-owned role capabilities.

The local email inbox is a Docker-only synthetic delivery adapter. It is safe
for an integration test because the code is generated and persisted by the
auth service, while the test reads only the isolated named volume. It must not
be used as a production delivery mechanism.

## Decision

Add a separate `make auth-browser-check` gate using a dedicated Compose
override. The override builds the SPA in `email-otp` mode, addresses the auth
service through the explicitly bounded local-only `auth-service` Compose
hostname, permits only the browser test origin in auth CORS, and mounts the
isolated local inbox read-only into the Playwright container. The hostname is
rejected by staging and production browser configuration validation.

The browser specification will cover:

1. standard sign-up, OTP verification, redirect to the requested workspace,
   API-backed capability loading, sign-out, and subsequent sign-in;
2. administrator bootstrap, tenant-scoped invitation creation, and invitation
   delivery through the local inbox; and
3. invited moderator sign-up, tenant binding, role-aware capability behavior,
   and denial of administrator-only role management.

The dedicated browser override sets
`AUTH_OTP_RESEND_COOLDOWN_SECONDS=0` only while `AUTH_ENV=local`. This keeps
the journey deterministic when it signs out and immediately signs in with the
same synthetic address. The auth service default remains the production-safe
60-second resend cooldown, and non-local environments reject this override.

The existing `make web-browser-check` remains the fast synthetic-credential UI
regression gate. Neither gate claims provider-backed delivery or production
OIDC PKCE; those remain deployment requirements.

## Alternatives considered

- **Add real auth to every existing UI journey:** rejected because it couples
  unrelated interaction/accessibility tests to identity state and increases
  test isolation cost.
- **Test only the auth service with HTTP requests:** rejected because it would
  not prove form submission, browser CORS, redirects, in-memory token handoff,
  or capability rendering.
- **Keep synthetic credentials as the only browser evidence:** rejected because
  it leaves the user-visible sign-up, OTP, and role flow unverified.

## Invariants and failure behavior

- The test uses only a disposable Compose project and synthetic email
  addresses.
- OTP files are read from a read-only test mount and are never exposed by the
  HTTP service.
- Tokens remain memory-only in the browser; the test must not persist them.
- A missing, malformed, stale, or mismatched inbox message fails the test.
- A moderator may view the access surface but may not call administrator role
  management endpoints.
- Compose cleanup is scoped to the dedicated project and removes only its
  synthetic volumes.
- The auth service remains non-root. Because a named volume is mounted after
  image build-time ownership setup, a short-lived `auth-inbox-init` service
  initializes only that volume with a minimal capability set before the auth
  service starts. The browser container retains a read-only mount.
- OTP lookup selects the newest matching challenge identifier, so a prior
  consumed message cannot be replayed accidentally when the same address is
  used for multiple journeys.

## Acceptance evidence

- `make auth-browser-check` passes against the current Docker-built revision.
- The auth browser result is retained under `test-results/auth-e2e`.
- Rust auth tests, frontend tests, `docker compose config`, and the existing
  `make web-browser-check` remain green.

