# Transactional email provider runbook

This runbook configures the Rust `auth-service` to deliver OTP and tenant
invitation messages through the server-owned Resend HTTPS adapter or the
explicit Google Workspace SMTP adapter. It does not
authorize self-signup, tenant membership, tax compliance, or payout access.
The static SPA must select this path explicitly at image-build time with
`VITE_AUTH_METHOD=email-otp` and an HTTPS `VITE_AUTH_SERVICE_BASE_URL`; without
that release setting the browser fails closed or uses the separately selected
OIDC method.

## Required deployment inputs

Inject these values from the staging/production secret and configuration
systems. Do not commit them, put them in a Dockerfile, or expose them to the
SPA:

```text
AUTH_ENV=staging|production
AUTH_EMAIL_MODE=provider
AUTH_LOCAL_BOOTSTRAP_ADMIN_EMAIL=
AUTH_EMAIL_PROVIDER_ENDPOINT=https://api.resend.com/emails
AUTH_EMAIL_PROVIDER_API_KEY=<secret-manager reference>
AUTH_EMAIL_FROM=Solana Tax Protocol <auth@example.com>
AUTH_APP_BASE_URL=https://app.example.com
AUTH_EMAIL_PROVIDER_TIMEOUT_MILLIS=3000
AUTH_INITIAL_ADMIN_EMAIL=first-admin@example.com
VITE_AUTH_METHOD=email-otp
VITE_AUTH_SERVICE_BASE_URL=https://auth.example.com
```

For Google Workspace SMTP, select `AUTH_EMAIL_MODE=google-smtp` instead of
`provider` and add the mode-specific settings below:

```text
AUTH_EMAIL_SMTP_HOST=smtp.gmail.com             # app-password; use relay host for ip-allowlist
AUTH_EMAIL_SMTP_PORT=587
AUTH_EMAIL_SMTP_SECURITY=starttls
AUTH_EMAIL_SMTP_AUTH=app-password|ip-allowlist
AUTH_EMAIL_SMTP_TIMEOUT_MILLIS=10000
AUTH_EMAIL_SMTP_USERNAME=noreply@klyran.com        # app-password only
AUTH_EMAIL_SMTP_PASSWORD=<secret-manager reference> # app-password only
AUTH_OTP_RATE_LIMIT_WINDOW_SECONDS=900              # production default; staging may use 300
```

The sender domain must be verified with the provider. Complete SPF, DKIM, and
DMARC configuration before enabling user traffic. `AUTH_APP_BASE_URL` must be
the exact HTTPS origin served by the release; invitation links append
`/auth/signup`.
`AUTH_INITIAL_ADMIN_EMAIL` is an optional one-time bootstrap allow-list entry,
not a password or bearer credential. The matching mailbox must complete the
normal email-OTP signup while no active administrator exists. Remove the
setting after successful enrollment; the service never makes administrator
assignable through the normal invitation or role endpoints.

## Google Workspace mailbox sender

`noreply@klyran.com` may be a Google Workspace user or alias. For Resend, the
selected delivery path is the existing server-owned HTTPS adapter. For direct
Google SMTP, the app-password username must be the full licensed Workspace
account that owns the app password; a sender alias may be used only when that
account is permitted to send as the alias.

1. In Google Workspace, create `noreply@klyran.com` or an approved send-as
   alias and confirm that `klyran.com` is the organization-owned domain.
2. In Resend, add `klyran.com` as a sending domain and copy the exact SPF,
   DKIM, and DMARC records shown by Resend into the authoritative DNS zone.
   Do not replace Google Workspace MX records. If Google Workspace already has
   an SPF TXT record, merge the Resend sender into that single SPF record
   rather than publishing a second SPF record.
3. Wait for the provider to report the domain as verified. The sender value
   used by the service is:

   ```text
   AUTH_EMAIL_FROM=Solana Tax Protocol <noreply@klyran.com>
   ```

4. Create a narrowly scoped Resend API key and inject it only at service
   runtime as `AUTH_EMAIL_PROVIDER_API_KEY`. Never put it in a Dockerfile,
   Compose build argument, frontend variable, source file, log, or chat.
5. Run the staging delivery check from an approved test mailbox before
   enabling general traffic.

The local Docker default remains `AUTH_EMAIL_MODE=local-file` so tests do not
send real mail. A real Resend delivery requires a staging/production-style
deployment with the provider key, verified sender, HTTPS application origin,
and the other non-local auth secrets. Do not switch the local fixture to a
real provider merely by adding a mailbox password.

To select the Google adapter, set `AUTH_EMAIL_MODE=google-smtp`. For
`app-password`, use `smtp.gmail.com` on port 587 with STARTTLS, the full
licensed Workspace address, and an app password. For `ip-allowlist`, use
`smtp-relay.gmail.com` with no credentials and a stable egress IP registered
in the Workspace SMTP relay policy. The service rejects mixed or incomplete
credential modes before binding its listener. Google recommends SMTP relay
for applications; use TLS and an administrator-controlled relay policy.

For local Docker only, an approved developer may run the real adapter with:

```text
AUTH_ENV=local
AUTH_EMAIL_MODE=google-smtp
AUTH_EMAIL_FROM=Solana Tax Protocol <noreply@klyran.com>
AUTH_EMAIL_SMTP_HOST=smtp.gmail.com
AUTH_EMAIL_SMTP_PORT=587
AUTH_EMAIL_SMTP_SECURITY=starttls
AUTH_EMAIL_SMTP_AUTH=app-password
AUTH_EMAIL_SMTP_USERNAME=noreply@klyran.com
AUTH_EMAIL_SMTP_PASSWORD=<untracked runtime secret>
```

The normal local default remains `local-file`; do not commit these values or
put the app password in a Dockerfile, build argument, browser variable, log,
trace, or chat. For staging/production, use a secret manager and a stable
egress policy rather than a developer `.env` file.

## Pre-deployment checks

1. Confirm the selected Resend account or Google Workspace organization and
   sender domain are owned by the organization. For Resend, scope the API key
   only to the required email-send capability; for Google, confirm the
   Workspace relay policy or app-password account is approved.
2. Confirm the secret is injected at runtime and is absent from image layers,
   build arguments, browser bundles, logs, traces, and support exports.
3. Start the auth service with the selected non-local mode (`provider` or
   `google-smtp`). It must fail before binding a listener when the endpoint,
   credential, sender, app origin, TLS, or authentication policy is invalid.
4. Verify `/readyz` and the redacted delivery-failure metric/log path. Never
   use a real customer address for a test without an approved privacy and
   support procedure.
5. Send one staging OTP and one invitation to an approved test mailbox. Verify
   the code, expiry wording, invitation link, provider/SMTP acceptance handling,
   and that no email address or code appears in service logs.
6. Load the exact built SPA and verify that the sign-in page displays the
   email-OTP entry point. A bundle with missing, conflicting, or non-HTTPS
   browser auth settings must render the unconfigured state instead.
7. If an initial administrator is configured, complete one approved mailbox
   signup and verify the issued token carries administrator scope/role claims,
   the durable role/audit record exists, a second administrator bootstrap is
   denied, and the deployment setting is removed before general traffic.

## Failure handling

- A provider timeout, non-success response, malformed response, or oversized
  response returns a generic authentication-unavailable response. The service
  does not reveal provider diagnostics to the browser.
- The challenge remains unconsumed when delivery fails; do not manually mark
  it verified or copy an OTP into a support channel.
- Provider retries must reuse the challenge/invitation idempotency key. Do not
  create a second logical challenge to work around an ambiguous provider
  response.
- If delivery is degraded, disable new protected sign-in traffic through the
  deployment control plane and preserve the local database/audit evidence for
  investigation. Do not enable the local-file inbox outside local Docker.

## Rotation and privacy

Rotate provider credentials through the secret manager with an overlap window
approved by Security. Re-run the staging delivery test after rotation. Retain
provider message IDs and redacted outcome evidence only for the approved
privacy period; do not retain OTP plaintext, raw email bodies, or unnecessary
recipient data in application logs.

OTP request policy uses three requests per rolling window, a 60-second resend
cooldown, and five verification attempts per challenge. The production default
window is 900 seconds. The hackathon staging deployment may set
`AUTH_OTP_RATE_LIMIT_WINDOW_SECONDS=300`; values below 300 seconds are rejected
outside local Docker.

Production readiness additionally requires bounce/complaint handling, delivery
rate alerts, provider status monitoring, escalation ownership, and a tested
rollback to the previous valid provider key.
