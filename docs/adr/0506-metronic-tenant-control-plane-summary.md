# ADR-0506: Add a server-derived tenant control-plane summary

- Status: Accepted
- Date: 2026-10-11
- Decision owners: Identity and web platform bounded contexts

## Context

The administrator role console already loads the tenant-scoped people and
invitation projections from the authentication service, but presents them as
one long legacy table. Administrators must repeatedly scan two tables to answer
the first operational questions: how many people are active, how many carry the
bounded moderator responsibility, and whether onboarding requests are waiting.

## Decision

Add a compact Metronic control-plane summary above the existing invitation and
people projections. The summary derives its values only from the already loaded
server responses. While either response is loading or invalid, the value is an
unknown em dash rather than a fabricated zero. The summary is explanatory UI;
the auth service remains the authority for tenant scope, role mutations,
invitation state, and audit records.

Use the existing semantic status/deck primitives and responsive layout tokens
already owned by the SPA. Do not add a second endpoint, browser-side role
assignment model, polling loop, or decorative chart.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Keep the two tables as the first content | Rejected | Forces administrators to scan detailed records before understanding the tenant posture. |
| Add a client-only dashboard with optimistic counts | Rejected | Counts can be stale or misleading and must not become an authority signal. |
| Add a separate analytics endpoint | Rejected | Adds latency and another contract for values derivable from the bounded projections already required by the console. |
| **Server-derived summary over the existing projections** | **Selected** | Improves scanability without expanding the trust boundary or changing mutation semantics. |

## Invariants and failure behavior

- Counts are derived only from validated `users` and `invitations` responses.
- Loading and error states render an unknown value, never an implied zero.
- The summary cannot enable, disable, assign, revoke, or authorize an action.
- Existing idempotent role and invitation mutations remain unchanged.
- The layout remains readable at narrow widths and uses the repository semantic
  color and typography tokens.

## Verification

- React tests prove loaded counts and unknown loading values.
- Existing role-management tests continue to prove tenant-scoped fetches,
  idempotency headers, invitation lifecycle, and authorization denial.
- The Docker auth browser flow continues to verify administrator, operator, and
  moderator journeys against the real service.
