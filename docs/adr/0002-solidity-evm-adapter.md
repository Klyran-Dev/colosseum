# ADR-0002: Isolate Solidity to an optional EVM adapter

- Status: Accepted
- Date: 2026-09-25

## Context

The requested technology set includes Solidity, but Solidity cannot implement the Solana settlement program. Adding a Solidity contract to the Solana hot path would create a second settlement authority and violate low-latency and correctness goals.

## Decision

Solidity is permitted only in a separate EVM adapter package for future interoperability, mirroring settlement intent/receipt semantics on an EVM chain. It has its own threat model, deployment keys, audit, CI, and release lifecycle. It is not required for Solana checkout and cannot hold authority over the Solana vault.

## Consequences

- The Solana product remains coherent and native.
- EVM support can be added without contaminating the Solana program.
- Cross-chain settlement is not atomic across chains and must be treated as asynchronous, with explicit bridge/custody risk.

