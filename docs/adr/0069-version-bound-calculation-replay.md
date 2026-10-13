# ADR-0069: Make calculation replay version-bound and fail closed

- Status: Accepted
- Date: 2026-09-26
- Scope: tax-engine calculation results and evidence replay

## Context

Tax results are durable evidence, not transient UI values. A stored result
already binds normalized facts and the canonical rule digest, but a future
engine implementation could produce different semantics while preserving both
digests. Replaying such evidence without an explicit engine-version contract
could silently reinterpret a historical calculation.

## Decision

Add the immutable `engine_version` to `CalculationResult`. New calculations
always write the current `ENGINE_VERSION`; pre-versioned evidence deserializes
with an empty version marker and therefore fails replay closed. Add a domain
`verify_replay` operation that:

1. rejects a result produced by a different engine version;
2. recalculates from the supplied rule set and normalized facts; and
3. compares the complete result, including component trace, digests, totals,
   explanations, and engine version, returning a typed replay mismatch on any
   difference.

The operation is deterministic and has no clock, database, network, or RPC
dependency. A future engine migration must provide an explicit compatibility
adapter or a new replay policy; it may not silently reuse this check across
semantic versions.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Compare totals only | Rejected | A changed base, rounding, component order, or explanation could be hidden while totals happen to match. |
| Compare facts and rule digests only | Rejected | Does not prove the engine interpreted the same immutable inputs identically. |
| Always replay with the latest engine | Rejected | Can silently reinterpret historical evidence after a semantic change. |
| Store an opaque result without a replay operation | Rejected | Prevents automated evidence verification and makes operators rely on ad hoc scripts. |
| **Version-bound full-result replay in the Rust domain** | **Selected** | Deterministic, typed, complete, low-latency, and reusable by API, storage, indexer, and audit tooling. |

## Consequences

- Result JSON gains a versioned field; old local documents can deserialize with
  an empty legacy marker but cannot pass replay unless they are explicitly
  migrated under a reviewed compatibility policy.
- Engine upgrades require migration/replay policy review and new vectors where
  semantics change.
- Replay mismatch is an evidence-integrity failure, never a best-effort warning.

## Verification

Tests must cover an exact replay, changed facts, changed rules, tampered trace,
and engine-version mismatch. The persisted quote and attestation evidence
remain tenant-scoped and must retain their existing digest bindings.
