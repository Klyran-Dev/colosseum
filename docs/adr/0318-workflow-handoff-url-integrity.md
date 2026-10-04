# ADR-0318: Preserve addressable URLs across workflow handoffs

- Status: Accepted
- Date: 2026-10-04
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React SPA workspace handoffs, browser history, document quote to payment review

## Context

The static SPA already has an allow-listed workspace path for every production
workflow. A document-tax quote can hand an operator to document payment review
with a calculation reference held in memory. The handoff previously changed
the selected React workspace but did not push the corresponding URL, leaving
the browser address on the invoice-tax page. That breaks refresh, bookmarking,
support-link capture, and browser history even though the visible page changes.

The URL must identify the workspace only. Calculation evidence, access tokens,
wallet material, tenant identifiers, and financial facts remain outside the
URL and must be reloaded through their existing authenticated boundaries.

## Decision

All user-initiated workspace changes, including child-workspace handoffs, use
one App-owned navigation function that:

1. resolves the destination through `pathForWorkspace`;
2. calls `history.pushState` exactly once when the path changes;
3. updates React workspace state after the history mutation; and
4. preserves or clears in-memory results according to the existing explicit
   handoff policy.

The document quote handoff sets its calculation reference in memory and then
navigates to `/operations/document-payment`. A refresh or copied URL opens the
correct payment workspace, but does not pretend that the non-URL calculation
evidence survived; the operator must provide or reload the authenticated
evidence as the payment workspace requires.

## Alternatives considered

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Change React state only | Minimal code | Stale URL, broken refresh/bookmark/history semantics | Rejected |
| Put calculation ID or evidence in the URL | Easy reload handoff | Sensitive/resource data disclosure and authority confusion | Rejected |
| Add a full client router | Rich nested route lifecycle | New dependency and duplicated authority/focus behavior | Rejected |
| **Use the existing typed History API adapter for every handoff** | Stable URL, no sensitive disclosure, one navigation policy, low overhead | In-memory evidence still requires explicit reload | **Selected** |

## Invariants and failure policy

- Every visible workspace has its canonical allow-listed path.
- A handoff never places bearer credentials, wallet keys, tenant IDs, amounts,
  evidence, signatures, or authorization decisions in the URL.
- One user handoff creates at most one history entry; `popstate` does not push.
- Refreshing a handoff URL renders the destination workspace and does not claim
  that an in-memory calculation or settlement intent was restored.
- A missing or unknown workspace path falls back to the existing quote route.

## Verification

- Browser regression covers document quote → document payment URL integrity and
  direct reload of the destination path.
- Existing navigation unit tests continue to cover all 15 workspace mappings,
  unknown-path fallback, and sensitive-value non-disclosure.
- The NGINX Docker browser gate runs the handoff against the production static
  edge and preserves the accessibility audit.

## References

- [ADR-0278: Addressable workspace navigation](0278-addressable-workspace-navigation.md)
- [ADR-0207: Document checkout workspace](0207-document-checkout-workspace.md)
