# ADR-0113: PostgreSQL evidence index-plan and retention contract

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Storage, and SRE
- Scope: append-only chain/evidence projections, query plans, retention

## Context

PostgreSQL is the durable evidence and projection store, but append-only
tables grow with chain activity, tax replay, settlement intents, and privacy
access. An index definition alone is not performance evidence: PostgreSQL may
choose a sequential scan after data volume, statistics, or predicate shape
changes. Retention also cannot be implemented as an unreviewed delete because
raw chain evidence supports replay and audit obligations.

The repository needs a repeatable, clean-checkout check that exercises the
actual indexed predicates at synthetic production-shaped volume and records
the retention boundary without treating local volume as a production SLO.

## Decision

Add a Docker-isolated `postgres-index-plan-drill` that:

1. Applies the complete PostgreSQL 18 migration and fixture set.
2. Seeds bounded synthetic rows into raw chain evidence, indexed receipts,
   settlement intents/events, and evidence-access audit records.
3. Runs `EXPLAIN (ANALYZE, BUFFERS)` for the production query shapes used for
   slot backfill, receipt projection, tenant evidence access, and intent
   lifecycle lookup.
4. Requires the declared supporting index names in the chosen plans and
   records row counts, planning/execution output, and database version.
5. Runs only against synthetic data and removes only its named Compose
   project/volume.

Retention is an explicit expand/contract operational policy, not an automatic
row deletion in this drill. Raw finalized chain evidence and settlement
lifecycle events require retention/legal ownership decisions before archival
or partition detach. The next production migration must introduce time-range
partitions and an approved archive/delete schedule with backup and replay
evidence; until then the readiness item remains open.

## Alternatives considered

### Index catalog existence only

Rejected. It does not prove the planner uses the index under realistic row
counts or that the predicate matches the application query.

### Force `enable_seqscan = off`

Rejected. It manufactures index evidence and can hide a bad selectivity or
statistics design.

### Delete old evidence during the drill

Rejected. Retention is a legal/operational authority decision and deletion can
destroy replay evidence. The drill must not invent a retention policy.

### `pgbench` throughput only

Rejected as the sole gate. Throughput does not assert the specific evidence
queries, tenant isolation predicates, or lifecycle indexes.

### **Bounded synthetic data plus normal planner `EXPLAIN` assertions**

Selected. It is deterministic, auditable, safe for CI, and complementary to
production-volume load, partitioning, archive, and PITR rehearsals.

## Invariants

- All drill data is synthetic and isolated from production credentials/data.
- Index assertions use normal PostgreSQL planner settings.
- Query predicates include stream/tenant/aggregate scope; no unbounded scan is
  accepted as the intended access path.
- Raw evidence remains immutable during the drill.
- Retention is not silently inferred from local test results.

## Verification and release impact

- The 2026-09-27 Docker run of `make postgres-index-plan-drill` seeded 25,000
  rows each into indexed receipts, raw chain evidence, raw chain transactions,
  and evidence-access audit records, plus 25,000 tenant settlement intents and
  10,000 lifecycle events. Normal PostgreSQL planning selected
  `indexed_receipts_slot_idx`, `raw_chain_evidence_slot_idx`,
  `raw_chain_transactions_slot_idx`, `settlement_intents_merchant_key_idx`,
  `settlement_intent_events_intent_time_idx`, and
  `evidence_access_audit_lookup_idx`.
- CI runs the drill on every change.
- The drill closes only local query-plan evidence; production-like data,
  partition/retention, archive, and regional plan evidence remain open.
