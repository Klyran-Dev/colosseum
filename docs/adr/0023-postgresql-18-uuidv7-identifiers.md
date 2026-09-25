# ADR-0023: Use PostgreSQL 18.x and UUIDv7 for internal identifiers

- Status: Accepted
- Date: 2026-09-25

## Context

The platform needs a durable relational store for projections, evidence, idempotency, outbox records, and operational workflows. It also needs distributed IDs that are sortable enough for index locality without central coordination. PostgreSQL 18 provides native UUIDv7 generation, while RFC 9562 standardizes UUIDv7.

## Decision

Use PostgreSQL 18.x, kept on the latest security patch for the major version, as the durable relational database. Store internal IDs in the native `uuid` type and use UUIDv7 for entity, calculation, intent, idempotency, outbox, audit, and correlation IDs. Use PostgreSQL `uuidv7()` or a cryptographically secure application generator. Pin the exact image tag and digest.

UUIDv7 is not used for secrets, bearer tokens, cryptographic digests, Solana public keys/signatures, or strict total ordering. Use CSPRNG tokens, content hashes, native Solana identifiers, or explicit sequence/slot/offset values where appropriate.

## Consequences

- Inserts and common range/index access have useful time locality without a central ID allocator.
- UUIDv7 reveals approximate creation time and must not be treated as confidential.
- UUIDv7 is not strictly monotonic under concurrency; business and ledger ordering require explicit ordering fields.
- PostgreSQL upgrades, migration discipline, backup/restore, lock budgets, and connection-pool management become release gates.

