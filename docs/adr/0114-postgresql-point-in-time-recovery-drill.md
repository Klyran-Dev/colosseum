# ADR-0114: PostgreSQL point-in-time recovery drill

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Storage, and SRE
- Scope: PostgreSQL base backups, WAL archiving, point-in-time recovery

## Context

Logical backup/restore proves that the schema and durable evidence can be
recreated, but it does not prove recovery between logical backup boundaries.
Settlement and tax evidence require a recovery procedure that preserves the
last known durable database state and can stop replay at an explicitly chosen
write-ahead-log position.

The local repository needs a safe, repeatable proof of the recovery mechanics.
It must not imply production RPO/RTO, encryption, retention, or legal
authority, because those require approved infrastructure, key custody, and
operational exercises outside Docker.

## Decision

Add a Docker-isolated `postgres-pitr-drill` that:

1. Starts the pinned PostgreSQL 18 image with `wal_level=replica`, WAL
   archiving, and a bounded archive command writing only to a temporary,
   project-owned directory.
2. Applies the complete migration and synthetic fixture set.
3. Takes a physical `pg_basebackup` before a marker transaction.
4. Commits a target marker, records the exact `pg_lsn`, switches WAL, then
   commits a second marker after the target position.
5. Restores the base backup into a disposable PostgreSQL instance and replays
   archived WAL to `recovery_target_lsn`.
6. Asserts that the target marker exists and the post-target marker does not.
7. Records the local recovery elapsed time and archive/LSN evidence, then
   removes only its temporary containers and files.

The drill uses synthetic credentials and unencrypted local files. Production
must use encrypted off-site WAL/object storage, KMS/HSM-managed encryption
keys, immutable retention, monitored archive lag, tested restore permissions,
explicit RPO/RTO, and a rollback/replay runbook.

## Alternatives considered

### Logical dump only

Rejected as the sole recovery proof. It cannot recover to an arbitrary WAL
position and can lose changes committed after the dump.

### PostgreSQL streaming replica only

Rejected as the sole proof. A replica can replicate corruption or operator
error and does not establish an independently restorable historical target.

### Managed-provider PITR only

Rejected as the repository gate. It is necessary to evaluate for production,
but it is not reproducible in a clean checkout and hides restore-command,
schema, and replay assumptions.

### **Physical base backup plus archived WAL and exact-LSN recovery**

Selected. It matches PostgreSQL recovery semantics, is deterministic in a
disposable Docker environment, and proves both the recovery boundary and the
absence of post-target data while leaving production key and retention
decisions explicit.

## Invariants

- The source and recovery instances use the pinned PostgreSQL 18 image.
- The target LSN is captured only after the target marker transaction commits.
- The post-target marker is committed after the target LSN and must not appear
  after recovery.
- Archive and base-backup paths are temporary and project-scoped.
- The drill never deletes repository data or production evidence.
- A local elapsed time is diagnostic evidence, not a production RTO claim.

## Verification and release impact

- The 2026-09-27 Docker run of `make postgres-pitr-drill` restored from a
  physical base backup and archived six WAL files to target LSN `0/4006470`.
  The target marker was present, the post-target marker was absent, and local
  recovery completed in 1 second. PostgreSQL reported replay through
  `0/4008478`; the marker boundary, rather than a byte-for-byte replay-LSN
  equality, is the correctness assertion.
- CI runs the drill on every change.
- The drill closes local physical-recovery mechanics only.
- Production encryption, off-site immutability, archive monitoring, measured
  RPO/RTO, restore authorization, partition/retention policy, and migration
  rollback rehearsal remain release gates.
