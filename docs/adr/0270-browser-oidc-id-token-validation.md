# ADR-0270: Browser OIDC ID-token validation boundary

## Status

Accepted. This decision closes the gap between the browser's OIDC request and
the token response while keeping the API as the authority for access-token
authorization.

## Context

The SPA requests the `openid` scope and sends a nonce in the Authorization Code
request. An OpenID Provider may therefore return an ID token. Accepting that
token without checking its signature, issuer, audience, time bounds, and nonce
would make the browser session boundary weaker than its documented contract,
even though the API independently verifies the access token.

## Decision

The browser will validate an ID token when one is returned by the token
endpoint, before establishing the signed-in UI session:

- Fetch the provider JWKS from discovery metadata using a bounded, same-origin
  HTTPS endpoint. The JWKS is cached only in memory for the current callback.
- Verify the compact JWS with `jose` Web Crypto APIs, allowing only the
  provider-approved asymmetric algorithms `EdDSA` and `RS256`; reject `none`,
  HMAC algorithms, unexpected key types, missing `kid`, and ambiguous keys.
- Require `iss` to equal the configured issuer after canonical trailing-slash
  comparison, `sub` to be a bounded non-empty string, `aud` to contain the
  configured client ID, and `exp`/`iat` to be safe integer seconds within the
  bounded clock-skew window.
- Require `nonce` to equal the one-time PKCE transaction nonce. A returned
  malformed or unverifiable ID token fails closed; it is never silently
  ignored.
- Continue to treat the access token as an opaque value. The API remains the
  authority for access-token signature, audience/resource, tenant, role,
  scope, revocation, and capability decisions.

The token response parser therefore requires an ID token when the configured
scope includes `openid`, and the callback flow passes the transaction nonce and
provider JWKS URI into the validator. The ID token is not persisted, logged,
sent to application APIs, or used for authorization decisions.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Ignore the ID token and rely only on the API access token | Smallest browser change | Leaves the OIDC nonce and `openid` contract unverifiable and weakens the documented callback boundary | Rejected |
| Decode claims without signature verification | Easy and fast | Claims are attacker-controlled and cannot establish an authenticated browser response | Rejected |
| Validate ID tokens in a server-side BFF | Strong central validation and token custody | Requires a new stateful session service, changes the static edge boundary, and adds a network hop | Deferred; compatible future evolution |
| Validate in the SPA with Web Crypto and a maintained JOSE library | Preserves static NGINX deployment, verifies the callback before UI session creation, bounded control-plane cost | Requires JWKS handling, provider rotation evidence, and dependency governance | **Selected** |

## Invariants and failure behavior

- No ID token is accepted without a valid signature, issuer, audience, nonce,
  subject, and bounded time claims.
- Provider metadata must include a same-origin JWKS URI; cross-origin, insecure,
  oversized, malformed, duplicate-key, or unsupported-algorithm material fails
  closed.
- A key rotation may retry once with a fresh in-memory JWKS fetch. The flow has
  no unbounded retry and no stale-persisted key fallback.
- ID-token validation is a sign-in control-plane operation and is never called
  from quote, tax calculation, settlement compilation, wallet signing, or
  reconciliation hot paths.
- The validator is loaded only during the OIDC callback, keeping its JOSE code
  out of the initial SPA bundle and preserving first-screen latency for local
  and already-authenticated sessions.
- A validation failure clears the access token and leaves protected workspaces
  unavailable until a fresh sign-in succeeds.

## Verification

- Unit tests cover signed EdDSA and RS256 tokens, issuer/audience/nonce/time
  validation, algorithm and key-type rejection, key rotation retry, malformed
  JWKS, oversized responses, and duplicate callback protection.
- Session tests prove an invalid ID token cannot establish a signed-in session.
- The browser suite continues to test the local fixture path; live provider
  redirect, CORS, rotation, logout, and revocation evidence remain staging
  release gates.
