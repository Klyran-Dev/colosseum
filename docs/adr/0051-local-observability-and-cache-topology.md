# ADR-0051: Profiled local cache and observability topology

- Status: Accepted for local development
- Date: 2026-09-26
- Owners: Platform / SRE
- Scope: Docker Compose cache, metrics, dashboards, and telemetry services

## Context

The local development contract requires the same operational boundaries that
will exist in deployment: a disposable Redis cache/lease service, a metrics
scraper, dashboards, and an OpenTelemetry ingress. The services must be
reproducible from a clean checkout and must not add a network dependency to
the tax calculation or settlement authority path.

The API, worker, and indexer now expose fixed-cardinality Prometheus-compatible
metrics. A topology is still needed to scrape those endpoints, retain local
time series, provide operator dashboards, and accept future OTLP traces and
metrics without coupling domain code to a telemetry backend.

## Decision

Add an explicit `observability` Compose profile and a `cache` profile:

- Redis 8.6.7 is disposable, bounded to local development, and has no
  authoritative data volume. It is available under both profiles so a cache
  or lease integration can be exercised without starting the dashboards.
- Prometheus 3.13.3 scrapes API, worker, and indexer metrics over the internal
  Compose network. The indexer target is intentionally optional and may be
  down when its profile is not enabled.
- Grafana 13.2.2 is provisioned with Prometheus as a local read-only data
  source and a checked-in service overview dashboard. Local credentials are
  supplied through environment variables and are never committed.
- OpenTelemetry Collector Contrib 0.142.0 receives OTLP gRPC and HTTP on the
  internal network and exports to a bounded local debug sink. It is an
  ingress boundary for future traces and spans, not a synchronous dependency.
- Every external image is pinned by an explicit stable tag and multi-platform
  manifest digest. The digest must be re-resolved through a reviewed
  maintenance change; `latest` is prohibited.
- The profiles use named volumes only for Prometheus and Grafana state. Redis
  persistence is disabled because cache and leases are disposable. Services
  have health checks, resource limits, and restart policy. Host ports are
  explicit and overridable.

The metrics endpoints remain internal deployment concerns. Production must
apply network policy and scrape authentication; local exposure is only for
operator development.

## Alternatives considered

- **Run observability tools on the host:** rejected because it violates the
  Docker-only toolchain contract and causes version drift.
- **Use a single all-in-one monitoring image:** rejected because it obscures
  failure domains, upgrade cadence, and least-privilege boundaries.
- **Send telemetry synchronously to a remote backend:** rejected because a
  telemetry outage must not change quote, settlement, or reconciliation
  behavior.
- **Persist Redis as a local authority:** rejected because Redis is explicitly
  disposable; PostgreSQL and the Solana ledger retain authoritative state.
- **Use floating `latest` tags:** rejected because the local topology must be
  reproducible and image provenance must be reviewable.

## Consequences

Positive:

- a clean checkout has a documented operator-oriented local observability
  path;
- metrics and dashboard configuration are reviewable repository artifacts;
- the telemetry boundary can evolve independently from the pure domain and
  request hot paths; and
- profiles keep the default quote feedback loop lightweight.

Costs and gates:

- local Prometheus data and Grafana state are not production backups;
- no production SLO claim is made until representative load, alert routing,
  scrape authorization, retention, and multi-replica aggregation are tested;
- Collector exporter configuration must be replaced with the deployment's
  approved exporter and retention policy; and
- image scans, signatures, SBOMs, and digest refreshes remain release gates.
