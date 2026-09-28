# ADR-0104: Govern jurisdiction policies with the same four-eyes control plane as tax rules

- Status: Accepted
- Date: 2026-09-27
- Owners: Tax Platform / Compliance / Security / Data
- Scope: Jurisdiction-policy authoring, approval, publication, retirement, and quote selection

## Context

ADR-0103 makes the jurisdiction policy authoritative for resolving verified
provider evidence. The initial policy table and local fixture establish the
data invariant, but direct fixture/SQL publication is not an acceptable
production authority boundary. A policy can change the tax outcome without
changing the tax-rule document, so policy provenance and approval must be
independently auditable while remaining bound to one tenant-owned rule release.

## Decision

1. Add an authenticated jurisdiction-policy control plane with idempotent draft
   creation, bounded provenance, and a canonical policy digest.
2. Use the lifecycle `draft -> reviewed -> approved -> published -> retired`
   with optional `blocked` terminal handling. Author, reviewer, and approver
   subjects are persisted and must be distinct. Publication and retirement
   require the approver identity; the author cannot review or approve its own
   policy.
3. Persist immutable append-only policy transition events and the first
   outbox publication event in the same serializable transaction as the policy
   mutation. Replays return the original response; a reused idempotency key
   with a different digest fails closed.
4. A policy draft must reference an existing tenant-owned rule release, and
   publication requires the referenced rule release to be `published` with a
   compatible effective window. Quote selection continues to require the
   currently effective published policy and canonical digest match.
5. Keep raw provider/customer evidence outside the policy control plane. The
   policy contains only the bounded, versioned precedence contract and
   redacted provenance references. Access to governance reads and transitions
   is authenticated and emits bounded audit/outbox evidence.

## Alternatives considered

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Direct SQL/fixture publication | Fast to bootstrap | Bypasses identity, review, idempotency, and audit | Rejected |
| Couple policy changes to rule JSON only | One approval surface | Cannot correct jurisdiction precedence independently; weak policy provenance | Rejected |
| External workflow/SaaS as authority | Rich workflow features | Adds hot-path/config availability dependency and replay drift | Rejected |
| **Authenticated Rust API plus PostgreSQL four-eyes lifecycle** | Tenant-safe, replayable, bounded, auditable, consistent with rule governance | Requires governance UI and identity-role provisioning | **Selected** |

## Invariants and failure modes

- A policy is never selected across tenant or rule-release boundaries.
- Published and retired policy identity, digest, document, provenance, and
  effective window are immutable; corrections are new policy/rule versions.
- A policy cannot publish while its rule release is not published or its
  effective window is incompatible.
- Unknown role, duplicate subject, invalid transition, oversized provenance,
  digest mismatch, and idempotency conflict fail closed.
- Policy read/transition failures never cause the quote path to fall back to a
  process-local or caller-supplied policy.

## Latency and operational budget

The quote hot path keeps one indexed policy join and canonical digest check;
governance writes are off the settlement path. Policy lookup must remain
within the existing quote database statement budget (900 ms timeout, 250 ms
lock timeout). Governance writes use the same bounded serializable retry
policy as rule drafts and do not hold application locks across awaits.

## Verification

- Domain/storage tests cover canonical digest, bounded provenance, lifecycle
  transitions, four-eyes subject separation, idempotent replay/conflict, and
  publication binding to a published rule release.
- API tests cover exact scope/role authorization and tenant-scoped 404 behavior.
- Docker integration covers a complete policy governance lifecycle and proves
  the resulting published policy is the one used by a quote.
- Production identity governance, legal/tax owner approval, and browser
  accessibility automation remain release gates.
