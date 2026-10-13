# ADR-0509: Server-owned domain role catalog and explicit provisioning

## Status

Accepted for the first administrator provisioning slice.

## Context

The email OTP service already persists tenant membership and generic identity
roles. The API authorization catalog, however, protects tax governance,
treasury, revenue, refund, and recovery actions with explicit domain roles.
Granting those authorities by mapping `administrator` or `moderator` to every
financial capability would violate least privilege and would make the browser
misrepresent the real authority boundary.

## Decision

Keep role definitions in the versioned repository contract
`contracts/authorization-roles-v1.json`. The auth service validates this
catalog before binding, exposes the bounded assignable role list to an
authenticated administrator, and accepts role mutations only for roles marked
assignable by that catalog. Active assigned roles contribute only the scopes
listed for that role to the next OIDC-compatible token. Generic administrator
and moderator responsibilities remain separate from domain authority.

The checked-in source contract uses `authorization-roles/v1`; the administrator
API exposes the intentionally narrower browser response contract
`auth-role-catalog/v1`, which omits scopes so the browser cannot become a scope
authority.

The invitation flow remains limited to standard membership and moderator
onboarding. Domain responsibilities are assigned after the person has joined,
through the tenant-scoped, idempotent administrator mutation endpoint. Every
mutation remains durable and auditable in the existing role mutation tables.

## Alternatives considered

1. Map administrator/moderator to all domain scopes: rejected because it
   collapses identity administration into money-moving and tax authority.
2. Let the browser compose role-to-scope mappings: rejected because the
   browser is not an authority boundary and could drift from the API.
3. Add a separate role database with tenant-specific arbitrary permissions:
   rejected for this slice because it introduces a second mutable policy
   authority before the role semantics and approval ownership are settled.
4. Versioned server-owned role catalog with explicit assignment: selected
   because the API and auth service share a reviewable contract, the token
   remains bounded, and assignment is auditable and tenant-scoped.

## Invariants and failure behavior

- Unknown, non-assignable, duplicate, or malformed roles fail closed.
- No role mutation can cross tenant boundaries or target an inactive user.
- A role assignment does not itself authorize a Solana signature or vault
  release; the API repeats scope, role, resource, policy, and chain checks.
- Scope derivation is deterministic, deduplicated, and limited to server-owned
  catalog entries plus the existing environment-specific base/admin/moderator
  profiles.
- Catalog parse failure prevents the auth service from starting.

## Consequences

Administrators can now provision the exact responsibility required by each
workspace, and the next sign-in token drives the existing server capability
snapshot. Production still requires an approved identity-provisioning policy,
administrator ownership, separation of financial approval authorities, and
staging evidence before enabling high-risk assignments.
