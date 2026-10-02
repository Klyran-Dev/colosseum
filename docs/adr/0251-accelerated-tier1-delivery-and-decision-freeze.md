# ADR-0251: Accelerated Tier-1 delivery and decision freeze

- Status: Accepted for planning
- Date: 2026-10-02
- Owners: Principal low-latency architecture, engineering, security, tax/legal
- Scope: Delivery sequencing and release governance for the Tier-1 product

## Context

The product has a broad Tier-1 scope: Solana settlement, deterministic global
tax calculation, exact FX evidence, provider policy, a production React/NGINX
surface, durable evidence, and operational controls. The schedule can be
shortened by running independent evidence-producing work in parallel, but
parallelism must not convert an unresolved legal, authority, security, or
chain question into an implementation assumption.

The proposed planning estimate is a production candidate in 3–4 weeks and a
controlled production launch in 5–7 weeks. This is conditional planning
guidance, not a delivery promise. External provider approval, HSM/KMS
provisioning, Linux validator evidence, independent audit time, and legal
sign-off are outside the team's direct control.

## Decision

Adopt a three-workstream plan with a Day 2 decision freeze. The workstreams
may execute concurrently after their interfaces, owners, and evidence
contracts are recorded. The Tier-1 definition of done, security gates, legal
approval, conformance requirements, and production-readiness evidence are not
reduced or waived by this schedule.

### Day 2 decision freeze

By the end of Day 2, the release scope must identify and obtain named-owner
approval for:

1. the initial FX provider set, backup provider policy, provider identities,
   and HSM/KMS signing boundary;
2. the initial supported jurisdictions, assets, mint/token-program policy,
   and explicit exclusions;
3. the legal/tax policy owner and escalation path for rule acceptance;
4. the staging environment, Linux/amd64 validator target, regions, RPC
   providers, and observability destination; and
5. the release SLOs, evidence retention, rollback authority, and incident
   commander rotation.

The freeze prevents implementation against undefined authority. A material
change after the freeze requires an ADR or ADR amendment, impact analysis,
updated contracts, and re-evaluation of affected evidence. It does not prevent
normal bug fixes that preserve the approved contracts and invariants.

### Parallel workstreams and accountable owners

| Workstream | Accountable owner | Parallel scope | Required evidence |
| --- | --- | --- | --- |
| Blockchain and conformance | Blockchain/conformance lead | Linux/amd64 SBF artifact, pinned Agave validator, wallet matrix, legacy SPL and extension-free Token-2022 flows, provider failover and finalized-chain reconciliation | Reproducible validator run, transaction fixtures, finalized signatures/slots, account and amount-conservation proofs, failover report |
| Platform and security | Platform/security lead | HSM/KMS ceremony, Linux/amd64 CI, image/dependency/SBOM pipeline, PostgreSQL backup/restore, observability, threat model, rollback, incident rehearsal | Key custody and rotation record, CI artifacts, scan report, restore checksum, dashboards/alerts, signed drill reports, security review disposition |
| Provider, tax, and legal integration | Provider/tax/legal lead | FX adapter and provider SLOs, supported jurisdiction/asset matrix, rule ownership, legal acceptance, evidence provenance, policy publication and rollback | Provider approval, signed FX evidence, rule review record, jurisdiction coverage matrix, policy digest, legal sign-off, open limitations |

Each workstream owns its artifacts but cannot declare production readiness by
itself. The Principal architect is the integration approver for cross-context
invariants, and the release authority records the final go/no-go decision.

## Stage gates and planning estimate

| Window | Gate | Exit evidence | No-go conditions |
| --- | --- | --- | --- |
| Days 1–2 | Decision freeze | Approved provider/jurisdiction/asset/legal/staging/SLO register | Any undefined authority, unsupported legal scope, or unowned critical dependency |
| Week 1 | Integration readiness | Provider and HSM/KMS interfaces, Linux/amd64 CI, pinned images, dashboards, local/staging topology | Keys not provisioned or controlled, missing deployment environment, unbounded dependency in the settlement path |
| Weeks 2–3 | Parallel conformance and performance | Wallet, validator, Token-2022, failover, regional latency, load, and reconciliation evidence | Unknown provider outcomes, RPC disagreement, failed finalized evidence, SLO budget breach, or unsupported extension behavior |
| Weeks 3–4 | Audit and operational readiness | Independent program/security audit, threat-model disposition, restore/rollback/incident rehearsals | Critical or high unresolved findings, failed restore/rollback, missing on-call or alert path |
| Weeks 5–6 | Staging canary and launch review | Canary report, evidence review, legal approval, release artifact/SBOM/provenance, staffed incident plan | Missing legal/provider approval, incomplete evidence chain, failed canary, or unreviewed exception |

The resulting planning estimate is 3–4 weeks to a production candidate and
5–7 weeks to controlled production. The estimate automatically extends when an
external prerequisite or release gate is incomplete; schedule pressure never
changes a fail-closed decision into a pass.

## Non-negotiable release gates

Controlled production is prohibited until all applicable items below have
reviewed evidence:

- exact release commit built and exercised by the Linux/amd64 validator and
  Solana conformance workflow;
- live wallet-signing and transaction-message verification matrix;
- legacy SPL and extension-free Token-2022 conformance, with unsupported
  extensions rejected explicitly;
- measured regional p50/p95/p99 quote, compile, submit, processed, confirmed,
  and finalized latency with workload, RPC, commitment, and resource profile;
- bounded RPC/provider failover, gap backfill, reconciliation, and unknown
  outcome handling;
- HSM/KMS key ceremony, separation of duties, rotation, revocation, recovery,
  and audit evidence;
- independent Solana program and security review with critical/high findings
  closed or an explicitly approved time-bound exception;
- PostgreSQL backup/restore verification, migration rollback plan, and tested
  operational runbooks;
- production observability, alert delivery, staffed on-call, incident
  commander, and communications plan;
- provider commercial/technical approval, signed FX evidence, tax-rule owner
  review, supported-jurisdiction matrix, and legal sign-off; and
- canary results, release integrity hashes, SBOM/provenance, public-safe
  documentation, and an explicit recorded go/no-go decision.

No document, dashboard, or UI may say “production-ready,” “compliant,” or
“supported” beyond the evidence-backed scope. Automated withholding remains a
calculation and settlement capability, not filing, reporting, or legal advice.

## Alternatives considered

- **Run sequentially:** easier coordination, but unnecessarily extends the
  schedule and delays independent evidence. Rejected.
- **Start all implementation immediately and decide scope later:** fastest
  apparent start, but creates rework and unsafe authority assumptions. Rejected.
- **Parallelize without a freeze or named evidence owners:** superficially
  fast, but makes integration and release accountability ambiguous. Rejected.
- **Parallelize behind a Day 2 freeze with unchanged gates:** preserves
  decision quality, exposes external blockers early, and gives a measurable
  path to the requested estimate. Selected.

## Consequences

Positive consequences include earlier discovery of provider, legal, key-custody,
validator, and regional performance blockers; lower schedule risk through
independent execution; and an evidence packet that can be reviewed by release
rather than reconstructed afterward.

Costs include more coordination overhead, a required decision register, clear
artifact ownership, cross-workstream integration reviews, and a higher risk of
merge contention. The release authority must protect the freeze and reject
pressure to relabel missing evidence as “follow-up work.”

## Required artifacts

The implementation plan must maintain the decision register, RACI/owner list,
scope matrix, latency report, provider SLO report, conformance report, key
ceremony record, threat-model disposition, restore/rollback drill reports,
canary report, legal approval, and final go/no-go record. Every artifact must
identify the commit, environment, inputs, timestamps, operator, and evidence
retention location.
