# ADR-0141: Validator-backed policy revocation conformance

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Solana Protocol, Security
- Scope: Policy issuer registry, attestation revocation, and settlement gate

## Context

The settlement program has a governed merchant/mint policy issuer registry and
stores each policy attestation in a deterministic PDA. Unit tests cover issuer
scope, revocation authority, expiry, sequence, and binding checks, but they do
not prove that a revoked attestation blocks the token CPI path in the deployed
SBF artifact or that balances remain unchanged after rejection.

## Decision

Extend the pinned external-validator conformance suite with a fresh legacy-SPL
mint. The test will initialize the registry and attestation, revoke the
attestation using the registry authority, then attempt settlement with the
exact bound amounts and policy digest. It must fail at the policy gate and
finalized source, vault, and destination balances must remain unchanged.

The test uses the real PDA/account topology and transaction serializer. It does
not forge a revoked account response or treat API state as authoritative.

## Alternatives considered

### Unit-test revocation helpers only

Rejected. Helper tests cannot prove the deployed instruction reads the revoked
PDA before CPI execution or that the transaction leaves token balances intact.

### Revoke through a synthetic RPC fixture

Rejected. A fixture can model a status response but cannot execute the token
program CPI or prove atomic rollback on the ledger.

### Selected: execute revoke and settlement against the pinned validator

Selected because it exercises the governed authority, deterministic account
binding, deployed SBF, real token accounts, and the exact failure boundary in a
bounded reproducible test.

## Invariants and failure behavior

- Only the registry authority can revoke the scoped policy attestation.
- A revoked attestation cannot authorize any settlement CPI.
- Rejected settlement leaves source, tax-vault, and destination balances
  unchanged.
- Missing validator, RPC readiness, or any failed assertion fails the
  conformance command and is retained as failure evidence.

## Evidence and remaining gates

The existing conformance harness retains the test output, validator logs,
Compose configuration, program hash, and runtime metadata. A successful Linux
CI run is required before marking the production-readiness revocation gate
green. Program audit, key ceremonies, target-cluster performance, staging
governance, and production approval remain separate gates.
