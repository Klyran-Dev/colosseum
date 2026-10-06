# ADR-0341: Versioned shell chrome and session localization

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React SPA shell controls, display preferences, and session boundary

## Context

ADR-0340 made the primary navigation and workspace orientation locale-aware.
The remaining shell controls—navigation disclosure, display settings, and
session actions—still remained English in a translated shell. That is a
material usability and accessibility defect: operators can orient themselves
in one language but cannot reliably understand the control used to sign in,
change locale, or change theme.

## Decision

Extend the same typed, immutable, versioned catalog with bounded shell-chrome
and session-control messages for `en-US`, `de-DE`, `ar-EG`, and `ja-JP`.
The catalog owns presentation strings only. Server-returned authorization,
identity-provider, and operational error text remains evidence from the
service and is not machine-translated in the browser.

`AuthSessionControl` receives the validated locale through the shell boundary
and uses the catalog for its labels, actions, and local-development warning.
Unknown locales use the canonical English catalog. No locale lookup, network
request, dynamic import, timer, or dependency is added to a financial path.

Workspace-panel field catalogs, server-originated messages, legal/tax copy
review, and human linguistic review remain separately tracked release work.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Leave shell controls in English | No implementation cost | Contradictory language experience and inaccessible session actions | Rejected |
| Browser/runtime machine translation | Broad apparent coverage | Nondeterministic financial terminology, network/privacy cost, hard-to-review output | Rejected |
| Pass ad-hoc labels from each component | Small local diffs | Drift, missing locales, inconsistent action language | Rejected |
| **Typed versioned shell/session catalog** | Deterministic, bounded, reviewable, zero runtime dependency | Requires deliberate translation and later panel catalogs | **Selected** |

## Invariants and failure behavior

- Localization cannot alter authorization, tenant scope, credentials, facts,
  tax calculations, amounts, wallet behavior, settlement, or evidence bytes.
- Locale and message keys are bounded at compile time; server text is never a
  catalog key.
- Missing catalog data falls back to non-empty canonical English strings.
- Credential values and identity-provider claims never enter localized text.
- RTL remains controlled only by the validated locale boundary.

## Latency and resource budget

- Static object lookup is O(1), with no request, dynamic import, or timer.
- The shell/session catalog must remain under 32 KiB before compression.
- The added work is outside quote, tax, authorization, wallet, settlement,
  and indexer critical paths.

## Verification

- Failing-first catalog tests cover every supported locale, fallback, bounded
  chrome/session keys, and dynamic timezone interpolation.
- Component tests prove localized display settings and session actions without
  rendering credentials or changing authorization state.
- Docker browser coverage verifies locale switching, RTL, axe, and sign-in
  control behavior through the real NGINX-served SPA.
- Manual linguistic, legal/tax, screen-reader, and operator review remain
  release gates.

## References

- [ADR-0146: Centralized locale, timezone, and RTL display contract](0146-centralized-locale-timezone-and-rtl-display-contract.md)
- [ADR-0340: Versioned shell and navigation localization](0340-versioned-shell-navigation-localization.md)
