# ADR-0534 — Seed synthetic published rules for persisted load drills

- Status: Accepted
- Date: 2026-10-13
- Owners: API platform and reliability engineering

## Context

The persisted quote and rule-simulation load drills run against an isolated
Docker Compose project and intentionally address synthetic published rules.
The migration runner only applies fixture seeds when `SEED_DIR` is set. The
drills started the API and therefore the migrator without that local-only
setting, so their warm-up requests correctly returned `404` for a missing
published rule. This made CI report a pool/load failure before exercising the
declared workload. The original pool drill also drove 256 same-key requests at
64-way concurrency through a one-connection pool, exceeding the API's bounded
serialization retry window on shared CI runners.

## Decision

The persisted quote and rule-simulation benchmark scripts explicitly export
`SEED_DIR=/seeds`. The Compose migrator already mounts the repository fixture
directory at `/seeds`. The pool saturation drill uses 16-way measured load and
64 same-key contention requests: enough to observe queueing and replay
convergence without turning a bounded reliability drill into an unbounded
retry test. A make-level contract checks the seed and workload declarations.

This setting is limited to synthetic local/CI evidence. Staging and production
configuration validation continues to reject `SEED_DIR`, and no production
service receives repository fixtures.

## Alternatives

- Seed directly from each benchmark script with SQL: rejected because it
  duplicates migration-owned fixture data and bypasses the migration runner.
- Make Compose seed fixtures by default: rejected because it would weaken the
  deployment boundary and could put synthetic data into staging or production.
- Remove the persisted load drills: rejected because connection-pool and
  idempotency behavior require a real database-backed workload.
- **Explicitly enable the mounted seed directory in the evidence harness**:
  selected because it is bounded, reproducible, and preserves the production
  fail-closed configuration contract.

## Invariants

- Fixtures are synthetic and isolated to the benchmark Compose project.
- The measured workload remains authenticated and database-backed.
- No fixture seed path is enabled by production or staging configuration.
- The warm-up request must succeed before latency or contention evidence is
  collected.

## Verification

- `make benchmark-fixture-contract`
- `make postgres-pool-drill` (256 measured requests, 16-way load, 64-way
  same-key contention)
- `make benchmark-rule-simulation` with the declared bounded workload
- CI rerun on the pushed revision
