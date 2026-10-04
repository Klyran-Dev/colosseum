# ADR-0062: Finalized chain verification before settlement-route activation

- Status: Accepted for implementation
- Date: 2026-09-26
- Deciders: Principal Low-Latency Blockchain Architecture
- Scope: Settlement Route Registry provisioning and activation

## Context

ADR-0061 removed caller-controlled merchant account fields from the settlement
compiler, but an off-chain route row is still only configuration. A malformed,
deleted, substituted, or extension-bearing Solana account must not become an
active checkout route merely because an operator wrote a database row. The
checkout path must remain a bounded database read and must not perform RPC
discovery, so verification belongs in a separate provisioning workflow.

## Decision

Add a dedicated route-verifier service. It polls a bounded number of
`pending_verification` and explicitly re-verifiable `suspended` routes, queries
the configured RPC providers with `getMultipleAccounts` at `finalized`
commitment, and promotes a route only after all checks pass.

The verifier requires:

- the merchant account to exist, be non-executable, be owned by the System
  Program, and contain no data, matching the on-chain `SystemAccount` boundary;
- the mint and both configured token accounts to exist, be non-executable, be
  owned by the selected SPL Token or Token-2022 program, and use the exact
  extension-free layouts accepted by the settlement program (82-byte mint and
  165-byte token accounts);
- both token accounts to encode the configured mint;
- the tax-vault token account owner to equal the deterministic
  `vault-authority` PDA for the configured merchant and mint;
- the mint-scoped `AssetConfig` PDA to be owned by the settlement program,
  initialized with the exact mint, token program, decimals, stablecoin kind,
  enabled state, discriminator, and derived bump;
- an RPC context slot and exact account response cardinality, with bounded
  response bytes and no null account values.

The verifier records a domain-separated evidence digest, finalized slot,
commitment, account metadata/data digests, and mint decimals in an append-only
verification table. The route row is activated in the same serializable
transaction that records the evidence. A route correction creates a new
version. A suspended route may be re-verified; an active or retired route's
account configuration and verification evidence cannot be mutated in place.

The service uses a bounded provider pool and per-attempt deadlines. Provider
disagreement or uncertainty fails closed; it never activates using a weaker
commitment or a partial response. The verifier is not on the quote, compile,
wallet-signing, or settlement-submit critical path.

Local synthetic fixtures may use an explicitly marked `local_synthetic`
verification record for Docker-only integration. Production activation accepts
only `finalized` evidence and requires an operator-authorized provisioning
workflow. No API route silently promotes a row or accepts a caller-supplied
verification digest.

## Alternatives considered

### Verify during quote or settlement compilation

Rejected. RPC latency, provider uncertainty, and account-data parsing would
pollute the low-latency hot path and create an unsafe retry/finality boundary.

### Trust database configuration without chain evidence

Rejected. PostgreSQL is a tenant-configuration projection, not authority for
Solana account existence, ownership, token layout, or vault authority.

### Verify with one RPC provider at processed/confirmed commitment

Rejected. It is vulnerable to provider faults and can activate against state
that is not final. The selected workflow uses bounded failover and finalized
commitment.

### Put route verification into the settlement program

Rejected. The program can and does validate accounts during settlement, but it
cannot govern an off-chain route lifecycle or provide a low-latency route
selection read without an RPC dependency.

## Invariants

1. Only a route with complete finalized verification evidence can be activated
   in a production environment.
2. The evidence digest binds route identity, account addresses, account owners,
   executable flags, bounded account-data digests, mint decimals, commitment,
   and observed slot.
3. The verified account order is fixed and the response must contain exactly
   the requested five accounts, including the asset-admission PDA.
4. Account substitution, null responses, unsupported owners, unsupported
   layouts, vault-authority mismatch, provider disagreement, and deadline
   expiry fail closed.
5. Activation and evidence insertion are atomic and idempotent for the same
   route/version/evidence digest.
6. Route verification never changes quote amounts, tax evidence, policy
   digests, or settlement instruction bytes.

## Consequences

Positive:

- Active routes have auditable, finalized chain evidence.
- Checkout remains one bounded tenant-scoped database read with no RPC call.
- Account ownership, token layout, mint binding, and vault authority are
  checked before operator activation.
- Provider outages do not silently create unsafe active configuration.

Trade-offs and release gates:

- Operators need a separate verifier deployment and an activation runbook.
- A route can remain pending during RPC degradation and must expose that state
  to operations.
- This verifies account topology and layout, not legal tax correctness,
  liquidity, issuer solvency, or governance authorization; those remain
  separate release gates.
