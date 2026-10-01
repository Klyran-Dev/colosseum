# ADR-0226: Tax Engine deployment preflight boundary

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Security, SRE
- Scope: Tax Engine service deployment configuration
- Bounded contexts: Tier-1 Tax Engine, Service Runtime, Platform Security

## Context

ADR-0225 adds a separately deployable Tax Engine service with an explicit
environment and service bearer credential. The shared non-local deployment
preflight did not yet validate those fields, so a staging or production
manifest could omit the service, mislabel it, or accidentally use the local
Compose token while the rest of the topology passed preflight.

## Decision

Extend `scripts/verify_deployment_config.sh` and its executable contract to
require, for staging and production:

- `TAX_ENGINE_ENV` exactly matching `DEPLOYMENT_ENV`; and
- a non-empty `TAX_ENGINE_AUTH_TOKEN` that is not the checked-in local
  development token.

The preflight validates names and known unsafe values only. It never prints or
cryptographically validates the secret. The Tax Engine binary remains the
runtime authority for syntax, bounds, local-token rejection, and listener
startup. TLS/mTLS
termination, service identity, secret injection, rotation, network policy, and
deployment approval remain independent production gates; this ADR does not
pretend an environment variable is transport security.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Leave Tax Engine configuration to operator convention | Rejected | A missing service label or local token can pass the shared preflight and reach a non-local rollout. |
| Create a Tax Engine-only deployment checker | Rejected | Duplicates cross-service environment policy and can drift from the rollout gate. |
| Validate only inside the binary | Rejected as the sole control | Direct starts are covered, but rendered manifests can still omit the service before runtime. |
| **Extend the shared preflight plus retain runtime validation** | **Selected** | Catches topology/configuration omission early while preserving independent process-owned fail-closed checks. |

## Invariants and failure behavior

- Non-local preflight fails when `TAX_ENGINE_ENV` is missing or mismatched.
- Non-local preflight fails when the token is missing, empty, or the local
  `local-development-only` value.
- Secret values never appear in failure output.
- A passing preflight does not prove token validity, mTLS, identity rotation,
  provider availability, SLOs, or production approval.
- Local Compose defaults remain unchanged and reproducible.

## Verification and release impact

- `make deployment-config-contract` proves a positive non-local Tax Engine
  configuration and missing/local-token failures.
- `make check` runs the contract.
- Staging/production service identity, TLS/mTLS, rotation, network policy,
  load/SLO evidence, and Linux validator evidence remain release gates.
