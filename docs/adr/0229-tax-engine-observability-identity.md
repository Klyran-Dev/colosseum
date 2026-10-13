# ADR-0229: Canonical Tax Engine observability identity

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Observability, SRE
- Scope: Tax Engine service telemetry identity
- Bounded contexts: Tax Engine, Service Runtime, Observability

## Context

The Tax Engine service uses the fixed Prometheus identity `tax_engine` for its
bounded calculation metrics, while its startup path previously requested the
hyphenated identity `tax-engine`. The shared telemetry configuration allow-list
did not accept either service-specific startup identity. As a result, a normal
Tax Engine process could fail during tracing initialization before binding its
listener, and a future attempt to accept both names would split dashboards,
alerts, and service-level aggregation.

## Decision

Use `tax_engine` as the single canonical identity across:

- tracing resource attributes and exporter configuration;
- fixed-cardinality Prometheus service labels;
- telemetry service-name allow-lists;
- dashboards, alerts, runbooks, and evidence queries.

The service starts tracing with `tax_engine`, and the allow-list accepts this
identity explicitly. The identity is a deployment-controlled constant, never a
request field or user-controlled metric label. `tax-engine`, arbitrary aliases,
and dynamic service names remain rejected.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Accept both `tax-engine` and `tax_engine` | Compatible with either spelling | Splits dashboards, alert selectors, and evidence aggregation | Rejected |
| Permit any syntactically valid service name | Less central configuration | Allows accidental cardinality and unreviewed telemetry identities | Rejected |
| Rename metrics to `tax-engine` | Matches the previous startup string | Requires a broad metric/dashboard migration and violates the existing canonical registry | Rejected |
| **Use the existing `tax_engine` identity everywhere** | Matches fixed metrics, preserves allow-list discipline, and makes startup deterministic | Requires callers and operational queries to use one explicit spelling | **Selected** |

## Invariants and failure behavior

- Tax Engine startup succeeds only with the explicitly allow-listed
  `tax_engine` identity.
- No user input can select or create a telemetry service identity.
- Existing metric names and fixed label cardinality remain unchanged.
- Unknown, hyphenated, or dynamically constructed identities fail closed at
  telemetry configuration validation.
- Telemetry remains non-authoritative: a metrics failure cannot authorize tax,
  settlement, or evidence state.

## Verification and release impact

- A regression test accepts `tax_engine` through the shared telemetry
  configuration and verifies the exact returned identity.
- Tax Engine service tests and clippy run with warnings denied.
- Dashboards and alert rules must use `service="tax_engine"`; any migration
  must be a separately reviewed compatibility change with bounded selectors.
