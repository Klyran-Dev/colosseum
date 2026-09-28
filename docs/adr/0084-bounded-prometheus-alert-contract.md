# ADR-0084: Versioned, bounded Prometheus alert contract

- Status: Accepted
- Date: 2026-09-26
- Owners: Platform/SRE
- Scope: API, worker, indexer, and local observability deployment

## Context

The services already expose bounded Prometheus metrics and the Compose profile
provisions Prometheus and Grafana. That is useful instrumentation, but it is
not an operational contract: a dashboard can be healthy while a service is
down, an error budget is burning, or the quote/compile latency budget is being
exceeded. Alert expressions must also be reviewed as code. An invalid rule
file must fail a build rather than silently disable paging after deployment.

The alert layer must not become a new source of financial truth. It observes
service and projection health only. Settlement authority remains the Solana
ledger and verified program evidence.

## Decision

Maintain a versioned Prometheus rule file under
`ops/observability/alert-rules.yml` and load it through the pinned Prometheus
container. The rule contract contains only metrics with bounded labels that
are emitted by the repository today:

- exporter/service availability;
- aggregate request error ratio;
- quote and settlement-compile p95 latency;
- rule-cache saturation as an informational signal only when the bounded
  cache-entry gauge is present.

Each rule declares an owner, severity, runbook URL, and stable alert name.
Alerts use `for` windows to suppress single-sample noise and `clamp_min` in
ratios to avoid division by zero. Expressions must not contain tenant IDs,
wallet addresses, transaction signatures, policy digests, or other
unbounded/user-controlled labels.

`promtool check config` and `promtool check rules` run against the exact
Prometheus image digest used by Compose. CI and the repository check target
must fail closed when either validation fails. The local profile exposes the
same rules to Prometheus and Grafana; production deployment must additionally
bind the validated rules to an approved Alertmanager/PagerDuty/on-call
receiver through the deployment environment. A local dashboard or a
successful rule parse is not evidence that production paging is configured.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Dashboard-only operational visibility | Rejected | Requires a human to notice a failure and provides no measured paging contract. |
| Application-specific alert code in each Rust service | Rejected | Duplicates policy, increases hot-path work, and couples financial services to notification delivery. |
| Alert rules embedded only in a managed cloud platform | Rejected | Not reproducible from a clean public checkout and weakens provider exit options. |
| **Versioned Prometheus rules validated by pinned `promtool`** | **Selected** | Declarative, reviewable, bounded, low overhead, and testable in CI. |

## Invariants and failure behavior

1. A malformed rule or Prometheus configuration blocks the repository check and
   deployment pipeline.
2. Missing service scrapes produce an availability alert; they never look like
   zero traffic or zero errors.
3. A zero-request window cannot produce a divide-by-zero error alert.
4. Alert expressions have bounded label cardinality and never expose sensitive
   evidence.
5. Alerting is advisory operational evidence. It cannot authorize, retry, or
   mark a settlement successful.
6. Alert rules are not a substitute for chain reconciliation, finalized
   evidence, or incident response runbooks.

## Operational consequences

- Rule changes require an ADR/alternative review when they change SLO meaning,
  severity, ownership, or alert cardinality.
- Production receivers, silences, escalation policy, and notification delivery
  remain environment-owned and require a controlled deployment configuration.
- The local rule-validation command is deterministic and does not contact a
  third-party paging provider.

## Evidence

- `ops/observability/alert-rules.yml`
- `ops/observability/prometheus.yml`
- `scripts/verify_observability_config.sh`
- `.github/workflows/ci.yml`
