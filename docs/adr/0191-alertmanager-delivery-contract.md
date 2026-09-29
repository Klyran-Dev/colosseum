# ADR-0191: Alertmanager delivery contract

## Status

Accepted — implement the local delivery evidence path; production receiver
selection and on-call routing remain deployment-owned.

## Context

ADR-0084 defines versioned, bounded Prometheus alert rules, but a rule file
only proves that Prometheus can evaluate an expression. It does not prove that
a firing financial-integrity, protocol-safety, or replay alert leaves the
metrics system, reaches a receiver, preserves its labels and annotations, or
can be tested without coupling notification side effects to settlement code.

The product must fail closed on financial state while making operational
failure visible. Alert delivery therefore needs an explicit trust boundary:
Prometheus and Alertmanager are operational projections, not authorities for
Solana funds, tax calculations, policy publication, or database state. A
receiver outage must not block API, tax, indexer, or settlement work; it must
create observable delivery failure and follow the deployment's bounded retry
and escalation policy.

## Decision

Add Alertmanager as the versioned notification router for the local
observability profile. Prometheus sends alerts to Alertmanager over the
internal Compose network. Alertmanager applies bounded grouping and retry
behavior and sends a webhook to a test-only Rust receiver fixture. The
fixture stores only bounded synthetic event summaries and exposes a health
endpoint plus an administrator test endpoint. It is never imported by
production services and is not a production incident-management system.

The contract requires:

1. Alertmanager configuration, receiver URL, grouping keys, retry behavior,
   and `send_resolved` semantics are versioned repository artifacts.
2. Alert labels and annotations remain bounded and include an alert name,
   severity, owner, summary, description, and runbook where applicable.
3. Prometheus-to-Alertmanager delivery is exercised by inducing a bounded
   worker scrape outage in an isolated stack and proving that the real
   Prometheus rule fires and the receiver observes the expected alert identity
   and status.
4. The delivery drill has a finite deadline, captures logs on failure, uses a
   unique Compose project and named volumes, and tears down only that project.
5. Production deployments replace the fixture with an approved receiver or
   incident platform, authenticate the route, restrict egress, configure
   secrets through the deployment secret provider, and demonstrate alert
   ownership, escalation, deduplication, suppression, and response-time SLOs.
6. No alert, receiver response, webhook payload, or notification retry can
   authorize a settlement, release a vault, publish a rule, mutate a
   projection, or advance a chain checkpoint.

The pinned Alertmanager image is selected from the current stable release at
bootstrap and is recorded by immutable multi-platform manifest digest in
Compose. Image refreshes follow the repository image maintenance policy.

## Invariants and budgets

- A delivery drill cannot change PostgreSQL, Solana, tax-rule, or settlement
  state.
- The receiver accepts at most a bounded request body and bounded in-memory
  event history; it rejects malformed or oversized webhooks.
- Receiver event summaries use fixed-cardinality fields; raw payloads and
  sensitive evidence are not retained by the fixture.
- The drill has one bounded startup deadline and one bounded delivery
  deadline; no retry loop is unbounded.
- Alertmanager is an operational dependency only. Its outage cannot make a
  financial operation appear successful or unsafe state appear safe.
- Production routing must prove receiver authentication, TLS or private
  network isolation, secret rotation, receiver availability, and alert
  delivery metrics before a readiness claim is made.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Service-local notification calls | Immediate failure proximity | Couples hot paths to notification outages, duplicates policy, and expands the settlement trust boundary | Rejected |
| Prometheus rules with no receiver drill | Low operational cost | Proves evaluation only; delivery regressions remain invisible | Rejected |
| Direct Prometheus webhook fan-out | Fewer components | Weak grouping, retry, deduplication, silence, and escalation semantics | Rejected |
| **Prometheus → Alertmanager → approved receiver; bounded local fixture for CI** | Standard operational boundary, deduplication/grouping, explicit receiver contract, reproducible local evidence | Adds one operational component and deployment-owned routing work | **Selected** |

## Verification and rollout

- Add `ops/observability/alertmanager.yml`, Prometheus Alertmanager wiring,
  and a pinned Compose service in the observability profile.
- Add the isolated Rust receiver fixture and
  `scripts/alert_delivery_drill.sh` / `make alert-delivery-drill`.
- Validate Prometheus and Alertmanager configuration in CI with pinned
  upstream validation binaries.
- Retain local/CI delivery evidence as a completed implementation gate while
  keeping production receiver, escalation, on-call acknowledgement, and
  regional alert SLO evidence open until deployment owners provide it.
