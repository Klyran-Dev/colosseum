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

