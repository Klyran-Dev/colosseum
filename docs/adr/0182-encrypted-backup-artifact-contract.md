# ADR-0182: Encrypted backup artifact contract

- Status: Accepted — local/CI implementation complete; production adoption pending
- Date: 2026-09-29
- Deciders: Principal Low-Latency Architecture, Storage, and SRE
- Scope: PostgreSQL logical backup artifacts and recovery evidence

## Context

The logical and physical PostgreSQL drills prove schema restoration and exact
WAL-position recovery, but their temporary local files are deliberately
unencrypted. A production financial-evidence backup must protect the dump and
WAL artifacts, detect tampering before restore, and keep key custody outside
the database and application request path. Local Docker cannot prove a cloud
KMS/HSM ceremony or production RPO/RTO, but it can exercise the artifact
integrity boundary and fail closed on tampering.

## Decision

Keep production encryption as an envelope-encryption port backed by an
approved KMS/HSM: the provider generates or unwraps a per-backup data
encryption key, performs authenticated encryption, records a key version and
artifact digest, and emits an auditable restore authorization. The database,
API, worker, and browser never receive the provider master key.

Add `make encrypted-backup-restore-drill` as a synthetic local/CI contract.
It creates a disposable random key, encrypts a PostgreSQL custom-format dump,
computes an integrity MAC, rejects a tampered ciphertext, decrypts only after
verification, and restores the result into an isolated PostgreSQL 18 target.
The key and artifacts exist only in a mode-0700 temporary directory and are
removed by the drill's scoped cleanup trap.

The local cryptographic construction is test scaffolding, not the production
algorithm or key-management decision. Production deployments must use the
approved KMS/HSM AEAD envelope, immutable off-site retention, key rotation,
restore authorization, archive monitoring, and measured RPO/RTO.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Plain dump/WAL files | Rejected | Exposes financial evidence at rest and provides no tamper boundary. |
| PostgreSQL `pgcrypto` with a database-held key | Rejected | Collapses database and key custody boundaries; weak rotation/audit semantics. |
| Disk encryption only | Rejected as the sole control | Does not protect exported artifacts, object storage, or an authorized database reader. |
| Application-managed long-lived backup key | Rejected | Creates an avoidable high-value secret in application/deployment paths. |
| **KMS/HSM envelope AEAD with local synthetic artifact drill** | **Selected** | Separates production key authority from services while keeping a reproducible integrity/restore contract. |

## Invariants and release impact

- A restore never consumes an artifact whose authenticated integrity check fails.
- Backup keys are never committed, logged, or supplied to the API/browser.
- Artifact identity, encryption key version, retention class, and restore
  authorization are recorded as evidence in production.
- Local success is not production encryption, off-site retention, or RPO/RTO
  evidence.
- Production adoption remains blocked until KMS/HSM, key recovery, retention,
  archive monitoring, measured RPO/RTO, and staging restore evidence pass.

## Verification

- `make encrypted-backup-restore-drill` passes against the pinned PostgreSQL
  18 image and proves clean restore plus tamper rejection.
- CI and the release-candidate workflow retain the public-safe command output.
- ADR-0114's exact-LSN PITR drill remains the separate physical-recovery
  contract.
