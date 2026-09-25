# Tier-1 global tax engine

## Purpose

The Tier-1 Tax Engine is a deterministic, versioned calculation system for global digital-asset commerce. It converts normalized transaction facts and approved jurisdiction rules into a tax result, a complete calculation trace, and a signed attestation that the Solana settlement program can verify.

The engine automates rule execution and evidence. It does not make a legal determination by itself, replace tax counsel, or guarantee registration, filing, reporting, or compliance.

## Bounded context responsibilities

The Tax Engine owns:

- normalized tax facts;
- jurisdiction and place-of-supply resolution;
- taxability, exemptions, thresholds, and tax categories;
- ordered tax components and calculation bases;
- progressive brackets and marginal rates;
- tax-on-tax and cascading dependencies;
- inclusive/exclusive pricing;
- minimums, maximums, caps, floors, credits, and thresholds;
- decimal scale, truncation, rounding, and residual allocation;
- rule provenance, effective dates, approval status, and immutable versions;
- calculation traces, evidence references, and replayable outputs;
- calculation attestations for settlement.

It does not own:

- private keys or wallet custody;
- Solana settlement authority;
- tax filing or fiat remittance;
- arbitrary web access during calculation;
- an unreviewed legal interpretation of a jurisdiction.

## Calculation pipeline

```text
Raw checkout facts
  -> normalize units, currency, timestamps, customer/merchant facts
  -> resolve jurisdiction and place of supply
  -> resolve tax category, exemptions, and registration/threshold state
  -> select immutable RuleSet
  -> build dependency graph of tax components
  -> calculate bases and progressive brackets
  -> apply min/max, caps, credits, and exemptions in declared order
  -> apply inclusive/exclusive transformations
  -> apply declared rounding/truncation at declared scope
  -> allocate residual atomic units deterministically
  -> emit totals + line/component trace + evidence references
  -> sign CalculationAttestation
```

Every stage is pure with respect to its inputs. External facts such as exchange rates, customer evidence, registration thresholds, or policy snapshots are resolved before calculation and included by digest in the calculation input.

## Supported scenario families

### Simple percentage

```text
tax = round(base * rate)
```

The rule must declare the base, rate scale, rounding mode, and rounding scope.

### Progressive/marginal brackets

```text
tax = sum(
  min(max(base - bracket.lower, 0), bracket.width) * bracket.rate
)
```

Brackets must be ordered, non-overlapping, explicit about open/closed boundaries, and tested at every boundary. The engine rejects gaps or overlaps unless the rule explicitly declares them.

### Tax-on-tax and cascading taxes

Each component declares `base_components`. For example, component B may use `line_net + component_A`, while component C may use `line_net + component_A + component_B`. Dependencies form a directed acyclic graph. Cycles fail compilation; implicit ordering is forbidden.

### Inclusive and exclusive tax

Rules explicitly declare whether the input price includes a component. Inclusive calculations solve the declared equation using fixed-point/rational arithmetic; they do not use binary floating point. The result includes the recovered net base and tax amount in the trace.

### Min/max, caps, floors, and credits

Each clamp declares its position in the evaluation sequence:

```text
raw_component
  -> percentage/bracket result
  -> credit deduction
  -> floor
  -> cap
  -> final component rounding
```

The engine never assumes that a cap precedes a credit or that a floor applies before rounding. This order is part of the rule digest and test vectors.

### Thresholds and cumulative periods

Threshold rules declare the measurement period, scope, currency, included transaction classes, and authoritative snapshot. Cumulative state is supplied as a signed evidence snapshot; the hot calculation remains deterministic and replayable.

### Truncation and rounding

Supported semantics are explicit enums, not language defaults:

- `floor`, `ceil`, `half_up`, `half_even`, `away_from_zero`;
- decimal scale and target currency precision;
- rounding scope: `component`, `line`, `invoice`, `settlement`;
- truncation before/after percentage multiplication;
- residual allocation strategy and recipient ordering.

No floating-point values may cross the domain boundary. Use fixed-point integers or exact rational intermediates, then convert once to the declared atomic currency unit.

## Rule lifecycle

```text
Draft -> Simulated -> Reviewed -> Approved -> Published -> Retired
   |        |           |           |           |
   +--------+-----------+-----------+-----------+--> Rejected
```

Published rules are immutable. Corrections create a new version with an effective window and supersession reference. A jurisdiction cannot have ambiguous active rules for the same tax regime, product category, and fact scope.

## Calculation result

```text
CalculationResult {
  calculation_id
  engine_version
  rule_set_id
  rule_digest
  facts_digest
  jurisdiction_resolution
  currency
  taxable_base
  components[]
  total_tax
  total_net
  rounding_residual
  trace[]
  evidence_refs[]
  generated_at
  expires_at
}
```

Each component includes its base, dependencies, rate/bracket, pre-clamp value, credits, min/max result, rounding mode, rounded amount, and human-readable explanation key.

## Low-latency implementation

- Compile approved rules into an in-memory immutable rule graph.
- Cache jurisdiction and merchant configuration with versioned invalidation.
- Use a warm Rust service for authoritative calculations.
- Precompute common product/jurisdiction paths without bypassing the rule digest.
- Return a calculation trace and attestation in the same API response.
- Keep calculation p99 bounded by rejecting unbounded rule graphs and excessive line counts.
- Use the same Rust domain crate for API, indexer verification, and optional WASM preview; the API remains authoritative.

## Test strategy

- Golden vectors for every scenario family and boundary.
- Property tests for monotonicity where a rule declares monotonic behavior.
- Metamorphic tests: line reordering, equivalent aggregation, currency precision, and zero-rate components.
- Fuzz tests for malformed graphs, overflow, thresholds, and decimal scales.
- Differential tests between native Rust and WASM preview.
- Replay tests from stored facts/rule/evidence bundles.
- Security tests for unauthorized rule publication and attestation substitution.

