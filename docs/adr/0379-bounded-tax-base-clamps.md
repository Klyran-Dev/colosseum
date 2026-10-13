# ADR-0379: Bounded tax-base clamps

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal Low-latency Tax Engine Architecture
- Scope: Tax Engine v2 rule DSL

## Context

The engine already supports exact linear base terms and min/max operations on
the resulting tax amount. Several global tax regimes instead define a taxable
base as an adjusted amount with a statutory minimum or maximum, such as a
deduction-adjusted base floored at zero or a capped levy base. Encoding those
rules as a tax-result floor/cap changes the legal meaning when a rate,
threshold, or progressive bracket is applied.

## Decision

Add optional `floor_atomic` and `cap_atomic` bounds to `TaxBase`. The engine
will evaluate the declared linear source sum with exact rational intermediates,
require an integral atomic result, apply the floor and then the cap, and pass
the bounded atomic base to threshold, price, rate, credit, result-clamp, and
rounding semantics.

Bounds are non-negative Solana atomic amounts, must fit `u64`, and floor may
not exceed cap. A rule without base bounds preserves v1 canonical bytes. A
rule using weighted terms or base bounds uses the existing version-2 envelope
and adds a canonical base-bound marker to its digest. Negative adjusted bases
remain invalid unless an explicit non-negative floor is declared.

## Alternatives considered

1. Encode base bounds as tax-result floors/caps. Rejected: this changes the
   taxable base and is not equivalent for progressive, threshold, or inclusive
   taxes.
2. Add an unrestricted expression language. Rejected: arbitrary recursion,
   implicit legal semantics, and unbounded evaluation are unacceptable in a
   money-calculation hot path.
3. Require every jurisdiction to precompute adjusted bases externally.
   Rejected: it shifts authority outside the published rule and breaks replay
   from the rule and facts bundle.
4. **Add bounded optional clamps to the existing linear `TaxBase`.** Selected:
   it covers the common min/max-base family while preserving exact arithmetic,
   bounded graph execution, stable v1 compatibility, and explicit v2 review.

## Invariants and verification

- No bound is negative, over `u64::MAX`, or contradictory.
- Exact rational evaluation and integral-atomic admission occur before clamps.
- Clamp order is floor then cap and is included in calculation trace/replay.
- A v1 document cannot contain weighted terms or base bounds.
- Golden vectors cover floor and cap; focused domain tests cover both bounds,
  negative adjusted bases, boundary equality, and property-based bound
  monotonicity/conservation.

## Consequences

The Tax Engine engine can represent more explicit global tax bases without an
arbitrary formula interpreter. Jurisdiction support is still not implied:
provenance, legal ownership, effective windows, facts, and published rule
governance remain required.
