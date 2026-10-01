# ADR-0221: Structured migration-connection lifecycle

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Storage, SRE
- Bounded contexts: Deployment Tooling, Database Schema Lifecycle

## Context

The migration runner must keep the PostgreSQL protocol driver alive while the
bounded migration operation executes. A detached `tokio::spawn` connection
task can report a migration result before a connection failure is observed,
and its lifetime is not owned by the runner's top-level operation. That is
inconsistent with the repository's structured-concurrency mandate and makes
deployment failure reporting ambiguous.

## Decision

The migration runner supervises the database connection future and migration
future in one bounded `tokio::select!` lifecycle:

- a migration result is returned when the migration completes successfully or
  fails with a migration error;
- a connection error fails the runner immediately and cancels the migration
  future before it can report success;
- an unexpected clean connection termination before migration completion also
  fails closed;
- when migration completes, dropping the owned connection future closes the
  client lifecycle without leaving a detached task behind.

The supervisor owns both futures and introduces no background task, unbounded
queue, retry, or database lock across an external operation.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Detached `tokio::spawn` connection driver | Rejected | Connection failures can be observed after a false-success migration result and task lifetime is not owned. |
| `tokio::try_join!` for both futures | Rejected | The PostgreSQL driver normally remains alive until the client closes, so successful migrations would wait indefinitely. |
| Poll connection health separately after migration | Rejected | A failure can occur during migration and the post-check cannot prove the migration was safe. |
| **Owned `tokio::select!` supervisor** | **Selected** | Propagates the first failure, cancels the sibling future, preserves bounded lifetimes, and returns promptly after successful migration. |

## Invariants and failure policy

- No migration success is returned after a connection-driver error wins the
  race.
- No detached migration or connection task survives runner completion.
- A clean connection termination before migration completion is an error.
- The supervisor does not retry a failed connection or migration; deployment
  orchestration owns bounded retry and operator approval.

## Verification and release impact

- Unit tests cover connection failure propagation and prompt migration
  completion without waiting for a long-lived connection future.
- Workspace Clippy and tests remain required; the source contains no detached
  `tokio::spawn` in the migration runner.
- This improves failure reporting but does not substitute for PostgreSQL
  backup/PITR, migration rollback, or staging deployment evidence.
