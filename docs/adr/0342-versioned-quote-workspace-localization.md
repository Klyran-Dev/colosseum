# ADR-0342: Versioned quote-workspace localization

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: Aggregate quote and tax-simulator workspace presentation

## Context

ADR-0341 localized the shell controls and session boundary, but the primary
quote and simulator forms still rendered English labels and guidance after an
operator selected another supported locale. The quote workspace is the first
financially meaningful operator journey, so a partially translated form is
misleading and increases input risk.

## Decision

Add a typed, immutable, versioned quote workspace catalog for `en-US`,
`de-DE`, `ar-EG`, and `ja-JP`. It covers the primary quote/simulator form,
location-verification disclosure, explicit input guidance, retry protection
copy, result summary, progressive technical disclosures, and non-advice
language.

The catalog is presentation-only. Financial values, currency codes, rule
references, jurisdiction values, calculation identifiers, evidence digests,
component labels, and server-returned explanations remain authoritative data
and are not rewritten by the browser. Unknown locales fall back to English.
Workspace panels with distinct domain contracts remain separate slices so a
catalog cannot become an unreviewed global translation layer.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep quote forms in English | No code change | Inconsistent financial workflow and higher operator input risk | Rejected |
| Runtime machine translation | Broad apparent coverage | Nondeterministic tax terminology, network/privacy cost, no reviewable copy contract | Rejected |
| Shared untyped string map | Fast initial wiring | Missing keys, accidental technical-value translation, weak compile-time review | Rejected |
| **Typed bounded quote-workspace catalog** | Deterministic, reviewable, local, explicit authority boundary | Requires deliberate locale and legal-copy review | **Selected** |

## Invariants and failure behavior

- Locale selection cannot change request payloads, amount strings, rule IDs,
  jurisdiction facts, authorization, wallet state, or evidence bytes.
- Every supported locale has non-empty primary-flow copy and deterministic
  English fallback.
- Technical and server-owned values remain outside the catalog.
- The existing explicit-input boundary remains: no example amount, rule,
  jurisdiction, or currency is introduced by localization.
- Tax withholding remains distinct from filing, reporting, legal compliance,
  and tax advice in every locale.

## Latency and resource budget

- Static in-bundle lookup is O(1); no network, dynamic import, timer, or
  financial-path dependency is introduced.
- The catalog must remain under 48 KiB before compression.
- Locale changes do not trigger quote calculation, persistence, RPC, wallet, or
  authorization work beyond the existing display-preference update.

## Verification

- Failing-first catalog tests cover every supported locale, required key,
  deterministic fallback, and preservation of protocol identifiers.
- App tests cover empty financial inputs and localized labels/placeholders.
- Docker browser coverage verifies German/Arabic primary-form presentation,
  RTL, axe, and unchanged route behavior through NGINX.
- Manual tax/legal terminology, screen-reader, and operator review remain
  release gates.

## References

- [ADR-016: Explainable, accessible tax UI/UX](0016-tax-ui-ux.md)
- [ADR-0273: Explicit financial-input boundary](0273-explicit-financial-input-boundary.md)
- [ADR-0340: Versioned shell and navigation localization](0340-versioned-shell-navigation-localization.md)
- [ADR-0341: Versioned shell chrome and session localization](0341-versioned-shell-chrome-and-session-localization.md)
