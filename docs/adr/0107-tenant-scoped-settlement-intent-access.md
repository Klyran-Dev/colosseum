# ADR-0107: Tenant-scoped settlement intent access

- Status: Accepted
- Date: 2026-09-27
- Owners: Payments, Security, and SRE
- Scope: Settlement intent storage adapter and authenticated submit/read/reconcile API

## Context

Settlement intents contain signed-transaction digests, provider signatures, and
reconciliation state. The authenticated API already knows the merchant scope,
but the storage read and transition methods accepted only an intent UUID. The
API loaded the row globally and checked `merchant_id` afterwards. That leaves a
cross-tenant lookup path in the persistence boundary, creates an avoidable
existence side channel, and makes it possible for a future internal caller to
forget the application-level check.

The same boundary must protect submit, status, and reconciliation. PostgreSQL
is the durable projection, but it must not become a tenant authority by
accident: every query and mutation must carry the authenticated merchant.

## Decision

Make `merchant_id` a required parameter of both storage operations:

- `load_settlement_intent(intent_id, merchant_id)` queries by both values;
- `transition_settlement_intent` carries `merchant_id` and includes it in the
  fenced `UPDATE` predicate;
- API submit, read, and reconcile handlers pass the authenticated principal's
  merchant ID before any intent row is returned or mutated;
- an ID belonging to another merchant is indistinguishable from a missing ID
  at the API boundary and returns `404 SETTLEMENT_INTENT_NOT_FOUND`;
- the append-only lifecycle event is inserted only after the tenant-scoped
  update reports exactly one affected row;
- the signed message, transaction digest, status machine, and provider
  finality rules remain unchanged.

The database primary key remains the UUIDv7 intent ID. The merchant predicate
is an authorization invariant, not a replacement for the globally unique
identity. A future high-volume listing endpoint must add a merchant-first
index and keyset pagination; this point lookup remains bounded by the primary
key plus tenant predicate.

## Invariants and failure behavior

- No storage method in this path can read or mutate an intent without a
  merchant scope.
- Cross-tenant IDs reveal neither existence nor lifecycle state.
- A concurrent transition from another request returns `false` and cannot
  append an event.
- No transaction bytes, private keys, or bearer credentials are persisted.
- Tenant scope is derived only from verified JWT claims; request JSON and path
  values cannot supply it.
- Storage errors fail closed; the API does not retry a state-changing request
  after an ambiguous database outcome.

## Alternatives considered

1. **Keep the application-level post-query check** — rejected because the
   database boundary remains unsafe for future callers and cross-tenant row
   existence is observable through different responses.
2. **Use a PostgreSQL session tenant variable/RLS only** — rejected as the sole
   control because pooled connections require careful reset/fencing and every
   transaction would need verified context installation; explicit predicates
   are auditable at the call site and remain effective in offline tests.
3. **Replace UUID lookup with merchant-prefixed opaque IDs** — rejected because
   it changes the public contract and conflates authorization with identifier
   design.

## Budgets and verification

- Add no network round trip and no additional RPC call.
- Keep the point lookup within one indexed PostgreSQL query and the transition
  within one bounded transaction using existing lock/statement timeouts.
- Unit tests prove the request object carries scope and the API authorizes
  before storage access.
- The storage SQL contract test keeps the tenant, next-status, signed-digest,
  and expected-status placeholders distinct; this prevents a parameter-slot
  collision from silently weakening or breaking the fenced transition.
- The clean Docker integration test compiles a real intent, attempts a read
  with a second merchant, and requires `404`; it also verifies a successful
  same-merchant lifecycle remains unaffected.
- `cargo test`, Clippy, security scanning, and `git diff --check` remain merge
  gates.

## Consequences

The API no longer returns `403` for a known-but-cross-tenant intent because
the safer public contract intentionally hides existence. Operators use
authenticated merchant-scoped support tooling or internal evidence workflows
for incident investigation; no browser endpoint bypasses this rule.
