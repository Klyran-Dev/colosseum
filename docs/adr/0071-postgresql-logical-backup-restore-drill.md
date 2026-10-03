# ADR-0071: Use a repeatable logical PostgreSQL backup/restore drill

- Status: Accepted
- Date: 2026-09-26
- Scope: PostgreSQL 18 durable evidence and disaster-recovery verification

## Context

PostgreSQL contains tenant-scoped tax evidence, idempotency records, outbox
state, indexer checkpoints, route-verification evidence, and settlement intent
projections. A pinned image and migration set do not prove that an operator can
restore a usable database. The restore must be isolated, must not modify the
source database, and must verify schema constraints that protect UUIDv7 and
replay-era evidence.

## Decision

Use a bounded `pg_dump` custom-format backup from a fresh, disposable,
Docker-managed PostgreSQL 18 source initialized by the repository's complete
migration and synthetic-fixture set, then restore it into a second disposable
PostgreSQL 18 container using `pg_restore --exit-on-error`. The drill verifies:

- the backup is non-empty and readable;
- the `facts_document` replay column and required evidence tables exist;
- PostgreSQL's UUIDv7 generation contract remains available; and
- the restored database can answer integrity queries before the disposable
  target is removed.

The source and target are isolated from the developer's running database; the
restore phase never drops or resets an existing project volume. Production
operations must additionally store encrypted
backups in an independent failure domain, test point-in-time recovery, record
RPO/RTO, validate access controls, and rehearse key recovery. This local drill
is evidence of the mechanism, not production disaster-recovery certification.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Filesystem snapshot of the PostgreSQL volume | Rejected as the sole gate | Fast, but storage-engine state, portability, and restore correctness depend on the volume provider. |
| Continuous physical/PITR backup only | Rejected as the local gate | Required for production RPO, but harder to run deterministically in a clean Docker checkout and does not replace logical schema verification. |
| **Docker-isolated logical dump and restore** | **Selected** | Portable, inspectable, repeatable, bounded, and directly validates the PostgreSQL schema/evidence contract. |
| Copy rows with application scripts | Rejected | Can omit extensions, constraints, indexes, ACLs, or append-only evidence semantics. |

## Consequences

- `make backup-restore-drill` requires Docker but no host PostgreSQL tooling or
  existing database state.
- The target container is disposable and explicitly named by the drill; no
  broad volume deletion is used.
- A successful local drill does not mark RPO/RTO, encrypted off-site backup,
  point-in-time recovery, or production restore exercises complete.
- Schema changes that affect durable evidence must keep the drill green and
  update the recovery runbook and migration rollback evidence.

## Verification

The command must fail on dump, restore, readiness, schema, UUIDv7, or integrity
query errors and must print a success marker only after the restored target has
passed all checks.

## Current local verification

On 2026-10-03, `make backup-restore-drill` passed against a fresh Docker
PostgreSQL 18 volume and the complete migration/fixture set. The logical dump
restored successfully and the post-restore schema, UUIDv7, and durable-evidence
integrity checks passed. This remains mechanism evidence; encrypted off-site
retention, key recovery, measured RPO/RTO, and production authorization remain
open.
