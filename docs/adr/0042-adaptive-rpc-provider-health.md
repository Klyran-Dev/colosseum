# ADR-0042: Adaptive bounded health scoring for finalized RPC providers

- Status: Accepted for the first multi-provider indexer pool
- Date: 2026-09-26
- Owners: Chain Data / SRE
- Scope: Solana finalized-page provider selection

## Context

ADR-0040 established a bounded failover wrapper, but a static provider order
causes every request to pay the latency of an unhealthy first endpoint. A Tax Engine
indexer must react to transport failures, RPC errors, malformed pages, and
timeouts without creating an unbounded retry storm or weakening commitment
semantics.

## Decision

Extend the bounded failover pool with per-provider atomic health state. At most
three configured providers and at most one bounded deadline per attempted
provider remain in force. Providers are ordered for each page request by a
small saturating health score, with stable configuration order as the tie
breaker. A successful finalized page increments the score and clears the
failure streak. Any failed attempt, timeout, malformed page, or provider RPC
error decrements the score and applies an exponential cooldown capped at five
seconds. If every provider is cooling down, the provider with the earliest
cooldown is tried once; the pool never spins or adds attempts.

Health is process-local operational state, not accounting state. It is stored in
atomics so concurrent fetches do not require a mutex or hold a lock across an
await. It is observable through bounded snapshots for metrics and operator
diagnostics. A health decision can reorder providers only; it cannot downgrade
`finalized`, accept a weaker page, or bypass coordinator validation.

## Alternatives considered

- Static round-robin: rejected because it repeatedly sends work to known-bad
  endpoints and adds tail latency during outages.
- Unbounded circuit-breaker retries: rejected because it can amplify provider
  outages and violate the indexer's bounded work contract.
- Distributed health state in PostgreSQL/Redis: rejected for the request hot
  path; it adds a network hop and turns an operational hint into a dependency.
- External service mesh health only: rejected because provider-specific JSON-RPC
  correctness and finalized-page validity are application semantics.

## Consequences

Positive:

- Healthy providers move ahead of degraded providers without a coordination
  lock or a remote health lookup.
- Cooldowns reduce tail latency and retry pressure during provider outages.
- The finalized-page contract, attempt limit, and fail-closed behavior remain
  unchanged.

Costs and follow-on gates:

- Process-local state resets during deployment; startup uses deterministic
  configuration order until evidence is collected.
- Metrics must expose provider outcome, score, cooldown, and finality errors
  without exposing credentials or endpoint query secrets.
- Fleet-level provider budgets and cross-region quorum policy remain separate
  operational decisions.
