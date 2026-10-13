# ADR-0495 — Complete authentication by returning to the requested workspace

## Status

Accepted

## Context

The dedicated sign-in and sign-up routes accept a `next` workspace path, but
authentication completion must explicitly restore that route. Email OTP
verification occurs without a document navigation, while OIDC Authorization
Code + PKCE returns through a provider redirect and therefore cannot rely on
React component state surviving the redirect.

Leaving the browser on the auth route after issuing a valid token creates a
functional dead end: the token is held in memory but the requested application
workflow is not shown. A return path must remain internal, bounded, and
non-authoritative; it selects UI navigation only and never grants API access.

## Decision

Use a shared auth-flow return-path contract:

- validate and normalize `next` as an internal workspace path;
- reject auth routes and external, protocol-relative, control-character, and
  fragment-bearing values;
- keep the OTP flow in memory and replace the auth URL with the validated
  workspace path after token issuance; and
- store only the validated OIDC return path in session storage for the provider
  redirect, consume it once on callback, and remove it even when completion
  fails.

The application state changes to the returned workspace in the same operation
as `history.replaceState`, and protected capability loading continues to use
the freshly issued token and the API's server decision.

## Alternatives considered

- **Leave the user on the auth route:** rejected because successful sign-in is
  not an end-to-end application flow.
- **Persist the access token and return path in local storage:** rejected
  because browser persistence expands the token exposure window and is not
  needed for this memory-only session contract.
- **Trust the provider's arbitrary redirect query:** rejected because it can
  create open redirects and does not bind the route to the initiating session.
- **Use a server-side redirect session:** deferred because the current SPA
  contract deliberately keeps the access token in memory and the bounded
  session-storage return path is sufficient for the public-client flow.

## Verification

- Unit tests cover auth-route rejection and one-time OIDC return-path storage.
- The Docker auth browser gate proves OTP completion redirects to the requested
  workspace and can then sign out and sign in again.
- OIDC callback code consumes the path once and fails closed when it is missing
  or malformed.

