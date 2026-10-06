# ADR-0382: Tax Engine v2 golden-vector coverage manifest

## Status

Accepted — the manifest and cross-language vector extension are required
regression contracts; legal/jurisdiction ownership remains separate.

## Context

The Tier-1 Tax Engine already has versioned JSON vectors for the major global
tax semantics and a separate v2 envelope for bounded linear tax-base
operations. The loader currently checks the v2 vector count, but a vector can
be replaced or removed while preserving that count. A count is therefore not
evidence that the published capability matrix is still represented in the
cross-language contract.

The v2 contract also needs one vector that combines a bounded weighted base
term with an explicit tax-on-tax dependency. This catches drift in signed
linear-base decoding, dependency ordering, and exact component conservation
at the same serialization boundary.

## Decision

Maintain an explicit required-ID manifest in the Rust golden-vector test. The
manifest names the v1 scenario families and the v2 linear-base scenarios. The
test rejects missing, duplicate, or unexpected schema IDs and still compares
every vector's exact aggregate and component results.

Add `weighted-component-cascade` to `test-vectors/tax-engine-v2.json`. Its
second component includes a signed unit-weight prior component in its base;
the vector must produce the same exact result as the declared dependency
graph, without introducing a second implementation or floating-point value.

The manifest is a test contract, not a legal coverage claim. A new semantic
family requires a reviewed vector, matrix update, and—when the wire or
canonical representation changes—a new schema or ADR.

## Alternatives considered

| Alternative | Reviewability | Drift detection | Cross-language value | Decision |
| --- | --- | --- | --- | --- |
| Keep only a vector count | Low | Weak; replacement is invisible | Partial | Rejected |
| Infer coverage from documentation text | Low | Weak; prose can drift | None | Rejected |
| Generate expected values in a second runtime | Partial | Adds arithmetic divergence | Risky | Rejected |
| **Explicit required IDs plus exact JSON execution** | **High** | **Detects removal, duplication, and replacement** | **Portable** | **Selected** |

## Invariants and failure policy

- Every required vector ID is present exactly once in its declared schema.
- A duplicate or unknown vector ID fails the test rather than being ignored.
- Expected atomic values remain canonical decimal strings; no JavaScript-safe
  integer conversion is introduced.
- The weighted component vector must compile and calculate with exact base,
  tax, net, and dependency results.
- Vector evidence does not publish a jurisdiction or authorize settlement;
  provenance, effective windows, tax-owner review, attestation, and chain
  evidence remain mandatory.

## Verification

- `cargo test --locked -p tax-engine executes_the_versioned_global_tax_golden_vectors`
  validates the v1 manifest and exact outputs.
- `cargo test --locked -p tax-engine executes_the_v2_linear_tax_base_golden_vectors`
  validates the v2 manifest and exact outputs.
- Workspace tests, Clippy, and public-document checks remain required after
  vector changes.
