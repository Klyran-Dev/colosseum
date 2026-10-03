# ADR-0275: Capability snapshot latency evidence

- Status: Accepted
- Date: 2026-10-03
- Owners: Platform, Security, Performance

## Context

The product shell requests a short-lived, server-owned capability snapshot so
that the browser can explain available work without interpreting JWT claims or
becoming an authorization authority. The endpoint performs authentication,
evaluates the bounded RBAC/PBAC/ABAC policy catalog, creates a UUIDv7 snapshot,
and returns a small JSON document. The endpoint is intentionally independent of
PostgreSQL and Solana RPC, but it still runs inside the authenticated API
process and must be measured under concurrent load.

The implementation plan requires representative latency evidence in addition to
unit and contract tests. A benchmark that only measures an in-process function
would miss HTTP parsing, Ed25519 verification, serialization, replica behavior,
and admission under concurrent requests. A production claim still requires
regional staging evidence; local synthetic results are only a development and
release-candidate diagnostic.

## Decision

Add `scripts/benchmark_authorization_capabilities.sh` and the
`make benchmark-authorization-capabilities` target. The benchmark:

1. Builds the pinned API image and starts PostgreSQL, the migration runner, and
   two API replicas in an isolated Compose project.
2. Uses an ephemeral Ed25519 signing key and a synthetic JWT with the complete
   capability catalog's scopes and roles. No key or token is persisted.
3. Warms and concurrently requests `GET /v1/authorization/capabilities` across
   both replicas with bounded request, concurrency, timeout, and response-file
   limits.
4. Verifies every response is HTTP 200, has the expected schema, subject,
   tenant, bounded expiry, fourteen known capability IDs, and the exact expected
   allow/indeterminate decisions. A successful HTTP response with malformed or
   incomplete authorization data is a benchmark failure.
5. Emits p50/p95/p99 milliseconds, request count, concurrency, replica count,
   status/error counts, and the configured p95 budget as machine-readable
   key/value evidence. The default local diagnostic budget is 25 ms; a reviewed
   `ALLOW_LATENCY_BUDGET_MISS=true` override is required to retain a diagnostic
   result that misses it.

The benchmark is not a production SLO, does not authorize a deployment, and
does not replace staging regional load, identity-provider, or browser evidence.
It must remain bounded and must never send financial payloads or expose
credentials in output.

## Local evidence

On 2026-10-03, the default workload completed 256/256 HTTP 200 responses at
concurrency 32 across two pinned local arm64 API containers. With the complete
fourteen-capability catalog, the observed p50/p95/p99 latency was
`1.052/1.915/3.114 ms`; the 25 ms p95 diagnostic budget was met. This remains
synthetic service evidence and is not a regional or production SLO approval.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Measure only `authorization_capabilities_inner` with a Rust microbenchmark | Fast and deterministic | Omits HTTP, JWT verification, serialization, and replica behavior | Rejected |
| Use an unbounded external load generator | Rich concurrency model | Adds supply-chain and environment variance, and can make local runs non-reproducible | Rejected |
| Add a browser-only timing assertion | Exercises the visible product | Includes browser scheduling and network noise while under-measuring the service boundary | Rejected |
| **Bounded Docker HTTP benchmark with two API replicas and semantic response checks** | Reproducible service-level evidence, low operational cost, and correct authorization assertions | Local synthetic evidence is not regional production SLO evidence | **Selected** |

## Invariants and acceptance criteria

- The benchmark never uses a real identity-provider key, tenant, token, or
  financial fact.
- Every measured response is authenticated, semantically valid, tenant-bound,
  and within the server-owned expiry contract.
- Workload size, concurrency, timeout, response retention, and p95 budget are
  positive and bounded before Docker starts.
- Two replicas may serve the requests, but no request may depend on shared
  in-process authorization state.
- The benchmark fails on transport errors, non-200 responses, malformed
  capability sets, response count mismatch, or an unreviewed p95 budget miss.
- The result identifies itself as `scope=local_synthetic` and is never used as
  evidence of production readiness.
