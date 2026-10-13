# ADR-0146: Centralized locale, time-zone, and RTL display contract

- Status: Accepted
- Date: 2026-09-27
- Owners: Principal Low-Latency Architecture, Web Platform, Accessibility
- Bounded contexts: Operator UI, Checkout UI, Governance UI, Evidence UI

## Context

The SPA already has rendered axe-core coverage and keyboard focus contracts,
but display formatting is still implemented independently in the root quote
workspace and lazy operational panels. That creates inconsistent number and
timestamp output, makes large atomic amounts vulnerable to accidental
JavaScript `Number` conversion, and leaves language direction and time-zone
selection implicit in the browser environment. A Tax Engine financial UI must make
display context explicit without changing server-owned money, tax, or evidence
semantics.

## Decision

Create one small, dependency-free web display contract in
`apps/web/src/formatters.ts`:

- format atomic integer values with `BigInt` and `Intl.NumberFormat` using an
  explicit locale, preserving exact integer and two-decimal atomic semantics;
- format ISO timestamps and Unix epochs with an explicit IANA time zone and
  locale, failing safely to the original value if the input or time zone is
  invalid;
- derive text direction from the selected locale using a bounded RTL language
  set (`ar`, `fa`, `he`, `ur`), with LTR as the safe default; and
- validate time zones through `Intl.DateTimeFormat` before using them.

The root SPA owns a small display-settings control with bounded language
options (`en-US`, `de-DE`, `ar-EG`, and `ja-JP`) and representative IANA time
zones. It sets the document language and the app root direction, while API
payloads, tax rules, atomic values, and authorization remain unchanged. Lazy
workspaces receive the display context as props and must not read ambient
locale state directly. An invalid time zone is visible to the operator and
uses UTC for safe rendering; it never changes server evidence.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Let each component use browser-default `Intl` formatting | Rejected | Produces inconsistent output, hides time-zone assumptions, and makes RTL behavior untestable. |
| Add a large i18n framework before establishing numeric/time semantics | Deferred | Translation catalogs are a later product track; the first safety boundary is exact locale-aware financial/time display with no new runtime dependency. |
| Convert atomic values to JavaScript `Number` for currency formatting | Rejected | Loses precision for large values and violates exact financial display semantics. |
| **Centralized pure formatter module plus explicit root display context** | **Selected** | Keeps formatting deterministic, testable, dependency-free, exact for `BigInt`, and composable across lazy workspaces. |

## Invariants and failure behavior

- Formatting never changes API payloads, signed bytes, facts, rule digests, or
  settlement amounts.
- Atomic values are never converted through `Number`.
- Invalid numeric input remains an explicit formatter error rather than a
  fabricated amount.
- Invalid timestamps or time zones do not crash the workspace; the original
  timestamp or UTC fallback is displayed and the invalid setting is visible.
- RTL changes logical document direction and layout flow; it does not reverse
  Solana addresses, digests, signatures, JSON, or other code-like evidence.
- The browser remains a display layer and never becomes a tax or settlement
  authority.

## Verification and follow-up

Unit tests cover locale grouping/decimal separators, large `BigInt` values,
timestamps in multiple time zones, RTL language detection, and invalid-input
fallbacks. Rendered accessibility tests cover the settings control and all
lazy workspaces under the RTL locale. Full translation catalogs, human review
with screen readers, real-browser responsive evidence, and legal/tax copy
review remain release gates.
