# ADR-0418 — Applicability-context management API

- Status: Accepted — implementation gate
- Date: 2026-10-06
- Decision owners: Tax Domain, Security, Platform, Principal Low-Latency Architecture

## Context

The applicability bounded context already has a signed attestation kernel,
PostgreSQL 18 immutable projections, idempotent creation, and four-eyes
transitions. It is not yet reachable through an authenticated API workflow.
Without that boundary, operators cannot author, review, publish, retire, or
inspect a context, while quote requests can only reference an out-of-band
identifier. This is an incomplete product boundary and encourages unsafe
operator workarounds.

## Decision

Add a strict, tenant-scoped management API over the existing storage port:

| Operation | Route | Scope | Required role | Effect |
| --- | --- | --- | --- | --- |
| Create | `POST /v1/tax/applicability-contexts` | `tax:applicability:author` | `tax_applicability_author` | Verifies the signed attestation and creates an immutable `draft` with UUIDv7 identity and idempotency |
| Read | `GET /v1/tax/applicability-contexts/{context_id}` | `tax:applicability:read` | none | Returns the tenant-owned projection, including its lifecycle and evidence digests |
| Transition | `POST /v1/tax/applicability-contexts/{context_id}/transition` | status-dependent | status-dependent | Applies only the bounded storage state machine and writes append-only evidence |

Transition authorization is explicit and selected from the requested target:

- `draft -> review`: `tax:applicability:author` and
  `tax_applicability_author`;
- `review -> approved` or `review -> rejected`:
  `tax:applicability:review` and `tax_applicability_reviewer`;
- `approved -> published` or `approved -> rejected`, and
  `published -> retired`: `tax:applicability:publish` and
  `tax_applicability_approver`.

The API derives `merchant_id` and `actor_subject` exclusively from the
authenticated principal. The create request may contain only an
issuer-signed `ApplicabilityAttestation`, bounded effective dates, an
idempotency key, and bounded provenance. It cannot contain a merchant ID,
status, governance subjects, calculated treatment, or a caller-owned digest.
The storage validator remains the second authority-sensitive boundary.

The response schema is `tax-applicability-context/v1`; unknown JSON fields are
rejected, UUIDs are native UUIDv7 strings, digest values are fixed 32-byte
arrays using the repository's existing wire convention, and all errors use the
existing bounded API error envelope. Management routes are not called by the
quote, wallet-signing, or settlement-submit hot paths.

## Invariants and budgets

- A context is created only after signature, merchant identity, facts/decision,
  evidence window, and policy binding verification succeed.
- Repeating the same tenant/idempotency key with identical canonical input
  returns the original context; a different input fails with conflict.
- A caller cannot read or transition another tenant's context, even if it has
  the UUID.
- Transition state and four-eyes role separation remain enforced by storage and
  database triggers; the API does not duplicate or weaken those rules.
- Request body, provenance, and response sizes stay within existing bounded
  service limits; no provider or RPC call is introduced.
- Management p99 target is 100 ms in-region for create/read excluding database
  pool starvation; quote and settlement hot-path budgets are unchanged.

## Rejected alternatives

1. Accepting applicability fields on quote requests makes the browser an
   authority and is rejected.
2. Leaving storage unreachable keeps the product incomplete and encourages
   privileged database access; it is rejected.
3. Calling an external provider for every quote is unbounded and breaks exact
   replay; it is rejected.
4. A generic administrator endpoint with caller-supplied tenant and role
   fields is rejected because it collapses RBAC/PBAC/ABAC boundaries.

## Verification plan

Write failing-first API tests for authentication, exact role/scope mapping,
unknown-field rejection, tenant-derived storage commands, bounded transitions,
and no-storage fail-closed behavior. Retain storage validation/idempotency
tests and the authenticated `make integration-check` evidence for signed
attestation creation, replay, tenant isolation, four-eyes publication,
append-only events, outbox records, and published immutability. API contract
examples and UI governance review remain required. Legal/tax-owner approval,
staging identity, and production treatment enablement remain external gates.
