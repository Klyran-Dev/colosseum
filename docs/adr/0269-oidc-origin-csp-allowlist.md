# ADR-0269: OIDC origin and CSP allow-list binding

## Status

Accepted. This decision closes the browser authentication and edge security
boundary for the static SPA without weakening the deny-by-default CSP.

## Context

ADR-0268 selects OIDC Authorization Code + PKCE for staging and production.
The edge CSP currently permits `connect-src 'self'` only. A browser therefore
cannot fetch OIDC discovery metadata or exchange an authorization code with an
external identity provider, even when the public client configuration is valid.

Adding `https:` globally would make the flow work, but would allow the SPA to
send network requests to any HTTPS origin if a later code path were changed.
Allowing arbitrary discovery endpoint hosts would create the same drift between
the identity configuration and the browser execution policy.

## Decision

1. Treat the configured OIDC issuer origin as a release input to the edge
   image. The build derives only its origin, never a path, query, credentials,
   or fragment, and adds that exact origin to `connect-src`.
2. Permit an empty OIDC issuer in local Docker builds. The generated local CSP
   remains exactly same-origin; local synthetic credential entry does not need
   an external identity provider.
3. Outside local development, an issuer must be HTTPS. Local HTTP is allowed
   only for loopback hosts, matching the browser auth configuration contract.
4. OIDC discovery metadata must use the exact configured issuer origin for both
   authorization and token endpoints. Cross-origin endpoint substitution fails
   closed before redirect or token exchange.
5. The generated NGINX configuration is validated with `nginx -t` in the
   immutable image build. A malformed issuer or unsafe origin fails the image
   build instead of silently producing a permissive policy.
6. The browser still validates the issuer, endpoint transport, callback state,
   PKCE verifier, response bounds, and token response. CSP is defense in depth,
   not an identity or API authorization boundary.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Global `connect-src https:` | Works with any provider | Permits arbitrary HTTPS egress and hides provider drift | Rejected |
| Keep `connect-src 'self'` | Strongest static policy | Makes configured production sign-in unusable | Rejected |
| Runtime CSP assembled from browser environment | No image rebuild | Browser-controlled configuration is not an authority and can drift from release evidence | Rejected |
| Same-origin BFF callback and token exchange | Keeps browser CSP same-origin and protects tokens with HttpOnly cookies | Requires a stateful session service and changes the static SPA boundary | Deferred; requires a separate ADR |
| **Build-time exact issuer-origin allow-list plus same-origin endpoint validation** | Preserves strict CSP, matches immutable release configuration, and fails closed on endpoint substitution | Provider origin changes require a new release | **Selected** |

## Invariants and failure behavior

- No production edge image contains a wildcard or scheme-only OIDC CSP source.
- The effective OIDC authorization and token endpoint origins equal the
  configured issuer origin.
- A missing, malformed, credential-bearing, fragment-bearing, or unsafe issuer
  cannot produce a valid non-local image configuration.
- A discovery response pointing to another origin is rejected before code
  exchange; no token request is attempted.
- Local builds preserve the existing exact CSP and do not require network
  access to an identity provider.
- The API remains responsible for access-token signature, issuer, audience,
  scope, tenant, role, resource, expiry, and revocation checks.

## Verification

- Unit tests reject cross-origin discovery endpoints and unsafe issuer URLs.
- Docker builds prove the local generated CSP remains same-origin and that a
  configured provider origin appears exactly once in `connect-src`.
- NGINX configuration tests run against the generated file before it is served.
- The real-browser suite continues to audit rendered WCAG A/AA behavior; a
  staging identity-provider run remains required for redirect registration,
  CORS, key rotation, revocation, logout, and outage evidence.
