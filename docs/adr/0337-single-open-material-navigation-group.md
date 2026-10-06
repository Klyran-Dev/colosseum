# ADR-0337: Single-open Material 3 navigation group

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React SPA product-shell navigation disclosure state

## Context

The operational shell has a complete task catalog grouped into Daily work,
Rules & access, Money movement, and Records & issues. Native disclosures keep
all routes discoverable, but accumulating independently opened groups allows a
long-running operator session to recreate the original cluttered sidebar.
That conflicts with ADR-0323's active-workspace hierarchy and makes the
primary task compete with lower-frequency work.

This is a presentation concern only. Group disclosure must not hide a route
from authorization, change a server decision, or alter the addressable
workspace URL. The navigation state must remain deterministic, keyboard
reachable, and inexpensive in the static React SPA.

## Decision

Represent the navigation disclosure as one explicit `WorkspaceGroup`, not a
set of independently open groups. The current workspace group is open on
initial render and after a route change. When an operator opens another group,
the previous group closes immediately. Attempting to close the active group
restores the current workspace group, so the shell never leaves the operator
without a visible route context.

The disclosure remains native `<details>` with semantic summaries and the
existing `aria-current="page"` marker. Lower-frequency workspaces remain
available through their group summary; no search-only or hidden route model is
introduced.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Accumulate open groups | Preserves local browsing context | Recreates sidebar clutter and weakens task hierarchy over time | Rejected |
| Close every group, including the active group | Maximum compactness | Removes current route context and increases navigation friction | Rejected |
| Search-only command palette | Very compact | Reduces discoverability and adds interaction cost for infrequent but important workflows | Rejected |
| **One explicit open group anchored to the current workspace** | Compact, deterministic, discoverable, native, and low-cost | Operators reopen a group when switching between categories | **Selected** |

## Invariants and failure behavior

- At most one navigation group is open at any time.
- The group containing the current workspace is open after every parent route
  change and after an attempted close of the active group.
- Every allow-listed workspace remains in the DOM when its group is open and
  remains addressable through the existing typed pathname mapping.
- Disclosure state never grants, denies, or infers RBAC/PBAC/ABAC access.
- No status color, risk badge, or random color is introduced by this change.
- Narrow navigation still closes after workspace selection; RTL and reduced
  motion preserve the same structure and meaning.

## Latency and resource budget

- O(1) in-memory state transition per disclosure or workspace selection.
- No network request, authorization evaluation, database read, or wallet/RPC
  operation is added to navigation.
- No additional bundle dependency or persistent storage is introduced.

## Verification

- Add a failing-first component test proving that opening a second group closes
  the first while the current workspace remains marked as the page.
- Run the complete web unit suite, typecheck, production build, WCAG checks,
  Material color contracts, and real-browser responsive/RTL journeys.
- Manually review wide, narrow, keyboard-only, screen-reader, light, dark,
  and system theme behavior before production release evidence is accepted.

## References

- [ADR-0323: Material 3 Expressive operational workspace composition](0323-material-expressive-operational-workspace-composition.md)
- [ADR-0317: Material 3 Expressive surface hierarchy and navigation density](0317-material-expressive-surface-hierarchy-and-navigation-density.md)
- [ADR-0326: Material 3 Expressive semantic state hierarchy](0326-material-expressive-semantic-state-hierarchy.md)
