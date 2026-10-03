# ADR-0284: Zero-copy tax-fact normalization

- Status: Accepted
- Date: 2026-10-04
- Owners: Tax Platform / Performance

## Context

Server-owned FX evidence must normalize source-currency amounts before tax
calculation while preserving the original evidence in the result and replay
digest. The common stablecoin settlement path has no FX conversion. The first
FX-bound implementation cloned the complete `TaxFacts` value even when the FX
field was absent. That copied bounded strings and collections on every
calculation and created a measurable regression in the compiled calculation
benchmark.

The optimization must not weaken validation, change canonical digests, alter
rounding, or permit a caller to mutate facts during calculation.

## Decision

`normalized_for_calculation` returns `std::borrow::Cow`:

- it validates the original facts and returns `Cow::Borrowed` when no FX
  conversion is present;
- it clones and normalizes into `Cow::Owned` only when server-owned FX evidence
  is present;
- the calculation remains read-only, and the original FX evidence remains
  attached to the result for replay and audit binding; and
- document and aggregate calculations use the same rule, so their arithmetic
  and evidence semantics cannot drift.

The ownership behavior is an explicit invariant, not an undocumented compiler
optimization. Tests must prove both branches. Criterion must measure the
compiled, uncached, and representative document workloads after changes.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Clone facts for every calculation | Simple ownership model | Copies bounded data on every no-FX request | Rejected |
| Always allocate a separate normalized structure | Explicit conversion output | Pays allocation cost for the dominant no-FX path | Rejected |
| **Borrow with `Cow`; own only after FX normalization** | Preserves one semantic boundary while removing the common allocation | Adds a lifetime-aware return type and focused tests | **Selected** |
| Duplicate no-FX and FX calculation implementations | Potentially minimal local overhead | Duplicates financial semantics and increases audit drift | Rejected |

## Invariants

- No-FX calculation returns borrowed facts and does not clone the facts graph.
- FX calculation owns normalized facts and clears the conversion marker only in
  the normalized working copy.
- Original facts digest, FX evidence, tax result, and replay verification remain
  unchanged.
- No mutable reference to caller-owned facts escapes the calculation boundary.
- Any future normalization must preserve the same borrowed/owned distinction or
  add a new benchmark and ADR.

## Verification

- Failing-first tests assert borrowed no-FX facts and owned FX-normalized facts.
- The full tax-engine suite passes with 50 tests.
- The native arm64 Criterion run on 2026-10-03 UTC reported:
  - compile: 775.32–785.73 ns;
  - uncached calculation: 1.4336–1.4431 µs;
  - compiled calculation: 669.14–673.97 ns; and
  - 64-line/16-component document: 256.24–257.71 µs.

These are local microbenchmark results, not Linux/amd64 service or regional
production SLO evidence.
