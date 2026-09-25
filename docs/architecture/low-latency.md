# Low-latency and reliability architecture

## Latency budget

The first production SLO should measure both user-perceived and chain-confirmed latency:

| Segment | Target | Notes |
| --- | --- | --- |
| Quote API p50/p99 | < 25 ms / < 100 ms | Local policy/asset/split cache; no chain RPC on cache hit. |
| Transaction compilation | < 10 ms / < 40 ms | Precomputed PDAs and account metadata. |
| Wallet handoff | user-controlled | Render immediately; do not hide signing latency. |
| Submit acknowledgement | < 150 ms / < 500 ms | RPC admission, not confirmation. |
| Processed confirmation | < 1 s / < 3 s | Network and RPC dependent; report actual telemetry. |
| Finalized projection | < 5 s / < 15 s | Backfill and reconciliation may be slower. |

Targets are engineering goals, not Solana guarantees. Measure by cluster, RPC provider, geography, transaction format, and priority fee.

## Critical-path rules

1. Keep the settlement account list bounded by a versioned split-plan maximum.
2. Pre-create recipient ATAs and vault token accounts during merchant onboarding where appropriate; do not create accounts in the checkout hot path unless explicitly budgeted.
3. Prefer one settlement instruction with CPIs over multiple user-submitted transactions.
4. Use exact integer arithmetic and precomputed policy digests.
5. Cache blockhashes and refresh before expiry; avoid durable nonces in the common path.
6. Simulate only when the result adds value; never make simulation the correctness authority.
7. Build a bounded fee/compute policy from recent observed data and cap it server-side.
8. Submit to a regional RPC pool with health scoring and bounded retries. Never mutate a signed transaction during retry.
9. Use v1 transactions when the account set fits the larger inline message envelope; use v0 plus Address Lookup Tables only for an explicitly supported larger account set. The compiler must understand both formats.
10. Keep NGINX and the API out of the authoritative ledger path; they prepare and observe, not settle.

## RPC delivery

- Maintain at least two independent RPC endpoints and one read-only fallback.
- Pin commitment level per operation: `processed` for UX, `confirmed` for operational status, `finalized` for accounting and release decisions.
- Track blockhash age, send latency, slot lag, retry rate, compute-unit consumption, priority fee, and dropped/replaced transaction rate.
- Do not use `getTransaction` polling as the only ingestion method. Combine subscriptions with periodic slot-based backfill.
- Detect provider disagreement and mark status `ReconciliationRequired` rather than guessing.

## Concurrency and contention

The main contention risks are a shared merchant config, shared policy registry, and shared vault state. Reduce write-lock contention by:

- versioning immutable policies and split plans;
- sharding vault and receipt PDAs by merchant and asset;
- not using one global counter account for all settlements;
- deriving deterministic receipt accounts from merchant + nonce;
- bounding recipients and account metas;
- treating merchant configuration changes as control-plane operations outside checkout.

## Failure behavior

- Stale policy: reject before token CPI.
- Token-program mismatch or extension mismatch: reject before token CPI.
- Recipient mismatch: reject before token CPI.
- Insufficient funds: transaction fails atomically.
- RPC timeout: status remains unknown until signature lookup and backfill resolve it.
- Indexer gap: halt projection advancement for the affected range and replay.
- Database outage: checkout can still use a previously signed client transaction; API quote/submit behavior is explicit and fail-closed for new quotes.

