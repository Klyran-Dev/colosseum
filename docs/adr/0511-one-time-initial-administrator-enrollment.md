# ADR-0511: One-time initial administrator enrollment

## Status

Accepted.

## Context

Non-local auth deployments correctly disable open self-signup and reject the
local bootstrap-admin setting. The normal invitation and role-management
endpoints require an existing administrator, which creates a first-user
deadlock: no administrator can exist to invite the first operator or moderator.

The initial authority must be provisioned without making `administrator` a
normal assignable role, accepting caller-supplied tenant IDs, or exposing a
long-lived bearer credential in the browser.

## Decision

Add the optional server configuration `AUTH_INITIAL_ADMIN_EMAIL`.

When the normalized configured email completes the ordinary email-OTP signup
flow, the auth service may provision a new tenant and administrator role only
if no active administrator exists anywhere in the auth database. The existing
transaction atomically creates the user, role, audit record, and token claims.

The setting is an email allow-list input, not a secret. Proof of mailbox
control still requires the configured transactional email provider and a
single-use OTP. Operators must remove the setting after successful enrollment.
The normal invitation endpoint and role catalog continue to reject
`administrator` as assignable. A configured email cannot elevate an existing
account, bypass OTP, choose a tenant, or create a second administrator.

## Alternatives considered

| Option | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Keep the first administrator as a manual SQL operation | No application code | Bypasses audit, token lifecycle, and repeatable deployment controls | Rejected |
| Make administrator a normal invitation role | Simple UI | Any administrator could delegate global authority; violates separation of duties | Rejected |
| Add a browser-visible bootstrap bearer token | Explicit proof of deployment intent | Secret exposure in URLs/client state and a new public credential boundary | Rejected |
| **One-time server-configured email enrollment** | Uses existing OTP, transaction, audit, tenant creation, and token path | Requires operator-approved mailbox and post-enrollment config removal | **Selected** |

## Invariants and failure behavior

- The configured email must equal the normalized OTP email exactly.
- The transaction checks for an active administrator while serializing the
  signup and cannot assign the role after an active administrator exists.
- Existing accounts cannot be elevated through this setting.
- Missing, invalid, or removed configuration fails closed to the normal
  invitation/self-signup rules.
- The setting is never returned to the browser, token, API response, log,
  trace, or email content.
- Bootstrap role assignment is recorded with a distinct audit reason.

## Acceptance evidence

- Auth-service unit tests cover exact email binding, no-admin precondition,
  one-time behavior, and the signup provisioning matrix.
- The provider runbook requires OTP, token-role, audit, second-attempt, and
  configuration-removal checks.
- Local Docker continues to use the separate local bootstrap email and the
  administrator/moderator browser journey; non-local deployments require
  explicit provider and provisioning evidence before enablement.

