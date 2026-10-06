# ADR-0391: Versioned replay-operations recovery language

- Status: Accepted
- Date: 2026-10-05
- Owners: Indexing, Web Platform, Security, Operations
- Bounded contexts: Replay Operations, Route Verification Evidence, Indexer Recovery

## Context

Replay Operations is an operator control surface for bounded indexer recovery
and read-only route-verification evidence. Its primary labels were already
localized, but transport status formatting, page-budget validation, projection
reload recovery, and malformed route-evidence handling still authored copy in
the component. That produced inconsistent recovery language at a control
boundary where a service acknowledgement must not be confused with finalized
chain evidence.

The browser cannot acknowledge chain finality, activate routes, submit funds,
or become the replay lifecycle authority. This change is presentation-only.

## Decision

Extend the existing replay-operations catalog with typed common recovery
messages in English, German, Arabic, and Japanese. The catalog owns bounded
HTTP status interpolation, page-budget validation, action failure, follow-up
projection reload failure, and malformed route-evidence copy.

HTTP statuses are rendered only after bounding them to `100..599`; all other
values become `unknown status`. Provider diagnostics and parser details remain
redacted. A projection reload failure remains an explicit reconciliation state;
the component does not invent a successful projection or chain outcome.

## Alternatives considered

1. Keep inline recovery strings. Rejected because operational controls would
   remain inconsistent and difficult to review across locales.
2. Use generic global error copy. Rejected because replay, targeted recovery,
   and route-verification evidence have distinct operator actions and safety
   semantics.
3. Localize in the API. Rejected because locale is a presentation concern and
   the API must remain locale-neutral and protocol-focused.
4. **Extend the replay-operations catalog with typed bounded messages.
   Selected.** This preserves bounded-context ownership, deterministic
   fallback, redaction, and the no-authority-change boundary.

## Invariants and acceptance criteria

- Every supported locale has non-empty replay recovery messages.
- Unknown locales resolve to the exact English catalog.
- HTTP status values are bounded before interpolation.
- Malformed route-verification evidence renders no evidence and cannot activate
  a route.
- Projection reload failure remains visible as reconciliation-required copy.
- The component contains no authored inline replay error or recovery literals.
- Failing-first catalog/UI tests, typecheck, production build,
  accessibility/color contracts, and browser journeys pass.
- No replay API payload, role check, retry budget, chain evidence, or route
  activation behavior changes.

## Consequences

Replay operators receive consistent localized recovery language and bounded
transport feedback. Future replay failure states must add catalog entries and
tests before rendering. The workspace continues to describe operational
evidence only and never implies payment movement, finalized settlement, tax
filing, or legal compliance.

