# ADR-0117: Bounded settlement provider-health metrics

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture and SRE
- Scope: API metrics, settlement RPC provider health

## Context

ADR-0115 makes settlement RPC routing adaptive, but a routing decision that
cannot be observed is not supportable in production. Operators need to know
whether a provider is degraded, cooling down, or recovering without exposing
RPC URLs, credentials, merchant identifiers, transaction signatures, or
unbounded provider labels.

## Decision

Expose three fixed-cardinality Prometheus series from the API when settlement
providers are configured:

- `stp_settlement_provider_health_score`
- `stp_settlement_provider_consecutive_failures`
- `stp_settlement_provider_cooldown_ms`

Each series has only `service="api"` and a bounded numeric
`provider_index` label. The coordinator exposes a snapshot of its process-local
health state; the metrics renderer never emits endpoint URLs or request,
merchant, signature, or digest labels. A configured provider pool remains
bounded at three endpoints by ADR-0056/0115.

Metrics are advisory operational evidence. They do not change provider order,
settlement authority, finality, or retry policy, and their absence when no
settlement coordinator is configured is valid.

## Alternatives considered

### Log every provider event only

Rejected as the sole signal. Logs are useful forensic evidence but are harder
to alert on and can create high-cardinality sensitive data under failure.

### Label metrics with endpoint URLs

Rejected. URLs can contain sensitive topology, create unstable cardinality, and
make dashboards and alerts brittle.

### Persist health in PostgreSQL

Rejected. Health is process-local advisory state; persistence adds hot-path
coordination and stale state without improving settlement correctness.

### **Fixed provider-index gauges from an in-memory snapshot**

Selected. It is low overhead, bounded, redacted, scrape-friendly, and aligned
with the adaptive health contract.

## Invariants

- At most three provider indices are rendered.
- No user-controlled or secret-bearing value enters a metric label.
- Scraping metrics never performs an RPC or database call.
- Metric absence cannot be interpreted as settlement success or provider health.

## Verification and release impact

- Unit tests assert fixed names, bounded provider indices, values, and redaction
  of endpoint/merchant/request identifiers.
- The API metrics endpoint includes the series only when a coordinator exists.
- Production scrape authentication, receiver delivery, dashboards, alert
  routing, and load-derived SLOs remain operational release gates.
