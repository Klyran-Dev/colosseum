# ADR-0135: Publish SLO, error-budget, incident, and dependency controls

- Status: Accepted
- Date: 2026-09-27
- Owners: Platform/SRE, Settlement, Tax, Security, Finance Operations

## Context

ADR-0021 establishes SLOs and error budgets as release controls, but the
repository did not yet contain a single operational contract with explicit
targets, error-budget policy, incident severity, recovery objectives, service
ownership, and dependency exit paths. A Tax Engine financial system cannot leave
those decisions implicit in dashboards or deployment-specific tribal
knowledge.

## Decision

Publish and maintain three versioned operational artifacts:

1. [`slo-error-budget-policy.md`](../operations/slo-error-budget-policy.md)
   defines user-journey SLOs, measurement windows, exclusions, error-budget
   burn policy, RTO/RPO targets, ownership, and release gates.
2. [`incident-severity-and-error-budget.md`](../runbooks/incident-severity-and-error-budget.md)
   defines severity, command roles, immediate containment, evidence handling,
   communication, and recovery requirements.
3. [`dependency-register.md`](../operations/dependency-register.md) records
   every external authority or operational dependency, its failure behavior,
   data boundary, owner, replacement path, and review cadence.

The authoritative unit of measurement is the user journey and financial
control, not an individual process. Metrics may be aggregated by region,
cluster, provider, and environment only when the label set is bounded and the
aggregation cannot hide a tenant or asset-specific safety failure.

Financial-integrity SLOs are zero-tolerance controls: an unauthorized outflow,
conservation mismatch, or accepted transaction without durable evidence is a
release-blocking incident regardless of the availability error budget.

Operational artifacts distinguish declared targets from measured evidence. A
target is never reported as achieved until a retained run includes workload,
region, provider, percentile, sample period, and resource profile.

## Alternatives considered

| Alternative | Why rejected |
| --- | --- |
| Leave targets in architecture prose and dashboards | No single reviewable contract, unclear owners, and dashboards can drift from release policy. |
| Use only infrastructure availability SLOs | Hides quote correctness, finality, reconciliation, and financial-safety failures behind a green HTTP metric. |
| Adopt a vendor-specific incident or status platform as authority | Creates lock-in and makes the public operating contract non-reproducible; integrations may be added behind the documented roles. |
| Treat all dependency failures as equivalent | Settlement, tax calculation, indexing, and UI dependencies have different fail-closed and recovery semantics. |
| Allow availability budget to trade away financial integrity | Incorrect or unauthorized money movement is never an acceptable availability trade-off. |

## Consequences

- Every production deployment must bind the generic owners and dependency
  classes to named teams, escalation routes, and approved providers.
- Error-budget burn can pause feature releases while preserving incident,
  security, and correctness work.
- RTO/RPO targets become testable release evidence rather than aspirations.
- The register makes provider substitution and vendor exit a design concern;
  it does not authorize a provider, key ceremony, or production deployment.
- Local Compose validation proves document shape and metric contracts only; it
  does not prove live paging, regional SLOs, or recovery objectives.

## Evidence and review

- `make public-docs-check` verifies that the artifacts exist and are linked.
- `make observability-config` verifies the Prometheus rule syntax separately.
- Production readiness requires retained SLO reports, alert-delivery tests,
  restore/failover drills, and named accountable owners.
- Review this ADR and the three operational artifacts quarterly and after any
  material provider, program, data-retention, or SLO change.
