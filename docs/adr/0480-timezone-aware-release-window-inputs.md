# ADR-0480: Use timezone-aware date-time inputs for release windows

## Status

Accepted

## Context

The treasury release form exposed `execute_after_unix` and
`expires_at_unix` as raw numeric text fields. Operators had to convert human
times into Unix epoch seconds manually, which was error-prone and made the
release window difficult to review. The API and Solana contracts still require
exact integer seconds, so the browser needs a presentation-layer conversion
without changing the authority-bound contract.

## Decision

Use native `datetime-local` controls with seconds precision. Display and parse
their values in the selected application time zone, convert them to safe
integer Unix seconds at the form boundary, and reject invalid or inverted
windows before submission. The selected time zone is shown alongside the
help text so the operator can interpret the values correctly.

## Alternatives considered

- Keep raw epoch-second inputs: rejected because they are difficult to enter
  and review safely.
- Use browser-local date/time conversion: rejected because it ignores the
  application's selected time zone and can change the intended instant.
- Use a date-time picker that omits seconds: rejected because release windows
  are protocol values and truncating seconds would change the request.
- Use timezone-aware `datetime-local` plus exact conversion: selected because
  it improves operator usability while preserving integer protocol values.

## Consequences

- Operators work with recognizable calendar times instead of epoch numbers.
- DST gaps and invalid windows fail closed before a release request is sent.
- The request payload remains exact integer Unix seconds and is unchanged at
  the server boundary.

## Verification

- Formatter tests cover UTC and Asia/Kolkata round trips.
- Vault-release UI test covers native date-time controls.
- Frontend test suite, typecheck, `git diff --check`, and Docker browser gate.
