# ADR-0387: Versioned document-settlement error language

- Status: Accepted
- Date: 2026-10-05
- Owners: Web Platform, Payments, Tax Engine UX
- Bounded contexts: Document Checkout, Wallet Boundary, Settlement Recovery

## Context

The document-checkout workspace is the wallet-facing, document-bound payment
flow. Its labels and successful states are localized, but several failure and
recovery branches still authored English directly in the component: route
loading, invoice-evidence verification, durable compilation, status recovery,
submission ambiguity, and reconciliation. That causes the highest-risk user
journey to change language at precisely the point where an operator needs a
clear next action.

The browser must not become an authority for protocol state. HTTP status,
durable intent state, transaction evidence, authorization, and settlement
transitions remain server- and chain-owned. This ADR is presentation-only.

## Decision

Extend the existing document-checkout catalog with a typed `errors` section in
English, German, Arabic, and Japanese. All operator-facing document checkout
failures use that catalog. HTTP statuses are bounded to `100..599`; durable
intent statuses accept only a bounded identifier grammar before interpolation.
Provider payloads, exception text, account values, signatures, and protocol
digests are never rendered as authored recovery copy.

The catalog is resolved before component effects so route-loading failures use
the selected locale consistently. The request, digest, authorization, wallet
signing, settlement, and reconciliation contracts are unchanged.

## Alternatives considered

1. Keep inline English strings. Rejected because the wallet-facing recovery
   path would remain inconsistent and difficult to review.
2. Reuse the aggregate checkout catalog. Rejected because document evidence,
   durable compilation, and invoice-specific recovery have different user
   actions and bounded-context ownership.
3. Localize messages in the API. Rejected because locale is a presentation
   concern and server/provider diagnostics must remain redacted.
4. **Extend the document-checkout catalog with typed bounded dynamic values.
   Selected.** This preserves ownership, fallback behavior, and the existing
   no-wire-change boundary.

## Invariants and acceptance criteria

- Every supported locale has non-empty document-checkout error copy.
- Unknown locales resolve to the exact English catalog.
- HTTP and durable-status interpolation is bounded and tested.
- Document checkout contains no authored inline operator error copy.
- Unknown submission outcomes continue to block repeat signing and direct the
  operator to authenticated recovery.
- Failing-first catalog tests, frontend tests, typecheck, production build,
  accessibility/color contracts, and real-browser journeys pass.
- No API, tax, attestation, digest, wallet, or Solana settlement behavior
  changes.

## Consequences

Document payment recovery is consistent across supported languages and the
operator receives an action-oriented message for each unsafe state. Future
document-checkout failure states must add catalog entries and tests before
they can be rendered. Legal and tax boundaries remain explicit: the UI
describes evidence, withholding authorization, and settlement recovery; it
does not claim filing, reporting, or legal compliance.
