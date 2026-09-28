# ADR-0092: Authenticated persisted Split Plan resolution

- Status: Accepted for implementation
- Date: 2026-09-27
- Owners: Settlement and Merchant platform owners

## Context

ADR-0091 introduced the immutable on-chain Split Plan PDA and deterministic
compiler artifact, but the enabled API still resolves only a single
`net_destination`. Exposing the new compiler with a caller-provided recipient
array would bypass tenant governance and make the browser an authority for
funds routing.

The API needs a durable, tenant-scoped Split Plan registry that can be selected
by an authenticated quote-bound request. The database is configuration and
evidence storage only; Solana account ownership, plan initialization, and the
atomic transfer remain authoritative on-chain.

## Decision

Add an append-only-by-version, tenant-scoped `settlement_split_plans` registry.

- Each row uses a PostgreSQL UUIDv7 identity, merchant scope, plan key, positive
  version, lifecycle, Solana merchant account, mint, canonical 32-byte plan
  digest, ordered recipient JSON, and effective window.
- Recipients are persisted as decimal-safe JSON fields containing only a
  canonical token-account string and positive basis points. The storage adapter
  validates the JSON by constructing the pure `split-plan` domain type and
  recomputing the digest before returning it.
- Published/active/retired rows are immutable in account topology, version,
  digest, recipients, and effective window. Corrections create a new version.
- Only `active` rows inside their effective window are selectable for checkout.
  Draft, suspended, retired, expired, malformed, or digest-mismatched plans
  fail closed.
- The new endpoint is additive:
  `POST /v1/settlement/split/compile-from-quote` accepts only authenticated
  `calculation_id`, `split_plan_id`, payer, nonce, policy sequence, and policy
  digest. It never accepts recipient accounts or amounts from the caller.
- The API loads the quote and split plan using the authenticated merchant ID,
  verifies quote expiry and plan lifecycle, then compiles the immutable
  Split Plan contract. The durable settlement intent request digest includes
  the plan ID and complete server-loaded plan document.
- Existing `/v1/settlement/compile-from-quote` remains v1 single-recipient
  behavior until a separate route migration enables split plans by default.

## Alternatives considered

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Accept recipients in the request body | Fastest API wiring | Caller-controlled destination substitution and no durable governance | Rejected |
| Store plans only in Redis | Low latency | Disposable, non-authoritative state and weak audit/recovery semantics | Rejected |
| Encode plans only on-chain and resolve by PDA | Strong chain authority | Requires RPC in the quote/compile hot path and weak tenant query ergonomics | Rejected |
| Add recipients directly to `settlement_routes` | One lookup | Couples route account topology to plan versioning and makes migration harder | Rejected |
| **Tenant-scoped PostgreSQL plan registry plus on-chain PDA verification** | Auditable, cacheable, quote-bound, and preserves Solana authority | Requires migration, immutable lifecycle rules, and activation evidence | **Selected** |

## Invariants

1. A plan loaded for checkout always belongs to the authenticated merchant.
2. A plan's stored digest equals the canonical digest of its ordered recipients.
3. Active selection requires lifecycle and effective-window validity.
4. The plan ID, version, digest, and recipient document are included in the
   compiler/intention evidence.
5. No request field can replace the server-owned gross, tax, net, recipient,
   or plan digest values.
6. One plan key/version is unique per merchant; published rows cannot mutate.
7. A malformed or stale plan never produces a `prepared_unsubmitted` split
   instruction.

## Consequences and remaining gates

The first implementation adds the migration, storage read boundary, and an
authenticated quote-bound compiler endpoint with unit and integration tests.
Plan authoring/review/approval, on-chain initialization and finalized plan
activation evidence, indexer decoding, UI authoring/review, and mainnet
governance remain separate release gates.

