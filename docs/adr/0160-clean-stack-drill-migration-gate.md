# ADR-0160: Clean-stack Docker drills must run the migration gate

- Status: Accepted
- Date: 2026-09-28
- Deciders: Principal Low-Latency Architecture, Storage, and SRE
- Scope: PostgreSQL-backed local operational drills

## Context

Operational drills must be reproducible from a clean checkout and a newly
created named volume. The projection-rebuild and PostgreSQL contention drills
previously started a healthy PostgreSQL container and executed seed SQL
immediately. A clean volume has no application schema at that point, so the
drills failed with missing relations. Relying on a prior developer `make
migrate` run makes evidence order-dependent and violates the Docker
clean-checkout contract.

## Decision

Every PostgreSQL-backed drill that owns a clean Compose project and named
volume must execute the repository's pinned `migrator` service successfully
after PostgreSQL is healthy and before it reads or writes application tables.
The drill remains responsible for its bounded synthetic fixtures and tears
down only its project-scoped containers, network, and named volumes.

The migration runner remains the single schema authority. Drill scripts must
not duplicate migration SQL, mount host database state, or depend on another
Make target having run first. The migration service is a one-shot gate; an
unsuccessful migration stops the drill and prevents misleading evidence.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Seed schema directly in each drill | Small apparent change | Duplicates schema knowledge, drifts from production migrations, and hides migration failures | Rejected |
| Depend on a pre-migrated shared volume | Fast repeated local runs | Not reproducible from a clean checkout; stale schema can mask defects | Rejected |
| Start every application service and inherit `depends_on` | Reuses Compose dependency graph | Adds unrelated services, ports, startup work, and failure modes to focused drills | Rejected |
| **Start PostgreSQL, wait for health, run the pinned migrator, then seed fixtures** | Uses the authoritative schema path, is bounded, minimal, and deterministic | Adds one migration step to each clean drill | **Selected** |

## Invariants and budgets

- No application table is accessed before `migrator` exits successfully.
- A migration checksum, ordering, lock, or SQL failure fails the drill.
- The drill does not alter production databases or reuse another project's
  containers, networks, or volumes.
- Migration execution is setup work and never runs in a quote, tax, signing,
  settlement, indexing, or reconciliation hot path.
- The added gate has no effect on steady-state service latency; drill startup
  remains bounded by the configured migration timeout and health deadline.

## Verification and release impact

- Run each drill from a clean project-scoped volume and require its existing
  assertions to pass.
- Keep `docker compose config`, migration-integrity, and public documentation
  checks in the normal verification set.
- A successful local drill proves clean-stack orchestration and synthetic
  invariants only; it does not close live-chain, regional SLO, backup/PITR,
  audit, or production migration rollback gates.
