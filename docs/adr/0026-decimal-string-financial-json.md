# ADR-0026: Represent financial JSON fields as decimal strings

- Status: Accepted
- Date: 2026-09-25

## Context

Solana atomic amounts are bounded by `u64`, while the tax engine uses exact
`i128` intermediates and rational rates. JavaScript numbers lose integer
precision above `2^53 - 1`. Emitting financial values as JSON numbers would
allow a browser or integration to display or resubmit a different amount than
the engine calculated.

## Decision

All public JSON atomic amounts, rational numerators/denominators, and
attestation `u64` amount/sequence fields are decimal strings. The Rust domain
parses and validates them at the boundary, and the React application formats
amounts with `BigInt`. Canonical signed payload bytes remain fixed-width
little-endian integers and are independent of JSON representation.

## Alternatives considered

- **JSON numbers:** rejected because JavaScript precision is insufficient for
  the full Solana amount range.
- **Floating-point or decimal JSON values:** rejected because they introduce
  rounding and language-runtime differences into financial inputs.
- **Base-10 strings only in the frontend:** rejected because every public
  integration boundary needs the same lossless contract.
- **Encode all API messages as binary:** deferred; binary transport may be
  added for the measured low-latency hot path, but it must preserve the same
  versioned semantic schema and golden vectors.

## Consequences

- API examples and generated clients must preserve strings.
- Human-readable formatting is a presentation concern and must not be parsed
  back from localized text.
- A future schema version may add a typed decimal object, but may not silently
  revert financial fields to JSON numbers.
