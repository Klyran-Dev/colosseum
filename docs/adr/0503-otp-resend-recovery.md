# ADR-0503: Server-timed OTP resend recovery

## Status

Accepted — 2026-10-11

## Context

The dedicated sign-in and sign-up pages let an operator request and verify a
single email OTP challenge. A code can expire, be delayed by a provider, or be
lost without indicating that the account or tenant is invalid. The auth service
already applies a resend cooldown and returns the remaining cooldown with each
challenge response, but the SPA did not expose that recovery path.

## Decision

Expose a `Resend code` action on the verification step. It calls the same
server-owned request endpoint with the original intent and normalized email,
replaces the current challenge only after a successful response, and disables
the action until the server-provided `retry_after_seconds` window has elapsed.
The UI displays the bounded remaining time and keeps `Use a different email` as
the separate escape path.

The browser does not manufacture challenge IDs, bypass cooldowns, or infer
whether an account exists. The auth service remains authoritative for rate
limits, challenge expiry, delivery, and identity provisioning.

## Alternatives considered

- **No recovery action:** rejected because delayed or expired delivery leaves a
  valid user at a dead end.
- **Client-only timer with a fixed cooldown:** rejected because it can drift
  from deployment policy and would create a false affordance.
- **Reset to the email step and let the user submit again:** rejected because
  it hides the intended recovery action and makes the operator re-enter data.
- **Server-timed resend using the existing endpoint:** selected because it
  preserves one authority boundary, existing rate limits, and the current
  Docker/provider adapters.

## Invariants

- A failed resend never replaces the currently valid challenge.
- A resend never exposes an OTP, access token, or account-existence signal.
- The action is disabled while a request is in flight and during the server
  supplied cooldown.
- Resetting the challenge clears the cooldown and code from the SPA state.
- Sign-in and sign-up retain the same server-bound intent and role policy.

## Verification

- Component tests cover the resend action and cooldown presentation.
- The auth-service and API tests remain unchanged because the endpoint contract
  is reused.
- The Docker browser gate exercises the existing request/verify journey after
  the UI change; a targeted component test proves the new recovery behavior.
