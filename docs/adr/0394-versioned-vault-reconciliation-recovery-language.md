# ADR-0394: Versioned vault-reconciliation recovery language

- Status: Accepted
- Date: 2026-10-05
- Owners: Web Platform and Treasury Controls

## Context

Vault Reconciliation is a release-stop control surface. It compares
server-owned evidence-derived expectations with finalized chain observations
and must stop operators on stale, mismatched, or unproven observations. It is
read-only: the browser cannot alter balances, clear an alert, or release
funds.

The workspace already had localized reconciliation states and fail-closed
parser handling, but lookup and history failures interpolated HTTP status
values directly in the component. That left the recovery boundary inconsistent
with other production financial workspaces and did not provide a typed,
reviewed status policy.

## Decision

Keep Vault Reconciliation recovery language in the versioned English, German,
Arabic, and Japanese catalog. The panel now:

1. Routes target and history HTTP failures through a catalog formatter.
2. Displays only integer HTTP statuses in the inclusive `100..599` range;
   invalid or out-of-range values display `unknown status`.
3. Keeps parser, provider, and transport diagnostics behind the existing
   `OperatorFacingError` redaction boundary.
4. Preserves evidence-derived expected balances, finalized observation
   status, stale/mismatch release-stop warnings, tenant authorization, and
   read-only behavior.
5. Leaves error codes, slots, balances, and evidence identifiers as canonical
   evidence values rather than translating or using them as authority.

## Alternatives considered

### Keep component status interpolation

Rejected. It bypasses the locale catalog and makes recovery behavior harder to
review consistently across high-risk workspaces.

### Display response or provider diagnostics

Rejected. Response bodies and exceptions are not presentation authority and
may disclose implementation details. The reviewed fallback remains the
default.

### Hide all transport status context

Rejected. A bounded status is useful support context and does not authorize a
retry, release, or state transition when kept in presentation-only copy.

### Localize canonical evidence values

Rejected. Slots, balances, error codes, and identifiers must remain exact and
replayable. Only surrounding labels and recovery copy are localized.

## Invariants

- A stale, mismatched, or error observation never renders as matched.
- An empty source list never renders as proof of reconciliation.
- The browser cannot infer, mutate, release, or authorize a vault balance.
- HTTP status text cannot influence release authority, retry policy, or
  evidence digests.
- The API, finalized chain evidence, and tenant-scoped read model remain
  authoritative.

## Verification

- Failing-first catalog tests cover every supported locale and invalid status
  bounds.
- Vault Reconciliation UI tests, typecheck, production build, browser
  journeys, accessibility/color contracts, and repository gates are required.

## Consequences

Operators receive consistent localized recovery guidance while release-stop
semantics remain explicit. Future reconciliation failure states must add
reviewed catalog entries and tests before component copy is introduced. This
ADR changes presentation and error disclosure only; evidence and release
authority remain unchanged.
