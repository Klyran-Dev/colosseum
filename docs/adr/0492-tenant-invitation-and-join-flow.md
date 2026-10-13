# ADR-0492: Tenant invitation and join flow

## Status

Accepted for the authentication vertical slice.

## Problem

Email OTP signup currently creates a new merchant tenant for every new email.
That is safe as a default self-signup boundary, but it leaves administrators
without a production path to onboard another person into their tenant.

## Decision

The auth service owns a durable invitation record. An active administrator with
`identity:role:manage` creates an invitation for a normalized email, a bounded
role, and a UUIDv7 idempotency key. The invitation is tenant-scoped from the
verified administrator token; the browser cannot supply a merchant ID.

When that email completes the existing `signup` OTP flow, the service consumes
the unexpired pending invitation in the same transaction as user creation,
assigns the invited role, writes invitation and role audit events, and issues a
token containing the resulting tenant and roles. An uninvited self-signup keeps
the local-only new-tenant behavior. Invitations expire, may be revoked, and may
not be reused.

The local email adapter writes a bounded invitation notification artifact. A
provider-backed email adapter remains required before non-local deployment;
the local artifact is never treated as proof of delivery.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Accept a merchant ID from signup | Rejected | A caller-controlled tenant is an authority bypass. |
| Make every signup join a configured tenant | Rejected | It destroys tenant isolation and is unsafe for public signup. |
| Hosted invitation provider as authority | Rejected | The repository-owned auth service already owns OTP identity and token claims. |
| Auth-service invitation plus signup-time transactional binding | Selected | Preserves tenant authority and binds identity, role, and audit state atomically. |

## Invariants and failure behavior

- The invitation email is normalized and bounded before persistence.
- Only active administrators can create or revoke invitations.
- A target email has at most one pending invitation per tenant.
- An invitation is accepted once, only before expiry, and only by the same normalized email.
- Signup either binds the user, role, invitation state, and audit events together or binds none of them.
- Cross-tenant, revoked, expired, duplicate-idempotency, and database-uncertain operations fail closed.

## Acceptance evidence

- The migrations enforce pending-invitation uniqueness, bounded roles, expiry
  state, UUIDv7 identifiers, and UUIDv7 idempotency keys.
- Auth unit tests cover OTP/token primitives; Docker smoke covers administrator
  invite, invited OTP signup, same-tenant binding, and role assignment.
- A separate Docker smoke covers pending-invitation revocation and replay of
  the same revocation idempotency key.
