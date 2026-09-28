# ADR-0089: Bounded PostgreSQL contention and timeout evidence

- Status: Accepted
- Date: 2026-09-27
- Owners: Platform / SRE
- Scope: Local and CI evidence for database lock, statement, schema, and index boundaries

## Context

The storage adapter uses a bounded pool and applies a 250 ms lock timeout and
900 ms statement timeout to critical transactions. These values are safety
contracts, not performance claims. Unit tests cannot prove PostgreSQL lock
behavior, migration ordering, or the presence of the indexes used by evidence
and replay queries.

The production system also needs account-lock contention, recipient fan-out,
partition/retention, and regional database load measurements. Those require
representative production-like data and remain separate release gates. This
ADR defines the repeatable baseline drill without pretending that one local
container proves a production SLO.

## Decision

Add a Docker-runnable PostgreSQL contention drill:

- Start an isolated PostgreSQL 18.6 container from the pinned Compose image and
  apply the complete migration/fixture set.
- Seed one synthetic checkpoint row, hold its row lock for one second, and
  prove a competing transaction fails closed at the 250 ms lock boundary.
- Execute a one-second `pg_sleep` under a 900 ms statement timeout and prove
  the statement is canceled rather than consuming an unbounded connection.
- Verify required primary/index definitions for checkpoints, raw evidence,
  normalized receipts, replay gaps, and reconciliation runs.
- Verify the lock holder commits its change and the failed transaction does
  not partially update the row.
- Remove only the drill's named Compose project and volume on exit.

The drill is an infrastructure contract and CI regression gate. It does not
replace production connection-pool saturation, tenant/account-lock load,
partition/retention rehearsal, encrypted PITR, or regional latency evidence.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Unit-test timeout constants only | Rejected | Does not exercise PostgreSQL lock manager, SQLSTATEs, migrations, or indexes. |
| Run the full quote load only | Rejected | Exercises HTTP/idempotency but does not deterministically create a database lock or statement timeout. |
| Unpinned `pgbench` workload | Rejected | Useful for throughput, but not a stable contract for the exact lock and schema invariants. |
| **Pinned Docker PostgreSQL with targeted SQL contention and catalog assertions** | **Selected** | Reproducible, bounded, easy to audit, and complementary to service load tests. |

## Invariants and failure behavior

1. A lock wait cannot exceed the configured lock budget in the critical
   transaction.
2. A long statement is canceled at the statement budget and leaves no partial
   mutation.
3. Required migrations and supporting indexes exist before the drill claims
   success.
4. The failed contender cannot alter the lock-holder's committed state.
5. The drill never uses production credentials, data, or a broad host cleanup
   command.

## Evidence

- `scripts/postgres_contention_drill.sh`
- `.github/workflows/ci.yml`
- `crates/storage/src/lib.rs`
- `db/migrations/0006_chain_indexer_evidence.sql`
- `docs/plans/production-readiness.md`

