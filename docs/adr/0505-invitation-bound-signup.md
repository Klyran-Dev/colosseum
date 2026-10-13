# ADR-0505: Bind invited sign-up to the invitation reference

- Status: Accepted
- Date: 2026-10-11
- Decision owners: Identity and web platform bounded contexts

## Context

Administrator invitations currently deliver a generic sign-up URL. The auth
service can still discover a pending invitation by normalized email, but the
browser and OTP challenge do not carry the invitation that initiated the
journey. That weakens the end-to-end contract and prevents the sign-up surface
from explaining why the recipient is being asked to create an account.

## Decision

Invitation emails include the server-issued invitation UUID as a bounded
`invite` query parameter on the dedicated sign-up route. The browser validates
the parameter as a UUIDv7-shaped identifier, preserves it only on the sign-up
route, and sends it back as `invitation_id` when requesting the OTP. The auth
service validates that the invitation is pending, unexpired, and belongs to
the normalized email before persisting the reference on the OTP challenge.

OTP verification consumes the exact bound invitation when one is present. The
existing email-based lookup remains available for ordinary self-sign-up and
legacy invitation links, so the rollout is backward-compatible. The server
returns the invited role only after the invitation/email match succeeds; the
browser uses it for presentation and never for authorization.

## Alternatives considered

1. Keep generic sign-up links and resolve by email only. Rejected because the
   initiating invitation is not explicit in the browser/server contract.
2. Put role or tenant data directly in the URL. Rejected because those values
   are presentation data and must remain server-owned.
3. Use a long-lived bearer invitation token. Rejected for this bounded local
   change because the existing UUID reference plus email OTP already provides
   the possession and identity checks; a separate token lifecycle would be a
   larger credential system.

## Invariants and failure behavior

- An invitation reference never authorizes an account by itself.
- The reference must match the normalized email, pending status, and expiry.
- A mismatched, revoked, expired, or malformed reference fails closed.
- The invitation reference is stored with the challenge and cannot be swapped
  between request and verification.
- Ordinary self-sign-up remains unchanged and has no invitation reference.
- The role returned to the browser is informational; durable role assignment
  remains transactional server work during verification.

## Verification

- Auth-flow unit tests cover invitation parameter validation and URL retention.
- Auth-page tests cover the invited-responsibility context without adding a
  second sign-in action.
- Rust tests cover escaped invitation links and the invitation-bound request
  contract.
- The Docker browser gate follows a real administrator-created invitation URL,
  provisions the invited moderator, and verifies the resulting role.
