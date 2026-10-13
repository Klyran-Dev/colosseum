# ADR-0519: Add an explicit Google Workspace SMTP email adapter

* Status: Accepted
* Date: 2026-10-13
* Owners: Platform and Security
* Supersedes: none
* Related: ADR-0496, ADR-0018, ADR-0021

## Context

The auth service already has a server-owned Resend HTTPS adapter and a
Docker-only local inbox. A deployment may instead need to send OTP and
invitation messages from a Google Workspace address such as
`noreply@klyran.com`. A mailbox address is not an SMTP credential, and adding a
mailbox password to the browser or image would create an avoidable credential
and abuse boundary.

## Decision

Add a separately selected `google-smtp` email mode to the Rust auth service.
The adapter uses Tokio-compatible Lettre with Rustls, creates a fresh bounded
message from the existing escaped text/HTML renderers, and sends through either:

1. Google Workspace SMTP relay at `smtp-relay.gmail.com` with
   `AUTH_EMAIL_SMTP_AUTH=ip-allowlist`, where Google Workspace administrator
   policy and a stable egress IP provide relay authorization; or
2. Google SMTP at `smtp.gmail.com` with `AUTH_EMAIL_SMTP_AUTH=app-password`,
   where the Workspace username and app password are injected only at runtime.

The default production choice remains the Resend HTTPS adapter because its
existing idempotency and provider-response contract is stronger for the
service. Google SMTP is an explicit deployment choice and is never inferred
from the sender address.

## Configuration contract

```text
AUTH_EMAIL_MODE=google-smtp
AUTH_EMAIL_FROM=Solana Tax Protocol <noreply@klyran.com>
AUTH_EMAIL_SMTP_HOST=smtp.gmail.com       # app-password; relay host for ip-allowlist
AUTH_EMAIL_SMTP_PORT=587
AUTH_EMAIL_SMTP_SECURITY=starttls
AUTH_EMAIL_SMTP_AUTH=app-password|ip-allowlist
AUTH_EMAIL_SMTP_USERNAME=noreply@klyran.com       # app-password mode only
AUTH_EMAIL_SMTP_PASSWORD=<runtime secret>         # app-password mode only
```

`starttls` is required for authenticated SMTP and for non-local relay mode;
implicit TLS is supported on port 465. Plain SMTP is not supported by this
adapter. Credentials are never included in logs, image layers,
build arguments, frontend variables, traces, or error messages.

## Invariants and failure behavior

- The mode, host, port, security, and authentication mode are parsed and
  bounded before the listener binds.
- App-password mode requires both username and password; relay mode rejects
  supplied credentials to prevent ambiguous authority.
- Non-local SMTP requires TLS. A provider connection error, rejected message,
  malformed response, timeout, or authentication failure returns the generic
  email-delivery failure and leaves the OTP challenge unconsumed.
- The existing challenge/invitation identifier remains the application-level
  idempotency key. SMTP itself is not treated as settlement or identity
  authority and does not make delivery equivalent to mailbox verification.
- Local Docker remains deterministic by default with `local-file`; a real
  Workspace credential must never be placed in the repository `.env` unless it
  is explicitly ignored and managed as a developer secret.

## Rejected alternatives

- Browser-direct Gmail SMTP: rejected because it exposes credentials and
  makes the browser an email authority.
- Gmail API/domain-wide delegation: deferred because it needs Workspace admin
  OAuth governance, service-account key custody, and a larger provider
  integration than this bounded SMTP adapter.
- Replacing the Resend adapter: rejected because it would remove the current
  provider evidence and force one mailbox vendor on every deployment.

## Operational requirements

The Workspace administrator must configure either SMTP relay source-IP
allowlisting or an account with 2-Step Verification and an app password,
depending on the selected mode. DNS SPF/DKIM/DMARC, rate limits, bounce and
complaint handling, secret rotation, and an approved test mailbox remain
deployment-owned gates. The adapter does not claim legal compliance or
deliverability solely because Google accepted an SMTP message.
