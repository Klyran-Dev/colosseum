# ADR-0004: Use signed, versioned policy attestations

- Status: Accepted
- Date: 2026-09-25

## Context

Tax parameters depend on jurisdiction, product, customer facts, thresholds, and time. A Solana program cannot query arbitrary external tax law or a web API during settlement. A browser-supplied percentage is not trustworthy.

## Decision

The protocol accepts only a canonical, signed policy attestation bound to merchant, product tax code, jurisdiction, asset, sequence, and validity window. The policy issuer set is registered and governed. The program enforces bounds and freshness. The fast path references a registry PDA; the fresh path uses a checked Ed25519 verification instruction and the instructions sysvar.

## Consequences

- Tax data provenance is explicit and auditable.
- Issuer governance and key compromise become first-class operational risks.
- The system automates a policy supplied by an authorized issuer; it does not make a legal determination or guarantee compliance.

