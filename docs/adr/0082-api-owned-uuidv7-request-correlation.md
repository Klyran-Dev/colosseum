# ADR-0082: API-owned UUIDv7 request correlation

- Status: Accepted
- Date: 2026-09-26
- Owners: Platform/API architecture

## Context

The API, edge, workers, indexer, and reconciliation paths need one bounded
correlation identifier for support, structured logs, traces, and user-visible
failure reports. The current NGINX `$request_id` is an edge-generated opaque
hex value and direct API traffic has no guaranteed identifier. Accepting an
arbitrary caller-provided value would permit log correlation spoofing and
would not satisfy the repository UUIDv7 identifier policy.

Correlation is diagnostic metadata, not an authorization credential and not a
strict ordering primitive. It must not be used as a nonce, signature, policy
digest, or evidence identity.

## Decision

The Rust API owns the `x-request-id` boundary. For every request, including
health, error, and rejected-authentication responses, the API generates a
cryptographically secure UUIDv7, overwrites any incoming `x-request-id`,
attaches it to the request and response, and exposes it to the tracing layer.
The value is bounded to the canonical UUID textual representation. NGINX
forwards the API-generated value and does not generate a competing identifier.

The identifier is propagated through application logs and downstream request
contexts where those ports exist. It is safe to display to a user as a support
reference, but it is not proof of request authenticity and never authorizes a
tenant or settlement.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| **API-owned UUIDv7 middleware** | **Selected** | One authority for direct and edge traffic; sortable diagnostic IDs; easy to test at the router boundary |
| Trust NGINX `$request_id` | Rejected | Non-UUIDv7 edge format, absent for direct API traffic, and not controlled by Rust services |
| Accept a caller-supplied `x-request-id` | Rejected | Enables spoofed log correlation and unbounded/invalid values |
| Generate UUIDv4 with `MakeRequestUuid` | Rejected | Does not meet the repository’s UUIDv7 correlation-ID mandate |
| Generate IDs in every handler | Rejected | Duplicated behavior and misses middleware failures, auth failures, and health endpoints |

## Invariants and acceptance criteria

1. Every API response has exactly one canonical `x-request-id` UUIDv7.
2. An incoming `x-request-id` never controls the generated value.
3. Request IDs are generated without network, database, RPC, or blocking work.
4. Request IDs are not used as authentication, idempotency, nonce, or evidence
   values.
5. Tests cover direct traffic, forged incoming IDs, and error responses.
6. NGINX integration proves the header survives the edge without exposing a
   second edge-generated identity.

## Consequences

The API adds a negligible allocation and UUID generation cost to each request.
Operations receives a stable support reference and can correlate edge and
service logs. Downstream services must preserve the value when making a
request, but they remain responsible for their own service-local boundaries.

