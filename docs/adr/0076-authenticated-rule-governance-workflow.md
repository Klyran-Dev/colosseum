# ADR-0076: Authenticated tax-rule governance workflow

- Status: Accepted
- Date: 2026-09-26
- Owners: Tax Platform / Security / Tax Governance
- Scope: Tax-rule control-plane lifecycle and publication authority

## Context

Database immutability prevents a published rule document or effective window
from being silently changed, but immutability alone does not prove that the
release was authored, independently reviewed, and approved by distinct people.
Allowing a caller to submit a target status without a governed lifecycle would
also make a production API vulnerable to accidental or unauthorized promotion.

The rule lifecycle must remain off the Solana settlement hot path. It must be
auditable, retry-safe, bounded, tenant/organization scoped by the identity
system, and fail closed when a state transition or duty separation is
ambiguous.

## Decision

Use one authenticated control-plane endpoint:

`POST /v1/tax/rules/{rule_set_id}/transition`

The only normal promotion path is:

`draft → simulated → reviewed → approved → published → retired`

The service requires both an exact JWT scope and an exact JWT role for each
gate:

| Target status | Scope | Role | Subject rule |
| --- | --- | --- | --- |
| `simulated` | `tax:rule:author` | `tax_rule_author` | assigns the author |
| `reviewed` | `tax:rule:review` | `tax_rule_reviewer` | must differ from author |
| `approved` | `tax:rule:approve` | `tax_rule_approver` | must differ from author and reviewer |
| `published` | `tax:rule:publish` | `tax_rule_approver` | only the recorded approver may publish |
| `retired` | `tax:rule:publish` | `tax_rule_approver` | only the recorded approver may retire |

Each transition runs in a serializable PostgreSQL transaction. It locks the
rule row, verifies the expected current status and governance revision, writes
the next status and immutable actor assignments, and appends a bounded JSON
evidence event in the same transaction. The database trigger independently
enforces the lifecycle, subject prerequisites, pairwise subject distinctness,
published/retired governance immutability, UUIDv7 event IDs, and append-only
event behavior. Concurrent or stale transitions fail with a conflict rather
than overwriting evidence.

The endpoint never accepts a caller-supplied merchant as an authorization
boundary. Organization/tenant membership, tax/legal ownership, and identity
provider lifecycle remain deployment-specific controls above this exact
scope/role gate and must be evidenced before publication is enabled in a
production environment.

## Alternatives considered

- **Database-only status mutation**: rejected because it provides no
  authenticated separation of duties or review evidence.
- **A separate workflow broker as the source of truth**: rejected because
  publication correctness would depend on asynchronous delivery and could
  diverge from the immutable rule row; asynchronous notifications may be
  added later, but the database transaction remains authoritative.
- **One privileged administrator role**: rejected because it violates
  four-eyes control and materially increases blast radius.
- **On-chain governance for tax-rule authoring**: rejected for the current
  product because global tax documents and evidence do not fit the bounded
  settlement instruction; only the attestation and settlement invariants are
  checked on-chain.

## Consequences

The control plane now has an explicit, reviewable state machine and durable
governance evidence. Operators need correctly provisioned scopes and roles,
and a rule cannot be published by a single subject who authored it. Rejected
or blocked legal/tax review remains a deployment policy decision; this ADR
does not claim jurisdictional compliance or tax advice.

The integration harness proves the five successful transitions through
retirement, same-subject review rejection, event count, and append-only event
protection. Production readiness still requires identity-provider role lifecycle, organization-level
policy, legal/tax owner evidence, security review, and operator ceremony
rehearsal.
