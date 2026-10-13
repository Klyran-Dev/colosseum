# Transactional email provider runbook

This runbook configures the Rust `auth-service` to deliver OTP and tenant
invitation messages through the server-owned Resend HTTPS adapter. It does not
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

The sender domain must be verified with the provider. Complete SPF, DKIM, and
DMARC configuration before enabling user traffic. `AUTH_APP_BASE_URL` must be
the exact HTTPS origin served by the release; invitation links append
`/auth/signup`.
`AUTH_INITIAL_ADMIN_EMAIL` is an optional one-time bootstrap allow-list entry,
not a password or bearer credential. The matching mailbox must complete the
normal email-OTP signup while no active administrator exists. Remove the
setting after successful enrollment; the service never makes administrator
assignable through the normal invitation or role endpoints.

## Pre-deployment checks

1. Confirm the provider account and domain are owned by the organization and
   that the API key is scoped only to the required email-send capability.
2. Confirm the secret is injected at runtime and is absent from image layers,
   build arguments, browser bundles, logs, traces, and support exports.
3. Start the auth service with `AUTH_EMAIL_MODE=provider`. It must fail before
   binding a listener when the endpoint, key, sender, app origin, or HTTPS
   policy is invalid.
4. Verify `/readyz` and the redacted delivery-failure metric/log path. Never
   use a real customer address for a test without an approved privacy and
   support procedure.
5. Send one staging OTP and one invitation to an approved test mailbox. Verify
   the code, expiry wording, invitation link, provider message ID handling,
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

Production readiness additionally requires bounce/complaint handling, delivery
rate alerts, provider status monitoring, escalation ownership, and a tested
rollback to the previous valid provider key.
