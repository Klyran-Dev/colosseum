# ADR-0378: Non-custodial Solidity receipt mirror

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal Low-latency Blockchain Architecture
- Scope: Optional EVM adapter; Solana settlement authority remains unchanged

## Context

The approved technology matrix includes Solidity, but the repository previously
contained only the architectural decision to defer an EVM adapter. That left
the optional boundary undocumented in code and without an executable
conformance contract.

An EVM implementation must not become a second authority for Solana funds,
hold assets, recreate tax arithmetic, or imply atomic cross-chain settlement.
The useful bounded capability is a non-custodial mirror of an already-finalized
Solana receipt for EVM-side indexing, reconciliation, or display.

## Decision

Add `evm-adapter/src/SolanaSettlementReceiptMirror.sol`, compiled with pinned
Solidity 0.8.37. The contract:

- stores only a bounded receipt projection keyed by an immutable receipt ID;
- accepts records only from a separately governed issuer;
- requires non-zero source digest, merchant, mint, and receipt identity;
- enforces `gross = tax + net` with checked arithmetic and `tax <= gross`;
- makes identical re-submission idempotent without emitting a duplicate event;
- rejects conflicting reuse of an identity;
- supports a two-step governance transfer, issuer rotation, and emergency pause;
- records the source domain supplied at deployment for explicit cross-chain
  context; and
- never transfers ERC-20/native assets, calls another contract, or authorizes a
  Solana transaction.

The adapter has its own deployment authority, threat model, audit requirement,
compiler/test lifecycle, and release artifact. It is not included in the
Solana checkout hot path. Solana finalized evidence remains authoritative; an
EVM record is a projection and may be absent or delayed.

## Alternatives considered

1. Leave Solidity as documentation only. Rejected: the selected technology
   boundary would have no executable compatibility evidence.
2. Recreate tax calculation or vault movement on EVM. Rejected: creates a
   second financial authority, cross-chain divergence, custody risk, and a
   materially larger audit surface.
3. Use an ERC-20 escrow/bridge adapter. Rejected: custody and bridge risk are
   outside the Solana-first launch scope.
4. **Use a minimal governed non-custodial receipt mirror with exact
   conservation and idempotency.** Selected: provides useful EVM
   interoperability while preserving authority and latency boundaries.

## Invariants and verification

- The contract has no payable functions, token interfaces, external calls, or
  arbitrary loops.
- Only the current issuer can record; only governance can pause, rotate the
  issuer, or transfer governance.
- Receipt identity reuse is either an exact idempotent replay or a fail-closed
  conflict.
- Stored amounts conserve exactly in unsigned 64-bit values.
- Paused or zero/invalid records cannot enter storage.
- Foundry example and fuzz tests cover authorization, conservation across the
  full bounded `uint64` input space, idempotency, conflict, pause, issuer
  rotation, and two-step governance.
- A pinned Docker Foundry image runs the tests; an independent EVM audit and
  chain-specific deployment review remain mandatory before use with real data.

## Consequences

The repository now provides an executable Solidity boundary without confusing
EVM projections with Solana settlement. It adds a small separately audited
package and Docker test image. It does not add EVM production support, bridge
support, or a claim of cross-chain atomicity.
