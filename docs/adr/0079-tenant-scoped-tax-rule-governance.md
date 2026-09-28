# ADR-0079: Tenant-scoped tax-rule governance and rule resolution

- Status: Accepted
- Date: 2026-09-26
- Owners: Tax Platform / Security / Data
- Scope: Rule-release ownership, quote rule resolution, and governance APIs

## Context

ADR-0076 and ADR-0077 added authenticated rule-governance transitions and an
evidence-first operator workspace. Their initial queries identified a rule
release by `rule_set_id` alone. That is insufficient for a multi-tenant
financial system: a principal from one merchant could read or mutate another
merchant's governance record if it learned the UUID, and a published quote
could resolve a same-key rule owned by a different tenant.

The tax engine remains a global, reusable calculation capability, but persisted
rule releases and their governance lifecycle are tenant-owned policy data. A
tenant may publish the same jurisdiction/regime/version as another tenant;
the database must not collapse those releases into one global uniqueness
domain.

## Decision

1. Add a non-null `merchant_id` owner to every `rule_sets` row.
2. Replace global `(external_id, version)` uniqueness with
   `(merchant_id, external_id, version)` uniqueness and add a tenant lookup
   index.
3. Require the authenticated principal's merchant ID in rule transitions and
   governance reads. Storage predicates must include both `id` and
   `merchant_id`; ownership mismatches return the same not-found boundary as
   any other missing rule.
4. Resolve published quote rules by `(merchant_id, external_id, version,
   jurisdiction, regime)` and preserve the merchant binding in the persisted
   calculation evidence.
5. Add a composite foreign key from `(tax_calculations.rule_set_id,
   tax_calculations.merchant_id)` to the owning rule release. This prevents
   storage-level cross-tenant association even if application code regresses.
6. Rule ownership is immutable from row creation through retirement; it cannot
   be changed by a governance transition or a direct SQL update. The migration
   refuses to guess ownership for existing rule rows. A
   deployment with pre-existing rows must run an explicit, reviewed backfill
   before the ownership column becomes `NOT NULL`.

## Alternatives considered

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Global rule catalog with API-only tenant checks | Reuses one release across merchants | A missed predicate becomes cross-tenant disclosure or policy substitution; database cannot enforce ownership | Rejected |
| Nullable owner with a global fallback | Easier migration and shared rules | Ambiguous authority and accidental global access; violates fail-closed tenant isolation | Rejected |
| Separate database/schema per merchant | Strong physical isolation | Operationally expensive, poor shared connection/pool behavior, difficult global reporting | Rejected for the first product boundary |
| **Non-null merchant owner plus composite database predicates/constraints** | Explicit ownership, bounded indexed queries, database-enforced association, supports duplicate keys per tenant | Requires an explicit migration/backfill and tenant-aware rule provisioning | **Selected** |

## Invariants and security properties

- A rule release has exactly one owning merchant.
- A `(merchant_id, external_id, version)` tuple identifies at most one release.
- A quote cannot resolve a published rule outside its authenticated merchant.
- A governance UUID from another merchant is indistinguishable from an
  unknown UUID at the API boundary.
- A calculation cannot reference a rule owned by another merchant.
- Rule ownership cannot be changed in any lifecycle state, including draft,
  through ordinary governance transitions or direct database updates.

## Migration and compatibility

Migration `0016` adds the ownership column, validates that no existing row is
unowned, replaces the uniqueness constraint, and adds the composite
calculation foreign key. It is intentionally not a data migration: production
operators must supply an audited backfill for legacy rows before applying it.
Fresh local fixtures assign the synthetic rule to the documented synthetic
merchant.

The API shape remains backward compatible for clients; ownership is derived
from the bearer principal and is never accepted in a request body or URL.
The governance response includes the owner for operator evidence. A rule
lookup that loses ownership is a release-blocking error, not a fallback to a
global row.

## Verification

- storage unit tests prove the tenant predicate is part of the rule-resolution
  and governance command contract;
- PostgreSQL integration proves same-key releases can coexist across tenants,
  cross-tenant governance reads/transitions return not-found, cross-tenant
  quote resolution fails closed, draft ownership mutation is rejected, and the
  composite calculation constraint rejects mismatched ownership;
- API tests prove the merchant is taken from authenticated claims rather than
  caller input;
- replay and settlement quote queries continue to require the authenticated
  merchant ID.

The Docker integration harness and PostgreSQL backup/restore drill passed with
migration `0016`, including the cross-tenant API boundaries, duplicate-key
coexistence, and composite foreign-key rejection cases.
