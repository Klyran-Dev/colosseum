# ADR-0528: Coordinate one active browser session without persisting bearer tokens

- Status: Accepted
- Date: 2026-10-13
- Decision owners: Platform architecture and security

## Context

The SPA intentionally keeps the access token in React memory. That prevents a
bearer token from surviving a reload or being exposed through browser storage,
but it also means two tabs can independently complete sign-in and continue
using different identities in the same browser profile. This is confusing for
operators and unsafe for an admin console because the visible browser context
can no longer be treated as one user session.

The immediate requirement is to stop conflicting identities across tabs while
preserving the existing token boundary. Reload persistence through a server
session cookie is a separate authentication migration and must not be implied
by this coordination signal.

## Decision

The SPA coordinates one active in-memory browser session per origin using a
versioned `BroadcastChannel` signal with a `storage` event fallback. A claim
contains only a CSPRNG session identifier, signal kind, and bounded issue time;
it never contains an access token, refresh token, email address, or authority
claim. When another tab claims a session, an already authenticated tab clears
its in-memory token, authorization snapshot, and OTP state and displays an
actionable sign-out explanation.

The API remains the authority: all protected requests still carry the current
bearer token and are independently checked for signature, expiry, tenant, role,
scope, and capability. The browser signal is advisory coordination only and
cannot grant or revoke server authority.

## Alternatives considered

| Alternative | Benefit | Risk / cost | Decision |
| --- | --- | --- | --- |
| Persist the access or refresh token in local/session storage | Easy reload and cross-tab reuse | Expands XSS and browser-storage credential exposure; violates the current token boundary | Rejected |
| Broadcast the bearer token between tabs | Cross-tab reuse without durable storage | Increases token exposure to every same-origin tab and still lacks server revocation | Rejected |
| **Broadcast a non-secret single-session claim and clear conflicting tabs** | Fixes the identity-confusion bug without persisting credentials; bounded and auditable | A second tab must authenticate again and a reload still requires sign-in | **Selected for this release** |
| HttpOnly Secure cookie with server-side refresh/session records | Standard reload persistence, cross-tab continuity, and central revocation | Requires a stateful BFF/session service, cookie/CSRF contract, migration, and deployment rollout | Deferred to a dedicated auth migration ADR |

## Invariants

- No bearer or refresh token is written to browser storage or cross-tab signals.
- A tab cannot remain authenticated under an older identity after another tab
  claims the browser session.
- A malformed or stale signal is ignored and never changes authorization.
- Server-side JWT verification and tenant authorization remain unchanged.
- Sign-out and token expiry clear local state even if cross-tab signalling is
  unavailable.

## Verification

- Unit tests validate signal shape, secret non-disclosure, malformed-input
  rejection, and disposal behavior.
- Browser auth checks must cover a claim in one tab causing the other tab to
  leave the authenticated state.
- The future cookie-session migration requires a separate threat model,
  CSRF contract, rotation/revocation design, and reload/e2e evidence before
  changing the browser storage boundary.
