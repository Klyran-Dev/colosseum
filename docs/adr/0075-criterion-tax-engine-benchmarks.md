# ADR-0075: Criterion benchmarks for the tax-engine hot path

- Status: Accepted
- Date: 2026-09-26
- Owners: Tax Platform / Performance
- Scope: Pure Rust Tier-1 tax-engine calculation boundary

## Context

The architecture defines quote and calculation latency budgets, but unit tests
alone cannot show the CPU cost of rule compilation or the difference between
the safe uncached calculation path and a reused immutable compiled rule graph.
The project must measure these functions without turning a benchmark into a
production correctness gate or making claims from an unrepresentative laptop
run.

## Decision

Use Criterion `0.8.2` as a pinned development dependency for the tax-engine
crate. The benchmark suite measures three independently named operations:

1. rule compilation, including validation, canonical digest, and dependency
   ordering;
2. uncached calculation, which exercises the safe public API; and
3. calculation using an already compiled immutable rule graph.

The fixture includes a tax-on-tax dependency and explicit rounding/clamp
semantics. Each benchmark consumes its result through `black_box`. Run it with
`make bench-tax-engine`; generated Criterion output is local evidence and is
ignored by Git.

Benchmark reports must record CPU/architecture, OS, Rust version, workload,
warm-up, sample count, confidence interval, and whether the run is native or
containerized. They do not establish service p50/p95/p99 or Solana/RPC
latency; those require a separate representative concurrent load test.

## Alternatives considered

- **Criterion**: selected for statistical sampling, outlier handling, and
  reproducible reports in the Rust ecosystem.
- **Ad hoc `Instant` loops**: rejected because they lack stable sampling and
  confidence reporting and are easy to distort with compiler optimization.
- **Benchmark only the full HTTP service**: rejected as the first signal
  because database, scheduling, network, and container noise would obscure the
  domain hot path; it remains a required follow-on measurement.
- **Run benchmarks in every ordinary unit-test invocation**: rejected because
  benchmark timing is environment-sensitive and should not make correctness
  tests flaky. The explicit target remains available to CI/performance jobs.

## Consequences

The repository now has a repeatable microbenchmark boundary for optimization
work and can compare compiled versus uncached calculation cost. Results must
not be used as a production latency claim until the documented service load,
contention, database, RPC, and regional workload evidence exists.
