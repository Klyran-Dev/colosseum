# ADR-0145: Reproducible persisted-rule simulation latency evidence

- Status: Accepted
- Date: 2026-09-27
- Owners: Principal Low-Latency Architecture, Tax Domain, API, SRE
- Bounded contexts: Tier-1 Tax Engine, Tax Rules, Governance, API, Operations

## Context

ADR-0144 added a server-owned persisted-rule simulation endpoint. The existing
quote-service load harness is not sufficient evidence for that endpoint: quote
requests intentionally exercise idempotency, durable calculation persistence,
outbox publication, and jurisdiction-provider evidence, while governance
simulation is an ephemeral, read-only operation whose latency should expose
authentication, PostgreSQL lookup, digest verification, compiled-rule-cache
reuse, calculation, serialization, and bounded connection-pool behavior.

The architecture must measure this path with a repeatable workload without
turning a local benchmark into a production performance claim or allowing a
benchmark client to provide its own rule, merchant, or jurisdiction evidence.

## Decision

Add `scripts/benchmark_rule_simulation.sh` and the
`make benchmark-rule-simulation` target. The harness:

1. starts an isolated Compose project using the pinned PostgreSQL 18/API
   images and synthetic published-rule fixture;
2. creates an ephemeral EdDSA JWT containing only the simulation read scope
   and a synthetic governance role;
3. warms the authenticated route, PostgreSQL pool, and immutable compiled-rule
   cache outside the measured sample;
4. sends a bounded number of facts-only simulation requests across two
   independent API containers at bounded concurrency;
5. validates every response is HTTP 200, `simulated_unattested`, and bound to
   the expected server-owned rule identity and digest;
6. reports status counts, error rate, pool-waiting peak, and p50/p95/p99
   request latency; and
7. runs a bounded identical read burst to expose cache and pool contention
   without creating financial rows or outbox events.

The default local diagnostic target is p95 <= 25 ms for 200 requests at
concurrency 32 across two replicas on the pinned Linux CI runtime. A miss is a
non-zero gate unless an operator explicitly sets
`ALLOW_LATENCY_BUDGET_MISS=true`; that exception is visible in the output and
must not be used as evidence of meeting the target. The target is a workload
budget, not a regional production SLO. Production acceptance still requires
regional runs with resource limits, representative rule sizes, connection
pool settings, provider/network conditions, and an approved error budget.

The harness is loopback-only by default, uses synthetic credentials and tax
facts, never signs or submits a Solana transaction, and never persists a
simulation result.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Reuse the quote-service benchmark unchanged | Rejected | It measures writes, idempotency, outbox, and provider evidence rather than the read-only governance path. |
| Benchmark only the pure Rust tax engine | Rejected as the sole gate | It omits authentication, PostgreSQL lookup/digest verification, cache behavior, HTTP, and JSON serialization. |
| Use an unpinned external load generator | Rejected | It weakens clean-checkout reproducibility and adds a second toolchain. |
| Send an unbounded request flood | Rejected | It measures overload, creates avoidable retry pressure, and cannot establish a bounded service budget. |
| **Pinned Compose stack with bounded `curl` workers and response invariants** | **Selected** | It measures the real service boundary, isolates state, reuses required Docker tooling, and keeps evidence auditable. |

## Invariants and failure behavior

- Every measured request has a bounded timeout and exactly one recorded sample.
- Any non-200 response, malformed response, rule identity mismatch, digest
  mismatch, or incomplete sample fails the harness.
- The workload cannot supply a rule document, merchant identifier, digest, or
  jurisdiction-resolution evidence.
- The benchmark uses no real keys, PII, tax evidence, or non-loopback endpoint.
- The harness does not infer production readiness from local latency; output
  explicitly identifies workload, runtime topology, and diagnostic status.
- A latency-budget exception is explicit, bounded to this invocation, and
  cannot silently convert a miss into a pass.

## Verification and follow-up

The CI job and local target retain the workload output as an artifact. The
default Docker run on 2026-09-27 completed 200 requests at concurrency 32
across two replicas with 0% errors, exact rule-digest binding, 64/64 successful
contention responses, and zero observed pool waiters; p50/p95/p99 was
1.958/3.001/6.125 ms against the 25 ms p95 diagnostic budget. This closes the
repeatable local service-evidence gap for persisted-rule simulation.
Regional latency, browser WCAG/RTL/localization evidence, live validator
compute/fee evidence, production cache invalidation, and legal/tax ownership
remain independent release gates.
