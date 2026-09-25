# ADR-0001: Use a Solana-native Rust/Anchor settlement core

- Status: Accepted
- Date: 2026-09-25

## Context

The product must execute tax withholding and revenue splitting atomically on Solana. Solana programs execute as sBPF programs, and the mature supported development path is Rust with Anchor. Solidity is not the native execution language for a Solana program.

## Decision

Implement the settlement, vault, policy, asset, split-plan, and receipt logic as Rust programs using Anchor-compatible interfaces. Use Solana CPIs to the appropriate token program. Keep the account model explicit and bounded.

## Consequences

- We get Solana account validation, PDA derivation, IDL generation, and native transaction semantics.
- Engineers must understand Rust ownership, account constraints, CPI rules, compute budgets, and token-program differences.
- The core cannot be shared directly with an EVM Solidity contract; shared behavior is expressed through versioned schemas and test vectors.

