# Database

The durable store is PostgreSQL 18.x. The migrations use native `uuid` storage
and `uuidv7()` defaults for internal identifiers. Historical evidence is
append-only; corrections create new rows and preserve the original calculation
or settlement record.

The persisted quote path uses a serializable transaction to select an exact
published rule release, insert calculation evidence, and complete a
merchant-scoped idempotency record together. The database is an evidence and
projection store; finalized Solana receipts remain authoritative for funds.
Atomic amounts use exact `numeric(20,0)` columns rather than signed `bigint`,
so the full Solana `u64` range is representable without narrowing at the
database boundary.

Compose mounts one clearly named synthetic rule fixture for local development
only. It is not a supported jurisdiction, legal interpretation, or production
seed and must never be copied into a release migration.

Migration `0004` enforces immutable published and retired rule releases and
rejects overlapping published effective windows for the same rule, jurisdiction,
and regime.

Migration `0005` adds bounded outbox delivery leases, retry/dead-letter state,
and the dispatcher index. JetStream acknowledgements never replace the
PostgreSQL event ledger.

Migration `0007` adds bounded, immutable raw Anchor settlement-event evidence
beside normalized finalized receipts. Evidence is retained for deterministic
projection rebuilds and forensic replay; it is not a substitute for the
authoritative Solana ledger.

Migration requirements:

- run migrations through a reviewed migration runner, never by editing production tables manually;
- use expand/contract changes for online deployments;
- set bounded `lock_timeout`, `statement_timeout`, and `idle_in_transaction_session_timeout`;
- use concurrent indexes where supported and justified;
- verify backups and restore into an isolated environment;
- measure query plans and index health under representative evidence volume;
- never treat PostgreSQL rows as the source of truth for settled funds; chain receipts remain authoritative.
