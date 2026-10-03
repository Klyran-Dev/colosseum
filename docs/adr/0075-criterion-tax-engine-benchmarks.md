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

## Current local measurement

On 2026-10-03, `make bench-tax-engine` completed on native arm64 macOS
27.0.1 (`Mac16,6`, 14 logical CPUs) with Rust 1.98.1, 3 seconds of warm-up,
and 100 samples per benchmark. The observed medians were approximately:

| Workload | Median | 95% confidence interval |
| --- | ---: | ---: |
| `tax_engine_compile` | 730.77 ns | 728.87–732.88 ns |
| `tax_engine_uncached_calculate` | 1.4475 µs | 1.4433–1.4526 µs |
| `tax_engine_compiled_calculate` | 688.89 ns | 686.97–690.98 ns |
| `tax_engine_document_invoice_64x16` | 254.39 µs | 253.68–255.18 µs |

Criterion reported positive change against the existing local comparison
baseline for three calculation workloads. Because that baseline has no
retained CPU, OS, commit, or toolchain provenance, this run is recorded as
measurement evidence rather than an attributable code regression. A matched
Linux/amd64 performance job must establish the release baseline and block or
approve any regression under the documented service budgets.

## Latest local measurement

On 2026-10-03 UTC, after ADR-0284, `make bench-tax-engine` completed on native
arm64 macOS 27.0.1 (`Mac16,6`, 14 logical CPUs) with Rust 1.98.1, 3 seconds of
warm-up, and 100 samples per benchmark. Criterion reported these time ranges:

| Workload | Reported time range | Change against the local comparison |
| --- | ---: | ---: |
| `tax_engine_compile` | 775.32–785.73 ns | +1.82%; within noise threshold |
| `tax_engine_uncached_calculate` | 1.4336–1.4431 µs | −3.08%; improved |
| `tax_engine_compiled_calculate` | 669.14–673.97 ns | −11.02%; improved |
| `tax_engine_document_invoice_64x16` | 256.24–257.71 µs | −4.69%; improved |

The prior run flagged a compiled-calculation regression because FX-aware
normalization cloned facts even when no conversion was present. ADR-0284 now
returns borrowed facts for that path and owns a normalized copy only when FX
evidence exists. The result preserves digest and replay semantics; the local
measurement is still not a production, regional, or Linux/amd64 SLO claim.
