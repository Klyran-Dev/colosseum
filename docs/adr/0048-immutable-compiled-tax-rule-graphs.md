# ADR-0048: Immutable compiled tax rule graphs on the calculation hot path

- Status: Accepted
- Date: 2026-09-26
- Owners: Tax Platform / Performance
- Scope: Tier-1 tax-engine rule evaluation

## Context

The tax engine must remain deterministic and replayable while serving a high
concurrency quote workload. Validating a published rule, calculating its
canonical digest, and topologically sorting its dependency graph on every
request creates avoidable CPU and allocation work. It also encourages callers
to invent unsafe mutable caches around the rule document.

The persisted `RuleSet` remains the source of truth. A compiled representation
must therefore be an optimization artifact that cannot change the rule,
evaluation order, or evidence digest.

## Decision

Expose `RuleSet::compile()` and an immutable `CompiledRuleSet`. Compilation:

1. validates component identifiers, rates, brackets, clamps, and references;
2. computes the canonical rule digest;
3. performs dependency cycle detection and stores a topological evaluation
   order; and
4. owns a private clone of the validated rule document.

`CompiledRuleSet::calculate()` validates only request facts, evaluates the
precomputed order using exact rational arithmetic, and emits the same
`CalculationResult` and rule digest as the uncached `calculate()` function.
The uncached function remains as the safe default and delegates through
compilation. Services may cache compiled values by the published rule digest
using bounded, version-aware eviction; cache contents are never authoritative.

The compiled graph is process-local and must be discarded when a published
rule is retired, superseded, or its digest changes. No network, database,
clock, or wallet dependency is introduced into the domain crate.

## Alternatives considered

- Revalidate and sort every request: rejected for the hot path because it adds
  predictable CPU and allocation overhead without increasing assurance after a
  published immutable rule has already been validated.
- Cache mutable JSON or share references into the persistence layer: rejected
  because mutation or stale aliasing could break digest binding and replay.
- Compile rules in a separate service: rejected because network hops and cache
  coherence would increase tail latency and move a deterministic domain
  concern outside the Rust calculation boundary.
- Generate machine code or use runtime code generation: rejected for the first
  production path because portability, auditability, and sandboxing costs are
  not justified by the bounded rule graph.

## Consequences

Positive:

- published rules can be evaluated with a bounded, allocation-conscious hot
  path;
- the cached artifact retains the same canonical digest and exact arithmetic;
- callers get a safe API rather than implementing ad hoc rule caches; and
- equivalence between compiled and uncached evaluation is covered by tests.

Costs and follow-on gates:

- the API/service layer still needs a bounded cache with metrics and invalidation
  before claiming a production latency improvement;
- cache hit ratio, compile latency, calculation latency, and p99 memory use
  must be measured under representative concurrent load; and
- compiled-rule eviction and rule publication invalidation require operational
  tests before controlled production.
