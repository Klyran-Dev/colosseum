# ADR-0232: Warm Tax Engine service concurrency evidence

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Tax Engine, SRE
- Scope: Warm Tax Engine HTTP adapter performance contract
- Bounded contexts: Tier-1 Tax Engine, Service Runtime, Performance Evidence

## Context

ADR-0225 deliberately keeps quote and settlement-critical calculations in the
in-process Rust domain library while adding a separately deployable warm Tax
Engine service for internal simulation and batch consumers. Criterion proves
the pure compilation and calculation functions, and ADR-0230 proves that the
real pinned image starts and serves correct requests. Neither proves the
service's bounded semaphore, HTTP serialization, cache-warmed execution, or
tail latency under concurrent requests.

A performance claim without workload metadata is not architecture evidence.
The service therefore needs a deterministic local contract that measures the
real image and records request count, concurrency, timeout, service limits,
image identity, host architecture, and p50/p95/p99 response latency. It must
remain synthetic evidence and must not be mistaken for regional production
SLO, live mTLS, or chain-finality evidence.

## Decision

Add `make benchmark-tax-engine-service`, an isolated Docker workload with
these defaults:

- 256 authenticated aggregate calculations at concurrency 32;
- one valid warm-up request before measurement, using one immutable rule digest;
- exact aggregate result checks on the warm-up and every measured response;
- a 10-second per-request timeout and a bounded startup/cleanup window;
- status/error accounting that fails on any non-200 response or missing sample;
- sorted response durations reported as p50, p95, and p99 in milliseconds;
- image ID, host architecture, service cache capacity, service concurrency,
  request count, and measurement timestamp in the public-safe result; and
- optional persistence to `TAX_ENGINE_BENCHMARK_OUTPUT`, containing only
  workload metadata and aggregate latency statistics.

The benchmark uses the same non-root pinned Tax Engine image and local
environment boundary as ADR-0230. It owns an isolated Compose project and
removes only its container, network, and volumes. It never calls the API
settlement path, uses a production credential, persists tax facts, or changes
authority. The output is diagnostic local evidence; no latency threshold is
declared a production SLO until a separately approved regional/resource-profile
plan exists.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Pure Criterion benchmark only | Fast and stable | Omits HTTP, container, auth, scheduler, and serialization costs | Rejected as the sole service gate |
| Full API quote benchmark | Exercises durable storage and tenant paths | Does not isolate the warm Tax Engine adapter and makes tail attribution ambiguous | Rejected for this boundary |
| Unbounded external load generator | Broad traffic shapes | Non-reproducible, resource-heavy, and difficult to keep synthetic/public-safe | Rejected |
| **Bounded local Docker concurrency contract** | Measures the real service image with explicit workload and cleanup | Local-only evidence; requires Docker | **Selected** |

## Invariants and failure behavior

- All measured requests use the authenticated service boundary.
- Every response is `200` and preserves exact atomic tax/net values.
- No sample is silently dropped, retried, or converted into a success.
- Request count, concurrency, timeout, cache capacity, and queue bounds are
  positive and bounded before the container starts.
- A startup, health, transport, status, parse, or cleanup failure exits
  non-zero and retains bounded diagnostic output.
- The benchmark cannot authorize settlement or make a production readiness
  decision.

## Verification and release impact

- The script is shell-syntax checked and exposed through the Makefile.
- A dedicated Ubuntu CI job runs the declared workload and retains the
  summary/output as a bounded public-safe artifact.
- The service unit tests, clippy, runtime contract, and benchmark run against
  the same locked workspace/image inputs.
- The result must state local/synthetic scope and retain production regional,
  mTLS, resource-profile, and consumer-authorization gates as open.
