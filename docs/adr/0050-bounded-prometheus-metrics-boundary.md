# ADR-0050: Bounded Prometheus-compatible service metrics

- Status: Accepted for the first observability slice
- Date: 2026-09-26
- Owners: Platform / SRE
- Scope: Rust service request and operation telemetry

## Context

The low-latency contract requires measured request latency, error rate, and
tail behavior. Structured logs alone cannot provide percentile histograms or a
stable scrape contract. Adding an unconstrained metrics registry would create
an accidental memory and cardinality attack surface, especially if operation
names or labels were derived from requests.

## Decision

Create a small `observability` crate with a fixed operation set supplied at
service construction. It records request count, error count, and bounded
millisecond histogram buckets using atomics and renders Prometheus text
exposition without a broker, database query, or allocation on each observation
beyond the scrape path. Unknown operations fail closed and cannot create new
series. The first histogram buckets are 10, 25, 50, 100, 250, 500, and 1,000
milliseconds plus `+Inf`.

The API exposes `/metrics` alongside health endpoints and records the fixed
`tax_calculate`, `tax_document_calculate`, and `tax_quote` operation
durations. Metrics are operational telemetry only; they never influence tax
results, authorization, settlement, or readiness. Production deployments must
place the endpoint behind network policy or a dedicated internal listener and
configure scrape authentication outside the domain path.

OpenTelemetry traces and a full collector/dashboard topology remain follow-on
work. This slice establishes the bounded metric contract first so service
instrumentation and SLO tests have a stable target.

## Alternatives considered

- Logs-only latency measurement: rejected because percentile aggregation and
  alerting are too expensive and fragile to reconstruct from logs.
- Dynamic labels from request data: rejected because merchant, rule, error
  text, signature, or URL labels would create high-cardinality memory and cost
  failures.
- A remote metrics service in the request path: rejected because telemetry
  must not add network latency or a new failure mode to quote/settlement.
- A heavyweight metrics dependency in the pure tax engine: rejected because
  domain evaluation must remain dependency-free and deterministic.

## Consequences

Positive:

- request latency and error histograms are scrapeable and bounded;
- metric cardinality is fixed by construction;
- observation uses atomics and does not block async tasks; and
- tests can assert the exact exposition contract.

Costs and follow-on gates:

- the API slice does not yet cover worker/indexer operation metrics;
- `/metrics` requires deployment-level access control and scrape policy;
- OpenTelemetry collector integration, dashboards, alert routing, and
  multi-replica aggregation remain production-readiness work; and
- p50/p95/p99 evidence still requires representative load tests.
