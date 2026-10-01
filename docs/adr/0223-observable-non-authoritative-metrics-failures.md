# ADR-0223: Observable non-authoritative metrics failures

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Observability, SRE
- Bounded contexts: Service Telemetry, API Operations, Worker Operations

## Context

`ServiceMetrics::observe` intentionally returns an error when a caller uses an
unregistered operation. Metrics are diagnostic and must never become a
dependency of tax calculation, settlement, reconciliation, or outbox
processing. The service paths therefore discarded the result with `let _ =`.
That preserved the financial path but made a programming/configuration defect
in the telemetry contract silent.

## Decision

Add a bounded `observe_or_log` adapter to `ServiceMetrics`. It records the
metric exactly as `observe` does and logs an error when the fixed operation
registry rejects an observation. Migrate API and long-running service call
sites to the adapter.

The adapter does not accept caller-created operation names, retry, allocate a
queue, block on an external system, or return an error to the money-moving
path. `observe` remains available for tests and code that needs explicit
validation.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Continue discarding `observe` errors | Rejected | Hides telemetry-contract regressions and weakens operational diagnosis. |
| Propagate metric errors through financial/API handlers | Rejected | Makes non-authoritative telemetry able to fail quote, settlement, or reconciliation behavior. |
| Add an asynchronous metrics error queue | Rejected | Adds lifecycle, backpressure, and shutdown complexity for a bounded programmer error. |
| **Log the bounded rejection synchronously through a shared adapter** | **Selected** | Preserves the hot-path authority boundary while making unexpected registration defects visible. |

## Invariants and failure policy

- Unknown operation names remain rejected by the fixed registry.
- Metrics failure cannot authorize, reject, retry, or mutate settlement state.
- Logging contains only the bounded service and operation identifiers; no
  merchant, wallet, tax facts, signature, or evidence payload is included.
- The adapter performs no network, database, or unbounded work.

## Verification and release impact

- Unit tests retain explicit `observe` rejection coverage and exercise the
  non-panicking adapter path.
- Source audits require production call sites to use `observe_or_log` rather
  than silently discard `observe` results.
- Metric export, scrape, and alert delivery remain separate production
  operations gates.
