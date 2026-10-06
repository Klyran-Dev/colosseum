# ADR-0416: Explicit applicability-treatment settlement semantics

- Status: Accepted
- Date: 2026-10-06
- Owners: Tax engine and settlement domain
- Related: ADR-0407, ADR-0409, ADR-0411

## Context

The applicability bounded context already distinguishes `taxable`,
`zero_rated`, `exempt`, `reverse_charge`, `withholding`, `out_of_scope`, and
`unresolved`. The arithmetic engine currently rejects every treatment except
`taxable`. That is a safe interim boundary, but it cannot support a global
tax product: a zero amount is not enough to explain whether a transaction was
zero-rated, exempt, outside scope, self-assessed by the customer, or withheld
from the payee.

Settlement also needs an unambiguous amount contract. `total_tax_atomic` is the
amount deducted from the seller's gross proceeds and therefore cannot be
reused as a reverse-charge amount that the seller does not collect.

## Decision

The pure Rust tax engine emits a versioned `TreatmentOutcome` on every new
calculation and document calculation. It keeps `total_tax_atomic` as the
amount collected from the settlement gross amount and adds explicit outcome
amounts:

| Treatment | Rule graph | Collected / `total_tax_atomic` | Self-assessed | Withheld | Taxable base |
| --- | --- | ---: | ---: | ---: | ---: |
| `taxable` | evaluated | calculated tax | 0 | 0 | gross |
| `zero_rated` | not evaluated | 0 | 0 | 0 | gross |
| `exempt` | not evaluated | 0 | 0 | 0 | 0 |
| `out_of_scope` | not evaluated | 0 | 0 | 0 | 0 |
| `reverse_charge` | evaluated | 0 | calculated tax | 0 | gross |
| `withholding` | evaluated | calculated tax | 0 | calculated tax | gross |
| `unresolved` | not evaluated | rejected | rejected | rejected | rejected |

For `reverse_charge`, component-level `tax_atomic` values are masked to zero
because they are not collected by the seller; the exact calculated amount is
retained in `TreatmentOutcome.self_assessed_tax_atomic` and the rational trace
remains available in the component evidence. For `zero_rated`, `exempt`, and
`out_of_scope`, components are empty because no rate graph is authoritative
for those outcomes. Gross-to-net conservation remains explicit in every
successful result.

The outcome is additive and canonical: it is included in the result digest
and replay equality. Historical results without the outcome remain legacy
evidence and fail the current engine-version replay gate rather than being
silently reinterpreted.

This ADR defines arithmetic/reporting semantics only. It does not claim legal
applicability, filing obligation, registration, deductibility, or jurisdiction
coverage. A jurisdiction owner must publish and sign the applicability
decision before the treatment can enter a production settlement route.

## Alternatives considered

| Alternative | Determinism | Settlement clarity | Replayability | Decision |
| --- | --- | --- | --- | --- |
| Continue rejecting every non-taxable treatment | Pass | Fail; product cannot support global cases | Pass | Rejected |
| Map all non-taxable outcomes to zero tax | Fail; loses legal/reporting distinction | Fail; reverse charge and withholding are misrepresented | Partial | Rejected |
| Let API adapters reinterpret treatment after arithmetic | Fail; multiple authorities | Fail; digest omits the interpretation | Fail | Rejected |
| **Pure engine `TreatmentOutcome` with explicit collection channels** | **Pass** | **Pass** | **Pass** | **Selected** |

## Invariants

- `unresolved` never produces a successful calculation.
- `total_tax_atomic` is always the amount removed from gross to produce net.
- `reverse_charge` never reduces seller net and records the calculated amount
  only as self-assessed tax.
- `withholding` reduces seller net and records the same amount as withheld.
- `zero_rated`, `exempt`, and `out_of_scope` never execute a rate graph.
- All outcome amounts are non-negative and bounded by the engine's atomic
  amount limits.
- A result replays only when its treatment outcome, facts digest, rule digest,
  and engine version all match.

## Verification

- Add failing-first unit tests for each treatment and conservation invariant.
- Add golden vectors covering aggregate and document calculations, including
  reverse charge with tax-on-tax dependencies and withholding with caps.
- Add serialization and replay tests for the outcome fields.
- Keep API, UI, settlement, and legal publication gates separate from this
  pure-domain implementation.
