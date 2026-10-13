# ADR-0529: Make OTP verification outcomes terminal and actionable

- Status: Accepted
- Date: 2026-10-13
- Owners: Authentication and platform architecture

## Context

Email OTP requests intentionally do not look up the account first. A uniform
request response prevents account enumeration and keeps the delivery boundary
independent of identity state. After the code is proven, however, the service
must resolve the requested authentication intent and the account-provisioning
policy before issuing a session.

The service already returns typed account-flow errors for the configured initial
administrator and for signup against an existing account. The browser was
discarding those typed outcomes and replacing them with a generic verification
message. In addition, a valid code could remain in `sent` state when account
resolution rejected the flow, weakening the single-use challenge invariant.

## Decision

Keep `/v1/auth/email/request` enumeration-resistant and make the verified OTP
boundary authoritative:

1. The service verifies and consumes a challenge exactly once before applying
   account-flow policy. It records the verified proof and terminal outcome in
   the existing append-only auth audit stream.
2. Only an existing active account, an approved invitation, an explicitly
   enabled self-signup, or the configured initial-administrator bootstrap can
   receive a token.
3. Invalid, expired, exhausted, replayed, disabled, and ordinary unknown
   sign-in attempts retain generic authentication failure copy.
4. Known recoverable outcomes use stable auth error codes and server-approved
   next actions. The SPA maps those codes to reviewed copy and recovery CTAs;
   it never renders arbitrary server error text.
5. The initial-administrator and existing-account branches remain explicit
   provisioning/sign-in transitions. No role, tenant, or account is created by
   a sign-in intent.

## Alternatives considered

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Check account existence during OTP request | Immediate feedback | Enables enumeration and creates different timing/delivery behavior | Rejected |
| Return raw API messages to the SPA | Small implementation | Provider/diagnostic leakage and inconsistent copy | Rejected |
| Keep typed outcomes but let a valid challenge remain reusable | Allows retry after changing intent | Violates single-use proof semantics and complicates audit/replay reasoning | Rejected |
| **Consume valid proofs, emit stable codes, and render bounded recovery actions** | Preserves privacy, enforces terminal OTP semantics, and gives users a recoverable path | Requires a small auth-specific client contract and tests | **Selected** |

## Contract

The request endpoint remains a generic `202 Accepted` challenge response. The
verification endpoint returns a token only for a successful session outcome.
Recoverable business outcomes use the existing bounded JSON error shape with
an optional `next_action` field:

```json
{
  "error": "ACCOUNT_SETUP_REQUIRED",
  "message": "Your email was verified. Finish account setup to continue.",
  "next_action": "signup",
  "retryable": false
}
```

The browser recognizes only the allow-listed codes and actions. A valid proof
with no permitted account outcome never produces a bearer token.

## Consequences

- Unknown-email requests remain privacy-preserving.
- Bootstrap and signup mistakes receive clear, actionable recovery.
- A verified OTP cannot be replayed after an account-policy rejection.
- Tests must cover request indistinguishability, terminal challenge state,
  concurrent verification, typed recovery, and no-token failure paths.
