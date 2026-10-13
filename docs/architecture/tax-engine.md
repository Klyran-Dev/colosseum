# Tax Engine deterministic tax engine

## Purpose

The Tax Engine is a deterministic, versioned calculation system for explicitly configured digital-asset commerce scenarios. It converts normalized transaction facts and approved, evidence-bound jurisdiction rules into a tax result, a complete calculation trace, and a signed attestation that the Solana settlement program can verify where the settlement integration is enabled. “Tax Engine” is an internal engineering and evidence standard; it does not claim global jurisdiction coverage, legal/tax approval, or compliance certification.

The engine automates rule execution and evidence. It does not make a legal determination by itself, replace tax counsel, or guarantee registration, filing, reporting, or compliance.

## Bounded context responsibilities

The Tax Engine owns:

- normalized tax facts;
- jurisdiction and place-of-supply resolution;
- arithmetic treatment of an authenticated applicability decision, including
  exemptions and thresholds;
- ordered tax components and calculation bases;
- progressive brackets and marginal rates;
- tax-on-tax and cascading dependencies;
- inclusive/exclusive pricing;
- minimums, maximums, caps, floors, credits, and thresholds;
- bounded taxable-base floor/cap operations applied before rate evaluation;
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
  -> verify and bind exact FX evidence when source and target currencies differ
  -> resolve jurisdiction and place of supply
  -> resolve the evidence-bound tax applicability context, exemptions, and
     registration/threshold state
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

Tax applicability is a separate bounded context under ADR-0407. The engine
does not infer a product category, customer status, registration obligation,
reverse-charge treatment, withholding treatment, or legal taxability from an
amount or the `regime` selector. It consumes a verified, versioned applicability
decision when that context is enabled; unresolved applicability fails closed.
The current repository has no production-supported jurisdiction or treatment.

The first integration slice is now present in `crates/tax-engine`: an optional
`ApplicabilityDecision` binds to the pre-applicability facts digest, is included
in the final facts digest and calculation result, and participates in replay.
Legacy facts without the field retain their historical digest. ADR-0416 now
defines and implements the explicit `TreatmentOutcome` contract: taxable and
withholding amounts are collected from seller proceeds, reverse-charge tax is
reported as self-assessed without reducing seller net, zero-rated preserves a
reportable taxable base with zero collection, and exempt/out-of-scope produce
no authoritative tax base. Unresolved treatment still fails closed. The API
does not accept caller-authored treatment or evidence; a server-owned, signed
applicability context remains required for production use.

`CalculationResult` and `TaxDocumentResult` carry the additive
`treatment_outcome` object with `arithmetic_tax_atomic`,
`collected_tax_atomic`, `self_assessed_tax_atomic`, `withheld_tax_atomic`, and
`taxable_base_atomic`. `total_tax_atomic` remains the collected amount used by
settlement conservation. The exact semantics are covered by
`test-vectors/tax-treatment-v1.json` and replay tests.

The policy boundary now provides the first signed applicability-attestation
kernel in `crates/policy`. `ApplicabilityAttestation` signs a distinct domain
containing the applicability facts and decision digests, jurisdiction
resolution digest, source evidence window, immutable rule release, merchant
identity, issuer key, and sequence. Verification rejects facts substitution
and any attestation that outlives its source evidence. This contract is not yet
accepted by quote, PostgreSQL, or Solana settlement handlers.

ADR-0244 makes foreign-exchange normalization explicit: a verified upstream
provider supplies a bounded source/target currency pair, exact rational rate,
quote identity, observation, target atomic quantum, rounding mode, and evidence
digest. The pure engine converts gross, net, lines, and cumulative snapshots
once before evaluating the rule graph; it never calls an FX provider or reads a
clock. The target `currency` is the calculation/settlement currency, and the
original FX envelope is retained in the facts digest and result for replay and
user explanation.

ADR-0101 defines the deterministic jurisdiction-resolution bounded context;
ADR-0102 binds its server-owned result into `TaxFacts` and
`CalculationResult`. Historical facts may omit the envelope for replay
compatibility, while new globally supported quotes must obtain and persist a
verified resolution before rule selection and attestation issuance.

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

Each component declares `base_components`. For example, component B may use `line_net + component_A`, while component C may use `line_net + component_A + component_B`. Dependencies form a directed acyclic graph. Cycles fail compilation; implicit ordering is forbidden. Whether a component is applicable is not inferred by this graph; it comes from the verified applicability context and explicit component exemption semantics.

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

Threshold rules declare a per-transaction or cumulative basis and whether the
threshold exempts only the amount up to the threshold or activates the full
transaction at the trigger. Cumulative state is supplied as a signed evidence
snapshot containing a period identifier, pre-transaction total, and evidence
digest; missing snapshots fail closed. Exemption codes are declared by the
rule and matched against evidence-bound fact codes. The hot calculation
remains deterministic and replayable.

### Truncation and rounding

Supported semantics are explicit enums, not language defaults:

- `floor`, `ceil`, `half_up`, `half_even`, `away_from_zero`;
- decimal scale and target currency precision;
- positive `quantum_atomic` output increment in asset atomic units, which is
  the canonical representation of decimal-scale policy;
- floor and cap amounts must be exact multiples of `quantum_atomic`, so a
  final quantization step cannot silently undercut a declared minimum or
  change a maximum boundary;
- rounding scope: `component` or `settlement` in the current aggregate-facts
  DSL. Component scope is the backward-compatible default and is required for
  tax-on-tax dependencies. Settlement scope rounds the complete independent
  component set once and allocates residual atomic units by largest fractional
  remainder with canonical component-ID ties;
- line and invoice scopes use ADR-0203's separately versioned, bounded
  `TaxDocumentFacts` contract; they are never inferred from aggregate
  `TaxFacts`. Line scope allocates within each line, while invoice scope
  allocates across canonical `(line_id, component_id)` cells;
- truncation before/after percentage multiplication;
- residual allocation strategy and recipient ordering.

No floating-point values may cross the domain boundary. Use fixed-point integers or exact rational intermediates, then convert once to the declared atomic currency unit.

ADR-0202 records aggregate scope, compatibility, rejection, and
residual-allocation semantics. ADR-0203 records the document facts and
line/invoice aggregation contract. A missing scope is serialized as the legacy
component behavior so existing published rules retain their canonical digest.

## Rule lifecycle

```text
Draft -> Simulated -> Reviewed -> Approved -> Published -> Retired
   |        |           |           |           |
   +--------+-----------+-----------+-----------+--> Rejected
```

Published rules are immutable. Corrections create a new version with an effective window and supersession reference. A jurisdiction cannot have ambiguous active rules for the same tax regime, product category, and fact scope.

Rule persistence uses the self-describing `RuleSetDocument` envelope
`solana-tax-protocol/tax-rule/v1`. The envelope is transport compatibility
metadata outside the financial `RuleSet`, so its introduction does not change
the canonical rule digest. New publications use the envelope; replay has a
bounded read-only adapter for historical bare documents and rejects unknown
schemas.

## Calculation result

```text
CalculationResult {
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

The Rust result binds the current engine version and exposes
`verify_replay(rule_set, facts)`. Replay recalculates and compares the complete
result, including component trace, digests, totals, explanations, and version;
it fails closed on a version mismatch or any tampering. Persistence adds the
calculation UUID and evidence-reference envelope around this domain result.
Persisted quotes also retain the bounded normalized `TaxFacts` document so the
storage adapter can perform tenant-scoped replay without trusting a client,
cache, or reconstructed request. Rows without that document are explicitly
legacy and non-replayable.
```

Each component includes its base, dependencies, rate/bracket, pre-clamp value, credits, min/max result, rounding mode, rounded amount, and human-readable explanation key.

## Low-latency implementation

- Compile approved rules into an in-memory immutable rule graph. The Rust
  domain exposes `CompiledRuleSet`; its canonical digest and dependency order
  are computed once, while the uncached API remains the safe default.
- Cache jurisdiction and merchant configuration with versioned invalidation.
- Use a warm Rust service for authoritative calculations.
- Precompute common product/jurisdiction paths without bypassing the rule digest.
- Return a calculation trace and attestation in the same API response.
- Keep calculation p99 bounded by rejecting unbounded rule graphs and excessive line counts.
- Use the same Rust domain crate for API, indexer verification, and optional WASM preview; the API remains authoritative.

## Test strategy

- The versioned [`tax-engine-v1.json`](../../test-vectors/tax-engine-v1.json)
  contract is executed by Rust and covers simple, progressive, cascading,
  tax-on-tax, inclusive/exclusive, thresholds, cumulative snapshots,
  exemptions, credits, min/max, truncation, and explicit rounding.
- The versioned [`tax-document-v1.json`](../../test-vectors/tax-document-v1.json)
  contract covers bounded line facts, line-scope rounding, invoice-scope
  rounding, canonical line ordering, and quantum residual allocation.
- Golden vectors for every scenario family and boundary.
- Property tests for monotonicity where a rule declares monotonic behavior.
- Metamorphic tests: line reordering, equivalent aggregation, currency precision, and zero-rate components.
- Fuzz tests for malformed graphs, overflow, thresholds, and decimal scales.
- Differential tests between native Rust and WASM preview.
- Replay tests from stored facts/rule/evidence bundles.
- Security tests for unauthorized rule publication and attestation substitution.
