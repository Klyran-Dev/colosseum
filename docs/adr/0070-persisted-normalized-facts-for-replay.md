# ADR-0070: Persist normalized facts for authoritative quote replay

- Status: Accepted
- Date: 2026-09-26
- Scope: PostgreSQL tax-calculation evidence and replay verification

## Context

The Tier-1 tax engine can verify a complete result in memory, but a durable
quote currently stores only the facts digest. A digest proves that a value was
bound to some facts; it does not provide the normalized facts required to
recalculate the result during an audit, reconciliation run, or support
investigation. Reconstructing facts from an HTTP request, cache, or client is
not authoritative and would violate tenant isolation and replay guarantees.

## Decision

Persist the bounded, normalized `TaxFacts` document in the tenant-scoped
`tax_calculations` row, alongside the existing digest, engine version, rule
digest, result document, and evidence-reference envelope. The facts document
is the canonical replay input for a quote; it is not a source for legal tax
advice and must contain no unnecessary raw PII or unbounded jurisdiction
payloads.

The storage adapter exposes a tenant-scoped replay-verification operation that:

1. loads the calculation and its published immutable rule release by merchant
   and calculation ID;
2. rejects legacy rows without a normalized facts document;
3. verifies the facts digest, rule digest, engine version, rule identity, and
   result envelope against their persisted columns; and
4. invokes the pure Rust `CalculationResult::verify_replay` operation and
   returns only a verified evidence summary.

The verification endpoint is read-only, authenticated, bounded by the normal
API request and database timeouts, and never authorizes settlement. A missing,
tampered, stale, or semantically incompatible evidence field fails closed.
Historical rows are not silently backfilled: an explicit migration must be
reviewed and evidence-backed before they can become replayable.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Store only the facts digest | Rejected | Cannot independently recalculate or explain a historical quote. |
| Store full facts in an external object store only | Rejected | Adds an availability and consistency dependency to authoritative replay. |
| Embed facts inside the result document | Rejected | Couples input and output schemas and makes evidence references ambiguous. |
| **Store normalized facts in PostgreSQL and verify the complete envelope** | **Selected** | Transactionally consistent with the quote, tenant-scoped, queryable, replayable, and bounded. |

## Consequences

- New quote creation writes facts and the first outbox event in one database
  transaction.
- The normalized facts schema becomes a versioned evidence contract. Changes
  require expand/contract migration, canonical-digest vectors, and replay
  compatibility review.
- Database encryption, least-privilege access, retention, and access auditing
  are mandatory in production even though the current facts model is minimized
  and excludes raw jurisdiction evidence.
- Existing rows created before migration 0014 remain non-replayable until an
  approved migration supplies evidence that is sufficient to recompute their
  original facts digest.

## Verification

Tests must cover exact replay, facts tampering, digest mismatch, rule-release
tampering, engine-version mismatch, absent legacy facts, tenant mismatch, and
idempotent quote replay. A PostgreSQL integration test must verify the actual
transactional write and read path before this gate is marked complete.
