# ADR-0034: Govern policy attestation issuers with merchant/mint-scoped registry PDAs

- Status: Accepted for the first governed issuer boundary
- Date: 2026-09-25
- Owners: Protocol / Security
- Scope: policy attestation registration and revocation

## Context

The first cached `PolicyAttestation` slice binds merchant, mint, amounts,
sequence, digest, expiry, and revocation, but it accepts the signer supplied to
the registration instruction as the issuer. That is insufficient for real
funds: a compromised or accidental caller could register a valid-looking
attestation without an independently governed issuer authority.

The issuer key and the authority that changes the issuer assignment are
different trust roles. The settlement instruction must remain O(1) and must not
evaluate a global tax graph or contact a remote registry.

## Decision

Add a `PolicyIssuerRegistry` PDA scoped by `[b"policy-issuer-registry",
merchant, mint]` with:

- an explicit governance `authority`;
- one active Ed25519 `issuer` public key;
- an `active` flag and PDA bump;
- immutable merchant and mint scope.

The registry lifecycle is:

1. A governance authority initializes the merchant/mint registry with an
   issuer key.
2. Only the registry authority can rotate the issuer or deactivate the
   registry.
3. Only the currently active issuer signer can register a policy attestation
   through the matching registry PDA.
4. Only the registry authority can revoke an attestation in that scope.
5. Existing attestations retain their recorded issuer and remain independently
   bound by digest, amount, expiry, and revocation checks; issuer rotation does
   not silently invalidate historical evidence.

The registry is an authorization boundary, not a tax authority. Production
deployment must back the authority with the configured multisig/timelock and
key ceremony described by the security and operations gates.

## Invariants

- Registry merchant and mint equal the PDA seed scope.
- Registration fails when the registry is inactive or the signer is not the
  active issuer.
- Rotation and deactivation fail unless the registry authority signs.
- Revocation fails unless the registry authority signs and the attestation is
  in the same merchant/mint scope.
- Settlement still verifies the cached attestation's exact binding and does
  not trust the registry as a replacement for policy evidence.

## Alternatives considered

See `docs/architecture/alternative-analysis.md` under “On-chain issuer
governance”. Arbitrary per-call signers were rejected because they do not
provide a durable authority boundary; a global registry was rejected for
contention and blast-radius reasons.

## Consequences

Positive:

- Policy registration has a durable, auditable, merchant/mint-scoped trust
  boundary.
- Governance and issuer responsibilities are separated.
- Issuer rotation and emergency deactivation are bounded state changes rather
  than code changes or undocumented operational assumptions.
- Settlement compute remains constant and does not add global lookups.

Costs:

- Registry initialization and rotation are additional governed transactions.
- The API/compiler must resolve and include the registry account when issuing
  attestations.
- Multisig/timelock integration, Token-2022 support, and fresh Ed25519
  instruction verification remain separate release gates.
