# Optional EVM adapter

The repository includes one deliberately narrow Solidity surface: a non-custodial projection
of finalized Solana settlement evidence. It is not a bridge, token
wrapper, tax authority, payment router, or source of funds truth.

`SolanaSettlementReceiptMirror` accepts a bounded receipt only from a governed
issuer. Each receipt commits to a source digest, merchant, mint, Solana slot,
and conserving `gross = tax + net` atomic amounts. Identical retries are
idempotent; a conflicting retry fails closed. Governance uses a two-step
transfer, and the issuer can be rotated or paused independently.

The contract has no token interfaces, payable methods, external calls, arbitrary
rule execution, or Solana signing authority. Solana finalized evidence remains
authoritative. Production deployment requires a separate EVM threat model,
independent audit, chain-specific deployment runbook, multisig, monitoring, and
legal approval; passing the local Foundry tests does not satisfy those gates.

Run `make evm-adapter-contract` for the static boundary contract and
`make evm-adapter-check` for the pinned Linux/amd64 Docker build and Foundry
test suite. The Docker build pins both the Foundry image and the official
Solidity 0.8.37 Linux compiler artifact by SHA-256.

See [ADR-0378](../adr/0378-non-custodial-solidity-receipt-mirror.md) for the
alternatives and authority decision.
