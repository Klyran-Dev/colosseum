# ADR-0461: Hide the stable access card after authorization is verified

- Status: Accepted
- Date: 2026-10-10
- Owners: Product engineering, accessibility, and operations

## Context

The authenticated shell repeated the same stable access result in four places:
the navigation identity, the system/session status row, the account menu, and
the page-heading “Your access” card. This pushed the task below the fold on
small screens and made a verified operator read the same state before every
workflow. The access card is still useful for public-access boundaries and
uncertain/expired states, where it explains a decision that needs attention.

## Decision

Render the page-heading access card only when the workspace is public or the
authenticated authorization snapshot is not in the stable `server-allowed`
state. Once access is verified, the navigation identity and account menu remain
the authoritative shell affordances for session and tenant context. Pending,
expired, unavailable, and signed-out states continue to show their existing
gates and recovery copy.

## Alternatives considered

1. **Keep the card on every page.** Rejected because stable state is repeated
   and displaces the task on narrow layouts.
2. **Remove all access status from the shell.** Rejected because public and
   degraded states still need an explicit boundary and recovery explanation.
3. **Make the card compact but always visible.** Rejected because it retains
   duplicated stable information and adds another responsive variant.
4. **Hide only the stable authenticated card.** Selected because it keeps
   uncertainty visible while making verified work task-first.

## Acceptance criteria

- Verified authenticated workspaces begin with their task content, without a
  duplicate page-heading access card.
- Public and uncertain/expired authorization states retain explanatory access
  context.
- Account-menu access facts remain available and keyboard accessible.
- Existing access gates, RTL/reflow behavior, and accessibility checks pass.
