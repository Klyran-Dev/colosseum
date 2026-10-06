# ADR-0330: Server-owned document evidence status for checkout

- Status: Accepted
- Date: 2026-10-04
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: authenticated document checkout, API evidence presentation, and browser protocol boundaries

## Context

The document-payment workspace currently asks an operator to paste a complete
issuer-signed attestation JSON document before the service can verify it. This
exposes protocol-shaped data in the primary task, invites copy/paste and
stale-record errors, and makes the browser appear to own evidence selection.
The signed record is already persisted by the authenticated issuer-ingestion
boundary and is loaded server-side by document settlement compilation.

The browser needs a concise, route-bound status and the minimum evidence needed
to explain the decision. It must not receive raw signed protocol material just
to render checkout, and it must not be able to mark an invoice verified.

## Decision

1. Keep `POST /v1/tax/document-quotes/{calculation_id}/attestation/verify` as
   the issuer-ingestion boundary. It accepts the signed document attestation,
   verifies its binding, and persists immutable evidence.
2. Add an authenticated, read-only
   `GET /v1/tax/document-quotes/{calculation_id}/attestation/status?route_id=...`
   endpoint for checkout. The server loads the tenant-scoped stored
   attestation, resolves the server-owned route, verifies time and document
   binding, and returns a bounded status response. The request contains only a
   calculation ID and route ID; it never contains an attestation document,
   amount, mint, recipient, or protocol authority.
3. Require the settlement-compilation scope for the status endpoint. A user
   may see only their authenticated tenant's evidence, and the response is
   useful only for the current server-owned route.
4. Return only status metadata: calculation ID, payload digest, issuer key
   bytes, expiry, and server check time. Raw signed JSON remains behind the
   issuer and audit boundaries.
5. Replace the visible JSON textarea in document checkout with a plain-language
   evidence status card and a collapsed technical-details disclosure. Missing,
   expired, route-mismatched, unavailable, timeout, and invalid evidence states
   remain explicit and keep payment preparation/signing disabled.
6. The browser may display a successful server response but may not infer
   authorization from it. Document compilation repeats the full server-side
   attestation verification before creating a durable payment intent.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep manual raw JSON paste | Small API change | High operator error, protocol clutter, stale evidence, poor accessibility | Rejected |
| Return raw attestation from a read endpoint | Easy browser reuse | Expands sensitive protocol surface and encourages client parsing/authority | Rejected |
| Let the browser choose a stored evidence record | Flexible UI | Client-controlled evidence selection and ambiguous route binding | Rejected |
| **Server-owned route-bound status read** | Low-latency, bounded, accessible, tenant-safe, no client authority | Requires a dedicated read contract | **Selected** |

## Contract

Successful response schema:

```json
{
  "status": "verified",
  "calculation_id": "0199...",
  "payload_digest": [0, 1, 2],
  "issuer_public_key": [3, 4, 5],
  "expires_at_unix": 1791120000,
  "checked_at_unix": 1791119700
}
```

`calculation_id` and `route_id` are UUIDv7. `payload_digest` and
`issuer_public_key` are bounded byte arrays. The endpoint returns a typed
not-found or conflict error for absent, expired, invalid, or unavailable
evidence; no success response is synthesized in the browser.

## Invariants and budgets

- Tenant scope comes only from the authenticated principal.
- The route is resolved from the tenant-scoped, finalized-verified route
  registry; caller-provided account topology is never accepted.
- A successful status response proves only that the stored evidence was
  checked at the returned server time. It does not prove settlement or tax
  filing/compliance.
- The hot path performs bounded indexed reads and signature/binding checks with
  a p95 local target below 25 ms once the database pool is warm.
- Existing compilation remains the final authority and repeats the checks;
  status reads cannot create intents, outbox events, or token movement.

## Verification

- Add failing API tests for missing authentication, missing route ID,
  tenant-scoped status lookup, and rejection of raw attestation fields.
- Add frontend parser and journey tests for verified, missing, expired,
  unavailable, and retry states; assert no raw JSON textarea is rendered.
- Run the full Rust/web check, browser accessibility journey, API contract,
  and public-documentation checks.

