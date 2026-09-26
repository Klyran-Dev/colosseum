# ADR-0028: Use a persisted published-rule registry and transactional idempotent quotes

- Status: Accepted
- Date: 2026-09-25

## Context

The initial API slice accepts a caller-supplied `RuleSet` and returns a UUIDv7
from process memory. That is useful for exercising the pure engine, but it
cannot support production replay, immutable rule releases, duplicate request
handling, or a durable calculation evidence trail. PostgreSQL is already the
durable projection/evidence store, while Solana remains authoritative for
settled money movement.

## Decision

Add a PostgreSQL adapter behind an application-owned port and expose a
production quote boundary that:

1. selects only an exact, published rule release within its effective window;
2. recomputes and compares the stored canonical rule digest before evaluation;
3. evaluates the pure tax engine without database, clock, or RPC dependencies;
4. records the calculation, input/rule digests, engine version, trace, and
   amounts in one transaction; and
5. binds a required merchant-scoped idempotency key to a canonical request
   digest. A retry returns the original response; a reused key with different
   input fails closed.

The transaction uses a bounded PostgreSQL connection pool, prepared SQL
statements, explicit lock/statement budgets, and row-level idempotency
serialization. A database row is evidence and a projection, not settlement
authority. The endpoint never claims attestation, wallet authorization, or
chain settlement.

The existing caller-supplied route remains available only as an explicitly
named simulation route when simulation mode is enabled. It is not used by the
production quote path and cannot be enabled by a production configuration.

## Alternatives considered

- **Keep rules and calculations in process memory:** rejected because restart,
  horizontal scaling, replay, and duplicate-request behavior are not durable.
- **Let the tax engine query PostgreSQL directly:** rejected because it breaks
  the pure bounded context and makes calculation latency and correctness depend
  on I/O.
- **Use Redis as the quote authority:** rejected because cache eviction,
  failover, and operator error cannot be the source of financial evidence.
- **Persist asynchronously through an outbox:** rejected for the calculation
  record and idempotency decision because the caller needs one durable result
  before receiving a successful quote; outbox publication remains appropriate
  for downstream notifications.
- **Accept the newest published rule automatically:** rejected because
  effective windows, jurisdiction, regime, and exact version must be explicit
  evidence inputs.

## Invariants and failure behavior

- Published rule rows are immutable; corrections are new versions.
- Stored rule JSON and stored rule digest must agree byte-for-byte under the
  engine's canonical digest contract.
- A merchant/idempotency-key pair maps to one request digest and one response.
- A calculation cannot be returned as successful unless its evidence row and
  idempotency row commit together.
- Missing, expired, ambiguous, malformed, or digest-mismatched rules fail
  closed before a calculation is returned.
- Database pool exhaustion, timeout, serialization conflict, or unavailable
  storage produces a retryable service error and never a fabricated quote.

## Consequences

- Production quotes gain durable replay and duplicate-request semantics.
- PostgreSQL is on the synchronous quote path, so the pool, indexes, timeout
  budgets, and database SLO must be measured and operated explicitly.
- Rule publishing, provenance, legal/tax approval, and signer attestation are
  still separate release gates; a persisted calculation is not legal advice or
  a settlement authorization.
