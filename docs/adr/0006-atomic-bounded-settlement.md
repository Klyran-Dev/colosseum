# ADR-0006: Settle tax and splits atomically with bounded fan-out

- Status: Accepted
- Date: 2026-09-25

## Context

The product value depends on payment, withholding, and net distribution not being independently mutable. Solana transactions are atomic, but account list, compute, and transaction-size limits require bounded fan-out.

## Decision

A single `settle` instruction validates policy, computes amounts, transfers tax to the vault, transfers net amounts to recipients, writes a receipt, and emits events. Split plans are versioned and have a maximum recipient count chosen from measured compute and message-size budgets. Larger plans require a separate batching design and are not silently truncated.

## Consequences

- There is no partial successful split.
- Recipient count and account preparation are product constraints.
- v1 and v0+ALT transaction compilation must be tested and observable.

