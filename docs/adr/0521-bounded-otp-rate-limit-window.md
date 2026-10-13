# ADR-0521: Keep OTP abuse windows configurable with a production floor

* Status: Accepted
* Date: 2026-10-13
* Owners: Platform and Security
* Related: ADR-0503, ADR-0518, ADR-0519

## Context

Email OTP requests need two independent controls: a short resend cooldown and
a longer rolling request window. The cooldown protects the mailbox and provider
from rapid duplicate sends; the rolling window limits brute-force, mailbox
enumeration, and delivery-cost abuse. The hackathon deployment is staging and
needs faster recovery during operator testing, but production must retain a
conservative default.

## Decision

Add `AUTH_OTP_RATE_LIMIT_WINDOW_SECONDS` to the server-owned OTP policy. The
default is 900 seconds (15 minutes). Non-local environments reject values below
300 seconds; local development may use values down to 60 seconds. The deployed
staging/hackathon environment uses 300 seconds (5 minutes). The 60-second
resend cooldown, three requests per window, source-IP limit, five verification
attempts, and five-minute challenge TTL remain unchanged.

The window is applied consistently to email, source, and latest-request queries
using a bound PostgreSQL interval parameter. It is never accepted from the
browser.

## Alternatives

- Keep a hard-coded 15-minute window: strongest fixed default, but slows
  controlled staging recovery and makes the operational policy opaque.
- Reduce the global hard-coded window to five minutes: rejected because it
  weakens production without an environment-specific guard.
- Remove request limits and rely on provider throttling: rejected because the
  application loses abuse controls and provider cost becomes the authority.
- **Bounded environment configuration with a production floor**: selected
  because staging can be testable while production remains fail-closed at a
  conservative minimum.

## Operational consequence

Changing the staging window does not invalidate existing challenges or alter
verification semantics. After the rolling window clears, the user must request
one new code from the deployed UI and use only that code. Rate-limit metrics and
the generic browser message remain intentionally non-enumerating.
