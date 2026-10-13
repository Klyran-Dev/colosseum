# ADR-0475: Keep invoice quote retry in the failure surface

## Status

Accepted

## Context

The invoice-tax workspace redacted transport/provider failures correctly, but
left the operator with only the primary form action as a recovery path. After
an asynchronous failure, the operator had to scroll back through the invoice
form and infer that the same request could be attempted again. That is poor
failure-state affordance for a production workflow, especially when the
request preserves an idempotency key for safe replay.

## Decision

Render a localized `Retry calculation` action inside the existing alert. The
retry reuses the current validated facts and idempotency key, remains disabled
while the request is active or the service/session is unavailable, and does
not expose provider diagnostics. Client-side validation errors continue to
require correction through the form rather than offering a misleading retry.

## Alternatives considered

- Require the operator to press the primary form action again: rejected because
  the recovery action is detached from the failure and may be below the fold.
- Automatically retry transport failures: rejected because it hides request
  boundaries and can create retry storms without an explicit operator action.
- Show raw provider details: rejected because diagnostics can disclose internal
  topology and are not actionable to the operator.

## Consequences

- The failure state has an immediate, accessible recovery action.
- The existing idempotency behavior continues to protect unchanged retries.
- No browser-side tax result or provider detail is substituted into the UI.
- Other asynchronous workspaces still need a separate inventory to ensure
  equivalent failure-path affordances.

## Verification

- Document quote UI test covers a failed request followed by an explicit retry.
- Frontend test suite, typecheck, `git diff --check`, and Docker browser gate.
