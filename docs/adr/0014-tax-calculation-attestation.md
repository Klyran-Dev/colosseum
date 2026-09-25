# ADR-0014: Attest off-chain tax calculations for on-chain settlement

- Status: Accepted
- Date: 2026-09-25

## Context

Executing a full global tax-rule graph on Solana would increase compute, account locks, transaction size, and upgrade risk. The chain still needs to prevent an API or browser from changing tax amounts.

## Decision

The Tax Engine emits a signed CalculationAttestation containing engine version, rule digest, normalized-facts digest, jurisdiction result, component totals, asset, merchant, amount bounds, expiry, and evidence references. The Solana program verifies the attestation or a registered digest and recomputes/validates conservation and configured bounds before moving funds.

## Consequences

- The hot path remains low latency and bounded.
- Attestation keys, expiry, replay protection, and evidence availability are security-critical.
- The chain verifies the agreed calculation and settlement constraints; it does not prove that external legal facts were correct.

