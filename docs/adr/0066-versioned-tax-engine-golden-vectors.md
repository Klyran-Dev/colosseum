# ADR-0066: Versioned tax-engine golden vectors

- Status: Accepted
- Date: 2026-09-26
- Owners: Tax platform engineering and tax-domain reviewers

## Context

The tax engine is the authoritative off-chain calculation bounded context. Its
fixed-point/rational arithmetic and rule-graph semantics must remain stable
across refactors, language bindings, engine upgrades, and legal-rule releases.
Example-based unit tests alone do not provide a reviewable contract for the
full global-tax scenario matrix.

## Decision

Maintain checked-in JSON vectors under `test-vectors/` using the versioned
schema `solana-tax-protocol/tax-engine/v1`. Each vector contains a complete
rule set, normalized facts, and expected aggregate/component results. Rust
tests consume the same artifacts through `include_str!` and compare exact
atomic amounts, effective bases, exemptions, and component identity.

The initial matrix covers simple exclusive rates, progressive marginal
brackets, tax-on-tax dependencies, cascading bases, inclusive pricing,
credits, floors, caps, truncation, cumulative thresholds, exemptions, and
explicit rounding behavior. New semantics require new vectors and a reviewed
schema or ADR when the contract changes.

The canonical facts encoder is also locked by an exact digest regression for a
cumulative-period snapshot. Each evidence digest is serialized exactly once;
the digest is not a legal or jurisdictional assertion and remains bound to the
external evidence contract.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Rust-only inline examples | Rejected as the only contract | Harder to review, share, or consume from another implementation; easy to omit scenario families. |
| Client-side JavaScript expected values | Rejected | The browser is not the tax authority and a second arithmetic implementation creates drift. |
| External tax-provider snapshots only | Rejected as the authority | Provider responses are evidence/adapters, not a deterministic protocol contract. |
| **Versioned JSON vectors consumed by Rust and reviewed by domain owners** | Selected | Portable, diffable, replayable, and explicit about exact atomic semantics. |

## Consequences

Rule authors must update vectors when a deliberate semantic change is made.
The vectors improve regression detection and future differential testing, but
they do not prove legal coverage for every jurisdiction, replace property/fuzz
testing, or authorize settlement by themselves. Published rules still require
provenance, effective windows, review, and attestation governance.
