# ADR-0239: Catalog-driven UUIDv7 invariant closure

## Status

Accepted for implementation. Migration `0058` closes the existing schema gap;
the executable UUIDv7 contract becomes catalog-driven and fails closed when a
new application-generated default is missing a version check.

## Context

ADR-0023 and ADR-0083 require PostgreSQL 18 native `uuid` values and UUIDv7 for
application-generated sortable identifiers. The original verification query
maintained a hand-written list of covered columns. That list had begun to lag
behind later migrations, so new intent, evidence, and lifecycle identifiers
could receive a `uuidv7()` default without receiving the database check that
proves version 7.

This is a schema-integrity issue, not a cosmetic documentation issue. A
malformed or non-v7 application identifier can weaken operational ordering,
make evidence queries non-conforming, and create inconsistent behavior between
clean bootstrap and upgraded databases.

## Decision

Use PostgreSQL catalog metadata as the source of truth for this invariant. The
verification query discovers every public column whose default expression
contains `uuidv7()`, then requires all of the following:

1. the column type is native `uuid`;
2. the default remains UUIDv7-generating; and
3. a validated check constraint proves `uuid_extract_version(column) = 7`.

Migration `0058` iterates over the same catalog set, adds a deterministic
bounded-name check constraint where absent, and validates it before recording
the migration. Existing rows are therefore checked during the migration, not
silently grandfathered into an unvalidated state. Future migrations that add a
`uuidv7()` default are automatically covered by the verification contract and
must add their own constraint or fail the deployment/bootstrap gate.

The catalog set is intentionally limited to default-generated UUIDv7 values.
Solana public keys, transaction signatures, cryptographic digests, external
tenant claims, and nullable lease/claim tokens generated for coordination are
not converted into UUIDv7 identifiers by this ADR.

## Invariants and failure policy

- A public `uuidv7()` default cannot exist without a native UUID type and a
  validated version-7 check.
- Constraint names are deterministic and bounded to PostgreSQL's identifier
  limit; name collisions are resolved with a digest suffix.
- Migration failure on an existing invalid row is fail-closed and leaves the
  migration ledger unchanged.
- The contract does not infer strict ordering from UUIDv7; business ordering
  continues to use explicit timestamps, chain slots, event sequences, or
  database sequences where required.
- No UUIDv7 value is used as a secret, bearer token, password-reset token,
  signature, Solana public key, or content-addressed digest.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`.

1. Extend the hand-written list: rejected because it will drift whenever a
   migration adds a generated identifier.
2. Scan migration source text only: rejected because source text does not prove
   the live upgraded schema, defaults, or validated constraints.
3. Add a PostgreSQL trigger that rewrites or rejects all UUID columns: rejected
   because it adds write-path overhead and would conflate UUIDv7 policy with
   externally supplied references and protocol identifiers.
4. **Use catalog discovery plus a corrective validated migration:** selected
   because it covers clean and upgraded schemas, keeps request paths unchanged,
   and fails closed on schema drift.

## Latency, migration, and resource budget

The runtime request path is unchanged: UUID generation and ordinary indexed
UUID comparisons remain native PostgreSQL operations. The verification query is
metadata-bounded by the number of public columns. Migration `0058` may scan
existing UUIDv7 columns while validating checks; it must run under the
migration runner's bounded statement timeout and an approved maintenance
window. It must not run from quote, settlement, tax, or reconciliation hot
paths.

## Verification plan

1. Add a migration-bootstrap contract fixture proving a later generated UUIDv7
   column is covered without editing a second allow-list.
2. Run the catalog contract against the clean PostgreSQL 18 schema and an
   upgraded schema containing all current migrations.
3. Prove an inserted UUIDv4 into each covered column is rejected, while UUIDv7
   inserts succeed.
4. Prove a malformed default/type or an unvalidated constraint fails the
   contract before service startup.
5. Retain Linux PostgreSQL 18 migration and backup/restore evidence; local
   execution without Docker is not release evidence.

## Consequences

The schema contract gains complete coverage with less manual maintenance. A
future migration that forgets the check will fail the database gate rather than
silently weakening identifier semantics. Upgrades may spend bounded time
validating existing rows, so rollout planning must account for table size and
statement-timeout evidence.
