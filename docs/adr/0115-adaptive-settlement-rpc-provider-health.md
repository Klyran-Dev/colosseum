# ADR-0115: Adaptive settlement RPC provider health

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Settlement, and SRE
- Scope: settlement submission, blockhash validation, and signature observation

## Context

The settlement coordinator already bounds provider count, request deadlines,
retry attempts, response shapes, and signed transaction bytes. Its provider
order is currently static, however. A degraded endpoint therefore remains the
first hot-path attempt for every request until process restart, increasing
tail latency and causing avoidable retry pressure.

The indexer has the same operational need, but provider-health behavior must
not fork into subtly different cooldown and score rules. Health is a routing
optimization only: it cannot turn a missing, malformed, stale, or conflicting
chain result into settlement success.

## Decision

Create a small reusable `indexer::AdaptiveProviderHealth` infrastructure
component and use it in both the finalized-page failover path and the
settlement coordinator. The component provides:

- a bounded score from -8 through +8;
- bounded exponential cooldown beginning at 25 ms and capped at 5 seconds;
- atomic success/failure counters and cooldown state;
- deterministic ordering by available status, descending score, then original
  provider index;
- a fallback to the provider with the shortest cooldown when all providers are
  cooling down.

Settlement rules are explicit:

- transport failures, timeouts, malformed responses, and RPC errors penalize
  the provider and permit bounded failover;
- valid provider responses, including a definitive `isBlockhashValid=false`
  result or a valid `unknown` signature status, reward provider health;
- a stale blockhash remains a fail-closed signed-message error and never
  falls through to another provider;
- the same canonical signed bytes are sent on every attempt;
- health affects ordering only and is never durable settlement state or proof
  of finality.

The component remains process-local and disposable. Production deployments
must expose bounded provider-health metrics and combine them with provider,
region, and chain-finality SLOs without using unbounded labels.

## Alternatives considered

### Fixed provider order

Rejected. It is deterministic but repeatedly pays the latency of a known
degraded provider and creates synchronized retry pressure.

### Per-request random provider selection

Rejected. It reduces predictability, complicates evidence and load modeling,
and does not quarantine a failing endpoint.

### Durable global health in PostgreSQL or Redis

Rejected for the hot path. Cross-replica shared state adds network latency,
coordination failure, and stale health races; provider health is advisory and
can be rebuilt after restart.

### Separate health algorithms in indexer and settlement

Rejected. Divergent cooldowns and score semantics make failover behavior hard
to reason about and operate.

### **Shared bounded process-local adaptive health**

Selected. It preserves deterministic bounded behavior, removes avoidable tail
latency after failures, reuses the existing Rust infrastructure boundary, and
does not elevate advisory health into settlement authority.

## Invariants

- No request attempts more than three configured providers.
- No provider cooldown exceeds five seconds and no task waits on a health lock.
- Provider order is deterministic for a fixed health snapshot.
- Health state cannot authorize a transaction, alter signed bytes, or claim
  commitment/finality.
- Definitive stale-blockhash evidence never fails over.
- All provider attempts remain deadline-bounded and cancellation-safe.

## Verification and release impact

- Unit tests prove cooldown ordering, recovery, bounded fallback, stale
  blockhash fail-closed behavior, and immutable bytes across failover.
- Existing finalized-page provider tests continue to pass against the shared
  component.
- This closes process-local adaptive routing evidence only. Live multi-region
  provider failover, RPC vendor SLOs, and production alert delivery remain
  open release gates.
