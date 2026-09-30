# ADR-0202: Explicit tax rounding scope and deterministic residual allocation

## Status

Accepted for implementation. The existing component-level behavior remains
the backward-compatible default; settlement-scope rounding is an additive DSL
capability. Line and invoice aggregation require a separately versioned
line-facts contract and are not inferred from one aggregate `TaxFacts` amount.

## Context

The Tier-1 tax engine already supports exact rational intermediates, explicit
rounding modes, truncation through `Floor`, tax-on-tax dependencies, and
replay-bound rule digests. Its rule document does not yet state whether a
rounding mode applies per component or after the complete settlement total.
Rounding dependent components independently can create a different result
from rounding the aggregate, while silently switching behavior would change
financial outcomes and replay digests.

## Decision

1. Add `RoundingScope` to `RoundingSpec` with `Component` and `Settlement`.
   Missing scope deserializes to `Component`, preserving existing documents
   and calculation results.
2. Add `quantum_atomic` to `RoundingSpec`, defaulting to `1`. It is the
   smallest allowed output increment in the asset's atomic units and is the
   exact integer representation of a jurisdiction's declared decimal-scale
   truncation/rounding boundary. It must be positive and fit `u64`; no decimal
   string or binary floating-point conversion is permitted.
   Floors and caps must be exact multiples of the quantum so the declared
   minimum/maximum remains meaningful after final rounding; incompatible rules
   are rejected at compilation.
3. `Component` is the default and remains the required scope for tax-on-tax
   or cascading dependencies. Each component is clamped and rounded before
   its amount becomes an input to a dependent component.
4. `Settlement` is allowed only when every component uses that scope and no
   component base references another component. The rule compiler rejects
   mixed scopes and dependent settlement-scope graphs rather than guessing
   whether a dependency sees a rounded or exact amount.
5. Settlement scope requires one rounding mode and quantum for the complete
   rule set.
   The engine sums exact post-credit, floor, and cap rational values, rounds
   that total once using the declared mode, then allocates the quantized result
   by largest fractional remainder in quantum increments. Ties resolve by
   canonical component ID. Allocation is bounded by the component count and
   preserves conservation.
6. Canonical digest encoding is additive: omitted/default `Component` scope
   retains the legacy bytes; an explicit settlement scope contributes a
   versioned marker, and a non-default quantum contributes an additional
   versioned integer field. Published rule corrections therefore remain
   immutable and replay-safe.
7. The public UI exposes the scope and atomic quantum and explains that
   settlement scope is rejected for tax-on-tax dependencies. It does not
   calculate or silently rewrite the rule in the browser.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep implicit component rounding | No schema or code changes | Cannot express aggregate settlement rounding; ambiguity remains hidden | Rejected |
| Allow mixed scopes and infer dependency semantics | Flexible authoring | A dependent component could consume rounded or exact values without a stable legal meaning | Rejected |
| Round the total and silently discard residual units | Simple implementation | Breaks conservation and creates unexplained value differences | Rejected |
| **Explicit component/settlement scope with bounded largest-remainder allocation** | Deterministic, replayable, conserving, compatible with tax-on-tax, and explainable | Settlement scope is deliberately restricted to independent components | **Selected** |
| Add line/invoice facts in this change | Broader tax coverage | Changes fact identity, API/storage contracts, UI, and golden vectors beyond this rounding boundary | Deferred to a dedicated line-facts ADR |

### Decimal-scale representation

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Binary floating-point decimal places | Familiar API | Non-deterministic financial boundaries and unsafe large-amount behavior | Rejected |
| Decimal strings interpreted by each adapter | Human-readable | Cross-language parser/scale drift and hidden currency assumptions | Rejected |
| **Positive `quantum_atomic` integer** | Exact, language-neutral, asset-precision aware, and cheap on the hot path | Rule authors must know the asset's atomic precision | **Selected** |

## Invariants and failure behavior

- Omitted scope behaves exactly like the existing component scope.
- A compiled rule cannot contain mixed scopes, and settlement scope cannot
  contain component dependencies.
- Settlement allocation is non-negative, bounded, deterministic, and sums to
  the one rounded settlement total.
- Every rounded result is a multiple of the declared positive atomic quantum.
- Every declared floor and cap is a multiple of that quantum; otherwise the
  rule is rejected rather than silently weakening a min/max boundary.
- No floating point, implicit language rounding, random ordering, or client
  supplied residual decision is permitted.
- The rule digest, calculation result, replay comparison, and browser contract
  include the selected scope and allocated component values.

## Verification and release gates

- Failing-first Rust tests cover backward-compatible deserialization, digest
  binding, rejected mixed/dependent rules, exact settlement rounding, largest
  remainder allocation, deterministic ID ties, and conservation.
- Golden vectors cover component truncation and settlement-scope half-up,
  floor, and tie allocation behavior.
- Rule-workbench tests cover authoring and validation of the new scope.
- Golden vectors cover non-default atomic quantum truncation/rounding and
  quantum-sized settlement residual allocation.
- Production legal/tax-owner approval, jurisdiction coverage, and line/invoice
  aggregation remain independent release gates.
