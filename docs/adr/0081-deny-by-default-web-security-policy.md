# ADR-0081: Deny-by-default web security policy at the NGINX edge

- Status: Accepted
- Date: 2026-09-26
- Owners: Platform security and web engineering

## Context

The browser application is a static same-origin React SPA. It does not need
cross-origin API access, inline styles, inline scripts, plugin content, or
cross-origin form submission. The edge already limits request rate and
connections, but those controls are not evidence that browser policy and
authentication boundaries are working. A Tier-1 product needs executable
checks for the headers that constrain browser execution and for unauthenticated
API access.

## Decision

Enforce the browser policy in the immutable NGINX image and test it through
the Docker integration harness:

- `default-src`, `connect-src`, `style-src`, and `script-src` are same-origin;
- `img-src` permits same-origin images and data URLs required by the UI;
- `base-uri` is `none`, `object-src` is `none`, `form-action` is same-origin,
  and `frame-ancestors` is `none`;
- `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`, and
  `Permissions-Policy` are required on edge responses;
- no `Access-Control-Allow-Origin` header is emitted for an untrusted origin;
- protected quote access without a bearer token must return `401` through the
  edge as well as at the API boundary.

TLS termination and HSTS are deployment responsibilities of the approved
public ingress. The local HTTP-only NGINX profile does not emit HSTS, which
would be misleading on a plaintext development origin.

## Alternatives considered

| Alternative | Benefit | Risk/cost | Decision |
| --- | --- | --- | --- |
| **Strict static NGINX policy plus integration assertions** | Applies before the SPA and API, has no application dependency, and is reproducible in the image | Wallet or analytics integrations must be explicitly reviewed and added | **Selected** |
| React meta tags only | Easy to author with the SPA | Cannot reliably constrain all responses and does not protect API/error paths | Rejected |
| API middleware only | Centralizes server policy | Does not protect static document/script loading and adds per-service drift | Rejected |
| Wildcard CORS | Convenient for integrations | Allows untrusted origins to read authenticated responses if credentials are later enabled | Rejected |
| CSP with `unsafe-inline` | Fewer frontend migration constraints | Weakens script/style injection defenses and hides accidental inline behavior | Rejected |

## Verification and consequences

The integration harness now fails if a required header is missing, if an
unauthenticated quote request is accepted, or if an untrusted origin receives
an allow-origin response. The Vite bundle uses external CSS and scripts, so
removing `unsafe-inline` does not require a runtime exception. Any future
third-party wallet or telemetry origin requires an ADR and an explicitly
bounded policy change.

This does not replace TLS, WAF, identity-provider governance, CSP violation
reporting, or production browser compatibility testing; those remain release
gates.
