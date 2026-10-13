# ADR-0491: Auth-service role administration

## Status

Accepted for the first production vertical slice.

## Problem

Email OTP establishes an authenticated account, but administrator and moderator
responsibilities must be assigned and revoked by a server-owned workflow. A
browser-controlled role selector, JWT decoding, or direct database mutation
would create an authority bypass and would not provide an auditable lifecycle.

## Decision

Keep role administration in the dedicated authentication bounded context. A
merchant is a tenant boundary with many authenticated users; migration `0077`
removes the earlier bootstrap-era one-user-per-merchant uniqueness constraint
and adds a tenant membership index. The
auth service verifies its own EdDSA access tokens, requires the administrator
role, derives the target tenant from the authenticated actor, and exposes a
bounded user directory plus moderator assignment/revocation endpoints. The
target user must belong to the actor's merchant tenant. Mutations require a
UUIDv7 `Idempotency-Key`, execute in a transaction, fence the role state, and
write an append-only auth audit event in the same transaction. The final
administrator cannot revoke their own administrator responsibility.

The browser receives only the server-owned capability snapshot and uses the
role-management panel as an affordance. Every mutation is authorized again by
the auth service. Newly assigned roles appear in a user's next token; existing
tokens remain bounded by their expiry and are not rewritten in the browser.

Local Docker may opt into a clearly named bootstrap administrator email for
synthetic fixtures. The setting is rejected outside `local` and is not a
production provisioning mechanism.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Put role writes in the React app or accept a role in signup | Rejected | Client input is not an authority boundary and is vulnerable to privilege escalation. |
| Add role routes to the settlement API | Rejected | Couples identity lifecycle to money-moving API state and duplicates auth-service ownership. |
| Delegate to a hosted identity provider now | Rejected | The current OIDC-compatible contract and email OTP persistence are repository-owned; migration would add an external dependency before the role contract is proven. |
| Auth-service routes with signed-token verification and PostgreSQL transactions | Selected | Preserves bounded-context ownership, reuses the existing EdDSA contract, keeps tenant checks beside identity data, and gives one atomic audit boundary. |

## Invariants and threat model

- Only an active administrator may enumerate users or mutate roles.
- Actor and target tenant IDs must match; the browser cannot supply a tenant ID.
- Only the bounded `moderator` role is assignable by this workflow.
- Assignment and revocation are idempotent and produce at most one mutation event
  for an idempotency key.
- An administrator cannot remove the last active administrator in a tenant.
- Disabled users cannot receive a new token or be managed as active operators.
- Email addresses are returned only to an authenticated administrator in the same
  tenant.

## Budgets and failure behavior

The list endpoint is bounded to 100 users per request. Role mutations use one
database transaction and a bounded service worker permit. Database, token,
idempotency, or audit uncertainty fails closed with no success response. The
browser shows loading, empty, failure, retry, and success states.

## Acceptance evidence

- Rust tests cover token verification, role allow-listing, tenant mismatch,
  idempotency-key validation, and last-administrator protection.
- PostgreSQL integration tests cover assignment/revocation and audit atomicity.
- The focused browser/UI contract test covers the capability-gated directory and
  idempotent mutation request shape; the Docker live smoke covers administrator
  OTP sign-in, same-tenant membership, moderator grant, idempotent replay,
  revoke, and durable audit rows.

