# ADR-0413: Atomic applicability-binding verification projection

- Status: Accepted
- Date: 2026-10-06
- Owners: Storage/indexer platform
- Scope: durable projection of ADR-0412 finalized account evidence

## Context

ADR-0412 verifies the receipt-scoped applicability PDA at finalized
commitment, but a verifier result held only in process memory cannot support a
replayable API/UI claim. The raw settlement event is already persisted by the
indexer, while the account verification is a separate bounded RPC operation.
The system needs a durable, idempotent boundary that cannot associate account
state with the wrong event, stream, receipt, or digest.

## Decision

Persist a dedicated append-only `applicability_binding_verifications` projection
using PostgreSQL 18 and UUIDv7. Its natural idempotency key is
`(stream_key, signature, event_index)`. A serializable transaction:

1. locks and loads the immutable raw event evidence;
2. requires an applicability settlement event discriminator and decodes its
   receipt, binding, merchant, mint, context ID, and attestation digest;
3. checks that the typed ADR-0412 verification record matches every event
   identity and digest, has a finalized provider slot at or after the event
   slot, and has a non-zero evidence identity;
4. inserts the complete verification record, or returns `Duplicate` only when
   the existing row is byte-for-byte equivalent; and
5. rejects substitutions, malformed evidence, conflicting replays, and
   missing raw events without changing any projection.

The table is an evidence projection, not settlement authority. Solana remains
the source of truth. The API and UI may expose a binding as finalized only when
this row exists and its raw event evidence remains available.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Store verifier output only in logs | Rejected | Not replayable, queryable, or transactionally bound to raw evidence. |
| Add mutable binding fields directly to `indexed_receipts` | Rejected | Couples a new evidence lifecycle to the legacy receipt projection and weakens append-only provenance. |
| Upsert without comparing existing bytes | Rejected | Allows provider/account substitution on replay. |
| Persist a verifier row before loading raw event evidence | Rejected | Creates an account-only claim with no event binding. |
| Dedicated immutable projection with serializable idempotent insert | Selected | Preserves legacy receipt shape, supports replay, and fails closed on conflicts. |

## Invariants

- A verification row references a persisted raw event in the same stream,
  signature, and event index.
- Event and account binding fields are exact byte matches.
- `provider_slot >= event_slot` and the commitment is exactly `finalized`.
- UUID identity is UUIDv7; all Solana counters and atomic values remain exact
  decimal numeric values or Rust `u64` values.
- Repeating an identical verification is idempotent; any differing replay is a
  conflict and cannot overwrite evidence.

## Consequences

The projection can be queried and audited independently of mutable operational
state. A later worker can schedule ADR-0412 verification candidates without
putting account RPC in quote, compilation, signing, or settlement submission.
The API/UI disclosure still requires a separate authorized read contract and
legal/treatment ownership; this ADR does not assert tax compliance.

