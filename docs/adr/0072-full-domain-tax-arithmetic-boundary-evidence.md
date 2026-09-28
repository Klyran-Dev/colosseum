# ADR-0072: Prove Tier-1 tax arithmetic at financial boundaries

- Status: Accepted
- Date: 2026-09-26
- Scope: Rust tax-engine arithmetic and release evidence

## Context

Golden vectors demonstrate representative global-tax semantics, but they do not
prove behavior at the Solana `u64` amount boundary or across every clamp and
rounding edge. A financial engine must demonstrate that zero, maximum atomic
amounts, exact half-rounding ties, invalid rates, and floor/cap boundaries are
deterministic and cannot silently overflow or violate gross conservation.

## Decision

Keep the production implementation exact and integer/rational-based, and add a
layered boundary evidence gate:

1. exact vectors for zero, `u64::MAX`, unit-rate, invalid-rate, invalid-clamp,
   and every rounding mode's tie behavior;
2. bounded property tests over arbitrary `u64` gross/net values with safe rate
   denominators and generated floor/cap ranges; and
3. explicit acceptance of only the typed arithmetic-overflow error where a
   generated rational intermediate cannot fit, never a panic or silent clamp.

The property tests assert amount conservation and non-negative bounded tax for
successful calculations. They do not replace legal rule vectors, fuzzing,
mutation testing, or the validator-backed on-chain gate.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Representative examples only | Rejected | Misses maximum amounts, tie behavior, and combinations of clamps. |
| Floating-point differential testing | Rejected | Floating point is not an authority for monetary semantics. |
| Exhaustive enumeration | Rejected | The input domain is too large; it is not a meaningful proof strategy. |
| **Exact vectors plus bounded full-domain property tests** | **Selected** | Gives deterministic boundary contracts and broad generated coverage without weakening the arithmetic model. |

## Consequences

- Arithmetic changes must update vectors and property invariants before
  implementation is accepted.
- A property failure is a release blocker even when aggregate totals appear
  plausible.
- This evidence covers the off-chain engine only; on-chain CPI conservation,
  compute limits, and token-account behavior require validator evidence.
