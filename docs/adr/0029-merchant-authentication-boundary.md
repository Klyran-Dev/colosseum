# ADR-0029: Authenticate persisted quote requests with scoped EdDSA JWTs

- Status: Accepted for the first protected API slice
- Date: 2026-09-25
- Owners: Platform Security / Payments Infrastructure
- Scope: Rust API authorization boundary for persisted quotes

## Context

The persisted quote endpoint writes calculation evidence and creates merchant-scoped idempotency records. Accepting `merchant_id` from an unauthenticated request makes that field attacker-controlled and permits cross-merchant probing, replay-key collisions, and evidence attribution errors.

The quote path must remain low latency and must not depend on a remote identity introspection call. Browser clients need a standards-based bearer-token integration, while service deployments need a deterministic, testable verification boundary.

## Decision

Require an OIDC-compatible compact JWT signed with Ed25519 (`alg=EdDSA`) for persisted quote requests. The API validates, in order:

1. Authorization header syntax and bounded token size;
2. JWT header type, algorithm, and `kid`;
3. signature against the configured immutable-in-memory JWKS key set;
4. issuer, audience, subject, `merchant_id`, `iat`, `nbf` (when present), and `exp` claims;
5. bounded clock-skew policy and token lifetime;
6. equality between the authenticated merchant claim and the request selector.

The authenticated merchant is the only source for the storage command. The request no longer accepts a caller-selected merchant UUID. Missing authentication configuration, invalid tokens, expired tokens, unknown keys, and scope mismatch fail closed. The verifier performs no network call and allocates only bounded token-sized data.

The initial service contract loads a deployment-provided JWKS JSON document at startup. Key rotation is performed by publishing an overlapping key set and restarting/reloading the service through the deployment control plane. A later ADR may add a network JWKS refresher only if it preserves bounded stale-key behavior and has a measured failure policy.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`: remote introspection, mTLS-only identity, and caller-supplied merchant IDs were rejected for the synchronous persisted-quote authority boundary.

## Invariants

- No persisted quote is created without a cryptographically verified principal.
- A principal may write only its own merchant scope.
- The API never logs bearer tokens or raw authorization headers.
- Signature, issuer, audience, and time checks are explicit; library or language defaults do not define policy.
- Token size, claim size, key count, and lifetime are bounded.
- Authentication failure responses do not reveal key-set, tenant, or evidence details.

## Security and operations

- The JWKS document is configuration, not user input, and must be delivered through a secret/configuration management system with access auditing.
- Ed25519 public keys are verification-only; private keys remain in the identity provider/HSM and never enter this repository or API container.
- The configured issuer and audience are deployment-specific and must not use local development values in production.
- Revocation before token expiry is an identity-provider and deployment concern in this first slice; short token lifetimes and key overlap reduce exposure. Immediate revocation requires an additional bounded deny-list or gateway policy and a separate ADR.
- Metrics count authentication failures by reason class, never by token, subject, or merchant identifier.

## Consequences

Positive:

- Merchant identity is bound before persistence and idempotency.
- Verification is local, deterministic, and suitable for the latency-critical request path.
- Standard JWT/JWKS interoperability avoids a proprietary token contract.

Costs:

- Key rotation requires an explicit reload/redeploy workflow in this slice.
- JWT revocation is bounded by token lifetime unless a separate control is added.
- Identity-provider claim mapping is part of deployment configuration and must be contract-tested.

## Revisit triggers

Revisit when a production identity provider, gateway, or regulatory requirement needs live key rotation, immediate revocation, delegated operator roles, or service-to-service mTLS. Any change must preserve merchant-scope binding and quote-path latency budgets.
