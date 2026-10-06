# ADR-0404: Typed localized authorization affordance reasons

- Status: Accepted
- Date: 2026-10-06
- Deciders: Principal UI Architecture, Security, and Principal Low-Latency Blockchain Architecture
- Scope: Browser navigation and accessibility descriptions for capability states

## Context

ADR-0365 requires authorization recovery copy to use a typed presentation
boundary. The current capability model already receives bounded server reason
codes, but the shell still exposes the English reason string in a navigation
title. That produces inconsistent language, leaks implementation-oriented
copy into a primary affordance, and makes the reason non-localizable.

## Decision

Represent every browser authorization affordance with a bounded
`UiAccessReasonCode`. The code is one of the approved presentation states:

- signed-out, server-check-required, server-denied, missing-expiry,
  unavailable-clock, expired-check, missing-capability-decision, unknown-policy;
- server-confirmed, permission-required, responsibility-required, or
  record-context-required.

The shell maps the code through the versioned English, German, Arabic, or
Japanese catalog. Raw reason strings never render in navigation titles,
accessible descriptions, or the page heading. The mapping is presentation
only; the API remains authoritative and protected handlers continue to
re-authorize every request.

## Alternatives considered

| Alternative | Localization | Diagnostic safety | Operator recovery | Decision |
| --- | --- | --- | --- | --- |
| Render server reason strings directly | Fail | Weak | Inconsistent | Rejected |
| Use one generic message for every state | Pass | Pass | Weak; loses actionable context | Rejected |
| Infer a reason from translated text | Fail | Weak | Unstable | Rejected |
| **Map a bounded reason code through the shell catalog** | **Pass** | **Pass** | **Pass** | **Selected** |

## Invariants and budgets

- Unknown reason codes fail closed before rendering an allow affordance.
- No tenant identifier, credential, raw response, stack trace, or server
  diagnostic is included in presentation copy.
- Reason-code lookup is an in-memory O(1) operation with bounded static data;
  it adds no network, database, or wallet-signing work.
- Locale changes affect copy only and cannot change the authorization decision.
- Every supported locale has a non-empty value for every approved code.

## Verification

- Authorization unit tests assert reason-code classification for anonymous,
  indeterminate, denied, expired, and server-evaluated states.
- Shell localization tests assert all locales cover the complete code set.
- Product-shell tests assert a localized reason is rendered and the raw English
  reason is absent from the navigation title.
- `make check` and the real-browser UI gate remain required.

