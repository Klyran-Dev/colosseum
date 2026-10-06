# ADR-0409: Server-owned applicability-context persistence and quote binding

- Status: Accepted
- Date: 2026-10-06
- Deciders: Principal Low-Latency Architecture, Tax Domain, Compliance, Security
- Scope: Applicability context persistence, quote binding, attestation evidence

## Context

ADR-0407 separates tax applicability from arithmetic. The pure Rust
`tax-applicability` crate and signed policy attestation kernel now exist, but a
quote still has no server-owned applicability reference. Accepting treatment,
classification, or evidence directly from a browser would make the caller an
authority and would allow a quote to be replayed without its legal evidence.

The persistence boundary must support immutable published context, tenant
isolation, UUIDv7 identifiers, four-eyes governance, exact replay, and bounded
latency. It must not put PostgreSQL, a tax provider, or an unbounded policy
graph inside the arithmetic hot path.

## Decision

Persist a server-owned `TaxApplicabilityContext` projection in PostgreSQL 18.
The record is identified by a UUIDv7 `context_id` and contains the canonical
facts digest, decision digest, policy identity/version/digest, jurisdiction
resolution digest, evidence digest, signed applicability attestation, effective
window, immutable provenance, governance status, and accountable tenant.

The lifecycle is:

```text
draft -> review -> approved -> published -> retired
                  \-> rejected
```

Only `published` contexts can be selected for production quotes. Published and
retired contexts are immutable. Corrections create a new UUIDv7 context and a
supersession reference. Governance events are append-only and require distinct
author, reviewer, and approver subjects where the existing governance model
requires four-eyes approval.

The quote boundary accepts only a server-issued `applicability_context_id` for
an applicability-bound quote. The API derives tenant scope from the
authenticated principal, loads the published context, verifies the signed
attestation at the quote time, and persists the exact context and attestation
digest with the calculation. The browser cannot submit treatment, policy
digest, evidence digest, merchant identity, or attestation JSON. The current
additive compatibility slice leaves the reference optional for explicitly
arithmetic-only quotes; the follow-up gate must compare the context's
classification facts, decision, jurisdiction, rule release, and evidence
digests with the server-owned quote facts before enabling legal treatment
semantics.

The context is resolved before the quote hot path and may be served from a
bounded immutable cache keyed by `(merchant_id, context_id)`. A cache miss may
read PostgreSQL once; tax arithmetic itself remains pure and performs no I/O.
Unknown, non-published, stale, tenant-mismatched, malformed, or conflicting
contexts fail closed. No fallback to `regime`, zero tax, or an external provider
is permitted.

## Data and invariants

- `id`, context IDs, governance event IDs, and calculation IDs use PostgreSQL
  native `uuid` with UUIDv7; strict ordering uses timestamps and event IDs.
- The stored JSON is re-parsed into the typed Rust attestation and validated;
  database columns are projections, not authority.
- `facts_digest` equals the canonical digest of the stored applicability facts.
- `decision_digest` equals the canonical digest of the stored decision.
- The decision facts digest equals the facts digest, and the attestation binds
  both digests plus the jurisdiction-resolution digest.
- `expires_at` cannot exceed the source evidence validity window.
- Published context fields and attestation evidence cannot be updated or
  deleted; retention applies only through an approved archival policy.
- Every applicability-bound quote records the selected context ID and
  attestation digest; settlement later binds the same values and never trusts a
  browser copy. Arithmetic-only compatibility quotes record no context.
- The database is authoritative for the immutable context projection only;
  signed attestation verification remains authoritative for cryptographic
  integrity, and finalized Solana evidence remains authoritative for funds.

## Latency and failure budget

- Cache hit: p99 <= 1 ms for context lookup and validation, excluding network.
- PostgreSQL cache miss: p99 <= 20 ms in-region with a bounded single-row
  query and a deadline; the quote fails closed on timeout.
- No provider call, migration, rule compilation, or arbitrary predicate graph
  execution is allowed on the settlement signing path.
- Context load, validation, and cache metrics use fixed labels and bounded
  cardinality. Repeated malformed context IDs are rate-limited at the API.

## Alternatives considered

| Alternative | Evidence ownership | Hot-path latency | Replay/governance | Decision |
| --- | --- | --- | --- | --- |
| Accept treatment and attestation JSON from the browser | Fail; caller becomes authority | Fast but unsafe | Fail | Rejected |
| Resolve applicability through an external tax SaaS per quote | Partial; provider is authority | Unbounded and failure-prone | Partial | Rejected |
| Put the full applicability graph on Solana | Strong settlement binding | Compute/transaction limits and legal-rule rigidity | Partial | Rejected |
| Store mutable policy rows and recalculate on replay | Fail; historical meaning changes | Variable | Fail | Rejected |
| **Immutable PostgreSQL context projection plus signed attestation and bounded cache** | **Pass** | **Bounded and measurable** | **Pass** | **Selected** |

## Implementation sequence

1. Add the immutable context and append-only event schema with UUIDv7 and
   database constraints.
2. Add typed storage validation and idempotent create/load/transition ports;
   write tests before production handlers.
3. Add an optional server-owned context reference to quote commands and persist
   the selected context/attestation digest with the calculation. The reference
   is verified as published, tenant-scoped, effective, and evidence-live.
4. Require the reference for published quote paths; preserve simulation-only
   historical adapters without making production claims.
5. Add the versioned applicability-bound settlement-client and Solana
   instruction contracts defined by [ADR-0411](0411-versioned-solana-applicability-settlement-binding.md).
6. Add UI progressive disclosure and review-required states only after the API
   contract and legal/tax ownership are approved.

## Consequences

This adds a governance and storage lifecycle before global tax treatment can be
published. That friction is intentional: no jurisdiction or treatment becomes
production-supported merely because arithmetic code exists. It also means the
first implementation can be reviewed and replayed independently of the live
quote or settlement services.
