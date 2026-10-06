# ADR-0381: Bounded single-flight Tax Engine rule compilation

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal Low-Latency Architecture, Tax Engine, SRE
- Scope: Warm Tax Engine compiled-rule cache miss behavior
- Bounded contexts: Tier-1 Tax Engine, Service Runtime, Performance Evidence

## Context

The warm Tax Engine service already has an immutable digest-keyed LRU cache and
a bounded blocking semaphore. A cache miss currently releases the cache lock
before compiling, which keeps unrelated cache hits responsive but permits
multiple concurrent requests for the same rule digest to compile the same
immutable graph simultaneously. A burst against a newly published rule can
therefore create avoidable CPU contention and inflate tail latency without
changing the result.

The settlement and quote hot paths remain in-process and do not depend on this
service. This decision is limited to the separately deployable warm adapter.
Compilation must remain deterministic, bounded, and non-authoritative.

## Decision

Add a process-local single-flight map keyed by the canonical 32-byte rule
digest. Each entry owns a bounded `OnceLock` containing the shared compilation
result. The first request executes validation-independent compilation and
inserts the immutable `Arc<CompiledRuleSet>` into the existing LRU; concurrent
requests for the same digest wait on and reuse that result. Compilation errors
are shared with the waiters and the in-flight entry is removed after completion.

The map is bounded by the service's existing maximum admitted calculation
concurrency: requests acquire the semaphore before entering compilation, so
there can be no more in-flight digest entries than permits. No network, cache,
database, clock, or external lock is introduced. A successful result is still
looked up from the LRU on later requests, and the single-flight map is never a
source of authority or freshness.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Compile under the LRU mutex | Simple and naturally coalesces misses | Blocks unrelated hits and distinct-rule compiles behind rule compilation | Rejected |
| Allow duplicate compilation | Minimal implementation | CPU stampede and avoidable p95/p99 inflation during rule bursts | Rejected |
| Distributed Redis/lock service | Cross-replica coalescing | Adds network failure, latency, and a non-authoritative dependency to the service path | Rejected |
| **Bounded process-local single-flight keyed by digest** | Coalesces identical misses without blocking unrelated cache work; preserves deterministic failure sharing | Coalescing is per replica; bounded cleanup must be tested | **Selected** |

## Invariants and failure behavior

- At most one compilation closure executes for a digest within a service
  instance at a time.
- Every waiter receives the exact same successful immutable compiled graph or
  the same typed failure; no waiter silently falls back to a duplicate compile.
- The in-flight map is bounded by admitted service concurrency and is removed
  after success or failure; repeated distinct-rule misses cannot accumulate
  completed coordination entries.
- The LRU remains bounded and maintains one queue entry per digest; single-flight
  does not change recency or eviction semantics.
- A cache or in-flight coordination lock failure returns a service failure and
  never authorizes a financial operation.
- Rule validation, canonical digest calculation, arithmetic, and response
  serialization remain unchanged.
- Shutdown remains bounded by the existing service task and semaphore
  lifecycle; no detached coordination task is introduced.

## Verification and release impact

- A failing-first concurrent test uses a bounded barrier and test-only compile
  instrumentation to prove one compilation for many same-digest callers,
  shared success, cleanup, and subsequent LRU reuse.
- A bounded repeated-miss test exercises distinct rule digests through the
  semaphore and asserts that the in-flight map returns to zero after each
  completed request.
- Existing exact-result, cache-bound, authentication, body-limit, overload,
  runtime, and mTLS tests remain mandatory.
- The complete Tax Engine service suite (23 tests) and Clippy pass after
  implementation. The post-change local Docker benchmark returned 256/256
  authenticated HTTP 200 responses at concurrency 32 with p50/p95/p99 of
  1.466/3.256/5.705 ms on native arm64. The benchmark is warm and therefore
  does not replace the cold-miss concurrency test; its output remains synthetic
  evidence and cannot establish a regional or production SLO.
- Linux/amd64 resource profiles, regional latency, rule-publication invalidation,
  and production identity evidence remain separate release gates.
