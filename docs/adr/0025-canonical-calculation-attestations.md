# ADR-0025: Use a canonical signed calculation-attestation envelope

- Status: Accepted
- Date: 2026-09-25

## Context

The tax engine result cannot be trusted by the settlement program merely
because an API returned a tax amount. The result must bind to the merchant,
asset, jurisdiction, rule version, normalized facts, amount conservation,
issuer identity, validity window, and monotonic issuer sequence. The same
bytes must be verifiable off-chain and at the Solana boundary.

## Decision

Create a policy bounded context with a pure Rust canonical contract. The
attestation payload uses an explicit domain separator, schema version,
length-prefixed UTF-8 text, fixed-width little-endian integers, fixed 32-byte
digests, and bounded fields. The payload is SHA-256 hashed and signed with
Ed25519 by a separately governed issuer. Verification requires:

- schema, text, currency, identity, digest, sequence, and amount validation;
- `tax + net = gross` conservation;
- a non-negative validity window no longer than 24 hours;
- payload digest equality and Ed25519 signature validity;
- explicit current-time validation supplied by the adapter; and
- binding equality against the requested calculation.

The API exposes verification only as `verified_unsettled`. It does not mint
authority, submit transactions, custody keys, or imply that verification is
legal compliance. Issuer signing remains a separate service boundary; this
repository does not put issuer private keys in the API or web application.

## Alternatives considered

- **Sign serialized JSON:** rejected because field ordering, number encoding,
  and cross-language canonicalization would be ambiguous.
- **Trust the API response without a signature:** rejected because the API is
  an integration and projection layer, not the policy authority.
- **Verify only the tax amount:** rejected because it permits replay or
  substitution across merchants, assets, rules, facts, and validity windows.
- **Put the full rule graph on-chain:** rejected because global tax rules are
  too variable and unbounded for the settlement hot path.

## Consequences

- The TypeScript client, issuer, API, and Solana verifier must use the same
  versioned encoding and golden vectors.
- A schema change requires a new schema version and compatibility ADR; it may
  not silently change signed bytes.
- Issuer key rotation, sequence monotonicity, revocation, and on-chain
  instruction-sysvar verification remain mandatory follow-on work.
