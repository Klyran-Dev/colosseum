# ADR-0386: Versioned financial-workspace error language

- Status: Accepted
- Date: 2026-10-05
- Owners: Web Platform, Payments, Tax Engine UX
- Bounded contexts: Document Quote, Settlement Checkout, Wallet Boundary

## Context

The primary document-quote and settlement-checkout flows already use localized
catalogs for labels, instructions, status, and successful results. Several
operator-facing failure paths still construct English copy inline, including
service unavailability, missing authorization, expired quotes, route and split
plan mismatches, wallet recovery, and reconciliation outcomes. This creates
language drift, makes Arabic and German flows regress to English at the moment
an operator most needs clarity, and makes review of safety-critical copy
difficult.

These messages are presentation copy only. HTTP status, support references,
durable intent state, authorization decisions, and settlement behavior remain
server-owned and must not be translated or inferred in the browser.

## Decision

Add reviewed `errors` sections to the existing versioned document-quote and
checkout message catalogs for every supported locale. The catalogs own the
operator wording for validation, transport fallback, wallet, expiry, durable
intent, and reconciliation states. Dynamic values are inserted only through
typed message functions (for example, a bounded HTTP status or an already
validated durable-intent status), never through raw provider or exception
text.

Parsing and domain helpers continue to receive the authored copy as an
explicit argument where they throw `OperatorFacingError`. Ordinary exceptions
continue to use the workspace fallback. No API, digest, authorization,
settlement, tax, or protocol contract changes are permitted in this ADR.

## Alternatives considered

1. Keep inline English strings. Rejected because it fails localization and
   makes safety-critical recovery copy inconsistent.
2. Add one generic global error dictionary. Rejected because workspace state
   and operator action differ materially between document evidence and wallet
   settlement.
3. Return localized error copy from the API. Rejected because locale is a
   browser presentation concern and server errors may contain untrusted or
   sensitive detail.
4. **Extend each existing bounded workspace catalog. Selected.** This keeps
   domain ownership local, preserves the existing locale fallback contract,
   allows message-specific review, and avoids a new runtime dependency.

## Invariants and acceptance criteria

- Every supported locale has non-empty copy for every financial error key.
- Unknown locales resolve to the exact English catalog.
- No production error path in the document-quote or aggregate-checkout
  surfaces contains authored English inline copy.
- Dynamic status/HTTP values are bounded and passed through typed functions;
  provider, parser, and exception diagnostics remain redacted.
- No request payload, calculation digest, settlement artifact, wallet
  authorization, or state transition changes.
- Failing-first catalog tests, frontend tests, typecheck, production build,
  Material/color/accessibility contracts, and real-browser journeys pass.

## Consequences

Operators receive consistent plain-language recovery guidance in the selected
UI language. Catalog review becomes part of the public UI contract. Adding a
new safety-critical failure state requires catalog entries and tests before it
can be rendered. Legal/tax meaning remains deliberately conservative: the UI
describes withholding and settlement state, never legal compliance or filing.
