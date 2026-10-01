# ADR-0230: Executable Tax Engine container runtime contract

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Tax Engine, SRE
- Scope: Tax Engine service delivery and local/CI verification
- Bounded contexts: Tier-1 Tax Engine, Service Runtime, Docker Platform

## Context

The Tax Engine service has domain and HTTP unit coverage, a pinned Dockerfile,
and Compose health checks. Those checks did not previously prove that the
locked image starts with its real entrypoint, initializes observability, binds
the configured port, enforces authentication and request limits, calculates a
real request, and exposes only the intended fixed-cardinality metrics. A
compile-only contract can therefore miss image/runtime drift or a startup
configuration mismatch.

## Decision

Add an explicit Docker runtime contract, `make tax-engine-runtime-contract`,
that builds the pinned `tax-engine` image in an isolated Compose project and
executes a bounded real-process probe. The contract must prove:

- image startup and `/healthz` liveness;
- authenticated `/readyz` and rejection of missing credentials;
- aggregate calculation through the actual HTTP listener with exact decimal
  result assertions;
- malformed/oversized request rejection before calculation;
- `/metrics` reachability on the private service endpoint, canonical
  `service="tax_engine"` identity, and absence of the service token; and
- deterministic cleanup of only the contract's container and project state.

The contract uses synthetic local-only credentials and facts. It never enables
simulation authority, invokes Solana, or treats a runtime probe as production
tax, settlement, or compliance evidence. Production identity, mTLS, provider,
load, and SLO gates remain separate.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Unit tests only | Fast and deterministic | Cannot detect image entrypoint, environment, port, or container drift | Rejected as the sole gate |
| Full end-to-end stack only | Broad coverage | Slow, noisy, and can hide which Tax Engine boundary failed | Rejected as the only Tax Engine contract |
| Ad hoc host process smoke test | Simple local debugging | Violates the Docker-first runtime boundary and can use a different toolchain | Rejected |
| **Pinned Docker service runtime contract** | Real image/process, bounded focused failures, CI-reproducible | Requires Docker and a synthetic local container | **Selected** |

## Invariants and failure behavior

- The probe uses an isolated Compose project name and removes only its own
  container/network state on exit.
- Credentials, request facts, and response evidence are synthetic and never
  written to repository files or logs beyond bounded failure diagnostics.
- A failed health, authentication, calculation, body-limit, metric, or cleanup
  assertion returns non-zero; no assertion is downgraded to a warning.
- The runtime contract does not call the service from the settlement hot path
  and cannot authorize or mutate Solana funds.
- The metric assertion uses a fixed service/operation selector; it does not
  accept user-controlled labels or inspect high-cardinality values.

## Verification and release impact

- `make tax-engine-service-contract` remains the fast unit/HTTP test target.
- `make tax-engine-runtime-contract` is included in `make check` and CI.
- Docker image scanning and the runtime contract are independent: a clean scan
  does not prove behavior, and a passing behavior probe does not waive scan or
  production identity requirements.
- Production mTLS, secret delivery, regional resource profiles, live provider
  identity, and SLO evidence remain release gates.
