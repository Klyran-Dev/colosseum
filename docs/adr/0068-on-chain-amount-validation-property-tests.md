# ADR-0068: Property-test on-chain amount validation across the `u64` domain

- Status: Accepted
- Date: 2026-09-26
- Scope: `programs/settlement` amount-validation boundary

## Context

Settlement arithmetic is a value-safety boundary. Example tests covered a
normal conserving tuple, a mismatch, and zero gross, but they did not exercise
the full `u64` pair space where `tax + net` can overflow. The on-chain program
must reject overflow and non-conserving values before any token CPI.

The test must not imply that host-side validation proves token-program CPI
behavior or finalized ledger deltas. Those remain validator-backed release
gates under ADR-0065.

## Decision

Use a deterministic, bounded `proptest` property in the on-chain crate's host
test target. Generate arbitrary `u64` tax and net values, derive the wrapping
gross value, and assert that validation succeeds if and only if the checked
sum is the same non-zero gross value. Keep the production validator unchanged;
the property test exercises the exact private validation function compiled
from the on-chain program source.

The test is paired with validator-backed tests for actual CPI execution,
legacy SPL, extension-free Token-2022, duplicate nonce behavior, and
unsupported extensions. Neither test class substitutes for the other.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Hand-picked boundary examples only | Rejected as sufficient coverage | Useful regression cases, but cannot explore the overflow surface of two arbitrary `u64` operands. |
| Fuzz the full instruction through a validator only | Rejected as the only fast gate | Strong runtime evidence but too slow and environment-dependent for every host test run. |
| Use a custom pseudo-random loop | Rejected | Weaker shrinking and failure reproduction than a maintained property-testing framework. |
| **Bounded `proptest` over arbitrary `u64` tax/net pairs plus live conformance** | **Selected** | Finds arithmetic edge cases quickly with reproducible shrinking while preserving a separate ledger-level proof obligation. |

## Invariants

- `gross > 0` is required.
- `tax + net` must be computed with checked arithmetic.
- The checked sum must equal the supplied gross amount.
- Overflow, zero gross, and any mismatch must fail before token movement.

## Evidence

`programs/settlement/src/lib.rs` contains the property and example tests. The
live CPI and finalized balance evidence remains tracked separately in
`docs/plans/production-readiness.md` and ADR-0065.
