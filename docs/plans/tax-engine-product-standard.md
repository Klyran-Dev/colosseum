# Tax Engine product standard

## Definition

Tax Engine is this repository's internal product-quality and evidence standard for trusted financial infrastructure, not an external certification and not a statement that the current implementation has met every requirement. A Tax Engine release must be safe to operate under normal load, degraded dependencies, adversarial input, rule changes, operator error, and incident response. It must be explainable to users, reviewers, auditors, and public contributors.

Tax Engine does not mean universal legal coverage by default. Coverage is jurisdiction-, tax-regime-, product-, and effective-date-specific, with provenance and an explicit review status.

## Product pillars and evidence

| Pillar | Tax Engine requirement | Evidence required |
| --- | --- | --- |
| Financial integrity | Amount conservation, atomic settlement, deterministic tax, no unauthorized vault movement | On-chain invariants, property tests, audit, reconciliation reports |
| Tax coverage | Explicit jurisdiction/rule coverage, provenance, effective dates, scenario vectors, legal/tax ownership | Published coverage matrix, rule bundles, review history, replayable calculations |
| Security | Threat model, least privilege, key separation, secure SDLC, disclosure process | Audit reports, key ceremony, SBOM, scan results, incident runbooks |
| Reliability | SLOs, error budgets, multi-RPC strategy, backfill, graceful degradation, DR | Dashboards, load/chaos results, RTO/RPO drill evidence |
| Performance | Measured p99 budgets for quote, calculation, compile, submit, and finalization | Benchmark reports with workload, region, provider, and resource profile |
| UX trust | Explainable totals, accessible workflows, explicit status, no misleading compliance claims | Usability/accessibility tests, design review, localization checklist |
| Compatibility | Versioned API/IDL/rule schemas, deprecation policy, migration and rollback | Compatibility matrix, contract tests, migration rehearsal |
| Operations | On-call ownership, runbooks, alerts, audit logs, release provenance | On-call rotation, runbooks, alert tests, signed artifacts |
| Public repository | Clear scope/status, reproducible development, contribution/security policies, licensing decision | README, AGENTS, ADRs, CI, SECURITY, CONTRIBUTING, changelog |
| Business continuity | Backups, restore verification, key recovery, provider/vendor exit plan | Restore and recovery drills, export format, dependency register |

## Product lifecycle

The product lifecycle is explicit and never uses “MVP” to excuse incomplete financial behavior:

1. **Architecture:** design, alternatives, ADRs, threat model, domain contracts.
2. **Development:** production-quality vertical slices on localnet and Docker.
3. **Pre-production:** staging with synthetic data, independent testing, failure drills, and security review.
4. **Controlled production:** restricted merchants/assets/jurisdictions, monitored error budget, staffed incident response.
5. **General availability:** approved coverage matrix, audit closure, DR evidence, public release artifacts, and support process.

## Tax coverage governance

The product must publish a coverage matrix with, at minimum:

- jurisdiction and tax regime;
- product/service categories;
- place-of-supply assumptions and required facts;
- tax components and ordering;
- rate/bracket/threshold version;
- inclusive/exclusive semantics;
- exemption and evidence requirements;
- rounding/truncation and residual behavior;
- effective window and review date;
- legal/tax owner and provenance;
- known limitations and unsupported cases.

Unsupported or ambiguous cases must return a clear non-settlement status or require merchant review. They must not silently fall back to a nearby jurisdiction or default rate.

## Financial operations

- Reconcile tax vault balances, settlement receipts, calculation totals, and external release claims.
- Make refunds, reversals, corrections, chargebacks, and post-settlement adjustments explicit workflows with new evidence; never mutate historical receipts.
- Maintain an immutable audit trail for policy, rule, calculation, settlement, release, authority, and configuration changes.
- Define who owns tax filing, remittance, registration, customer communication, and evidence retention for each supported product.

## Globalization and usability

- Use locale-aware formatting without changing atomic calculation semantics.
- Support currencies, currency precision, time zones, daylight-saving transitions, calendar boundaries, language, RTL layouts, and accessibility.
- Do not expose sensitive jurisdiction evidence to recipients or logs unnecessarily.
- Present tax results in plain language, with drill-down to exact calculation semantics and evidence.

## Launch gates

No production claim or mainnet general availability until:

- all critical/high audit findings are closed or formally accepted;
- coverage matrix and rule provenance are approved;
- golden vectors and replay evidence pass;
- SLOs and error budgets are live;
- disaster recovery and key-recovery drills pass;
- API/IDL/rule compatibility is tested;
- Docker/CI builds are reproducible and artifacts are signed;
- public repository documentation is complete;
- support, incident response, security disclosure, and change-management ownership is assigned.

