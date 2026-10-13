# ADR-0372: Bounded operational observability dashboard

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal Low-Latency Blockchain Architecture and Platform Operations
- Scope: Local and release-review Grafana dashboard configuration

## Context

The repository already provisions Prometheus, Alertmanager, OpenTelemetry, and
Grafana, but a minimal service overview did not expose the safety signals an
operator needs to triage a Tax Engine system: protocol pause state, replay backlog,
database pool pressure, tax-rule cache behavior, and settlement-provider health.
Dashboard queries must remain non-authoritative and must not create high-
cardinality series or disclose tenant, merchant, wallet, transaction, or
correlation identifiers.

## Decision

Keep one read-only, repository-provisioned service dashboard and extend it to
nine fixed operational panels. Queries use only the existing bounded metrics
and fixed service/status/provider-index labels. Dashboard variables remain
disabled. The dashboard is an operational signal surface; it cannot authorize
settlement, prove finality, or replace append-only evidence and runbooks.

Add `scripts/verify_grafana_dashboard_contract.mjs` and run it in `make check`.
The contract validates the read-only dashboard, required panels and metrics,
empty variable set, and absence of sensitive identity labels.

## Alternatives considered

| Alternative | Triage value | Cardinality/authority risk | Decision |
| --- | --- | --- | --- |
| Keep the four-panel service overview | Low; hides safety conditions | Low | Rejected |
| Add per-tenant or per-wallet dashboard variables | High for one tenant | High; leaks identifiers and creates unbounded series | Rejected |
| Treat Grafana as a settlement/read-model authority | Superficially convenient | Unacceptable financial-integrity risk | Rejected |
| **Use one read-only fixed-cardinality safety dashboard** | **High for incident triage** | **Bounded and non-authoritative** | **Selected** |

## Invariants

- Dashboard configuration is immutable/read-only and variables are empty.
- Queries use fixed-cardinality service, status, or provider-index labels only.
- Sensitive identifiers never appear in dashboard configuration.
- A dashboard signal cannot authorize, finalize, reconcile, or mutate funds.
- Production dashboards still require authenticated scrape access, retention,
  alert delivery, ownership, and load-derived SLO evidence.

## Verification

- `make grafana-dashboard-contract`
- `make observability-config`
- `make check`
- `make alert-delivery-drill`

