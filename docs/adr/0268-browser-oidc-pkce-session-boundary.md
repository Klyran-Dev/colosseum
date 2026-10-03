# ADR-0268: Browser OIDC PKCE session boundary

## Status

Accepted. This decision replaces production browser bearer-token entry with a
standards-based sign-in boundary while retaining an explicitly local-only
synthetic credential path for Docker fixtures and automated tests.

## Context

The API already verifies OIDC-compatible EdDSA access tokens, but the SPA
currently asks an operator to paste a bearer credential into a workspace. That
is an unsafe and confusing production experience, encourages credential
handling in business forms, and prevents one authenticated session from being
shared consistently across workspaces.

The selected React/Vite/NGINX application is a static SPA. It cannot safely
hold a client secret or use a server-rendered callback. The browser therefore
needs a public-client OIDC flow that does not persist access tokens and that
does not turn client claims into authorization decisions.

## Decision

Use the OIDC Authorization Code flow with PKCE (S256) for staging and
production:

1. The release supplies only public OIDC configuration: issuer, client ID,
   redirect URI, scopes, and optional audience/resource. No client secret is
   accepted by the SPA.
2. The browser obtains provider metadata from the configured issuer, validates
   the issuer and HTTPS/local transport rule, generates a CSPRNG state, nonce,
   and code verifier, and stores only the bounded short-lived transaction in
   session storage.
3. The browser redirects to the provider and exchanges the returned code once
   at the validated token endpoint using the exact redirect URI and verifier.
   The access token remains in React memory only. It is never placed in
   localStorage, URL state after callback processing, telemetry, or logs.
4. The API remains the authority for issuer, audience, signature, expiry,
   tenant, scope, role, resource, and revocation policy. The browser does not
   decode JWT claims or enable a protected action from a claim.
5. The session is cleared at expiry, sign-out, malformed token response, or
   callback mismatch. The safe recovery is a fresh sign-in redirect.
6. A local credential field exists only when the build explicitly sets
   `VITE_LOCAL_UI_MODE=credential` and the deployment context is `local`.
   Staging and production builds reject that mode at configuration parsing and
   show a clear sign-in configuration failure instead of a fake login.
7. All protected workspaces consume the same in-memory session value. A
   workspace may not maintain a second credential field in the production
   shell.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Paste a bearer token into each workspace | Works with synthetic fixtures | Exposes credential handling in product forms, duplicates state, and is not a customer sign-in experience | Rejected for non-local builds |
| OIDC implicit flow | Simple redirect | Tokens in the front-channel and weaker replay protection | Rejected |
| Authorization Code + PKCE in the SPA | Standard public-client flow, no client secret, bounded token exposure, compatible with static NGINX | Requires provider CORS/redirect registration and callback handling | **Selected** |
| Server-side BFF cookie session | HttpOnly token protection and central revocation | Requires a stateful callback/session service and changes the static edge boundary | Deferred; may supersede this ADR if a BFF is approved |
| Wallet signature as employee identity | Native wallet interaction | A wallet proves control of a key, not organizational identity, roles, tenant, or revocation | Rejected |

## Invariants and failure behavior

- No production build renders a bearer-credential input.
- No client secret, refresh token, private key, or raw access token is stored
  persistently or logged.
- PKCE state, nonce, and verifier are one-time, bounded, origin-bound, and
  expire quickly; mismatch fails closed.
- Discovery and token responses have bounded size, strict JSON fields, exact
  endpoint validation, and no automatic duplicate token exchange.
- An expired or unavailable session leaves protected workspaces unavailable;
  it never reuses a stale capability snapshot or claims access.
- The API reauthorizes every protected request regardless of the browser
  session state.

## Latency and availability budget

The sign-in redirect is a control-plane operation and is not in quote,
transaction compilation, or wallet-signing hot paths. After sign-in, attaching
the in-memory bearer header adds no network hop. Capability refresh remains
bounded by the existing 60-second server snapshot and request timeout.

## Verification

- Pure tests cover config rejection, PKCE encoding, state/nonce/verifier
  bounds, callback mismatch, expiry, malformed provider responses, and token
  response strictness.
- React tests cover sign-in, local-only credential visibility, shared session
  propagation, sign-out, and expired-session fail-closed behavior.
- Real-browser tests cover the local fixture path and a mocked provider callback
  without claiming a live identity-provider approval.
- Staging evidence must prove redirect allow-listing, issuer/audience mapping,
  key rotation, revocation behavior, logout, and provider outage handling.
