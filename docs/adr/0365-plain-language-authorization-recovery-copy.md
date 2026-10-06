# ADR-0365: Plain-language authorization recovery copy

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture, Security, and Principal Low-Latency Blockchain Architecture
- Scope: Browser capability-snapshot failure presentation

## Context

The browser must reject expired or malformed server capability snapshots and
keep protected actions indeterminate. The parser necessarily needs precise
diagnostic messages for tests and support, but exposing those messages in the
primary operator shell makes a recoverable access problem look like an
implementation failure. It also makes localization and consistent recovery
copy impossible.

## Decision

Use a typed parser error for the expiry boundary and a small presentation-key
state in React. The shell maps that state through the localized operational
catalog:

- expired snapshot: explain that the access check is out of date and that a
  refresh is in progress;
- missing expiry: explain that protected actions remain unavailable until the
  access check is refreshed;
- all other capability, transport, timeout, and response-shape failures: say
  that access could not be confirmed and keep protected actions unavailable.

Raw parser and transport diagnostics never render in the primary product
surface. They remain in typed code paths and service-side telemetry/support
records. This is a presentation boundary only: the API continues to
re-authorize every protected operation, and the browser never derives an
allow decision from the error state.

## Alternatives considered

| Alternative | Operator clarity | Localization | Diagnostic safety | Decision |
| --- | --- | --- | --- | --- |
| Render parser/network error text directly | Fail; exposes implementation language | Weak | Weak | Rejected |
| Replace every error with one generic sentence | Partial | Pass | Pass | Rejected; expiry needs actionable recovery guidance |
| **Typed failure key mapped by the localized shell catalog** | **Pass** | **Pass** | **Pass** | **Selected** |
| Put diagnostics in a client-visible technical drawer | Partial | Higher surface area | Risk of leaking response detail | Deferred until a separately governed support-evidence contract |

## Invariants and failure policy

- An expired, malformed, unavailable, or timed-out capability response never
  authorizes a protected action.
- Presentation keys contain no tenant ID, credential, raw response, stack
  trace, or server secret.
- Locale changes update the visible recovery copy without reusing stale text or
  changing the authorization snapshot.
- The parser remains independent of React, localization, network, and clock
  side effects except for its explicit input clock.
- The wallet-signing and settlement paths do not wait on or depend on this
  presentation mapping.

## Verification

- Parser tests classify the expiry boundary with a typed error.
- Operational catalogs cover English, German, Arabic RTL, and Japanese.
- App tests verify plain-language expiry copy and assert the parser diagnostic
  is absent from the rendered shell.
- The pinned NGINX/Playwright journey verifies the expired snapshot, protected
  navigation `Needs access check` state, and axe accessibility result.

