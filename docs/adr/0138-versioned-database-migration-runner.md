# ADR-0138: Versioned, checksum-verified database migration runner

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Storage, and SRE
- Scope: PostgreSQL schema-change execution and deployment safety

## Context

The former local bootstrap mounted the complete SQL directory into
`docker-entrypoint-initdb.d`, and the development `migrate` target replayed
all files. That was useful for a clean fixture but was not a sufficient
production migration contract: an already-applied migration could drift
without being detected, two deployers could race, and the database had no
authoritative record of which exact source bytes were applied.

Financial evidence, settlement intents, and tenant-scoped authority require a
fail-closed schema lifecycle. A migration must be applied once, in lexical
version order, under a database-wide deployment lock, and its exact source
digest must be retained. A changed historical migration must stop deployment;
it must never be silently reinterpreted as a correction.

## Decision

Add a small Rust `migration-runner` binary and library that:

1. discovers only bounded, numerically ordered `NNNN_name.sql` files;
2. creates a PostgreSQL `schema_migrations` ledger with the migration ID,
   SHA-256 checksum, applied timestamp, and execution duration;
3. takes a session advisory lock before reading or changing the ledger;
4. skips an already-applied migration only when its checksum matches exactly;
5. applies pending files in lexical order and records the checksum only after
   the SQL succeeds; and
6. fails closed on malformed names, duplicate IDs, missing files, checksum
   drift, database errors, or a migration that exceeds the bounded execution
   timeout configured by the deployment.

Files with balanced top-level `BEGIN;`/`COMMIT;` markers retain their explicit
transaction boundary. Files without those markers are wrapped in a runner-owned
transaction; an unbalanced marker pair fails closed. Seed fixtures are also
executed in one bounded transaction.

The runner is a deployment tool, not an application request-path dependency.
Compose now starts it as a one-shot migration service before API, worker,
indexer, and route-verifier processes. Synthetic fixtures are mounted into a
separate explicit seed directory and are executed only by the local Compose
configuration; production deployments must omit `SEED_DIR`.

Migration files must be transaction-safe or explicitly carry a separately
reviewed non-transactional exception. Rollback is forward-only: destructive or
incompatible changes require expand/contract steps and a new corrective
migration, never mutation of an applied file.

## Alternatives considered

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| PostgreSQL init-directory ordering only | Zero application code; convenient local bootstrap | No applied ledger, no drift detection, no deployment lock, weak rollback boundary | Rejected for staging/production |
| Re-run all idempotent SQL files on every deploy | Simple operator command | Idempotence is not a proof of compatibility; partial failures and drift are opaque | Rejected |
| External migration SaaS/tool with provider-specific state | Mature operational features | Adds vendor dependency and makes clean-checkout evidence harder to reproduce | Rejected as the repository authority |
| **Rust runner with PostgreSQL ledger, advisory lock, and SHA-256 checksums** | Typed bounded tool, deterministic ordering, fail-closed drift detection, low operational overhead | Requires a deployment image and a staged Compose integration | **Selected** |

## Invariants

- One migration ID maps to one immutable checksum.
- A checksum mismatch for an applied ID blocks all further migrations.
- Only one runner can mutate the ledger at a time for the database.
- A failed SQL file is never recorded as applied.
- The runner never edits or deletes an applied migration row.
- Migration execution is outside settlement, quote, tax, and RPC hot paths.
- Rollback uses a compatible application deployment or a new corrective
  migration; it never rewrites historical SQL.

## Verification and release impact

- Unit tests cover filename validation, ordering, duplicate IDs, checksum
  stability, and checksum drift.
- The isolated PostgreSQL drill runs two concurrent runners, proves one applies
  the complete migration set while the other skips every file, verifies a
  subsequent idempotent run, proves a failed migration leaves no partial table,
  and then injects a changed historical file to prove fail-closed drift
  detection.
- CI runs the runner tests and the isolated drill.
- Production adoption additionally requires encrypted backup/PITR evidence,
  migration rollback rehearsal, deployment ownership, and an approved
  staging migration-service run.
