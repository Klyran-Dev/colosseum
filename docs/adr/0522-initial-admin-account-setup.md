# ADR-0522: Make Initial Administrator Setup Explicit in Email OTP Auth

- Status: Accepted
- Date: 2026-10-13
- Owners: Authentication and platform architecture

## Context

The staging deployment disables open self-signup while allowing one configured
initial administrator email to bootstrap the first account. A valid OTP on the
sign-in journey for that email previously returned the generic
`The code is invalid or expired.` message because no user record existed yet.
That made a valid code indistinguishable from an expired or mistyped code.

## Decision

Keep account provisioning server-owned and intent-bound, but return explicit
recoverable errors after the OTP digest is verified:

- configured initial-admin email without a provisioned user: direct the user
  to complete account setup from the signup journey;
- signup intent for an email with an existing user: direct the user to sign in;
- all other unknown-account sign-in attempts retain the generic authentication
  failure response to avoid unnecessary account enumeration.

The browser already exposes the corresponding navigation links. No account,
role, or authorization state is created by a sign-in request.

## Alternatives considered

1. Keep the generic error for every case. Rejected because it hides a required
   first-run setup action and causes repeated OTP requests.
2. Allow sign-in to provision the configured initial administrator. Rejected
   because provisioning must remain explicit and bound to the signup intent.
3. Enable open self-signup in staging. Rejected because deployment policy
   intentionally keeps provisioning closed except for the configured bootstrap
   path.

## Consequences

- A valid bootstrap OTP now produces an actionable message instead of a false
  expiry diagnosis.
- The service remains the authority for provisioning and role assignment.
- The new branch is covered by auth-service unit tests and does not change OTP
  digest, expiry, attempt, or replay semantics.
