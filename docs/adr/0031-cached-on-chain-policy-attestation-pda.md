# ADR-0031: Require a cached policy-attestation PDA for settlement

- Status: Accepted for the first governed settlement boundary
- Date: 2026-09-25
- Owners: Protocol Security / Settlement Infrastructure
- Scope: Solana settlement program policy authority

## Context

The first settlement instruction checked amount conservation and token-account authority but accepted a non-zero policy digest supplied by the caller. That proves token movement, but it does not prove that the digest, merchant, asset, and exact amounts were approved by a governed issuer.

The settlement instruction must remain bounded. A global tax rule graph and remote identity or tax service cannot execute inside Solana's critical path.

## Decision

Add a `PolicyAttestation` PDA registered by an issuer signer and keyed by merchant, mint, and monotonically supplied policy sequence. The account stores:

- merchant and mint binding;
- issuer public key;
- non-zero policy digest;
- sequence;
- exact gross, tax, and net atomic amounts;
- expiry timestamp;
- revocation state.

The settlement instruction requires the PDA and rejects mismatched seeds, merchant, mint, digest, sequence, exact amounts, revoked state, or expired state. It still performs checked token transfers and receipt creation atomically. Registering or revoking an attestation is separate from payer settlement and remains governed by issuer-key policy; the first slice does not treat an arbitrary issuer signer as a production governance registry.

This is the cached O(1) verification path. A future fresh-attestation path may verify an Ed25519 instruction directly through the instructions sysvar, but only after issuer registry, compute, transaction-size, and replay budgets are measured.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`: API-only verification and on-chain rule-graph evaluation were rejected; fresh Ed25519 verification remains a later option.

## Invariants

- Settlement cannot use an unregistered, revoked, expired, or merchant/mint-mismatched policy.
- The policy account's exact amounts must satisfy `gross = tax + net` and match settlement arguments.
- A policy digest is not an amount authority by itself; the account binds the full amount tuple.
- Registration requires a signer and a positive sequence, expiry, and conserving amounts.
- Policy expiry is checked against the Solana clock, not API wall-clock state.
- Receipt nonce replay protection remains independent and deterministic.

## Consequences

Positive:

- Solana independently enforces the policy-to-settlement binding in bounded account reads.
- Revocation and expiry are explicit and auditable.
- The hot path avoids remote calls and unbounded tax computation.

Costs:

- Policy registration is an additional transaction and governance workflow.
- The first slice still needs an on-chain issuer registry/multisig integration before production deployment.
- A fresh signature-verification path is not yet implemented.
