# ADR-0083: Executable UUIDv7 schema contract

- Status: Accepted
- Date: 2026-09-26
- Owners: Data/platform architecture

## Context

ADR-0023 mandates PostgreSQL 18 native UUID storage and UUIDv7 for
application-generated sortable identifiers. The migrations already declare
UUIDv7 defaults and version checks for the principal entity, evidence,
idempotency, outbox, settlement, governance, and correlation IDs. A static
inspection, however, can drift as migrations evolve and does not prove that a
restored database retains the same contract.

## Decision

Maintain [uuidv7_contract.sql](../../db/verification/uuidv7_contract.sql) as an
executable schema contract. It enumerates every application-generated
sortable UUID column and fails closed unless each column is native PostgreSQL
`uuid`, defaults to `uuidv7()`, and has a database check requiring version 7.
The contract runs after the complete migration/fixture set in clean-stack
integration and after logical restore in the recovery drill.

The contract intentionally excludes UUID foreign-key references whose value is
owned by another entity, external identity-provider tenant claims, Solana
public keys/signatures, cryptographic digests, and replay claim tokens. Replay
claim tokens remain CSPRNG UUIDv4 values because they are coordination secrets,
not sortable identifiers.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| **Catalog-level executable SQL contract in integration and restore drills** | **Selected** | Proves the actual PostgreSQL schema and survives migration/restore changes |
| Static grep of migration files | Rejected | Cannot prove final catalog state or restored schema behavior |
| Application-only UUID version tests | Rejected | Does not protect direct SQL writes, defaults, or database constraints |
| Convert every UUID column to UUIDv7 | Rejected | Confuses foreign references, external identities, cryptographic values, and secret claim tokens with generated entity IDs |

## Invariants

1. Every enumerated generated sortable ID is native `uuid`.
2. Every enumerated generated sortable ID defaults to PostgreSQL `uuidv7()`.
3. Every enumerated generated sortable ID is protected by a version-7 check.
4. The contract fails migrations/verification rather than silently accepting a
   missing or changed column.
5. Claim tokens, hashes, Solana identifiers, and strict sequence values retain
   their explicitly documented representations.

