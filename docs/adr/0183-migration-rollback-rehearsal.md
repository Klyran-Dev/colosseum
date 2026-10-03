# ADR-0183: Expand/contract migration rollback rehearsal

- Status: Accepted — isolated rehearsal implemented and verified locally; production adoption pending
- Date: 2026-09-29
- Deciders: Principal Low-Latency Architecture, Storage, and SRE
- Scope: PostgreSQL deployment recovery and schema compatibility

## Context

ADR-0138 deliberately makes schema changes forward-only. Applied migration
files are immutable, and a destructive `down` migration would be unsafe for
financial evidence, settlement intents, and audit history. The repository has
verified migration ordering, advisory-lock serialization, checksum drift
rejection, and logical backup/restore mechanics, but it has not yet exercised
the operator sequence for a bad forward deployment: capture a pre-deploy
recovery image, apply an expand migration, restore the pre-deploy image, run
the compatible old schema, and then prove the forward migration can be safely
reapplied.

Without this evidence, “rollback supported” would be an unverified claim.

## Decision

Add a bounded Docker rehearsal that:

1. creates an isolated PostgreSQL 18 database and applies the repository's
   current migrations plus a synthetic, additive expand migration;
2. writes a pre-deployment sentinel and captures a custom-format logical
   backup before the expand migration;
3. applies the expand migration and verifies the new nullable column and
   compatibility index exist;
4. restores the pre-deployment backup into a separate database, verifies that
   the pre-deployment schema/data are present and the expand-only objects are
   absent, and proves the old-schema read contract still works; and
5. applies the expand migration to the restored database and verifies the
   forward recovery path, including preservation of the sentinel data.

The synthetic migration is mounted only for the rehearsal and is never added
to the production migration directory. The drill uses an explicit bounded
rollback marker and fixed assertions; it does not claim production RPO/RTO,
KMS/HSM key recovery, off-site retention, or a live deployment rollback.

Production schema changes must use expand/contract sequencing, an approved
pre-deploy backup/restore point, a compatible application version, bounded
lock/statement timeouts, and an operator-approved recovery decision. A
destructive down migration, historical-file mutation, or blind restore over a
live database is prohibited.

## Alternatives considered

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Destructive `down` migration | Fast on paper; familiar to some tooling | Can destroy evidence, is difficult to make data-safe, and diverges from immutable migration history | Rejected |
| Restore only, without reapply verification | Proves a recovery image can be opened | Does not prove the deployment can resume or that the new schema remains compatible | Rejected |
| Forward corrective migration only | Preserves history and is safe for additive changes | Cannot recover an incompatible application/schema pair without a pre-deploy image | Rejected as the sole contract |
| **Pre-deploy restore plus compatible application and forward reapply rehearsal** | Tests the operator decision, data preservation, old-schema compatibility, and forward recovery | Requires isolated database time and synthetic migration fixtures | **Selected** |

## Invariants

- The pre-deploy backup is captured before the synthetic expand migration.
- Restoring that backup removes every expand-only object and preserves the
  pre-deploy sentinel exactly.
- No applied repository migration file is edited or deleted.
- The old-schema read contract succeeds after restore.
- Reapplying the expand migration succeeds once and is idempotent thereafter.
- Recovery remains isolated from settlement, quote, tax, wallet, and RPC
  request paths.
- The drill output contains fixed-cardinality, public-safe evidence only.

## Verification and release impact

- `make migration-rollback-rehearsal` runs the Docker-isolated rehearsal
  against the pinned PostgreSQL 18 image.
- The 2026-09-29 run applied 48 baseline migrations, applied one synthetic
  expand migration, restored the pre-deploy image with the expand-only column
  and index absent, preserved the `pre-deploy` sentinel, and reapplied the
  expand migration with an idempotent repeat.
- CI and release-candidate workflows retain the bounded output as evidence.
- The drill is a local recovery-mechanics gate; production adoption still
  requires encrypted KMS/HSM backups, immutable off-site retention, measured
  RPO/RTO, staging approval, and an incident-authorized recovery runbook.

## Current local verification

On 2026-10-03, `make migration-rollback-rehearsal` passed with 60 baseline
migrations, one synthetic expand migration, pre-deploy restore, old-schema
read compatibility, forward recovery, and an idempotent repeat. The
pre-deploy sentinel was preserved. Production rollback authority and recovery
runbook approval remain open.
