# ADR-0224: Cross-language calculation-attestation golden vector

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Policy, Web, Security
- Bounded contexts: Policy Attestation, Wallet Checkout, Tax Engine

## Context

The Rust `policy` crate owns the canonical issuer-signed calculation-
attestation payload. The API verifies that payload and the Solana settlement
boundary consumes its digest, while the React application parses and presents
the server-owned evidence before wallet signing. Rust unit tests prove the
canonical bytes in isolation, but the repository did not have a shared policy
vector consumed by the TypeScript boundary. A future change to length prefixes,
integer endianness, text encoding, field order, or domain separation could
therefore pass language-local tests while changing the authority-bound digest.

## Decision

Add a versioned `calculation-attestation-v1` vector containing the unsigned
fields, canonical payload bytes, payload SHA-256 digest, issuer public key, and
signature. The Rust policy tests and a browser-compatible TypeScript test
consume the same fixture.

The canonical payload is domain-separated and ordered as follows:

1. UTF-8 domain separator `solana-tax-protocol/calculation-attestation/v1`;
2. little-endian `u16` schema version;
3. 32-byte merchant identity digest;
4. each bounded text field as little-endian `u16` byte length plus UTF-8 bytes,
   in rule-set, jurisdiction, currency order;
5. little-endian `u32` rule version;
6. asset mint, facts digest, and rule digest;
7. little-endian `u64` gross, tax, net, and sequence values;
8. little-endian signed `i64` issue and expiry timestamps; and
9. issuer key identifier.

The browser helper is a compatibility/parser primitive only. It does not
verify issuer authority, publish policy, create an attestation, or authorize
settlement; those remain server and on-chain responsibilities.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Rust-only canonical-payload tests | Rejected | Cannot detect a TypeScript serialization drift at the wallet boundary. |
| Compare JSON serialization | Rejected | JSON property ordering, number representation, and encoding are not the signed byte contract. |
| Browser-generated or browser-signed attestations | Rejected | Violates issuer authority separation and would turn an untrusted client into a policy authority. |
| **One versioned binary vector consumed by Rust and TypeScript** | **Selected** | Makes the cross-language digest contract executable, reviewable, deterministic, and independent of network state. |

## Invariants and failure behavior

- The Rust and TypeScript payload bytes and SHA-256 digest must match exactly.
- All fixed-width integers use the declared little-endian representation.
- Text lengths are UTF-8 byte lengths, not JavaScript UTF-16 code-unit lengths.
- Atomic amounts and sequence values are decimal strings and never pass through
  JavaScript `Number` conversion.
- Any malformed field, wrong digest length, non-canonical unsigned integer, or
  oversized text value fails before payload construction.
- A vector or compatibility-test failure blocks the change; it cannot be
  downgraded to a skipped browser test.

## Verification and release impact

- Add Rust fixture-consumption and TypeScript fixture-consumption tests.
- Add mutation cases for field order, text length, integer endianness, and
  digest tampering.
- Keep issuer key rotation, provider identity, legal approval, and finalized
  chain evidence as independent release gates.
