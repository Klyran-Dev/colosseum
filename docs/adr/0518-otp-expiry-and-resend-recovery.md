# ADR-0518: Make OTP expiry recoverable in the admin sign-in flow

- Status: Accepted
- Date: 2026-10-11
- Owners: Web platform and security engineering

## Context

The admin sign-in flow correctly rejected invalid, expired, or mismatched OTP
codes, but the rewritten UI did not expose the server challenge expiry or a
bounded recovery action. Operators could not distinguish an old inbox message
from the active challenge and had to reload the page to request another code.

## Decision

Consume the authentication service's `expires_at_unix` and
`retry_after_seconds` response fields. Show an explicit recovery status for an
active challenge, disable resend during the server-provided cooldown, allow a
new challenge after the cooldown, clear the prior code, and provide a
different-email reset. The server remains authoritative for challenge
validity, attempts, expiry, and rate limits; the browser only presents the
bounded state and requests a new challenge.

## Alternatives considered

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Force a full page reload | Small implementation | Loses context and does not explain which challenge is active | Rejected |
| Accept the previous code after expiry | Fewer support failures | Breaks replay and expiry guarantees | Rejected |
| Let the browser generate or extend expiry | Immediate UX control | Moves authentication authority into the browser | Rejected |
| **Server-bounded resend with visible expiry and reset** | Recoverable UX while preserving server authority and rate limits | Requires small client state machine and contract coverage | **Selected** |

## Acceptance criteria

- A fresh challenge displays an explicit active/expired state.
- Resend uses the server endpoint and respects `retry_after_seconds`.
- A resend clears the previous code and binds the UI to the new challenge ID.
- The user can reset the challenge to use another email.
- No client-side action can extend, validate, or bypass server OTP expiry.
