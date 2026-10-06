# ADR-0388: Versioned refund-execution recovery language

- Status: Accepted
- Date: 2026-10-05
- Owners: Web Platform, Payments, Treasury Operations
- Bounded contexts: Refund Execution, Wallet Boundary, Settlement Recovery

## Context

Refund execution is the final two-person wallet authorization path for an
approved refund. The workspace already had reviewed English, German, Arabic,
and Japanese labels and proof states, but its component still authored
authorization, preparation, reload recovery, signing, submission, and
provider-ambiguity messages inline. The flow therefore reverted to English at
the exact point where a release operator needs a precise, safe next action.

The browser does not own refund amounts, recipients, approval roles, signed
transaction validity, provider outcomes, or finalized proof. This decision is
presentation-only and must not change the refund artifact, wallet boundary,
request contract, or chain evidence model.

## Decision

Extend the existing refund-execution catalog with a typed `errors` section for
all supported locales. It owns both failure copy and safety-critical recovery
notices, including the distinction between preparation, durable reload
recovery, wallet-signing prerequisites, two-person progress, unknown provider
outcomes, and proof-pending acceptance.

HTTP status interpolation is bounded to `100..599`. Provider payloads,
exception text, transaction bytes, account addresses, signatures, and other
protocol values are not used as authored browser copy. Unknown provider
outcomes continue to prohibit alternate-byte submission and require finalized
transaction and balance reconciliation.

## Alternatives considered

1. Keep inline strings. Rejected because the final release boundary would
   remain inconsistently localized and difficult to review.
2. Reuse refund-claim or global checkout copy. Rejected because execution has
   different two-person signing, reload recovery, and proof-pending semantics.
3. Localize on the API. Rejected because locale belongs to the presentation
   boundary and server/provider diagnostics must remain redacted.
4. **Extend the refund-execution catalog with bounded typed messages.
   Selected.** This preserves bounded-context ownership, existing fallback
   behavior, and the no-wire-change boundary.

## Invariants and acceptance criteria

- Every supported locale has non-empty execution error and recovery copy.
- Unknown locales resolve to the exact English catalog.
- HTTP status values are bounded before interpolation.
- The component contains no authored inline execution error or recovery notice.
- Unknown outcomes remain fail-closed and do not permit alternate signed bytes.
- Failing-first catalog tests, frontend tests, typecheck, production build,
  accessibility/color contracts, and real-browser journeys pass.
- No refund artifact, role check, wallet signature, API payload, or finalized
  evidence behavior changes.

## Consequences

Refund-release operators receive consistent next-action language across the
supported locales without weakening the two-person or finalized-proof
boundaries. Future execution recovery states require catalog entries and
tests before rendering. The UI continues to describe operational evidence and
authorization state, not legal compliance or tax advice.
