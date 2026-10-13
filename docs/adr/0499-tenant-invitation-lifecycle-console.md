# ADR-0499: Tenant invitation lifecycle console

## Status

Accepted for the authentication vertical slice — 2026-10-11.

## Problem

The administrator surface can create a tenant invitation and the auth service
can revoke one, but the browser cannot inspect pending invitations or initiate
the revoke lifecycle. That leaves administrators without a reliable way to
detect stale onboarding requests, verify the intended role, or stop an invite
before it is consumed.

## Decision

Add a bounded `GET /v1/admin/invitations` projection owned by the auth service.
The authenticated administrator's tenant is derived from the verified token;
the browser never supplies a merchant identifier. The result is limited to the
most recent 100 invitations and includes only the normalized recipient email,
bounded role/status values, UUIDv7 invitation ID, creation and expiry times,
and the optional acceptance/revocation time needed to explain lifecycle state.

Extend the administrator Metronic workspace with an invitation lifecycle table.
Pending invitations expose a revoke action. Revoke requests use a fresh UUIDv7
idempotency key and the existing transactional `DELETE` endpoint; the UI
refreshes the server projection after success. Expired, accepted, and revoked
records remain visible as read-only history so the administrator can distinguish
absence from a terminal state.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Keep creation-only UI | Rejected | Leaves stale access requests unactionable and makes the existing revoke API undiscoverable. |
| Let the browser filter or infer tenant data | Rejected | Client input is not an authority boundary and could leak cross-tenant records. |
| Delete invitation rows after revoke | Rejected | Destroys auditability and makes replay/incident investigation ambiguous. |
| Auth-service projection plus append-only terminal state | Selected | Preserves tenant authority, bounded reads, lifecycle evidence, and idempotent mutation semantics. |

## Invariants and failure behavior

- Only an active administrator with `identity:role:manage` can list or revoke
  invitations.
- All reads and writes are tenant-scoped from the authenticated principal.
- The list is bounded to 100 records and never returns invitation secrets.
- Pending invitations may be revoked once; accepted, expired, and already
  revoked invitations are terminal and fail closed on a new revoke request.
- Duplicate revoke requests with the same UUIDv7 idempotency key replay the
  original response without a second mutation or audit event.
- Database uncertainty, malformed identity responses, or stale UI state never
  display a successful revoke.

## Verification

- Rust route tests cover bounded serialization and tenant-scoped invitation
  listing; existing service tests cover revoke idempotency and terminal states.
- React tests cover loading, lifecycle labels, revoke request shape, and
  refresh behavior.
- The Docker auth browser journey creates an invitation, verifies it in the
  lifecycle table, revokes it, and verifies the terminal status before the
  invited user can consume it.
