# ADR-0057: Durable settlement intents and lease-safe lifecycle events

- Status: Accepted
- Date: 2026-09-26
- Decision owners: Principal low-latency architecture, settlement, and storage
- Scope: PostgreSQL settlement intent persistence and submission enablement

## Context

RPC responses are not durable application truth. A provider can accept a
transaction and time out before returning its signature, a browser can close
after signing, and two retrying workers can observe the same intent. An
in-memory status or an idempotency key without the signed/instruction digests
would permit an accidental replay of a different payment.

## Decision

Persist a UUIDv7 `settlement_intents` aggregate keyed by merchant and
idempotency key, with immutable request and instruction digests, optional
signed-transaction digest, chain signature, slot, commitment, and bounded
error code. Persist append-only `settlement_intent_events` for every accepted
lifecycle transition.

Every transition is fenced by the expected current status in one database
transaction. The database check and the domain transition table both reject
regression. Replaying the same merchant/idempotency/digest returns the existing
intent; reusing the key with another digest is a conflict. A coordinator
timeout after possible provider acceptance becomes `unknown` and requires
signature reconciliation; it never creates a new signed transaction.

The storage adapter is not authoritative for chain finality. Only verified
`getSignatureStatuses` evidence or finalized indexer evidence can advance the
chain state. Events are retained long enough for incident reconstruction and
projection rebuilds; deletion/retention policy is an explicit operations
decision.

## Alternatives considered

1. **In-memory intent map:** rejected because it loses idempotency across
   replicas and restarts.
2. **One mutable status row with no events:** rejected because it cannot
   reconstruct evidence or diagnose concurrent transitions.
3. **Use the Solana signature as the primary key:** rejected because a signed
   transaction may not yet have a known signature and one intent may be
   represented by provider retries.
4. **Allow the client to choose the resulting status:** rejected; status is
   observed evidence, never client assertion.

## Release evidence

- PostgreSQL checks enforce UUIDv7 IDs, digest lengths, bounded text, status
  vocabulary, commitment vocabulary, and non-negative slots.
- Storage tests must cover same-key replay, digest conflict, fenced concurrent
  transition, invalid transition, append-only event creation, and unknown
  reconciliation.
- Production submit must remain disabled in deployment configuration until this
  schema, a real RPC coordinator, authentication scopes, signed-message
  binding review, and localnet/staging finality evidence are all approved.
