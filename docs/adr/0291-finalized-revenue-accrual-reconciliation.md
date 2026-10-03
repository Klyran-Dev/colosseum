# ADR-0291: Finalized revenue-accrual reconciliation boundary

- Status: Accepted
- Date: 2026-10-04
- Decision owners: Blockchain, platform, and tax-domain maintainers
- Scope: indexer-service to revenue-streaming/storage integration

## Context

ADR-0290 defines the pure proof adapter that accepts only a finalized indexed
split settlement and produces a conserving `FinalizedAccrual`. The remaining
authority boundary is the application adapter: it must connect a finalized
page to active revenue plans without allowing a browser, API request, or
untrusted event field to choose the recipient allocation.

The split settlement event identifies the immutable split-plan version and
digest, but does not carry every recipient allocation. Allocations therefore
must be recomputed from the server-owned, digest-validated split plan using
the canonical largest-remainder algorithm. A revenue plan is eligible only
when its asset, split-plan digest, policy digest, recipient, and slot window
match the finalized event. Storage then performs the tenant-scoped,
idempotent, cap-checked write in one transaction.

## Decision

Implement a bounded, pure application assembler in `indexer-service`:

1. Receive a validated `FinalizedPage` and server-loaded active plan/split-plan
   contexts.
2. Match only split settlements whose asset, plan digest, policy digest, and
   slot window match the revenue plan.
3. Recompute allocations from the validated split plan and select the exact
   recipient allocation by canonical public-key bytes.
4. Call the ADR-0290 proof adapter with the page's observed finalized slot.
5. Emit storage commands with a fixed machine actor subject. The storage
   transaction remains the authority for tenant binding, active status,
   idempotency, caps, and outbox publication.

The assembler is bounded by explicit context and page limits and has no HTTP,
RPC, database, clock, or wallet dependency. A future worker may load contexts
from PostgreSQL, but it must use the same pure assembler. Missing or conflicting
server-owned context is a reconciliation error; it is never silently converted
to an accrual.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Let the browser/API submit recipient and amount | Rejected | Caller-controlled allocation is not chain evidence and breaks the authority boundary. |
| Trust a recipient amount emitted by the split event | Rejected | The event does not prove the complete recipient allocation and would bypass the immutable plan digest. |
| Put RPC/database lookups inside the tax/revenue domain | Rejected | It harms determinism, replay, testing, and latency control. |
| Recompute allocations in storage SQL | Rejected | Duplicates canonical split arithmetic and makes cross-language drift likely. |
| Pure bounded application assembler over typed server projections | Selected | Keeps authority and I/O at the application boundary while preserving deterministic, testable allocation semantics. |

## Invariants

- Only finalized commitment is accepted.
- Observed finalized slot is greater than or equal to the settlement slot.
- `tax + net = gross` and `recipient <= net`.
- Stream recipient and policy/asset/digest identities are exact byte matches.
- Recipient amount comes only from the canonical server-owned split plan.
- Duplicate source signatures remain idempotent in storage.
- A missing or conflicting context produces no storage command.
- The browser never supplies proof, allocation, merchant-side accounts, or
  machine actor identity.

## Operational budget

The assembler performs O(page settlements × bounded contexts) work with a
maximum of 64 contexts and the indexer's bounded page size. It must remain
allocation-bounded, cancellation-safe, and free of network calls. Database
context loading and accrual writes are outside the settlement signing path and
must have explicit pool, statement-timeout, retry, and metric budgets.

## Consequences

Revenue accruals become an eventually consistent projection of finalized chain
evidence, with deterministic replay and no caller-supplied authority fields.
The accrual worker must expose pending, rejected, duplicate, and storage-error
metrics and retain evidence for operator repair. This ADR does not enable claim
signing or vault release; those remain gated by signed-message verification,
validator conformance, wallet-bound authorities, and independent operational
approval.
