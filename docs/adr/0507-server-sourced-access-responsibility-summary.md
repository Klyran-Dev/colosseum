# ADR-0507: Show server-sourced access responsibilities in the Access & approvals workspace

- Status: Accepted
- Date: 2026-10-11
- Scope: React SPA presentation of the authenticated authorization snapshot

## Context

The Metronic Demo 3 shell already shows the signed-in person's responsibilities
inside the account menu. The Access & approvals workspace still presents only a
generic policy matrix, so an operator must infer what the current server
decision means from a long list of actions. That increases cognitive load and
makes the administrator, moderator, and merchant-operator journeys look alike.

The API capability response is the authority for the current tenant, roles, and
capability decisions. The browser must not decode tokens or derive permissions
from role names. This change is therefore limited to presenting the returned
roles and the returned capability counts; it does not grant, predict, or route
any authority.

## Decision

Add a compact “Your responsibilities” summary above the policy matrix. Render
server-returned role labels using the existing localized shell presenter and
show bounded counts for available, unavailable, and context-dependent actions
from the returned capability decisions. Anonymous, loading, and unavailable
snapshots render an explicit neutral state rather than zeroes or inferred
permissions.

## Alternatives

1. Keep the generic matrix only. Rejected because users have no concise answer
   to “what is my responsibility here?”
2. Decode roles or scopes from the JWT in the browser. Rejected because token
   claims are not the server's tenant/resource decision and would duplicate the
   authorization boundary.
3. Add client-side role-to-capability rules. Rejected because it can create
   misleading affordances when server policy changes or resource context is
   required.
4. Add a separate profile endpoint. Rejected because the existing short-lived
   capability snapshot already carries the verified identity and decisions.

## Invariants and acceptance criteria

- Only `AuthorizationSnapshot.roles` and `AuthorizationSnapshot.capabilities`
  are displayed; no role is mapped to a permission in the browser.
- The summary does not add a sign-in action or duplicate the shell's session
  control.
- An incomplete, anonymous, loading, or failed snapshot never displays a
  fabricated zero count.
- Role labels are localized through the existing bounded presenter, with a
  readable fallback for unknown server roles.
- Existing policy rows, server decisions, accessibility semantics, and all
  existing locales remain intact.
- Unit tests cover authenticated, anonymous, and incomplete/error states.

