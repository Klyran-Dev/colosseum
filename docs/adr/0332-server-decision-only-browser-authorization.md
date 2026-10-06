# ADR-0332: Server-decision-only browser authorization affordances

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal Low-Latency Architecture, Security, and Product UI
- Scope: React RBAC/PBAC/ABAC capability affordances

## Context

The React authorization helper retained `roles`, `scopes`, and attribute fields
on its snapshot model for display and test compatibility. Its fallback path
could nevertheless use those fields to infer an allow or deny result when a
snapshot was marked `server-allowed` but did not contain a decision for the
requested policy. That made the browser a second authorization evaluator and
could show a protected workspace based on incomplete, stale, or incorrectly
constructed presentation data.

The API and every protected handler remain the authorization authority. The
browser may render a server decision, but it must not reconstruct that decision
from copied JWT claims, role arrays, scope arrays, tenant identifiers, or
resource attributes.

## Decision

`evaluateUiAccess` uses only the exact server capability decision for the policy
being rendered. A parsed `server-allowed` snapshot with no matching decision is
`indeterminate` and must not produce an optimistic affordance. Anonymous and
server-denied states retain their existing explicit outcomes. The retained
metadata fields are non-authoritative and are not used as an authorization
fallback.

The API continues to re-authorize every protected operation. This ADR changes
only the browser presentation boundary; it does not grant or revoke access and
does not alter handler policy semantics.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Reconstruct decisions from browser role/scope/attribute fields | Rejected | Duplicates authority, permits stale or incomplete metadata to drive UI, and can disagree with resource-aware server policy. |
| Hide every protected workspace until a decision is present | Rejected | Loses the useful distinction between denied and not-yet-evaluated states and weakens recovery guidance. |
| **Render the exact server decision; otherwise remain indeterminate** | **Selected** | Preserves server authority, gives honest loading/recovery semantics, and adds no request-path latency or new dependency. |

## Invariants and failure policy

- A browser-only role, scope, tenant, or attribute value cannot produce an
  `allow` result.
- A missing policy entry in an otherwise `server-allowed` snapshot is
  `indeterminate`, never `allow`.
- A server-provided `deny` or `indeterminate` decision is rendered as returned
  after its bounded parser validates the decision and reason code.
- API and Solana authority boundaries remain unchanged and must repeat their
  own checks.
- Tests must prove both that a complete server decision renders and that
  contradictory or missing presentation metadata cannot override it.

## Latency and operations

The decision remains an in-memory map lookup. The new missing-entry branch is
constant time and does not add database, RPC, broker, or network work to the
browser hot path. An indeterminate result is observable through the existing
accessible UI state and does not imply a financial or settlement outcome.

## Verification

- Vitest proves explicit allow/deny/indeterminate server decisions are used.
- Vitest proves a `server-allowed` snapshot without the exact policy decision
  remains indeterminate even when copied roles, scopes, and attributes look
  sufficient.
- `make web-check` and `make web-browser-check` remain required.
- The capability snapshot parser continues to require the complete canonical
  policy catalog, so production snapshots cannot omit decisions silently.

