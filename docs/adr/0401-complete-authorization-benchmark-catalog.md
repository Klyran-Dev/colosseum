# ADR-0401: Complete authorization benchmark catalog

- Status: Accepted
- Date: 2026-10-05
- Owners: Platform, Security, Performance

## Context

The server-owned capability catalog is the PBAC/ABAC presentation contract for
every production workspace. ADR-0275 added a bounded two-replica HTTP
benchmark, but its semantic key list had not been extended when the revenue
streaming workspace was added. The service correctly returned four
`revenue_stream.*` decisions, while the benchmark rejected the response before
measuring it. A benchmark that omits a catalog branch can report green while
never validating that branch.

## Decision

Keep the authorization service as the sole policy authority and extend the
benchmark to validate the complete nineteen-capability catalog, including the
four revenue-stream decisions. The synthetic principal intentionally lacks the
revenue-stream scopes, so those decisions must be explicit `deny` results with
the canonical `scope_missing` reason. Resource-dependent policies must remain
explicit `indeterminate` results with `resource_context_required`.

Add `make authorization-benchmark-contract` to statically verify that the
benchmark contains every catalog identifier and validates indeterminate ABAC
decisions. The contract runs as part of `make check`. This prevents a future
workspace addition from silently shrinking benchmark coverage; it does not
make the browser or benchmark an authorization authority.

## Alternatives considered

| Alternative | Coverage safety | Change detection | Operational cost | Decision |
| --- | --- | --- | --- | --- |
| Leave the benchmark's hand-maintained subset | Fail; omitted workspaces can regress unseen | Fail | Low | Rejected |
| Discover keys dynamically from each response | Partial; a server can omit a key and still self-validate | Fail; catalog shrinkage is hidden | Low | Rejected |
| Load the production catalog directly at benchmark runtime | Pass for key coverage | Partial; runtime fixture coupling can hide source drift | Medium | Rejected |
| **Exact static catalog assertions plus an executable source contract** | **Pass; every ID and decision branch is named** | **Pass; missing IDs fail before Docker load** | **Low and bounded** | **Selected** |

## Invariants

- All nineteen catalog identifiers are present in the benchmark's exact key-set
  assertion.
- Every expected `allow`, `deny`, and `indeterminate` decision is asserted.
- `indeterminate` remains a valid, fail-closed ABAC outcome and is never
  treated as permission to perform a protected action.
- The benchmark uses only synthetic identity and tenant data.
- The benchmark's local p95 budget remains a diagnostic gate; a budget miss
  requires an explicit reviewed override and is not silently relaxed.

## Verification

- `make authorization-benchmark-contract` fails when any catalog identifier is
  removed from the benchmark and passes after the complete set is restored.
- The one-request Docker benchmark passes semantic validation across the
  complete catalog.
- The full concurrent benchmark remains subject to the documented 25 ms p95
  budget and separate Linux/staging/production evidence gates.
