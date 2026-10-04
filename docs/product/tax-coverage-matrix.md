# Tax coverage matrix

This is the public capability register for supported tax behavior. A jurisdiction is not supported merely because a rule can be represented by the engine. It must have approved provenance, current test vectors, required facts, operational ownership, and a published effective window.

## Current status

No jurisdiction or tax regime is currently published as production-supported. The repository is in the architecture/design phase.

## Engine semantics evidence

The following is the capability boundary of the deterministic engine, not a
claim that any jurisdiction using the behavior is legally supported. Each
scenario is represented by a versioned vector and is executed by the Rust
domain tests. A jurisdiction may use these semantics only after its own facts,
provenance, owner, effective window, and legal review are published below.

| Capability | Explicit semantics | Vector evidence | Domain evidence |
| --- | --- | --- | --- |
| Simple tax | Flat rational rate over a declared base; exclusive price semantics | `simple-exclusive` | `calculates_simple_percentage` |
| Progressive tax | Ordered marginal brackets with bounded lower/upper edges | `progressive-marginal` | `calculates_progressive_brackets` |
| Tax-on-tax / cascading | Component dependency graph; dependent base includes declared prior components only | `tax-on-tax-cascade` | `calculates_tax_on_tax_with_dependency_order`, `compiles_dependencies_before_dependents` |
| Inclusive and exclusive pricing | Explicit gross/net equation; inclusive thresholds require an explicit equation | `inclusive-price` | `calculates_inclusive_tax`, `rejects_inclusive_threshold_without_an_explicit_equation` |
| Thresholds and exemptions | Per-transaction and cumulative-period threshold basis with evidence-bound snapshots; matching exemption codes | `cumulative-threshold`, `exemption-and-rounding` | `applies_per_transaction_exempt_up_to_threshold`, `cumulative_threshold_consumes_a_bound_snapshot`, `matching_exemption_code_zeroes_only_the_declared_component` |
| Credits, floors, and caps | Ordered credit, floor, and cap operations with checked non-negative results | `credit-floor-cap-truncation`, `minimum-and-maximum` | `applies_credit_floor_cap_and_truncation`, `rejects_invalid_rate_and_clamp_boundaries` |
| Truncation and rounding | Declared rounding mode, positive atomic quantum, scope, and deterministic tie behavior | `credit-floor-cap-truncation`, `ceil-rounding` | `rounds_exact_ties_according_to_each_declared_mode`, `component_rounding_uses_declared_atomic_quantum` |
| Residual allocation | Invoice, settlement, and line scopes use canonical largest-remainder or cell allocation without breaking conservation | `settlement-scope-largest-remainder`, `settlement-scope-quantum-residual`, `tax-document-v1.json` | `settlement_scope_rounds_once_and_allocates_residual_by_canonical_id`, `invoice_scope_rounds_once_and_allocates_by_line_then_component_id` |
| Replay and conservation | Canonical facts/rule/engine trace binding; gross equals tax plus net within declared scope and settlement bounds | All tax vectors | `calculation_result_replays_only_with_the_same_engine_and_trace`, property conservation tests, and Solana `u64` boundary tests |

### Deliberate non-claims

- The engine does not infer jurisdiction, taxpayer status, registration,
  filing obligation, or legal applicability from an amount or rate.
- A supported behavior is not a supported jurisdiction. Production publication
  requires the complete capability record below and an approved rule release.
- Binary floating point is not part of the calculation contract. Rates,
  thresholds, amounts, rounding, and residuals use exact integer/rational
  semantics.
- Rules that contain cycles, ambiguous precedence, overlapping brackets,
  missing evidence, invalid effective windows, unsupported asset precision, or
  out-of-bound settlement amounts fail closed.

## Capability record

Every published row must include:

| Field | Meaning |
| --- | --- |
| Jurisdiction | Country/region/state or other tax territory |
| Regime | VAT, GST, sales tax, withholding, digital services tax, or other named regime |
| Product categories | Products/services for which the rule applies |
| Required facts | Customer, merchant, location, registration, evidence, and transaction facts |
| Place-of-supply model | Facts and precedence used to determine jurisdiction |
| Components | Tax components and dependency/order semantics |
| Rate/brackets | Versioned rate, progressive bracket, threshold, cap, or floor behavior |
| Price semantics | Inclusive/exclusive and tax-base behavior |
| Rounding | Precision, truncation, mode, scope, and residual allocation |
| Exemptions/credits | Eligibility facts and evidence requirements |
| Effective window | Start, end, timezone, and supersession reference |
| Provenance | Source references and evidence digest |
| Owner/reviewer | Engineering and legal/tax accountability |
| Test bundle | Golden vectors, boundary cases, and replay fixture identifier |
| Limitations | Explicit unsupported facts, amounts, assets, or edge cases |
| Status | Draft, review-required, published, expired, retired, or blocked |

## Publication rule

Only rows with `Published` status may be used for a production settlement. `Draft`, `Review required`, `Expired`, `Retired`, and `Blocked` rows must fail closed or route to an explicit merchant-review workflow. Historical calculations retain the exact rule and coverage record used at the time of settlement.
