# ADR-0340: Versioned shell and navigation localization

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React SPA shell navigation and workspace context

## Context

The display-preference boundary already validates locale, formats money and
time, and sets document direction, but the primary navigation and workspace
heading remained English for every supported locale. That creates an
inconsistent operator experience and makes the language selector overstate
its coverage.

## Decision

Add a typed, immutable, versioned shell message catalog for the four supported
locales: `en-US`, `de-DE`, `ar-EG`, and `ja-JP`. The first slice localizes the
product shell's navigation groups, workspace labels/descriptions, risk
descriptions, and workspace status announcement. Message lookup is pure and
bounded; unknown locale keys fail closed to the canonical English catalog.

Workspace panels and their domain terminology remain a separately tracked
localization slice. Until those catalogs are complete, the product must not
claim full translated-workspace coverage; the shell catalog is still useful
because it localizes the primary orientation and navigation boundary.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep English labels and localize formatting only | No code change | Language control is misleading and orientation remains inaccessible | Rejected |
| Runtime machine translation | Broad apparent coverage | Nondeterministic, unsafe for tax/financial terminology, network latency and privacy risk | Rejected |
| Independent ad-hoc strings per component | Fast first change | Drift, missing locales, inconsistent terminology, no reviewable catalog | Rejected |
| **Typed versioned shell catalog with explicit locale entries** | Deterministic, reviewable, bounded, no runtime dependency, easy golden tests | Requires deliberate translation review and later panel catalogs | **Selected** |

## Invariants and failure behavior

- Localization is presentation-only and cannot alter authorization, facts,
  tax calculations, amounts, wallet behavior, settlement, or evidence.
- Locale keys and workspace identifiers are compile-time bounded; no raw server
  string becomes a translated message key.
- Missing or malformed catalog data returns the canonical English message and
  never renders an empty navigation label.
- RTL direction remains controlled by the existing locale boundary.
- Technical identifiers and evidence remain in the appropriate technical
  disclosure; translation never changes their bytes or digest inputs.

## Latency and resource budget

- Static in-bundle lookup with O(1) object access.
- No network request, dynamic import, timer, or dependency.
- Four bounded shell catalogs add less than 16 KiB before compression.
- Zero impact on quote, tax, authorization, wallet, settlement, and indexer
  paths.

## Verification

- Add failing-first catalog tests for all supported locales, every workspace,
  deterministic fallback, and non-empty labels/descriptions.
- Add shell UI coverage for locale-driven navigation and status text.
- Add real-browser locale coverage with RTL/axe checks and retain manual
  linguistic, legal/tax terminology, and assistive-technology review as a
  release gate.

## References

- [ADR-0146: Centralized locale, timezone, and RTL display contract](0146-centralized-locale-timezone-and-rtl-display-contract.md)
- [ADR-0339: Polite workspace navigation announcement](0339-polite-workspace-navigation-announcement.md)
