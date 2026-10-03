# ADR-0277: Complete server-owned workspace capability catalog

## Status

Accepted — implemented and locally verified; staging and production evidence
remain open.

## Context

The product shell presents a real operations surface across tax preparation,
governance, money movement, and recovery. The server-owned capability snapshot
currently describes only part of that surface. A signed-in operator can
therefore see a generic access state for document payments, adjustments,
refunds, and reconciliation even though each workspace has a different
protected API boundary.

That is a product and authorization explanation defect. It does not make the
browser an authority, but it prevents the browser from accurately explaining
the next safe action. It also makes a latency benchmark incomplete because it
does not validate the full navigation contract.

## Decision

The API capability catalog is one bounded, canonical policy entry per
production workspace. Every entry names the exact protected entry-point scope,
the role required for the primary operation, and any context that cannot be
resolved before a record or jurisdiction is selected.

The catalog covers these capability IDs:

```text
access.control
evidence.replay
refund.claim.cancel
refund.claim.create
refund.payment.release
settlement.adjustment
settlement.document.wallet
settlement.wallet
tax.document.quote.create
tax.preview.run
tax.quote.create
tax.rule.govern
vault.reconciliation
vault.release
```

The response remains a server-owned affordance snapshot. The React shell may
show `Available`, `Not available`, or `Needs context` beside a workspace, but
it never grants access, hides an audit trail, or skips the protected API's
exact re-authorization. A denied workspace remains discoverable so an
operator can understand why it is unavailable and request the correct role.

The primary capability for a workspace represents its main operator action,
not every action available inside the workspace. Read, approve, submit,
reconcile, and chain-evidence checks continue to be enforced independently by
their handlers.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Keep seven broad capabilities | Small response and little maintenance | Several high-consequence workspaces have no explainable access state | Rejected |
| Return every API endpoint as a capability | Very precise | Leaks transport structure into product UX and creates an unbounded policy surface | Rejected |
| Hide workspaces the browser believes are denied | Less visual clutter | Turns an affordance into an authorization decision and hides the reason an operator needs | Rejected |
| **One bounded primary capability per workspace, with handler-level re-authorization** | Complete UX coverage, stable product vocabulary, bounded low-latency response, and preserved server authority | Primary status is not a complete permission matrix; technical details must explain the distinction | **Selected** |

## Invariants and failure behavior

- The API owns the catalog and emits a decision for every known workspace.
- IDs, required scopes, roles, and attribute keys are compile-time bounded.
- The response is still capped by the existing snapshot lifetime and UUIDv7
  identity contract.
- Missing scope or role is `deny`; unavailable resource context is
  `indeterminate`; unknown reason codes fail closed in the browser.
- A stale, missing, malformed, or failed snapshot never enables a protected
  operation.
- Navigation status is explanatory only. Every handler repeats tenant,
  resource, role, scope, state, evidence, and chain checks.
- The benchmark validates the complete sorted catalog and exact decision
  semantics across two API replicas.

## Verification

- Rust API tests assert the complete capability set and representative missing
  scope, missing role, and missing context decisions.
- React tests assert every navigation item has a matching capability and that
  denied/indeterminate status is visible without disabling unexplained access.
- The browser parser rejects partial or unknown catalogs before rendering a
  server-allowed snapshot.
- The Docker capability benchmark validates all catalog IDs and decisions at
  bounded concurrent load.
- Existing browser accessibility and tenant-session replacement journeys
  remain mandatory.

## Remaining gates

This closes the local product-contract gap only. Staging identity-provider,
real multi-tenant, screen-reader, regional latency, and production
authorization evidence remain release gates.
