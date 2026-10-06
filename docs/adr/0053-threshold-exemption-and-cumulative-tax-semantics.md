# ADR-0053: Evidence-bound threshold, exemption, and cumulative tax semantics

- Status: Accepted
- Date: 2026-09-26
- Owners: Tax Platform / Legal-Tax Governance
- Scope: Tier-1 tax-engine rule DSL and calculation facts

## Context

Thresholds and exemptions are common global tax semantics, but “threshold” is
ambiguous unless the rule declares whether it applies per transaction or to a
cumulative period, and whether crossing it taxes only the excess or activates
the whole transaction. A boolean exemption without an evidence-bound reason
is equally difficult to audit and replay.

The current engine already has exact rational arithmetic and immutable rule
graphs. It must add these semantics without reading a clock, database, or tax
service during calculation.

## Decision

Add an optional, versioned threshold to each component:

- `basis`: `PerTransaction` or `CumulativePeriod`;
- `mode`: `ExemptUpTo` or `TriggerAt`; and
- an exact non-negative atomic threshold.

The semantics are deterministic:

- `PerTransaction + ExemptUpTo`: taxable base is the current base less the
  threshold, floored at zero;
- `PerTransaction + TriggerAt`: the full current base is taxable only when it
  reaches the threshold;
- `CumulativePeriod + ExemptUpTo`: the engine consumes the remaining
  threshold using a supplied pre-transaction cumulative snapshot; and
- `CumulativePeriod + TriggerAt`: the full current base is taxable only when
  the snapshot plus current base reaches the threshold.

Rules may declare bounded exemption codes. Facts carry the asserted codes and
an exemption applies only when a declared component code matches. The facts
also carry an optional cumulative-period snapshot containing a period ID, the
pre-transaction cumulative base, and a non-zero evidence digest. A cumulative
rule without that snapshot fails closed.

The snapshot is input evidence, not mutable engine state. The caller must
obtain and authorize it outside the pure engine; its digest is included in the
facts digest and therefore in any calculation attestation. Inclusive tax with
a threshold is rejected unless the separate explicit equation in ADR-0362 is
declared; an exemption remains valid for inclusive tax and yields zero.

The calculation trace records raw base, effective threshold-adjusted base,
threshold, exemption match, raw tax, clamp result, and final rounded tax.
Threshold and exemption declarations are included in the canonical rule
digest, with exemption codes encoded in sorted order.

## Alternatives considered

- **Treat thresholds as ordinary credits:** rejected because credits apply to
  tax output while thresholds change the taxable base or activation condition.
- **Read cumulative totals from PostgreSQL during calculation:** rejected
  because it adds latency and nondeterminism to the domain and creates a race
  around the authoritative snapshot.
- **Accept a caller boolean such as `is_exempt`:** rejected because it is not
  explainable or bound to approved evidence.
- **Implicitly activate all cumulative thresholds once crossed:** rejected
  because jurisdictions differ between excess-only and whole-transaction
  activation; the mode must be explicit.
- **Silently apply a per-transaction fallback when a snapshot is missing:**
  rejected because it would under- or over-collect and make replay diverge.

## Invariants and budgets

- All threshold and cumulative values fit the Solana `u64` atomic bound.
- Cumulative values are non-negative and are never mutated by the engine.
- Every cumulative calculation requires a non-zero evidence digest and period
  identifier.
- Exemption code lists and rule graph collections remain bounded.
- The hot path performs only bounded in-memory comparisons and exact integer /
  rational operations; no network, clock, or database access is allowed.
