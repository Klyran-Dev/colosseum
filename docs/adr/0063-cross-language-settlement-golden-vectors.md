# ADR-0063: Cross-language settlement golden vectors

- Status: Accepted
- Date: 2026-09-26
- Deciders: Principal Low-Latency Blockchain Architecture
- Scope: Rust settlement compiler and React wallet transaction boundary

## Context

The Rust settlement compiler is the authoritative producer of the program ID,
instruction data, PDA values, account order, signer flags, and writable flags.
The browser must reconstruct those exact values in order to request a wallet
signature, but JavaScript and Rust have separate serializers and public-key
implementations. Unit tests in either language alone cannot prove that the
wallet boundary preserves the reviewed settlement artifact.

An account-order or byte-encoding drift can produce a rejected transaction or,
worse, a transaction that remains structurally valid but targets an unintended
account. A client-provided digest is not evidence of equivalence.

## Decision

Maintain versioned, checked-in JSON golden vectors under `test-vectors/`.
Each vector contains:

- the canonical compiler request, including decimal-string atomic amounts;
- the complete expected compiled artifact;
- the exact instruction data encoding;
- every account meta in canonical order, including signer and writable flags;
- all derived PDAs and the fixed System Program and token-program accounts.

Rust tests deserialize the vector and assert that the authoritative compiler
produces the complete expected artifact. TypeScript tests consume the same
vector and assert that the browser transaction builder emits the identical
program ID, instruction bytes, account order, signer flags, and writable flags.

Vectors are schema-versioned and immutable once published. A compiler or
wallet serialization change requires a new vector version plus an ADR or an
explicit superseding decision. Vectors contain synthetic public keys and
amounts only; they never contain private keys, signatures, live RPC responses,
or production tax evidence.

## Alternatives considered

### Rust-only compiler tests

Rejected. They do not exercise the browser serializer or prove that wallet
transaction construction preserves the Rust artifact.

### TypeScript-only transaction tests

Rejected. The browser would become an accidental authority for instruction
bytes and account derivation without proving equivalence to the Rust compiler.

### Runtime Rust/WASM compiler shared with the browser

Rejected for the current boundary. It would reduce duplication but would add a
larger browser bundle, complicate wallet integration, and blur the service
authority boundary. Reconsider only with measured bundle and latency evidence.

### Snapshot-only tests generated independently in each language

Rejected. Independently generated snapshots can agree with each implementation
while both drift from the protocol contract.

## Invariants

1. Rust and TypeScript consume the same versioned vector.
2. The complete compiled artifact must match, not only its instruction bytes.
3. Any account substitution, order change, signer change, writable change, PDA
   change, or program-ID change fails the relevant test.
4. Decimal-string financial values remain lossless across the vector boundary.
5. A vector never authorizes a transaction or implies that the transaction was
   signed, submitted, confirmed, or finalized.

## Consequences

The wallet boundary gains an executable compatibility contract and reviewable
regression evidence. The repository carries a small amount of synthetic test
data, and protocol changes require deliberate vector versioning. Golden vectors
do not replace signed-message verification at the API submission boundary,
localnet execution tests, or independent program audits.
