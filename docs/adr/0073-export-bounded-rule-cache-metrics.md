# ADR-0073: Export bounded compiled-rule cache metrics

- Status: Accepted
- Date: 2026-09-26
- Scope: API observability for the immutable tax-rule cache

## Context

The quote adapter uses a bounded digest-keyed LRU cache for compiled immutable
rule graphs. The cache is an optimization only; PostgreSQL remains the source
of truth and every lookup revalidates the published rule release. Internal
hit/miss/eviction counters exist, but operators cannot currently observe cache
pressure or effectiveness from the Prometheus endpoint.

## Decision

Expose fixed-name, fixed-label Prometheus metrics from the API metrics endpoint:

- `stp_tax_rule_cache_hits_total`;
- `stp_tax_rule_cache_misses_total`;
- `stp_tax_rule_cache_evictions_total`; and
- `stp_tax_rule_cache_entries`.

The only label is the constant service name `api`; no merchant, rule ID,
jurisdiction, or request-derived label is permitted. Values are read from
atomic bounded counters and the current bounded entry count. Cache metrics do
not authorize a quote, alter invalidation, or become a financial source of
truth.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| No cache telemetry | Rejected | Prevents tail-latency diagnosis and capacity planning. |
| Request-derived rule/jurisdiction labels | Rejected | Unbounded cardinality and tenant-data disclosure risk. |
| Separate metrics sidecar | Rejected for this boundary | Adds a dependency and can disagree with the process-local cache state. |
| **Fixed-label metrics rendered with the existing API endpoint** | **Selected** | Low overhead, deterministic cardinality, and directly observes the cache that serves requests. |

## Consequences

- Dashboards can alert on miss/eviction ratios and unexpected cache growth.
- Concurrent load, invalidation drills, and p99 evidence remain separate
  performance gates; telemetry alone does not prove latency improvement.
