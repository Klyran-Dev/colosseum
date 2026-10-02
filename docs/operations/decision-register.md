# Tier-1 decision register

Status: `OPEN` decisions are not approvals and must not be used to enable
production traffic. This register is the operational companion to
[ADR-0251](../adr/0251-accelerated-tier1-delivery-and-decision-freeze.md).
Every frozen choice must have an accountable role, written acceptance
criteria, retained evidence, and an explicit revisit trigger.

The repository deliberately contains no invented vendor, legal, licensing, or
custody choice. A decision becomes `FROZEN` only after the accountable owner
records the decision, evidence location, approval date, and supersession or
rollback path in the relevant ADR and production-readiness gate.

## Decision states

| State | Meaning | Allowed system behavior |
| --- | --- | --- |
| `OPEN` | No approved choice exists | Local synthetic fixtures only; fail closed for non-local enablement |
| `EVIDENCE_REQUIRED` | A candidate exists but proof is incomplete | Contract tests and staging preparation only; no production traffic |
| `FROZEN` | Owner and required reviewers approved the decision with evidence | Eligible for controlled staging or production consideration, subject to all other gates |
| `SUPERSEDED` | A later decision replaces this one | Historical evidence remains immutable; new traffic follows the replacement |

## Accelerated-delivery decisions

| ID | Decision | State | Accountable role | Required evidence before `FROZEN` | Revisit trigger |
| --- | --- | --- | --- | --- | --- |
| `DEC-0251-01` | Primary FX provider and bounded backup/allowlist | `OPEN` | Provider/Tax/Legal owner | Contract, data-processing terms, provider identity keys, freshness/error SLOs, regional failover test, legal acceptance | Provider outage, contract change, legal/rate-policy change |
| `DEC-0251-02` | Supported jurisdictions, tax regimes, currencies, and Solana assets | `OPEN` | Legal/Tax owner | Published coverage matrix, rule provenance, effective windows, legal/tax owner attestation, golden vectors, asset/token-extension review | Law/rule/asset change or failed reconciliation |
| `DEC-0251-03` | FX and tax evidence legal owner and approval workflow | `OPEN` | Legal/Tax owner | Named accountable organization, four-eyes review, signed provenance, retention/deletion policy, incident escalation path | Ownership change, jurisdiction expansion, audit finding |
| `DEC-0251-04` | Staging topology, regions, Linux/amd64 validator target, and data residency | `OPEN` | Platform/SRE owner | Immutable environment manifest, region/resource profile, network/TLS identity, synthetic-data proof, validator capability evidence | Region, residency, platform, or cluster change |
| `DEC-0251-05` | RPC provider set and read/submission failover policy | `OPEN` | Settlement/SRE owner | Provider contracts, commitment/finality behavior, immutable-byte retry evidence, disagreement/reconciliation drill, regional p50/p95/p99 | RPC outage, provider protocol/price change, SLO breach |
| `DEC-0251-06` | Quote, compile, signer, submission, and finality SLO budgets | `OPEN` | Principal Architect + SRE | Workload definition, warm-up/sample size, p50/p95/p99 by region/provider, CPU/memory/connection profile, error budget | Material workload, topology, dependency, or traffic change |
| `DEC-0251-07` | Evidence retention, privacy, deletion, and export policy | `OPEN` | Security/Privacy owner | Data classification, retention schedule, encryption/key lifecycle, access audit, deletion/restore evidence, legal review | Law, contract, incident, or data-classification change |
| `DEC-0251-08` | Rollback authority, pause authority, and incident commander | `OPEN` | Governance/SRE owner | Named role coverage, approval matrix, emergency-pause and rollback rehearsal, escalation/on-call rota | Authority or staffing change; incident review |
| `DEC-0251-09` | HSM/KMS vendor, key ceremony, rotation, revocation, and recovery | `OPEN` | Security/Governance owner | Vendor/security review, non-exportability proof, ceremony record, dual control, overlap/revocation test, backup/recovery and latency evidence | Key event, vendor change, custody finding, or recovery failure |
| `DEC-0251-10` | Public-source license | `OPEN` | Repository owner | Explicit approved license decision and committed `LICENSE`/`LICENSE.md`; public-repository review | Business, legal, or contribution-policy change |

## Freeze record template

When a decision is approved, add a dated immutable entry to the relevant ADR
or a superseding ADR containing:

```text
decision_id: DEC-...
state: FROZEN
decision: <one precise choice>
accountable_role: <role>
approvers: <roles or approved public-safe identities>
effective_at_utc: <timestamp>
evidence: <public-safe artifact paths or controlled evidence references>
rollback_or_exit: <bounded procedure and authority>
revisit_trigger: <measurable condition>
```

An `OPEN` or `EVIDENCE_REQUIRED` row is a release blocker, not a default. The
absence of a row or evidence reference must fail the launch review rather than
being interpreted as approval.

