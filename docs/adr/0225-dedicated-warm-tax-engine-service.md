# ADR-0225: Dedicated warm Tax Engine service adapter

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Tax Engine, Platform, SRE
- Bounded contexts: Tax Engine, Service Runtime, Local Platform

## Context

The Tax Engine is correctly implemented as a pure Rust domain library,
but the Compose topology did not provide the separately deployable Tax Engine
service required by the repository architecture contract. The API and worker
currently link the library in-process, which is the right low-latency shape for
the quote and settlement-critical paths, but it leaves no warm service adapter
for independently scaled simulation, batch calculation, or future internal
consumers.

Adding a network hop to the settlement hot path would violate the bounded
latency and fail-closed authority rules. Conversely, adding a second tax
implementation would create semantic drift and a second calculation
authority.

## Decision

Create a dedicated Rust `tax-engine-service` binary that shares the `tax-engine`
domain crate and exposes an authenticated internal HTTP adapter:

- `POST /v1/calculate` for aggregate facts;
- `POST /v1/calculate-document` for bounded document facts;
- unauthenticated `/healthz` and `/metrics` on the private container health
  plane for liveness and Prometheus scraping;
- authenticated `/readyz` and calculation endpoints for operational state and
  tax work; and
- a bounded request body and fixed operation metrics.

The service compiles immutable rule documents into a process-local, bounded
LRU cache keyed by the canonical rule digest. Cache misses compile the shared
domain graph; calculations never call a database, RPC, clock, filesystem, or
wallet. Admission uses a bounded semaphore and 500 ms queue deadline. The
cache is an optimization only and never changes the rule digest or calculation
semantics.

The API keeps using the in-process library for synchronous quote, attestation,
compile, and settlement paths. The new service is for explicitly internal
batch/simulation consumers until a separately governed service-to-service
latency, identity, TLS/mTLS, and failure contract authorizes another use.

Every process requires an explicit `TAX_ENGINE_ENV` and a non-empty
`TAX_ENGINE_AUTH_TOKEN`; local Compose values are synthetic only. Staging and
production token provisioning, transport encryption/mTLS, rotation, and
network policy remain deployment gates.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Keep only the library embedded in API processes | Rejected for the full topology | Lowest latency, but does not satisfy the separately deployable warm-service boundary or independent batch/simulation scaling. |
| Call the Tax Engine service from every API quote and settlement request | Rejected | Adds network, queueing, retry, and availability dependencies to the financial hot path. |
| Implement a second Node/JavaScript tax engine service | Rejected | Duplicates exact arithmetic and rule semantics and creates cross-language authority drift. |
| **Rust service adapter sharing the pure library, with in-process hot-path execution retained** | **Selected** | Satisfies topology and scaling requirements while preserving deterministic semantics and low-latency settlement behavior. |

## Invariants and failure behavior

- Service output is byte-compatible with the shared Rust domain library and
  preserves decimal-string atomic values.
- Unknown or invalid rule schemas, facts, arithmetic overflow, cycles, and
  unsupported document scopes return bounded client errors and never partial
  results.
- Missing/invalid environment or auth configuration fails before binding the
  listener.
- Missing or incorrect service authentication returns `401` without invoking
  the engine.
- Metrics contain only fixed-cardinality, non-sensitive operation series and
  are reachable only on the private Compose/service network; the edge does not
  publish this listener.
- Cache capacity, body size, queue wait, blocking concurrency, and metric
  cardinality are fixed and bounded.
- Service failure cannot authorize, reject, or mutate Solana funds; the API
  hot path does not silently fall back from a failed network call because it
  does not depend on this adapter.

## Verification and release impact

- TDD covers authentication, health/readiness, bounded cache reuse/eviction,
  exact calculation parity, malformed requests, and body limits.
- Compose config, Docker image hardening, bootstrap, and public-documentation
  contracts include the service.
- CI builds the pinned service image, so the Dockerfile and locked Rust
  workspace remain release-visible rather than being validated only by local
  unit tests.
- Production identity, TLS/mTLS, rotation, resource-profile, regional SLO,
  and production workload-identity issuance/rotation evidence remain
  independent release gates.
