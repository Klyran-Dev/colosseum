# ADR-0096: Authenticated, idempotent rule-draft authoring

- Status: Accepted
- Date: 2026-09-27
- Owners: Tax Platform / Governance
- Scope: Creating tenant-owned draft tax-rule releases before lifecycle promotion

## Context

The governance bounded context could read and transition a persisted rule, but
there was no production API or UI path to create a draft. Operators therefore
had to rely on database fixtures, which bypassed authenticated tenant scope,
idempotency, canonical rule validation, creation provenance, and outbox
publication. That is not a complete authoring lifecycle.

## Decision

Add `POST /v1/tax/rules` as the only product draft-creation boundary. It:

- requires exact `tax:rule:author` scope and `tax_rule_author` role;
- derives `merchant_id` and the author subject from the authenticated principal;
- accepts a bounded idempotency key, explicit external identity/version,
  jurisdiction/regime, bounded effective Unix-second window, complete
  `RuleSet`, and provenance object;
- requires the top-level identity to match `RuleSet.id` and `RuleSet.version`;
- validates and compiles the complete rule graph through the pure Rust engine
  before persistence and stores its canonical digest;
- creates status `draft` with the authenticated author subject;
- commits the rule row, immutable `create` governance event, idempotency
  response, and `tax.rule.draft.created` outbox record in one serializable
  PostgreSQL transaction;
- replays exact idempotent input with the same server-owned response and
  rejects key reuse or tenant/external-id/version conflicts.

The governance UI exposes a structured metadata form plus a canonical JSON
editor for the full Tax Engine rule graph. It loads the resulting server-owned
UUIDv7 snapshot before enabling lifecycle actions. Client-side JSON parsing is
only an input ergonomics check; server validation and persisted evidence remain
authoritative.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Database-only fixtures or operator SQL | Rejected | Bypasses tenant authorization, audit evidence, idempotency, and product UX. |
| Client-only draft state | Rejected | Cannot support review duties, replay, immutable evidence, or multi-operator work. |
| Create directly as `published` | Rejected | Violates four-eyes governance and effective-window safety. |
| External tax-rule SaaS as the authoring authority | Rejected | Introduces an opaque dependency and cannot own this protocol's canonical digest/evidence contract. |
| **Authenticated Rust API plus transactional PostgreSQL/outbox creation** | **Selected** | Preserves domain validation, tenant scope, durable auditability, replay, and asynchronous publication without putting a broker in the write path. |

## Invariants and failure behavior

1. A caller cannot select another merchant's rule owner or author subject.
2. A draft's rule identity, canonical digest, provenance, and effective window
   are immutable evidence of the create command.
3. Invalid, cyclic, ambiguous, over-bounded, or identity-mismatched RuleSets
   fail before a durable rule row is created.
4. A timeout or retry cannot create two drafts for one merchant/idempotency key.
5. Creation never implies simulation, publication, attestation, compliance, or
   settlement authority.
6. The first publication record is committed with the draft row; the NATS
   worker is downstream and at-least-once.

## Evidence

The vertical slice is covered by storage validation/digest tests, API exact-role
tests, TypeScript response-boundary tests, a PostgreSQL migration, Docker
integration for authenticated creation, exact idempotency replay, tenant
ownership, immutable creation/transition history, and outbox delivery.

