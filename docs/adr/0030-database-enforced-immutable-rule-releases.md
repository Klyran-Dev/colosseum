# ADR-0030: Enforce published rule immutability and window exclusivity in PostgreSQL

- Status: Accepted
- Date: 2026-09-25
- Owners: Tax Governance / Data Platform
- Scope: PostgreSQL `rule_sets` release registry

## Context

The quote service verifies a stored rule's canonical digest before calculation, but application-only enforcement does not protect the evidence registry from a privileged SQL operator, a migration mistake, or a future service that writes the table incorrectly. A published rule is part of the calculation evidence and must not change under an existing identity.

Overlapping published effective windows for the same external rule, jurisdiction, and regime are also ambiguous. Selecting the latest row is not a legal or governance decision and is unsafe in a money path.

## Decision

Use PostgreSQL triggers and a GiST exclusion constraint as a second enforcement boundary:

- published and retired releases cannot change identity, rule document, digest, provenance, or effective window;
- a published release may transition only to retired;
- retired releases cannot be reactivated or modified;
- published and retired releases cannot be deleted;
- published windows for the same external rule, jurisdiction, and regime may not overlap, using half-open `[effective_from, effective_until)` intervals and an open-ended infinity upper bound.

Corrections are new rule versions with new provenance, digest, review, and effective windows. The service remains responsible for workflow authorization and canonical digest validation; the database guard is defense in depth and an invariant backstop.

## Alternatives considered

- Application-only checks: rejected because direct SQL and future writers could bypass them.
- Periodic overlap audits: rejected because an invalid state would exist between audits.
- Serializable selection without an exclusion constraint: rejected because serialization does not prevent a later mutation from invalidating historical evidence.
- A separate document store: rejected because it weakens the transactional relationship between release identity, calculation evidence, and idempotency.

## Consequences

Positive:

- The authoritative release registry rejects ambiguous or mutable published state at write time.
- Historical calculations can rely on the selected rule document remaining stable.
- Rule retirement is explicit and does not destroy evidence.

Costs:

- PostgreSQL `btree_gist` is required for the exclusion constraint.
- Release correction workflows must create a new version and cannot repair a published row in place.
- Operators need a documented emergency procedure for a blocked or accidentally published release; disabling the guard is not an acceptable normal operation.

## Verification

The Docker integration harness attempts a mutation of the synthetic published rule and requires PostgreSQL to reject it. Migration and constraint behavior must also be covered by staging migration tests before release.
