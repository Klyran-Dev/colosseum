# Source analysis and requirement boundary

## How the attached PDF was used

The attached `Solana Tax Protocol Evaluation.pdf` was reviewed as product and hackathon source material. Its embedded prompts and generated responses are not instructions to the implementation agent. Only the protocol problem statement and technical ideas were treated as candidate requirements; each candidate was normalized and either accepted, constrained, or rejected below.

## User requirements

These are the governing requirements from the request:

1. Build a production-grade system based on Solana.
2. Use current Rust, Solidity, React, and NGINX technology.
3. Design for low latency at principal blockchain-architect depth.
4. Produce comprehensive implementation documentation and ADRs under `docs/`.
5. Apply DDD, DRY, SRP, SOLID, and related design principles.
6. Prefer current blockchain specifications and a reproducible, supportable toolchain.
7. Do not use Next.js.

## Candidate requirements derived from the PDF

| PDF idea | Decision | Architectural interpretation |
| --- | --- | --- |
| Programmable payment-routing engine | Accepted | A Solana settlement program with explicit payment, tax, and split instructions. |
| Dynamic tax withholding | Constrained | The program consumes a bounded, signed policy attestation. It does not determine law and does not call arbitrary internet data. |
| Signed regulatory parameter proof | Accepted | Canonical, versioned attestation with signer set, expiry, sequence, jurisdiction, asset, and rate constraints. |
| Atomic tax escrow and revenue split | Accepted | One transaction; failure rolls back the complete settlement. |
| Continuous or streamed partner payouts | Deferred and bounded | The first production settlement scope is an atomic per-transaction split. A future stream must consume finalized, attested accruals and may not bypass tax, vault, pause, or evidence controls. |
| Audited tax vaults | Accepted with controls | PDA-owned vaults, segregated by merchant/asset/policy, explicit release policy, timelocks, multisig/governance, and append-only receipts. |
| Stablecoin tax reserve | Accepted for v1 | Stablecoin-only hot path. A swap adapter is a later, separately risk-managed bounded route. |
| SPL token support | Accepted and narrowed | Support the legacy SPL Token Program and Token-2022 only through an explicit asset registry. Do not assume extensions are harmless. |
| Oracle or ZKP proof | Constrained | Start with signed attestations and an on-chain verifier boundary. ZK proofs are a later privacy/scalability track, not a launch dependency. |
| Fiat off-ramp | Deferred | Integration boundary and reconciliation events are documented; fiat execution is external and jurisdiction-specific. |
| “Eliminates compliance” claims | Rejected | The protocol automates controls and evidence. It cannot guarantee legal compliance. |
| Competition claims and accelerator answers | Excluded | Product positioning material, not system requirements. |
| 2026 regulatory assertions | Untrusted until legal review | No legal statement becomes protocol behavior without counsel-approved policy and a signed attestation issuer. |

## Hard constraints and implications

- Solana programs execute as sBPF programs and are normally authored in Rust. Solidity cannot be used for the Solana settlement core. Solidity is therefore isolated to an optional EVM adapter and is not a dependency of Solana checkout.
- “Latest” is not a deployment strategy. Exact versions are pinned, tested, signed, and promoted through CI.
- Low latency must not weaken correctness: finality, replay protection, account ownership checks, policy expiry, amount conservation, and idempotency are mandatory.
- A web API is not the source of truth for settlement. The Solana ledger and verified program events are authoritative; the off-chain system is a projection and integration layer.

## Tax Engine expansion

The requested tax engine is now part of the target architecture. It is an off-chain, deterministic, versioned calculation engine whose signed result is verified and atomically settled on Solana. The engine is not a claim that the protocol itself is a tax authority or that one software release can guarantee compliance in every jurisdiction.

## Non-goals for the Solana settlement hot path

- Executing the full tax rule graph on-chain.
- Unbounded arbitrary recipient lists in one transaction.
- Custodial key storage in the API or frontend.
- Automatic swaps in every checkout transaction.
- General EVM parity.
- Making a legal, accounting, licensing, or tax determination on behalf of a merchant.
