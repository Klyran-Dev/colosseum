# ADR-0151: Durable settlement-adjustment workflow

## Status

Accepted and implemented for proposal/governance evidence. This ADR does not
authorize token movement or claim that a refund has been paid.

## Context

ADR-0150 establishes the deterministic adjustment domain, but an in-memory
ledger cannot survive process failure, provide tenant isolation, or support an
auditable author/reviewer workflow. A production operator needs to propose a
bounded adjustment, reload it after a timeout, approve or reject it under
four-eyes control, and inspect the complete immutable evidence without
mutating the original settlement receipt.

## Decision

Add a PostgreSQL-backed, tenant-scoped adjustment workflow:

- `settlement_adjustments` stores the canonical request fields, request digest,
  domain digest, status, proposer/approver subjects, and bounded signed atomic
  deltas. New IDs and event IDs use native UUIDv7 constraints.
- `settlement_adjustment_events` is append-only and records propose, approve,
  reject, void, and posting-state evidence. A first proposal event and the
  first outbox publication record commit in the same transaction.
- A unique `(merchant_id, idempotency_key)` fence makes retries return the
  original response only when the complete request digest matches. A different
  request under the same key is a conflict.
- The domain content digest excludes the server-generated adjustment UUID;
  otherwise a retry would create a false idempotency conflict before the
  database fence can return the original record. The persisted record UUID
  and request digest still identify and bind the durable request.
- All reads and transitions include `merchant_id` derived from the verified
  principal. Cross-tenant IDs are indistinguishable from not-found.
- API scopes are exact: `tax:adjustment:propose`,
  `tax:adjustment:read`, `tax:adjustment:approve`, and
  `tax:adjustment:reject`. The approver cannot equal the proposer. The
  application calls the pure domain transition and the database trigger also
  rejects invalid lifecycle edges.
- Only `proposed`, `approved`, `rejected`, and `voided` are exposed in this
  slice. `posted` requires a later governed on-chain posting ADR and cannot be
  forged by an API transition request.

## Failure and consistency rules

- Invalid arithmetic, UUID version, digest, amount, reason, or asset input is
  rejected before a database write.
- Database serialization/deadlock retries are bounded by the existing storage
  policy; no unbounded retry or broker is in the critical path.
- A timeout after commit is recovered by the idempotency key and read route.
- A timeout before commit leaves no proposal event or outbox event.
- Historical proposal fields and domain digest are immutable after creation;
  lifecycle evidence is appended rather than overwritten.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`. PostgreSQL plus a
transactional outbox is selected because it is already the authoritative
durable evidence store, supports tenant-scoped compare-and-set transitions,
and keeps NATS asynchronous. Redis-only state, mutating receipts, and a
browser-owned workflow are rejected.

## Consequences

The product gains crash-safe, replayable adjustment governance with explicit
customer/operator status. It also adds schema, retention, privacy, and UI
surface area. Approval is evidence, not payment; the readiness plan remains
open until chain posting, finality, reconciliation, chargeback evidence, and
legal/tax operations are implemented and tested.

## Verification

- Migration bootstrap checks UUIDv7, append-only event guards, immutable
  financial fields, and valid lifecycle transitions.
- Storage integration proves create/replay/conflict, cross-tenant isolation,
  four-eyes approval, rejection, and event/outbox atomicity.
- API tests prove exact scope/role gates and bounded error responses.
