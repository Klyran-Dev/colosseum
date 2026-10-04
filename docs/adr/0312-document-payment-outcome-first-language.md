# ADR-0312: Outcome-first language for document payment review

## Status

Accepted — implemented in the document-payment workspace; manual operator,
localization, and assistive-technology review remain release gates.

## Context

The document-payment flow exposed protocol vocabulary as its primary interface:
attestation, settlement route, compile, client arithmetic, and payer wallet.
Those terms are useful in evidence and support workflows, but they force a
finance or operations user to understand implementation details before deciding
whether to review and authorize a payment. The resulting screen was visually
dense and technically framed even though the underlying authorization boundary
was sound.

The product must remain honest about what the browser does: it presents
server-owned invoice evidence and a payment artifact for wallet approval. It
must not imply that a wallet signature proves settlement or that withholding is
tax filing, reporting, legal compliance, or tax advice.

## Decision

Use outcome-first, plain-language labels in the primary document-payment path:

- “Review and authorize this payment” is the workspace heading.
- “Invoice evidence”, “Your wallet”, “Payment route”, and “Payment review” are
  the primary concepts.
- The primary actions are “Verify invoice evidence”, “Prepare payment for
  review”, and “Approve and send payment”.
- Status copy explicitly distinguishes invoice verification, payment
  preparation, wallet approval, and finalized network evidence.
- Technical evidence remains available through the existing review result,
  accessible status text, support correlation identifiers, and the server-owned
  evidence returned by the workflow. It is not removed or renamed in protocol
  payloads, API contracts, Solana instructions, or audit records.

This is a presentation-only change. It does not alter authorization, RBAC/PBAC/
ABAC decisions, tenant scope, tax arithmetic, account resolution, signed bytes,
settlement state transitions, or failure semantics.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep protocol vocabulary as the primary copy | Exact for engineers | High cognitive load and poor task clarity for finance/operators | Rejected |
| Hide technical evidence completely | Cleanest visual surface | Removes audit, support, and incident-response context | Rejected |
| Put all meaning in tooltips | Small footprint | Weak keyboard, touch, screen-reader, and localization experience | Rejected |
| **Use plain-language primary copy with inspectable technical evidence** | Clear decision flow while preserving auditability and fail-closed semantics | Requires copy contracts and journey coverage | **Selected** |

## Invariants and failure policy

- Visible copy must never claim that a payment settled before finalized chain
  evidence exists.
- Evidence verification, payment preparation, wallet approval, and settlement
  remain separate states with separate loading, timeout, failure, and
  reconciliation-required messages.
- Required inputs remain labeled and operable; copy simplification cannot hide
  an input needed to establish server-owned evidence.
- Technical identifiers and exact evidence remain available to authorized
  operators and support workflows.
- No UI label may imply tax filing, reporting, legal compliance, or tax advice.
- The language contract fails the build if approved task language regresses or
  retired primary terminology returns.

## Verification

- `apps/web/src/documentCheckout-ui.test.tsx` asserts the outcome-first labels
  and rejects the retired technical primary strings.
- `apps/web/e2e/ui-journeys.spec.ts` covers the evidence handoff through the
  reviewed payment artifact in a real browser.
- `scripts/verify_ui_language_contract.sh` pins the approved result language.
- `make web-check` and `make web-browser-check` cover type safety, component
  behavior, accessibility, theme, RTL, reduced motion, and failure paths.

## References

- [ADR-0272: Payment review progressive disclosure](0272-payment-review-progressive-disclosure.md)
- [ADR-0282: Plain-language primary UI with progressive technical disclosure](0282-plain-language-progressive-disclosure-ui.md)
- [ADR-0311: Solana Aurora Material 3 role palette](0311-solana-aurora-material-palette.md)
