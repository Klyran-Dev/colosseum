# ADR-0496 — Production transactional email adapter

## Status

Accepted for implementation; provider account, DNS, retention, and delivery
operations remain deployment gates.

## Context

The authentication service currently has a real `EmailSender` port, but its
only implementation writes OTP and invitation messages to a Docker-only local
inbox. That is appropriate for deterministic local browser evidence and is
explicitly prohibited outside `AUTH_ENV=local`. A staging or production
deployment therefore needs a server-owned transactional email adapter that
does not expose provider credentials to the browser, API, or logs.

## Decision

Implement a bounded HTTPS adapter for the Resend email API. The adapter is
selected only when `AUTH_EMAIL_MODE=provider` and requires:

- `AUTH_EMAIL_PROVIDER_API_KEY` from the deployment secret manager;
- `AUTH_EMAIL_PROVIDER_ENDPOINT`, defaulting to
  `https://api.resend.com/emails`;
- `AUTH_EMAIL_FROM`, a verified sender identity; and
- `AUTH_APP_BASE_URL`, the exact application origin used in invitation links.

The adapter sends both OTP and invitation messages with explicit text and HTML
parts, an idempotency key derived from the immutable challenge or invitation
identifier, an explicit timeout, redirect rejection, HTTPS enforcement outside
local development, and a bounded response body. Provider status, response
content, API keys, email addresses, OTP codes, and invitation details are not
logged. Any non-success or malformed provider response fails the auth request
closed and leaves the challenge available for a bounded retry policy.

The Docker local inbox remains the only local default and is unchanged. A
loopback HTTP provider endpoint is allowed only by unit tests/local fixtures;
staging and production require HTTPS. Provider delivery is synchronous for the
first slice so the service does not report a challenge as sent before the
provider accepts it. A later outbox change may decouple delivery only after it
preserves the challenge state machine and idempotency evidence.

## Alternatives considered

| Option | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| SMTP from the auth service | Broad provider compatibility | TLS/auth configuration, connection lifecycle, retry semantics, and delivery evidence become service-owned | Rejected for the first adapter |
| **HTTPS Resend adapter** | Small bounded JSON contract, explicit API idempotency, straightforward secret isolation, easy local fixture testing | Provider dependency and account/DNS governance remain deployment work | **Selected** |
| Provider-specific SDK | Strong typed convenience | Adds another runtime abstraction and can hide request/response bounds | Rejected; use the documented HTTP API |
| Browser/provider-direct delivery | No server transport code | Exposes credentials and makes identity authority client-controlled | Rejected |

## Invariants and failure behavior

- Non-local startup fails closed unless provider mode, endpoint, API key,
  verified sender, and application origin are valid.
- Local-file mode cannot be selected in staging or production.
- Provider credentials never appear in response bodies, logs, traces, metrics,
  browser bundles, or Docker build arguments.
- Requests are bounded by body size, timeout, redirect policy, and retry budget.
- Challenge and invitation identifiers are the only idempotency keys; retries
  cannot silently create a second logical delivery.
- Provider failure returns a generic authentication-unavailable response and
  records only redacted delivery outcome evidence.

## Acceptance evidence

- Unit tests cover configuration validation, HTTPS/loopback endpoint policy,
  payload escaping, idempotency headers, success responses, non-success
  responses, malformed responses, oversized responses, and timeout behavior.
- A local HTTP fixture proves the real adapter request without sending mail.
- A staging deployment test proves secret injection, verified sender use,
  provider acceptance, redacted logs, and retry behavior.
- Production readiness still requires provider account ownership, SPF/DKIM/DMARC,
  bounce/complaint handling, privacy retention approval, rate-limit capacity,
  alerting, and an on-call runbook.

