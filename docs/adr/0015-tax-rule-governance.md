# ADR-0015: Govern tax rules as immutable, provenance-backed releases

- Status: Accepted
- Date: 2026-09-25

## Context

Tax rules change over time and can be misconfigured or maliciously altered. A global engine needs provenance, effective dating, review, approval, rollback, and jurisdiction ownership.

## Decision

Rules move through draft, simulation, review, approval, publication, and retirement. Published versions are immutable and content-addressed. Each version records jurisdiction, tax regime, source references, legal/tax owner, effective window, review date, precedence, engine compatibility, and approval evidence. Author, reviewer, and approver roles are separated.

## Consequences

- Rule changes are auditable and reversible by publishing a corrected version.
- Governance operations are slower than editing a database row, intentionally.
- Missing or conflicting provenance is a hard failure for publication.

## Operational rollback contract

Rollback means retiring the affected published release and publishing a new
version with corrected canonical rule content, provenance, effective window,
review evidence, and approval evidence. It never means mutating the published
row, reopening a retired release, or silently changing the rule selected by an
existing calculation. The Docker integration gate rehearses this sequence and
asserts that version 1 remains `retired`, version 2 is `published`, and the
server-owned calculation uses the corrected version.
