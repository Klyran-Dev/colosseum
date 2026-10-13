# ADR-0527: Explicit tenant lifecycle, readiness, and administrator promotion

## Status

Accepted for implementation.

## Problem

The product currently uses `auth_users.merchant_id` as both the tenant
identifier and the implicit workspace entity. A configured initial administrator
can create the first tenant, but open self-signup creates additional users with
no explicit tenant profile, no real setup checklist, and no controlled path to
promote a second tenant administrator. This makes the first-run experience
ambiguous and encourages unsafe manual database changes.

## Decision

Introduce an explicit, tenant-scoped `auth_tenants` entity and server-owned
tenant context endpoint. Existing users are backfilled one row per
`merchant_id`; new signup provisions the tenant row atomically with the user.
Readiness is derived from durable records, not client state:

- workspace profile has a non-default display name;
- at least one active settlement route has finalized verification evidence;
- at least one effective published jurisdiction policy is joined to a published
  rule release.

Tenant administrators may update the display name through an idempotent,
audited mutation and may promote an active same-tenant user through a dedicated
administrator endpoint. The ordinary role catalog continues to keep
`administrator` non-assignable; UI role selectors cannot grant it.

## Alternatives

See `docs/architecture/alternative-analysis.md`.

- Keep using `merchant_id` as an implicit tenant: rejected because it cannot
  represent workspace lifecycle or readiness safely.
- Let the browser choose or request a tenant/admin role: rejected because the
  browser is untrusted.
- Make every self-signup an administrator: rejected because it collapses tenant
  governance and makes invitations meaningless.
- Add a platform-admin hierarchy now: rejected; platform authority needs a
  separate authority model and review.

## Invariants and threat model

1. The authenticated principal derives tenant scope; no tenant ID is accepted
   from a form or URL as an authority boundary.
2. Readiness is informational and never authorizes settlement, tax, or release.
3. Administrator promotion is same-tenant, idempotent, durable, and audited.
4. The last existing administrator cannot be removed by this change; demotion
   remains a separately designed recovery workflow.
5. Missing or suspended tenant context fails closed and produces actionable
   recovery copy rather than fabricated readiness.

## Sequence

```text
OTP verify -> provision auth_tenants + auth_users + role -> issue token
                                   |
                                   v
                         tenant context / readiness
                                   |
                  admin profile update or admin promotion
```

## Budgets and operational behavior

- Tenant context is one bounded indexed PostgreSQL read, at most three
  readiness existence checks, and no Solana/RPC/provider work.
- Results are bounded to three setup steps and may be refreshed at 30 seconds.
- Profile and promotion writes use UUIDv7 idempotency keys and one transaction
  containing the durable mutation and audit event.
- Unknown database state, cross-tenant targets, stale tokens, and provider
  failures fail closed.

## Acceptance criteria

- Every authenticated tenant has an explicit tenant row after migration.
- A self-signup tenant receives a real context response with setup state.
- The administrator can name the workspace and see readiness change only after
  durable records satisfy the step.
- An administrator can promote a same-tenant user without exposing an admin
  role selector to arbitrary users.
- Frontend tests cover ready, in-progress, blocked, malformed, and unavailable
  tenant context states.
