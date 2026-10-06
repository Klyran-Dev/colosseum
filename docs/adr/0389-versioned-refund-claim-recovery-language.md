# ADR-0389: Versioned refund-claim recovery language

- Status: Accepted
- Date: 2026-10-05
- Owners: Web Platform, Payments, Treasury Operations
- Bounded contexts: Refund Claim, Wallet Boundary, Settlement Recovery

## Context

Refund claim is the first wallet-boundary step in the governed refund flow.
The workspace had a reviewed four-locale catalog for labels, finalized proof,
and reconciliation controls, but its component still authored authorization,
preparation, reload recovery, signing, submission, and reconciliation notices
inline. Recovery therefore fell back to English at a financial boundary where
the operator must distinguish transport acknowledgement from finalized refund
evidence.

The browser does not own correction values, refund amounts, recipients,
approval roles, signed bytes, provider outcomes, or chain finality. This ADR
changes presentation copy only.

## Decision

Extend the existing refund-claim catalog with a typed `errors` section in
English, German, Arabic, and Japanese. The catalog owns failures and safety
notices for authorization, preparation, durable reload recovery, two-person
signing, submission ambiguity, and finalized-claim reconciliation.

HTTP status interpolation is bounded to `100..599`. Provider payloads,
exception text, transaction bytes, account addresses, signatures, and other
protocol values are not rendered as authored operator copy. Unknown outcomes
continue to prohibit alternate-byte signing/submission and require finalized
chain evidence.

## Alternatives considered

1. Keep inline strings. Rejected because the wallet boundary remains
   inconsistently localized and difficult to review.
2. Reuse refund-execution or global checkout copy. Rejected because claim has
   correction-bound preparation and finalized-claim reconciliation semantics.
3. Localize messages in the API. Rejected because locale belongs to the
   presentation boundary and provider diagnostics must remain redacted.
4. **Extend the refund-claim catalog with bounded typed messages. Selected.**
   This preserves bounded-context ownership, fallback behavior, and the
   no-wire-change boundary.

## Invariants and acceptance criteria

- Every supported locale has non-empty claim error and recovery copy.
- Unknown locales resolve to the exact English catalog.
- HTTP status values are bounded before interpolation.
- The component contains no authored inline claim error or recovery notice.
- Unknown outcomes remain fail-closed and do not permit alternate signed bytes.
- Failing-first catalog tests, frontend tests, typecheck, production build,
  accessibility/color contracts, and real-browser journeys pass.
- No refund artifact, role check, wallet signature, API payload, or finalized
  evidence behavior changes.

## Consequences

Refund-claim operators receive consistent, action-oriented language across
supported locales while the chain-evidence and two-person boundaries remain
unchanged. Future claim recovery states require catalog entries and tests
before rendering. The UI continues to describe operational evidence, not tax
filing, legal compliance, or tax advice.
