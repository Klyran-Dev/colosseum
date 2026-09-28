# ADR-0095: Bounded browser API deadlines and cancellation

- Status: Accepted
- Date: 2026-09-27
- Owners: Web Platform / Product Security
- Scope: React SPA calls to the authenticated API and health endpoint

## Context

The browser calls several independently loaded control-plane and settlement
endpoints. An unbounded request can leave a production screen in an apparent
working state, retain stale authorization context, and create duplicate user
actions during a provider or network incident. Ad-hoc `AbortSignal.timeout`
usage also does not provide one testable policy or consistent caller-driven
cancellation for route changes and component unmounts.

## Decision

All browser-to-API calls use the shared `fetchWithTimeout` boundary in
`apps/web/src/api.ts`. It:

- applies a positive, explicit deadline before invoking `fetch`;
- preserves caller cancellation and passes the caller's abort reason through;
- cancels its timer and listener after completion;
- distinguishes a deadline as `TimeoutError` so the product can render a
  recoverable failure state;
- never substitutes cached, fabricated, or client-calculated authority after
  a timeout.

The current budgets are two seconds for health, five seconds for ordinary
quote/control-plane/compile/status/reconciliation calls, and fifteen seconds
for signed settlement submission. The submission budget covers bounded API
coordination only; it does not turn a provider timeout into confirmed chain
finality. Every operation remains responsible for its own stale-data and
reconciliation UX.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| No client deadline | Rejected | Allows indefinite pending UI and repeated user actions. |
| Ad-hoc `AbortSignal.timeout` at each call site | Rejected | Duplicates policy, complicates testing, and does not unify caller cancellation. |
| Retry automatically in the browser | Rejected | Can duplicate side effects and hides whether a settlement provider accepted bytes. |
| **One bounded fetch boundary with explicit per-operation budgets** | **Selected** | Centralizes deadline/cancellation semantics while preserving endpoint-specific risk budgets and fail-closed UX. |

## Invariants and evidence

1. No production browser request runs without a finite deadline.
2. Component unmount and route changes can cancel in-flight reads without
   updating stale state.
3. A timeout is an error state, never a successful quote, settlement, or
   reconciliation result.
4. Settlement submission uses a longer bounded request budget than reads but
   still reports provider ambiguity for server-owned reconciliation.
5. Unit tests cover timeout abort, caller cancellation, and invalid budgets;
   TypeScript compilation and the full web test suite are required.

