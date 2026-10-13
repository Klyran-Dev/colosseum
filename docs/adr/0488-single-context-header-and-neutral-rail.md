# ADR-0488: Use one context summary and a neutral navigation rail

Status: Accepted

Date: 2026-10-10

## Context

The shell has a modern status deck, but public workspaces still repeat the
same access explanation in a page-context card and the first status card. The
expanded purple rail also attracts more attention than the financial and tax
workflow itself, especially in the light theme.

## Decision

Use the status deck as the single summary of access, deployment, network, and
API context. Keep the page-context card only for an active authorization error
or pending server decision where the extra explanation is actionable. Public
and stable authenticated workspaces do not render a second access summary.

Use a neutral semantic rail surface in both schemes. The active workspace
remains marked by the primary role and the rail remains keyboard and
screen-reader discoverable through its existing accessible names and
descriptions. Brand identity stays in the mark and the active state rather
than a full-height saturated background.

## Alternatives considered

- Keep both summaries for reassurance: rejected because the same state is
  presented twice before the user can act.
- Remove all access context: rejected because protected-state explanations
  and authorization failures still need a visible, truthful home.
- Keep a saturated brand rail: rejected because it competes with workflow
  content and reduces the calm Material surface hierarchy.
- Make the rail visually plain and remove active treatment: rejected because
  navigation orientation and current location would become ambiguous.

## Invariants

- Authorization errors and pending server decisions remain visible and fail
  closed.
- No access card grants permission or implies settlement authority.
- Rail labels, focus order, touch targets, and mobile disclosure behavior stay
  accessible.
- All colors continue to resolve through repository-owned semantic roles.
