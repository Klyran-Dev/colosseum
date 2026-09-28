# ADR-0122: Versioned tax-rule document envelope

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Tax Domain, and SRE
- Scope: tax-rule persistence, publication, replay, and public rule contracts

## Context

The tax engine already validates an in-memory `RuleSet` and executes a
versioned golden-vector document, but the database publication boundary has
historically stored a bare `RuleSet` JSON object. A bare object does not state
which document contract produced it, weakens forward compatibility, and makes
schema migration or safe rejection of an incompatible rule representation
ambiguous. Published rules are financial evidence and must remain replayable
without allowing a future service to silently reinterpret them.

## Decision

Introduce the canonical `RuleSetDocument` envelope:

```json
{
  "schema": "solana-tax-protocol/tax-rule/v1",
  "rule_set": { "id": "…", "version": 1, "components": [] }
}
```

The Rust tax-engine validates the exact schema identifier, validates and
compiles the contained rule set, and preserves the existing rule digest over
the canonical `RuleSet` so attestation and settlement bindings do not change.
New rule publications persist the envelope. Replay adapters accept the
envelope and a bounded legacy bare `RuleSet` form only for already-persisted
historical evidence; they never write new bare documents. Unsupported schema
identifiers fail closed. The envelope is a document-compatibility boundary,
not a substitute for the immutable rule version or digest.

## Alternatives considered

### Add a schema field directly to `RuleSet`

Rejected. It changes the financial rule digest and every existing constructor
and vector, coupling domain identity to transport compatibility.

### Store only the bare `RuleSet` and infer its version from fields

Rejected. Inference cannot distinguish an incompatible future representation
from a valid historical document and makes fail-closed replay ambiguous.

### Replace every historical row in one migration

Rejected. Rewriting evidence changes operational risk and complicates audit
replay. New writes use the envelope while historical rows remain immutable.

### **Versioned envelope with bounded legacy replay adapter**

Selected. It makes new evidence explicit, preserves existing digests and
replay, supports expand/contract migration, and keeps the hot calculation path
free of schema-discovery work after validation.

## Invariants and verification

- Only `solana-tax-protocol/tax-rule/v1` is accepted for new documents.
- Envelope validation always validates the contained rule graph before use.
- The rule digest is identical before and after envelope serialization.
- New draft publication rows persist the envelope, never a bare `RuleSet`.
- Historical bare documents remain replayable only through an explicit
  compatibility branch and are never re-emitted as new evidence.
- Unsupported schema, malformed envelope, graph cycle, and invalid arithmetic
  fail closed before calculation or settlement.
- Rust unit tests cover round-trip, schema rejection, digest stability, and
  legacy parsing; storage tests cover new envelope persistence and replay.

## Consequences

Positive: rule documents become self-describing, migration-safe, and suitable
for public schema tooling without changing settlement digests.

Cost: replay code carries a temporary legacy branch, and future schema changes
require a new identifier, parser, compatibility tests, and ADR.
