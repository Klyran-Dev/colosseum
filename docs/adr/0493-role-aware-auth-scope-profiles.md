# ADR-0493: Role-aware authentication scope profiles

## Status

Accepted for the authentication vertical slice.

## Problem

The email-OTP issuer currently applies one global scope list to every token.
That makes a signed-in administrator indistinguishable from a standard
operator at the scope layer: the administrator UI requires `audit:read` and
`identity:role:manage`, while granting those scopes to every self-signup would
overstate authority. Moderator sign-in also needs a bounded, explicit profile
rather than an unexamined global default.

## Decision

Keep scope authority in the auth service and derive the final token scope set
from three deployment-owned profiles:

- base scopes for every active account;
- administrator scopes when the durable `administrator` role is active;
- moderator scopes when the durable `moderator` role is active.

The service de-duplicates scopes, preserves deterministic configuration order,
and only recognizes these two elevated roles for profile expansion. The
browser never selects a profile or edits a scope claim. The API continues to
verify the signed token and evaluate its own scope/role policy on every
request.

Local Docker grants `tax:quote` to the base operator, `audit:read` to
moderators, and `audit:read identity:role:manage` to administrators. Non-local
deployments must declare all three profile variables explicitly.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| One global scope list | Rejected | Gives standard accounts privileged or misleading claims. |
| Browser-side scope/role composition | Rejected | Client state is not an authority boundary and can become stale or forged. |
| Hosted identity-provider group mapping | Deferred | Valid production integration, but it would make the local repository-owned issuer unable to prove the first vertical slice. |
| Auth-service role-aware scope profiles | Selected | Keeps tenant role state and signed claims in one bounded context with deterministic deployment configuration. |

## Invariants

- Base users never receive administrator-only scopes from role profile logic.
- A profile is expanded only from roles read from the durable auth database or
  an explicitly configured base role; browser input cannot add roles.
- Scope lists are bounded, deterministic, and de-duplicated before signing.
- API authorization remains authoritative; a scope claim alone never bypasses
  a required role or resource check.
- Missing non-local profile configuration fails closed before the listener binds.

## Acceptance evidence

- Pure auth tests cover base, moderator, administrator, duplicate, and unknown
  role profile derivation.
- Docker smoke verifies real OTP tokens produce distinct capability decisions
  for a standard user, invited moderator, and administrator.
- The browser gate continues to verify the server-owned capability snapshot and
  role-management UI without decoding JWT claims in the client.
