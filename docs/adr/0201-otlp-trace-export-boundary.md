# ADR-0201: Bounded OTLP trace-export boundary

## Status

Accepted — implementation begins with the shared Rust service bootstrap. A
live authenticated collector and production backend remain deployment gates.

## Context

The repository already exposes bounded Prometheus metrics and provisions an
OpenTelemetry Collector in the Docker observability profile. The API, worker,
indexer, and route-verifier binaries currently initialize JSON logging only,
so a production operator cannot correlate request spans across the service
boundary or verify that the documented OTLP path is executable.

Telemetry must not become a settlement, tax, database, wallet, or
reconciliation dependency. A collector outage must not delay or reject a
request, change a tax result, or alter chain authority. At the same time,
telemetry configuration must be bounded, explicit, and safe for public
deployments.

## Decision

1. Centralize tracing initialization in the `observability` crate and use it
   from every long-running Rust service binary. Keep JSON stdout logging and
   the existing fixed-cardinality Prometheus endpoint.
2. Add opt-in OTLP trace export using the current compatible Rust stack pinned
   in the workspace: `opentelemetry`/`opentelemetry_sdk`/`opentelemetry-otlp`
   `0.33.0` and `tracing-opentelemetry` `0.34.0`. Use OTLP gRPC to the
   collector, a bounded batch processor, and a dedicated background export
   thread. The application critical path never waits for export.
3. Enable export only when
   `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` is non-empty. Explicit `http://`
   endpoints require `OTEL_ALLOW_INSECURE_TELEMETRY=true` and are for local
   Docker only; production endpoints must use TLS. Invalid configuration
   fails service startup before accepting traffic. Export failures after
   startup are logged and dropped by the bounded exporter path.
4. Use `OTEL_TRACE_SAMPLE_RATIO` with a finite range of `0.0..=1.0`, default
   `1.0` when export is enabled, and a parent-based ratio sampler. Bound the
   queue, batch size, export timeout, and scheduled delay through validated
   configuration with conservative defaults. No user-controlled value becomes
   a metric or span label.
5. Set `service.name` from the binary-owned service name (`api`, `worker`,
   `indexer`, or `route_verifier`) and keep the API-owned UUIDv7 request ID in
   the existing request span. Do not add tax facts, PII, private keys,
   evidence plaintext, signed bytes, bearer tokens, or full request bodies to
   spans.
6. Shut down the provider with a bounded flush during graceful service exit.
   A flush failure is operational evidence only and must not roll back or
   mutate authoritative state.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| **Shared `tracing` layer with opt-in OTLP gRPC** | Reuses existing spans/logging, bounded async export, one policy for all services, collector portability | Adds pinned telemetry dependencies and a startup/configuration surface | **Selected** |
| Export Prometheus metrics through OTLP and remove `/metrics` | One protocol | Duplicates or changes existing scrape contracts and weakens local alert compatibility | Rejected; Prometheus remains the metrics authority |
| Synchronous HTTP/gRPC export in request handlers | Simple delivery semantics | Adds exporter latency, failure coupling, and retry amplification to money/tax paths | Rejected |
| Service-specific tracing implementations | Local flexibility | Configuration drift, inconsistent redaction, and duplicated lifecycle code | Rejected |
| Collector-only stdout scraping | No SDK dependency | No structured span context or cross-service trace semantics | Rejected |

## Invariants and failure behavior

- Export is bounded by a finite queue, finite batch, finite timeout, and
  bounded exporter worker; no unbounded channel or retry loop is allowed.
- Telemetry is never authoritative for tax, settlement, ledger, PostgreSQL,
  authorization, or reconciliation decisions.
- No endpoint is contacted when OTLP is disabled. Invalid enabled
  configuration fails closed at startup; later transport failure fails open to
  business traffic and remains visible in service logs/collector health.
- Service names and configuration-derived attributes are fixed allow-listed
  values. User input cannot create span names, metric labels, or resource
  cardinality.
- Shutdown flush is bounded and best-effort; it cannot change committed
  evidence or chain state.

## Verification and release gates

- Unit tests cover disabled configuration, ratio/queue/batch/timeout bounds,
  insecure endpoint policy, service-name allow-listing, and malformed values.
- Rust service startup tests/builds cover the shared bootstrap; the local
  observability profile must accept OTLP gRPC traffic without changing the
  API request contract.
- A Docker collector evidence drill must prove one request span reaches the
  collector with the API-owned UUIDv7 correlation field and without sensitive
  financial payload fields.
- Production collector authentication, TLS, backend retention, sampling
  calibration, dashboards, alert routing, and on-call response remain open
  deployment gates.

## Consequences

The documented observability architecture becomes executable across every
Rust service while preserving low-latency and fail-open business behavior.
The repository gains several pinned dependencies and must maintain exporter
compatibility as OpenTelemetry Rust evolves. OTLP receipt is evidence of
telemetry transport, not evidence of settlement finality or tax compliance.
