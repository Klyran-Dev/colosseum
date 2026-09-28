# ADR-0116: Deterministic settlement wire budget

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture and Settlement
- Scope: settlement instruction compilation and signed-message admission

## Context

Solana packet size is a hard admission boundary, but an absolute 1,232-byte
parser limit is not a production budget. The settlement compiler must prove
that its largest supported legacy transaction has room for serialization
variation, future metadata, and operational safety. The signed-message
verifier must also prevent unreferenced static keys from inflating an otherwise
valid transaction.

Compute-unit limits and v0 address-table lookups require target-cluster and
validator evidence. They must not be invented from a byte estimate.

## Decision

Add a pure, dependency-light legacy wire-size estimator to
`settlement-client` using Solana short-vector lengths and the exact bounded
settlement account/data shape. Enforce:

- absolute packet ceiling: 1,232 bytes;
- settlement admission ceiling: 976 bytes;
- minimum safety margin: 256 bytes;
- maximum split fan-out: the existing eight recipients;
- maximum legacy settlement instruction account count: 20;
- no address-table lookups in the legacy wire profile and no extra static
  account keys at the signed-message boundary; the separately governed v0+ALT
  profile is defined by ADR-0161.

The compiler rejects a future shape that exceeds the admission ceiling. The
coordinator compares the signed message's static-key count to the exact
compiled account domain before provider submission.

This is a conservative serialized-size contract, not a compute-unit claim.
Compute-unit limits and priority-fee policy are governed by ADR-0162; v1 and
v0/ALT target-cluster evidence remain separate release gates.

## Alternatives considered

### Enforce only the Solana packet maximum

Rejected. It catches malformed oversized input but leaves no safety margin and
does not constrain compiler growth or unused static keys.

### Estimate JSON/base64 response size

Rejected. JSON/base64 is an API envelope, not the serialized wire transaction
admitted by the validator.

### Query a live RPC during compilation

Rejected. It adds hot-path latency and provider dependency to a deterministic
compiler without proving future cluster behavior.

### Permit arbitrary static keys for wallet compatibility

Rejected. Unreferenced keys increase packet size and review surface without
changing the compiled settlement authority.

### **Exact bounded legacy wire estimate plus static-key admission check**

Selected. It is deterministic, testable without a network, and leaves
compute/ALT decisions to the evidence required for those features.

## Invariants

- Every compiler-produced legacy settlement is at least 256 bytes below the
  1,232-byte packet ceiling under the declared maximum shape.
- Account fan-out and instruction data lengths are bounded before serialization.
- A signed message cannot contain static keys that are not part of the exact
  compiled settlement domain.
- The byte budget never authorizes a transaction, changes account ownership,
  or substitutes for validator execution evidence.

## Verification and release impact

- Unit and property tests cover compact-length boundaries and maximum split
  fan-out wire estimates.
- Signed-message tests reject extra static keys and continue to reject account,
  writable, signer, program, and data substitutions.
- Compute-unit margin, priority-fee policy, v1, and v0+ALT paths remain open
  target-cluster release gates.
