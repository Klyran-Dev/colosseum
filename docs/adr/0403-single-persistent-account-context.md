# ADR-0403: Single persistent account context in the navigation shell

- Status: Accepted
- Date: 2026-10-06
- Owners: Product shell and authorization boundaries

## Context

The operations shell exposed the same authenticated-account context in the
sidebar context block, the persistent access state at the bottom of the
sidebar, the page access card, and the account menu. Repeating identity and
authorization status increased visual density and made it harder to identify
which surface was actionable. It also encouraged operators to treat a
presentation duplicate as an additional authorization signal.

The shell still needs to make protection state visible without hiding the
server-owned RBAC/PBAC/ABAC result or the recovery path for an unavailable
authorization check.

## Decision

The sidebar contains one persistent, compact access-state surface only. It
communicates whether sign-in and protected actions need attention, but does not
repeat tenant or subject identity. Verified identity details remain available
in the account menu, and the current workspace retains its page-level access
context and progressive protection disclosure.

The browser does not derive or merge authorization decisions. Removing a
duplicate presentation surface cannot change `AuthorizationSnapshot`, policy
evaluation, navigation access, or protected action behavior.

## Alternatives considered

| Alternative | Density | Identity clarity | Authorization safety | Decision |
| --- | --- | --- | --- | --- |
| Keep account context in every shell region | Fail | Repetitive | Pass | Rejected |
| Remove all persistent access state | Pass | Partial | Fail; recovery state is easy to miss | Rejected |
| Move all identity and access information into a modal | Pass | Partial | Partial; adds interaction cost | Rejected |
| **Keep one persistent access state and progressive identity details** | **Pass** | **Pass** | **Pass** | **Selected** |

## Invariants

- The sidebar has exactly one persistent access-state presentation.
- Tenant IDs and subjects are never rendered in the persistent sidebar state.
- The account menu remains the only shell surface for verified identity detail.
- Server-owned capability decisions, reason codes, expiry, and failure states
  remain unchanged and accessible through the page context, navigation context,
  and account/access workspaces.
- This is a presentation-only change with no API, persistence, wallet, tax,
  or settlement authority impact.

## Verification

- Add a failing-first shell regression that proves verified account context is
  rendered once in the shell while persistent access state remains visible.
- Run the shell/UI tests, typecheck, production build, real-browser journeys,
  accessibility checks, and the full repository gate.
- Retain manual review for keyboard, screen reader, responsive, RTL, and
  authorization-operator comprehension before any production UI claim.
