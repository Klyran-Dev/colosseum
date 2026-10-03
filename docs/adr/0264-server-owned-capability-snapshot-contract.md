# ADR-0264: Server-owned capability snapshot for the product shell

## Status

Accepted. This decision implements the production gate identified by
ADR-0262. The snapshot improves navigation and explanation; it never replaces
authorization in a protected API handler or settlement compiler.

## Context

The React shell has a typed catalog of workspaces and required permissions, but
the browser must not decode JWT claims or infer tenant authority. Without a
server response, an authenticated operator sees only an indeterminate state and
cannot understand which actions are available, which need another approver, or
which require a record-specific condition.

The capability response must be low-latency and bounded. It should require no
database or RPC call, derive the account from the verified principal, expose no
raw bearer token, and remain safe when a policy or attribute cannot be
evaluated. A stale or unavailable snapshot must never enable an action.

## Decision

Add `GET /v1/authorization/capabilities`.

The endpoint:

1. verifies the EdDSA JWT, issuer, audience, expiry, and key snapshot through
   the existing authenticator;
2. derives the tenant/account from the authenticated principal;
3. evaluates the bounded server policy catalog against exact scopes and roles;
4. returns one decision per known workspace policy with `allow`, `deny`, or
   `indeterminate` plus a stable reason code;
5. returns a UUIDv7 snapshot identifier and an expiry no later than the bearer
   token expiry and the bounded snapshot lifetime;
6. marks policies with missing resource attributes as `indeterminate` rather
   than guessing; and
7. performs no storage, broker, RPC, filesystem, or external identity-provider
   call on the request path.

The wire contract is versioned as `authorization-capabilities/v1`:

```json
{
  "schema": "authorization-capabilities/v1",
  "snapshot_id": "019...",
  "subject": "operator-123",
  "tenant_id": "019...",
  "expires_at_unix": 1800000060,
  "capabilities": {
    "tax.quote.create": {
      "decision": "allow",
      "reason_code": "scope_and_role_satisfied"
    },
    "tax.rule.govern": {
      "decision": "indeterminate",
      "reason_code": "resource_context_required"
    }
  }
}
```

The browser may use `allow` to improve navigation and explain the next step,
but the API remains authoritative. The browser must treat an expired,
malformed, missing, or failed snapshot as indeterminate and keep protected
actions unavailable until the real operation receives a fresh server decision.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Decode JWT claims in React | No extra request | Untrusted/stale claims and no tenant/resource policy evaluation | Rejected |
| Put capabilities in the login token | One response at login | Long-lived policy staleness, large tokens, difficult revocation and resource context | Rejected |
| Query PostgreSQL for every shell load | Rich resource context | Adds storage latency and an avoidable failure domain to navigation | Rejected |
| Add a policy BFF in front of the API | Central policy surface | Creates a second authority and an extra hop on protected operations | Rejected |
| **Bounded server snapshot from verified principal and policy catalog** | Low latency, no persistence dependency, explainable decisions, server authority retained | Resource-specific conditions remain indeterminate until the protected workflow supplies context | **Selected** |

## Invariants and failure behavior

- Tenant identity is always derived from the verified principal; no query or
  browser field can select it.
- Unknown policy IDs, unknown decisions, malformed responses, expired
  snapshots, and server errors fail closed in the UI.
- A capability response never contains a bearer token, private key, raw tax
  evidence, or a claim that settlement or legal compliance occurred.
- Protected handlers continue to call their existing exact scope, role,
  tenant, resource, and chain-evidence checks.
- Snapshot generation is bounded to the fixed policy catalog and has no
  unbounded user-controlled collection.

## Verification

- API contract tests prove missing authentication, insufficient scope, role
  denial, resource-context indeterminacy, UUIDv7 identity, and bounded expiry.
- React parser tests reject malformed and expired snapshots and preserve
  fail-closed behavior.
- Product-shell tests prove a server `allow` is represented as verified,
  while failed or indeterminate fetches never become enabled access.
- API and web tests remain green with the existing security, accessibility,
  and low-latency gates.
