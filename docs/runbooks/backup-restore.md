# Runbook: PostgreSQL backup and restore

## Trigger and owner

Use for database corruption, regional recovery, migration rehearsal, or a
scheduled disaster-recovery test. The database owner coordinates; application
and chain-data owners verify replay and evidence integrity.

## Procedure

1. Declare the recovery target, RPO/RTO, incident/change ID, and approved
   backup timestamp. Protect the backup with encryption and least-privilege
   access.
2. Restore into an isolated target. Never overwrite the source before the
   restored schema, UUIDv7 contract, migration version, roles, and indexes are
   verified.
3. Run `make backup-restore-drill` from a clean checkout for the logical
   repository contract, and `make postgres-pitr-drill` for the physical
   base-backup plus exact-LSN WAL replay contract. Run the projection rebuild
   drill and calculation replay checks on the restored target.
4. Compare row counts, evidence digests, checkpoints, outbox state, replay
   leases, and reconciliation results. Record measured restore duration and
   data-loss boundary.
5. Cut over only through the approved change process, with connection fencing,
   health checks, and a rollback point. Reconcile chain state after cutover.

## Stop conditions

Stop on failed integrity checks, missing migrations, UUIDv7 contract failure,
unknown backup provenance, or divergence from finalized Solana evidence. The
repository drill is not a measured production RPO/RTO claim.
