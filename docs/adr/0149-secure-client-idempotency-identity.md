# ADR-0149: Secure client idempotency identity across browser contexts

- Status: Accepted
- Date: 2026-09-28
- Owners: Principal Low-Latency Architecture, Web Platform, Security
- Bounded contexts: Browser Checkout, Browser Governance, Browser Treasury

## Context

The SPA creates client-only idempotency keys for retry-safe commands. The
initial implementation called `crypto.randomUUID()` directly during React
state initialization. Browsers expose that method only in secure contexts;
the local HTTP NGINX edge therefore crashed before rendering the product shell.
This failure also made the real-browser contract unable to exercise its
intended accessibility and workspace paths.

The server remains the authority for UUIDv7 entity identifiers, including
settlement intents, calculations, governance records, and audit events. A
browser idempotency key is an opaque request key, not a server entity ID,
secret, bearer token, signature, or financial authority.

## Decision

Use one small browser identity port for client-only idempotency keys:

- call `crypto.randomUUID()` when the browser provides it;
- otherwise call `crypto.getRandomValues()` and set RFC 4122 version-4 and
  variant bits before canonical serialization;
- fail closed if no cryptographically secure random source exists; and
- use the same helper in quote, governance-draft, and vault-release forms.

The helper must not use timestamps, counters, `Math.random()`, a user input,
or a server round trip to manufacture a request key. Server-generated UUIDv7
identifiers remain unchanged and are returned by authoritative API responses.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Require HTTPS for all local browser tests | Rejected as the sole fix | HTTPS is correct for production, but it hides a legitimate clean-checkout local failure and makes local bootstrap unnecessarily fragile. |
| Use `Math.random()` or a timestamp fallback | Rejected | Predictable collisions and unsafe retry identity under concurrency. |
| Generate UUIDv7 in the browser | Rejected | A client request key is not a server entity ID and adding time-ordered semantics would confuse authority and ordering guarantees. |
| **Use native `randomUUID()` with a `getRandomValues()` v4 fallback** | **Selected** | Preserves CSPRNG guarantees in secure and local contexts, keeps the browser dependency-free, and maintains server UUIDv7 ownership. |

## Invariants and verification

- Every returned value is canonical UUID syntax with version 4 and RFC variant
  bits when the fallback is used.
- No fallback can succeed without a CSPRNG.
- The helper never claims that a request key is a calculation, settlement,
  attestation, or audit identifier.
- Unit tests cover both fallback generation and missing-CSPRNG failure.
- The real-browser UI journey must load the shell over the local edge, proving
  that browser initialization does not depend on secure-context-only APIs.
