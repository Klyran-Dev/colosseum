# ADR-0390: Versioned vault-release recovery language

- Status: Accepted
- Date: 2026-10-05
- Owners: Web Platform, Treasury Operations, Security
- Bounded contexts: Vault Release, Route Verification, Operational Recovery

## Context

The governed vault-release workspace already used a four-locale catalog for
labels, route evidence, lifecycle controls, and authority disclosures. Its
failure and recovery paths still authored English text inside the component,
including route loading, claim proposal, claim lookup, response parsing, and
claim transitions. That made a financial control surface inconsistent across
locales and allowed parser details to leak into operator copy.

The browser records a request or operator evidence; it does not authorize
token movement. Server-owned route selection, two-person approval, finalized
ledger evidence, and the existing fail-closed API behavior remain unchanged.

## Decision

Extend the vault-release catalog with a typed `errors` section in English,
German, Arabic, and Japanese. The catalog owns authorization, route loading,
empty-route, proposal, lookup, response-validation, evidence-validation,
transition, mismatch, and refresh-recovery messages.

HTTP status interpolation is bounded to `100..599`. Unknown statuses render as
`unknown status`. Provider payloads, parser implementation details, account
addresses, signatures, transaction values, and other protocol values are not
used as authored operator copy. A successful proposal may display the returned
request identifier as a separate server value; it is not translated or used as
authority.

Malformed claim and transition responses fail closed with reviewed generic
messages. They do not update local financial state. A transition accepted by
the service but followed by a failed refresh remains an explicit
reconciliation-required state.

## Alternatives considered

1. Keep inline strings. Rejected because the treasury control surface remains
   inconsistent, difficult to review, and vulnerable to parser-detail leakage.
2. Reuse checkout or refund catalogs. Rejected because vault release has
   server-owned route selection, two-person governance, and ledger-transition
   semantics that do not belong to those bounded contexts.
3. Localize messages in the API. Rejected because locale belongs to the
   presentation boundary and the API must remain locale-neutral.
4. **Extend the vault-release catalog with typed bounded messages. Selected.**
   This preserves bounded-context ownership, deterministic fallback, and the
   no-wire/no-ledger-change boundary.

## Invariants and acceptance criteria

- Every supported locale has non-empty vault-release error and recovery copy.
- Unknown locales resolve to the exact English catalog.
- HTTP status values are bounded before interpolation.
- Malformed claim or transition payloads never become local financial state.
- The component contains no authored inline vault-release error or recovery
  notice.
- Route selection remains server-owned and only the route identifier is sent
  for merchant-side accounts.
- Failing-first catalog/UI tests, typecheck, production build,
  accessibility/color contracts, and real-browser journeys pass.
- No API payload, authorization role, transition state machine, ledger proof,
  or finalized evidence behavior changes.

## Consequences

Treasury operators receive consistent, action-oriented recovery language
across supported locales. Future vault-release states must add catalog entries
and tests before they can be rendered. The page continues to describe
operational evidence only; it does not claim tax filing, legal compliance, or
tax advice.

