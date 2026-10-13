# ADR-0478: Render treasury sign-in readiness as status

## Status

Accepted

## Context

The treasury release form rendered the current sign-in readiness as a
read-only `<textarea>`. Although it could not be edited, it looked like an
operator input and occupied the same visual treatment as evidence and release
fields. That blurred the distinction between authorization state and data the
operator must provide, especially in a sensitive money-movement workflow.

## Decision

Render sign-in readiness as a localized status surface with `role="status"`.
It reports either that the account is ready or that sign-in is required,
while retaining the explanation that credentials remain in memory and that
the account must be authorized. No token or credential value is rendered.

## Alternatives considered

- Keep the read-only textarea: rejected because it presents non-input state as
  an editable-looking control.
- Hide the status and rely only on the shell sign-in gate: rejected because
  the treasury form still needs an in-context explanation of why its controls
  are unavailable.
- Use a status card: selected because it preserves context, has correct
  semantics, and removes false affordance without changing authorization.

## Consequences

- Operators can distinguish required input from authorization state at a
  glance.
- The form becomes less cluttered and more consistent with the other
  workflow evidence surfaces.
- Authorization remains server-owned and fail-closed; this is a presentation
  change only.

## Verification

- Vault-release UI test asserts status semantics and absence of a sign-in
  textbox.
- Frontend test suite, typecheck, `git diff --check`, and Docker browser gate.
