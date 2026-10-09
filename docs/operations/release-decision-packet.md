# Tier-1 release decision packet

This packet turns the open rows in the [Tier-1 decision register](decision-register.md)
into an executable handoff for the people who must provide external decisions
and evidence. It does not choose a vendor, jurisdiction, license, custodian,
or production region on behalf of the repository owner.

## Freeze order

Freeze decisions in this order because each later decision depends on the
earlier authority boundary:

| Order | Decision IDs | Required owner | Why it comes here |
| --- | --- | --- | --- |
| 1 | `DEC-0251-02`, `DEC-0251-03` | Legal/Tax owner | Defines what the product may calculate, where it may operate, and who owns the evidence. |
| 2 | `DEC-0251-01`, `DEC-0251-05` | Provider/Settlement/SRE owners | Selects the approved FX/RPC topology without allowing client-owned provider authority. |
| 3 | `DEC-0251-04`, `DEC-0251-07` | Platform/SRE and Security/Privacy | Defines regions, residency, retention, identity, and operational data boundaries. |
| 4 | `DEC-0251-06`, `DEC-0251-08` | Principal Architect and SRE/Governance | Sets measurable budgets, rollback authority, pause authority, and on-call ownership. |
| 5 | `DEC-0251-09` | Security/Governance owner | Enables key custody only after movement, evidence, and recovery requirements are fixed. |
| 6 | `DEC-0251-10` | Repository owner and counsel | Publishes source only after the license and contribution obligations are approved. |

An `OPEN` row is not a default. Until the row is frozen, the corresponding
non-local system path must remain disabled and the repository must continue to
use only synthetic local fixtures.

## Handoff record

For every decision, the accountable owner should provide one completed record
using this shape:

```text
decision_id: DEC-0251-XX
state: FROZEN
decision: <precise approved choice>
scope: <jurisdictions, assets, regions, providers, or environments>
accountable_role: <role, not an informal alias>
approvers: <approved public-safe identities or controlled reference>
effective_at_utc: <RFC 3339 timestamp>
evidence:
  - <controlled contract or approval reference>
  - <retained test/load/validator/ceremony artifact>
  - <public-safe documentation path, if applicable>
rollback_or_exit: <bounded action, authority, and expected state>
revisit_trigger: <measurable condition>
```

The record belongs in the relevant ADR or a superseding ADR. Do not edit a
historical evidence record to make it match a later decision; add a new
version and preserve the previous digest and approval trail.

## Evidence packet requirements

### Legal and tax scope

The Legal/Tax owner must provide:

- the supported jurisdiction, regime, currency, and Solana-asset matrix;
- source provenance, effective windows, rule review dates, and legal owner for
  each published rule;
- customer language separating automated withholding/evidence from filing,
  advice, and compliance; and
- privacy, retention, deletion, export, and merchant-responsibility approval.

This evidence freezes `DEC-0251-02`, `DEC-0251-03`, and the legal portions of
`DEC-0251-07`. It must not be inferred from the existence of a tax-engine test
vector.

### Provider and settlement topology

The Provider and Settlement/SRE owners must provide:

- provider contracts and identity/key allowlists;
- bounded freshness, disagreement, failover, and outage behavior;
- RPC commitment/finality behavior and immutable-byte retry policy;
- regional p50/p95/p99 quote, compile, submit, confirmation, and finality
  measurements with workload, sample, resource, and error-budget metadata; and
- evidence that provider disagreement, unknown transaction outcomes, and
  finalized indexer gaps fail closed and enter reconciliation.

This evidence freezes `DEC-0251-01`, `DEC-0251-05`, and `DEC-0251-06` only when
the corresponding provider and regional artifacts are retained.

### Platform, identity, and custody

The Platform/SRE and Security/Governance owners must provide:

- immutable staging manifest, regions, residency, TLS/mTLS identities, and
  synthetic-data separation;
- HSM/KMS non-exportability, dual-control ceremony, rotation overlap,
  revocation, recovery, and measured signer latency;
- PostgreSQL backup/PITR restore, key recovery, RPC failover, replay, pause,
  rollback, and incident-rehearsal evidence; and
- named rollback authority, pause authority, incident commander, escalation
  path, dashboards, alert delivery, and on-call coverage.

This evidence freezes `DEC-0251-04`, `DEC-0251-07`, `DEC-0251-08`, and
`DEC-0251-09` only after independent review of the artifacts.

### Public-source release

The repository owner and counsel must explicitly choose and approve the source
license before public source release. The choice must add the approved
`LICENSE` or `LICENSE.md`, update the public-repository contract, and record
the contribution and redistribution obligations. No license is inferred from
the absence of a file, and no unapproved license file may be added merely to
make a release check pass.

## Go/no-go rule

The launch review may proceed only when every applicable decision is
`FROZEN`, every required artifact is retained and independently reviewable,
and the [production-readiness plan](../plans/production-readiness.md) names no
unresolved critical/high finding. A green local `make check` is necessary but
cannot substitute for this packet, Linux/amd64 validator evidence, staging
identity/provider evidence, HSM/KMS custody, legal approval, independent
audit, manual accessibility review, or regional SLO evidence.
