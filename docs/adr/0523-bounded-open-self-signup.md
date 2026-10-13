# ADR-0523: Bounded Open Self-Signup

- Status: Accepted
- Date: 2026-10-13
- Owners: Authentication and platform architecture

## Context

The deployed staging workspace needs a public account-creation path for
demonstration and evaluation. Unbounded self-signup would create an avoidable
automation and mailbox-cost surface, while per-email OTP limits do not bound
the number of new tenant records created in one day.

## Decision

Allow open self-signup only when `AUTH_SELF_SIGNUP=true` is explicitly set for
the deployment. Enforce a server-side global quota of 25 successful regular
self-signup provisions per UTC calendar day by default, configurable through
`AUTH_SELF_SIGNUP_DAILY_LIMIT` with a bounded non-local range of 1–10,000.

The quota is serialized with a PostgreSQL transaction advisory lock and counts
only committed `user_provisioned` audit events tagged `self_signup`. Invited
workspace joins and the configured initial-administrator bootstrap are separate
provisioning modes and do not consume this public quota. Existing OTP request,
resend, expiry, and attempt limits remain in force.

## Alternatives considered

1. No public signup. Rejected for the staging evaluation flow; retained as the
   default when the environment does not explicitly opt in.
2. Frontend-only or edge-only counting. Rejected because it is bypassable and
   cannot atomically coordinate multiple auth-service instances.
3. Per-email quota only. Rejected because it does not bound aggregate tenant
   creation or mailbox cost.
4. **Database-serialized global daily quota.** Selected because it is
   authoritative, auditable, race-safe, and compatible with the current
   PostgreSQL deployment.

## Consequences

- Staging can accept public registrations without allowing an unbounded signup
  burst.
- Production remains closed by default and must opt in explicitly with an
  approved quota and abuse-monitoring plan.
- At the UTC-day boundary, the quota resets without a scheduled cleanup job.
- The UI describes administrator invitations and access grants accurately; it
  does not promise automatic cross-tenant organization membership.
