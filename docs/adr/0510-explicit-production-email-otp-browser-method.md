# ADR-0510: Explicit production email-OTP browser method

## Status

Accepted.

## Context

The repository-owned Rust authentication service supports two deployment email
delivery modes: a Docker-only local inbox and a bounded HTTPS transactional
email-provider adapter. The SPA previously exposed email OTP only when
`VITE_LOCAL_UI_MODE=email-otp` was used in a local build. That prevented a
staging or production release from using the separately designed auth service,
even when provider delivery, HTTPS, and provisioning policy were configured.

The browser must not guess between OIDC and email OTP, and a static bundle must
not contain provider credentials. The auth service remains the identity and
token authority; the browser only chooses the release-configured entry point
and sends the user-entered email/code to that service.

## Decision

Add the release-time `VITE_AUTH_METHOD` switch with the values `oidc` and
`email-otp`.

- `VITE_AUTH_METHOD=email-otp` enables the existing email OTP UI in local,
  staging, or production when `VITE_AUTH_SERVICE_BASE_URL` is valid.
- Non-local email OTP requires an HTTPS auth-service URL. The auth service
  independently requires `AUTH_EMAIL_MODE=provider`, provider credentials,
  verified sender configuration, HTTPS application origin, and approved
  provisioning policy before binding its listener.
- `VITE_AUTH_METHOD=oidc` requires complete OIDC provider configuration.
- The explicit methods are mutually exclusive with local credential entry and
  with each other. OIDC settings combined with explicit email OTP fail closed.
- `VITE_LOCAL_UI_MODE=email-otp` remains a backwards-compatible local-only
  alias for Docker development; it cannot enable non-local email OTP.
- The SPA continues to keep access tokens in memory and never receives or
  embeds email-provider credentials.

## Alternatives considered

| Option | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Keep email OTP local-only | Smallest browser configuration | Leaves the production auth-service path unreachable from the product | Rejected |
| Always prefer email OTP when a service URL exists | Simple deployment inputs | Ambiguous releases and accidental auth-method changes | Rejected |
| **Explicit `VITE_AUTH_METHOD` release switch** | Deterministic, auditable, mutually exclusive, compatible with static Vite builds | Requires one additional release input | **Selected** |
| Import a provider SDK into the SPA | Convenience for one provider | Exposes credentials/authority and couples the browser to delivery infrastructure | Rejected |

## Invariants and failure behavior

- Missing or invalid method configuration renders the auth surface as
  unconfigured; it never silently falls back to a weaker method.
- Non-local email OTP cannot use HTTP, loopback, the local inbox, or local
  self-signup.
- Provider mode and provisioning authorization are enforced by the server,
  not by a Vite flag.
- The API continues to verify the same issuer, audience, EdDSA signature,
  expiry, revocation policy, tenant, roles, and scopes for either entry path.

## Acceptance evidence

- Browser-auth unit tests cover explicit production email OTP, HTTPS rejection,
  OIDC/email-OTP conflict, local compatibility, and unconfigured states.
- Docker auth-browser evidence covers request, delivery, OTP verification,
  signup, signin, administrator invitation, moderator access, and server
  capability checks.
- A production deployment must additionally retain provider acceptance, DNS
  authentication, secret-injection, key-rotation, revocation, and provisioning
  evidence before enabling the method.

