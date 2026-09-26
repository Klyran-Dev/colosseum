# ADR-0037: Enforce JWT scopes and roles at the service boundary

- Status: Accepted
- Date: 2026-09-25
- Owners: Security / API Platform
- Scope: authenticated API operations

## Context

Merchant-scoped authentication identifies a subject and merchant, but identity
alone must not authorize quote creation, settlement compilation, policy
publishing, or evidence access. The synchronous quote path must stay low
latency and must not call a remote introspection service for every request.

## Decision

Accept OAuth/OIDC-compatible JWT `scope` (space-delimited) and optional `roles`
claims. The verifier normalizes and bounds both collections. Each protected
handler declares the required operation scope and fails with HTTP 403 when the
principal is authenticated but not authorized. The first persisted quote
operation requires `tax:quote`; future control-plane operations receive
separate scopes rather than reusing this one.

Merchant identity remains derived from the validated `merchant_id` claim. A
scope never substitutes for merchant binding, issuer/audience checks, expiry,
signature verification, or future revocation policy.

## Invariants

- Missing or malformed scopes do not grant access.
- Authentication failures return 401; authenticated-but-unauthorized requests
  return 403 without revealing policy internals.
- Scope matching is exact and case-sensitive; no prefix or wildcard matching.
- Claim size and collection counts are bounded to protect the hot path.
- Authorization is repeated in the service handler, even when an edge gateway
  also enforces it.

## Alternatives considered

See `docs/architecture/alternative-analysis.md` under “API authorization
claims”. Authentication-only and gateway-only enforcement were rejected;
remote introspection remains appropriate only for bounded control-plane paths.

## Consequences

Positive:

- Least-privilege operations are explicit and locally enforceable.
- Quote latency does not depend on an authorization network round trip.
- Future roles and administrative workflows can be added without weakening
  merchant binding.

Costs:

- Identity-provider claim issuance and revocation must be governed and tested.
- Scope policy changes require coordinated identity/API deployment and audit.
- Static/local synthetic tokens must carry only test scopes and never represent
  production authority.
