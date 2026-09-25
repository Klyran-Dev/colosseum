# ADR-0020: Use versioned contracts with explicit compatibility and deprecation

- Status: Accepted
- Date: 2026-09-25

## Context

The platform has Solana IDLs, tax-rule schemas, calculation attestations, API responses, database projections, and frontend contracts. Uncoordinated changes can corrupt evidence or strand clients during a financial operation.

## Decision

Version every externally meaningful contract. Define backward/forward compatibility, migration, deprecation, sunset, and rollback behavior. Maintain canonical serialization vectors and contract tests across Rust, TypeScript, on-chain IDLs, and stored evidence. Never silently reinterpret a historical rule or calculation.

## Consequences

- Schema changes require more design and test work.
- Historical calculations remain replayable.
- Clients receive explicit migration paths instead of accidental breakage.

