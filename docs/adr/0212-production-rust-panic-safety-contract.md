# ADR-0212: Production Rust panic-safety contract

- Status: Accepted
- Date: 2026-09-30
- Owners: Architecture / Rust Platform
- Scope: Rust request, settlement, tax, storage, indexer, worker, and verifier paths

## Context

The repository mandate is fail-closed behavior for request handling, money
movement, tax calculation, and reconciliation. A manual review or a text scan
can miss a newly introduced `unwrap`, `expect`, or explicit panic in a
production compilation unit. Test fixtures still need a concise way to assert
that a deliberately valid fixture was built, so the policy must distinguish
production code from test code without weakening production binaries and
libraries.

## Decision

Every Rust crate root applies the following lint contract for non-test
compilations:

```rust
#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]
```

The repository lint gate remains:

```text
cargo clippy --locked --workspace --all-targets -- -D warnings
```

This means production libraries, binaries, and the Solana program cannot
introduce these panic paths without an explicit, reviewable exception. Test
and benchmark code may use assertions for fixture construction, but it may
not be copied into production modules. Existing production code that parsed a
fixed RPC-fixture program ID with `expect` was converted to checked-in bytes;
the test suite proves the byte/string compatibility contract.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Review checklist only | Rejected | Human review is necessary but does not provide a repeatable regression gate. |
| Repository-wide text grep | Rejected | It produces false positives for tests and benchmarks and cannot understand Rust configuration. |
| Workspace-wide Clippy deny without configuration | Rejected | It rejects valid test fixture assertions and makes test-only setup indistinguishable from production code. |
| **Per-crate `cfg_attr(not(test), deny(...))` plus the existing workspace Clippy gate** | **Selected** | It enforces the safety contract at the production compilation boundary while retaining focused test assertions and a reproducible CI check. |

## Invariants and failure policy

- A production compilation must not contain `unwrap`, `expect`, or `panic!`
  unless a future ADR explicitly changes this contract.
- Error propagation, typed validation, or a bounded invariant-preserving
  constructor is required at request, settlement, tax, storage, indexer, and
  reconciliation boundaries.
- Test-only exceptions remain outside production compilation and must not be
  used to justify a runtime fallback.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` is a merge
  gate; a failure blocks delivery.
- This contract does not replace property tests, fuzzing, threat review,
  timeout/backpressure tests, or operational failure drills.

## Consequences

New workspace crates must include the same crate-root lint contract before
they are considered production code. The rule catches an important class of
low-latency safety regressions at compile time with no runtime overhead. It
also makes intentional test-only assertions explicit and keeps the review
surface small. `unsafe`, unchecked external input, silent error suppression,
and dependency-level panics require their separate security and architecture
reviews.
