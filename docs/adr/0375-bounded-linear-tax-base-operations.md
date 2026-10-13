# ADR-0375: Bounded linear tax-base operations

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal Low-latency Blockchain Architecture
- Scope: Tax Engine rule DSL and calculation trace

## Context

Global tax regimes commonly define a taxable base as an explicit weighted or
subtractive combination of declared amounts: a taxable share of a bundled
price, a statutory deduction, or a component base net of an excluded levy.
`TaxBase.sources` currently represents only an additive set of distinct
sources. Reusing a source to imply weighting is ambiguous and is correctly
rejected by ADR-0361, but leaving every weighted or subtractive rule
unrepresentable limits the global engine.

The engine must not become a general-purpose expression interpreter. A rule
base is part of signed financial evidence and must have bounded work,
deterministic serialization, explicit dependency semantics, and a trace that
can be replayed across languages.

## Decision

Add an explicit `Weighted` base reference in the next rule-document envelope
schema. A weighted term names exactly one `Gross`, `Net`, or component source
and carries a signed integer numerator and positive integer denominator. A
negative numerator is the explicit subtractive form. A component may contain
at most 128 distinct source identities, and a weighted source has the same
dependency edge as its underlying component source.

The operation is linear only. It does not permit source-by-source
multiplication, conditionals, arbitrary functions, loops, external facts, or
implicit duplicate terms. Coefficients and intermediate sums use exact
rational arithmetic. Because Solana money is integral atomic units, a base
whose exact result is not an integer atomic amount fails closed with a typed
calculation error; it is never silently rounded. Tax rounding remains the
declared component/settlement/line/invoice operation after the base is
resolved.

Legacy additive rules retain their canonical bytes and v1 document envelope.
Rules containing a weighted term require the v2 envelope and add an explicit
canonical digest marker. `RuleSetDocument::new` selects the minimum envelope
that can represent the rule. Historical v1 documents cannot opt into the new
operation by changing a JSON shape alone.

The implementation keeps a bounded integer fast path for legacy additive
bases with no weighted terms or base clamps. Generalized rational evaluation
is entered only for v2 operations. This preserves v1 arithmetic semantics
without adding rational or allocation overhead to the common calculation
path. The fast-path classification is covered by a focused test; Criterion
remains the performance evidence gate.

The authoring surface exposes the operation as a structured source, sign,
numerator, and denominator row. Advanced JSON cannot bypass server-side
schema, coefficient, dependency, and exact-integrality validation.

## Alternatives considered

1. Continue rejecting weighted/subtractive bases. Rejected: safe but does not
   cover common global tax-base semantics.
2. Treat repeated sources as weights. Rejected by ADR-0361: not
   self-describing and too easy to create accidentally.
3. Permit a general expression AST with arbitrary operators. Rejected:
   unbounded validation/compute surface, difficult legal review, and poor
   on-chain attestation bounds.
4. Use binary floating point or silently round fractional bases. Rejected:
   non-replayable financial results and hidden legal semantics.
5. **Use a versioned, bounded linear term list with exact rationals and an
   integral-atomic admission gate.** Selected: it adds common capability,
   preserves deterministic evidence, and keeps hot-path work bounded.

## Invariants and verification

- Weighted coefficient denominator is positive, numerator is non-zero, and
  both fit the declared bounded integer domain.
- Source identities are unique after unwrapping weighted terms.
- Component dependencies remain acyclic and are evaluated before dependents.
- Exact base sums that are fractional or overflowed fail before tax
  calculation, persistence, attestation, or settlement.
- Legacy rule digests and v1 serialized documents remain byte-compatible.
- Golden vectors cover positive and subtractive weighting; focused domain tests
  cover dependent components, fractional non-integral rejection, duplicate
  identity rejection, coefficient bounds, and v1/v2 envelope selection.
- Legacy additive bases are classified into the integer fast path; weighted or
  bounded bases use the generalized exact evaluator.

## Consequences

The engine can express a bounded class of real global tax bases while keeping
the authority boundary explicit. Legal owners must still approve the meaning
of each coefficient and effective rule release; representability is not legal
applicability. More complex nonlinear or jurisdiction-specific operations
remain separate future DSL decisions.

## Performance evidence

On 2026-10-05, the first native arm64 run after the generalized base evaluator
was introduced showed regressions in legacy compilation and uncached
calculation because every base paid for rational arithmetic and duplicate
source allocation. The integer fast path and allocation-free bounded duplicate
check were then added under this decision. A matched rerun on native arm64
macOS 27.0.1 (`Mac16,6`, 14 logical CPUs) with Rust 1.98.1, three seconds of
warm-up, and 100 Criterion samples measured:

| Workload | Median | Result against retained local baseline |
| --- | ---: | --- |
| `tax_engine_compile` | 761.71 ns | 25.84% faster |
| `tax_engine_uncached_calculate` | 1.4933 µs | 14.08% faster |
| `tax_engine_compiled_calculate` | 672.24 ns | 1.96% faster |
| `tax_engine_document_invoice_64x16` | 256.78 µs | 2.16% faster |

These are local microbenchmark measurements. They do not establish service,
regional, Linux/amd64, RPC, or production p50/p95/p99 SLOs.
