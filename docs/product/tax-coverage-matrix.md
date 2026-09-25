# Tax coverage matrix

This is the public capability register for supported tax behavior. A jurisdiction is not supported merely because a rule can be represented by the engine. It must have approved provenance, current test vectors, required facts, operational ownership, and a published effective window.

## Current status

No jurisdiction or tax regime is currently published as production-supported. The repository is in the architecture/design phase.

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

