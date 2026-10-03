# ADR-0283: Per-transaction atomic splits before continuous streaming

- Status: Accepted
- Date: 2026-10-04
- Owners: Settlement and Tax Architecture

## Context

The attached evaluation describes “streaming net payouts” to partners. That
phrase is useful product language, but it is broader than the first production
settlement authority implemented in this repository. Tax withholding depends on
the exact payment facts, approved rule version, jurisdiction evidence, asset,
rounding scope, cumulative snapshot, and quote/attestation lifetime. A
continuous token stream introduces mutable accrual state, epoch boundaries,
partial cancellation, tax corrections, revocation timing, and another path by
which funds could move without the settlement program's complete evidence
boundary.

The product must not imply that an atomic payment split is equivalent to a
continuous stream. The boundary needs to be explicit before a future feature
adds a second money-moving authority.

## Decision

The first production settlement scope is a bounded, atomic, per-transaction
split:

1. The authenticated quote and signed calculation attestation bind the gross,
   tax, net, rule, jurisdiction, asset, and expiry facts.
2. One bounded Solana settlement instruction deposits the withheld amount into
   the governed tax vault and allocates the net amount according to an
   immutable, versioned Split Plan.
3. The transaction succeeds or fails as one conservation boundary. Finalized
   chain evidence, not a database status or a stream cursor, is authoritative.
4. “Real-time” means the split is prepared and authorized at the payment
   boundary; it does not mean that the protocol continuously transfers tokens
   between payments.

Continuous partner streaming is deferred to a separate Revenue Streaming
bounded context. It may be proposed only when its design proves, at minimum:

- every accrual is bound to a finalized settlement, policy/rule digest, asset,
  recipient, and immutable stream-plan version;
- no stream path can bypass tax withholding, vault governance, pause controls,
  tenant authorization, or append-only evidence;
- epoch close, cancellation, correction, refund, revocation, and cumulative
  tax semantics are deterministic and fail closed;
- account fan-out, compute units, rent, contention, cancellation, and finality
  budgets are bounded and measured; and
- the new authority has its own threat model, alternative analysis, ADR,
  program audit, cross-language vectors, browser flow, and Linux validator
  evidence.

No current API, UI, plan, or documentation may claim continuous streaming as a
delivered settlement capability. A future integration may consume finalized
settlement evidence to drive an external payout schedule, but that schedule is
not permitted to become authoritative for Solana funds without the separate
bounded context and gates above.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Continuous on-chain streams first | Smooth partner payouts and familiar stream semantics | Mutable accrual state, epoch/finality complexity, partial cancellation, tax-window ambiguity, and a second token-authority surface | Rejected for the first production scope |
| **Atomic per-transaction Split Plan** | One conservation boundary for tax, vault, and recipients; bounded accounts; deterministic evidence | Partner payouts occur per settled payment rather than continuously | **Selected** |
| Off-chain accrual ledger with periodic batch payout | Flexible schedules and fewer on-chain accounts | Delayed settlement, custody ambiguity, and reintroduced reconciliation risk | Rejected as a settlement authority; possible later integration only |

## Invariants

- A recipient cannot receive more than the server-bound net amount allocated by
  the immutable Split Plan.
- A tax vault deposit and recipient allocation cannot be observed as partially
  successful within one settlement instruction.
- A future stream cannot mutate a historical settlement or replace finalized
  evidence with a stream cursor.
- Any future stream correction or cancellation must be a separately governed,
  replay-protected, evidence-bearing transition.

## Consequences

The current bounded Split Plan implementation remains aligned with the product
and PDF problem statement without over-claiming a continuous payout feature.
The system accepts per-payment latency and bounded fan-out in exchange for a
smaller authority surface, simpler tax replay, and stronger auditability. A
future streaming design will require new code and evidence; it cannot be added
by changing copy or by reusing the current settlement endpoint with a cursor.

## Evidence

- [Source analysis](../source-analysis.md) records the PDF requirement boundary.
- [Bounded Split Plan decision](0091-bounded-immutable-split-plan-context.md)
  defines the current per-transaction split model.
- [Split Plan conformance plan](../plans/production-readiness.md) keeps Linux
  validator and finalized balance evidence as release gates.
