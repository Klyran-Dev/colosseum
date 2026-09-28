# ADR-0049: Bounded digest-keyed compiled-rule cache

- Status: Accepted
- Date: 2026-09-26
- Owners: Tax Platform / Performance
- Scope: Published-rule quote application boundary

## Context

ADR-0048 provides an immutable `CompiledRuleSet`, but the persisted quote path
still parses and compiles the same published rule for every request. The
database remains the authority for publication status, effective windows, and
the stored canonical digest; however, repeated compilation adds avoidable tail
latency and allocations after the rule document has been validated.

The cache must never allow stale or mutated tax rules to become authoritative.
It must be bounded under adversarial rule churn, safe under concurrent quote
requests, and observable without adding a metrics dependency to the domain
crate.

## Decision

Add a process-local bounded LRU cache at the PostgreSQL application adapter.
After the transaction loads a published rule document and verifies its stored
digest against the canonical digest, the adapter obtains a compiled rule from
the cache using the 32-byte canonical digest as the key. A miss compiles the
validated document and inserts an immutable `Arc<CompiledRuleSet>`; a hit
reuses the artifact. Cache entries contain no mutable database handles and
are safe to discard at any time.

The cache has a fixed capacity of 256 entries for the first slice. Insertion
evicts the least recently used entry. The adapter exposes bounded counters for
hits, misses, evictions, and current entries so the service layer can publish
metrics and alert on churn or a sudden hit-rate regression. Cache locking is
short, synchronous, and never held across an await or database operation.

The quote transaction still queries the published rule on every request. This
preserves effective-window, retirement, and database authorization semantics;
the cache only removes deterministic compilation work. A future control-plane
rule-release notification may optimize lookup separately, but it requires a
new consistency ADR.

## Alternatives considered

- No cache: rejected because published immutable rules make repeated graph
  compilation unnecessary on the quote hot path.
- Unbounded process map: rejected because an authenticated control-plane caller
  could create unbounded rule churn and exhaust memory.
- Cache by rule ID/version alone: rejected because the canonical digest is the
  integrity boundary and must distinguish changed or corrupted documents.
- Redis as the compiled-rule cache: rejected for the first slice because a
  network hop and serialization would increase quote tail latency; Redis may
  be used later as disposable cross-replica coordination, never as tax
  authority.
- Cache the rule lookup and effective-window decision: rejected because it
  would complicate publication/retirement consistency and requires explicit
  invalidation semantics.

## Consequences

Positive:

- repeated quotes reuse validated dependency order and canonical digest;
- memory use is bounded and cache churn is observable;
- database publication/effective-window semantics remain authoritative; and
- the pure tax-engine crate remains free of storage and concurrency concerns.

Costs and follow-on gates:

- each replica has its own cache and must expose metrics with instance labels;
- representative concurrent load must prove p50/p95/p99 improvement and no
  lock contention regression; and
- rule publication workloads must test digest changes, retirement, expiry, and
  bounded eviction before a production latency SLO is claimed.
