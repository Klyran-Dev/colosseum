# ADR-0005: Restrict the settlement hot path to approved stablecoins

- Status: Accepted
- Date: 2026-09-25

## Context

Tax reserves denominated in a volatile asset can change value before filing or payout. Converting in the same transaction introduces oracle, liquidity, slippage, and extra account-lock risk.

## Decision

V1 accepts only an asset registry of approved stablecoin mints, with explicit token program and extension policy. Non-stable assets and DEX conversion are separate adapters with max-slippage and stablecoin-output checks.

## Consequences

- Tax reserve accounting is easier to explain and reconcile.
- The initial product has narrower asset coverage.
- Stablecoin issuer, depeg, freeze, redemption, and off-ramp risk remain external risks and are documented rather than hidden.

