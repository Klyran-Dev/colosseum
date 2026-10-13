# ADR-0525: Public Customer Checkout Session Handoff

- Status: Accepted
- Date: 2026-10-13
- Owners: Checkout and Solana settlement engineering
- Related: ADR-0099, ADR-0200, ADR-0524

## Context

The product has a tenant-authenticated quote and wallet settlement workflow,
but a merchant cannot safely hand a quote to a customer. Putting a merchant
JWT in a payment link would expose tenant capabilities. Accepting quote,
route, or amount fields from an unauthenticated browser would make the browser
an authority for money movement.

## Decision

Add a short-lived public checkout session with these properties:

1. A merchant creates it only from an authenticated, non-expired, attested
   quote and a finalized active settlement route belonging to the same tenant.
2. The API generates a high-entropy opaque token, returns it once, and stores
   only its SHA-256 digest. The token is sent in a dedicated header and is not
   accepted as a merchant bearer token or query parameter.
3. Public read, compile, submit, and status routes are separate from merchant
   routes. Every public request resolves the session to the server-owned
   merchant, calculation, route, and settlement intent.
4. The customer supplies only a wallet public key and canonical decimal nonce.
   The browser wallet signs the server-compiled transaction; the API never
   receives or stores a private key.
5. One session binds to at most one settlement intent. Expired, unknown,
   mismatched, or already-bound sessions fail closed.
6. This is a guest checkout identity, not a durable customer account. Customer
   accounts, profiles, saved wallets, and order history remain future scope.

## Invariants and acceptance criteria

- Public access cannot enumerate merchant quotes, routes, capabilities, or
  intents without the matching session token.
- The public compile response is derived from the persisted quote, attestation,
  and active route; client-supplied merchant-side addresses and amounts are
  not accepted.
- Session token digests, not raw tokens, are persisted or logged.
- A session cannot bind two different settlement intents, including under
  concurrent requests.
- Quote expiry, attestation validity, route activation, signed-message
  verification, replay protection, and finalized chain evidence remain the
  existing settlement gates.
- The customer UI shows gross, tax, net, asset, expiry, and wallet signing
  state, and never shows a fabricated payment success state.

## Failure model and rollback

Unknown token, expired session, quote/route mismatch, missing attestation,
stale route, invalid wallet address, duplicate nonce, invalid signed message,
RPC ambiguity, and provider disagreement fail closed. A release can roll back
the public UI and API routes independently; existing authenticated settlement
routes and persisted evidence remain authoritative.

## Revisit conditions

Revisit when customer identity, saved payment methods, refunds addressed to a
customer, invoices, or cross-device recovery are required. Those needs require
a separate identity and privacy design rather than extending this token.
