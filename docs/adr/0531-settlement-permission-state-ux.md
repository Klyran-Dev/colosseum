# ADR-0531 — Make settlement permission and route-loading states truthful

## Status

Accepted

## Context

Settlement preparation is gated by the server capability `settlement.wallet`.
The browser query for finalized route evidence is disabled when that capability
is not allowed. TanStack Query can still represent a disabled, data-less query
as pending, which caused the UI to show “Loading finalized route evidence…”
beside “Fresh permission required.” The combination was technically safe but
misleading and exposed implementation terminology to operators.

## Decision

The UI will derive route-loading state as `capability === allow` and
`query.isPending`. It will not show loading for a disabled query. Permission
states will use reviewed operator-facing labels: settlement access not
assigned, workspace setup required, or settlement access being verified.
The server capability decision remains authoritative; this change does not
grant a scope, bypass authorization, or fabricate route evidence.

## Consequences

- Operators see one truthful next step instead of conflicting status messages.
- Missing settlement responsibility remains fail-closed and actionable.
- Finalized route evidence is still fetched only after server authorization and
  remains unavailable when no verified active route exists.
- The behavior is covered by a browser component regression test.
