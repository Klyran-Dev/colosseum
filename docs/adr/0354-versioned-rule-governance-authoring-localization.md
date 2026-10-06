# ADR-0354: Versioned rule-governance and authoring localization

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React tax-rule governance workspace and structured rule workbench

## Context

The rule-governance workspace is a high-consequence authoring surface. It
contains lifecycle transitions, provenance evidence, progressive brackets,
tax-on-tax dependencies, rounding, thresholds, exemptions, and a server-bound
simulation. The shell already carries a locale and time zone, but the
governance and workbench copy remained English-only. That creates a production
usability and review risk for operators working in German, Arabic RTL, or
Japanese.

The browser must never translate or mutate protocol values. Rule JSON, field
values, status discriminators, evidence payloads, UUIDv7 references, digests,
and API request bodies remain canonical. Only labels, explanations, options'
presentation text, and bounded status descriptions are localized.

## Decision

1. Add immutable, typed, versioned catalogs for governance and structured rule
   authoring in `en-US`, `de-DE`, `ar-EG`, and `ja-JP`.
2. Pass the existing shell locale through GovernancePanel into RuleWorkbench;
   no translation work enters the tax-engine, signing, or settlement paths.
3. Keep machine-facing values stable: `structured`/`json`, `flat`/
   `progressive`, `Exclusive`/`Inclusive`, rounding and threshold enum values,
   component IDs, JSON, status discriminators, and evidence remain unchanged.
4. Localize fail-closed validation, lifecycle, loading, empty, simulation, and
   provenance guidance. Unrecognized future status or enum values remain
   visible in their raw form rather than being guessed.
5. Use progressive disclosure for exact JSON and evidence. The localized
   primary path remains understandable without exposing protocol jargon.

## Alternatives considered

| Alternative | Benefit | Risk | Decision |
| --- | --- | --- | --- |
| Leave governance in English | No implementation cost | Excludes operators and weakens review clarity | Rejected |
| Runtime machine translation | Broad language coverage | Unstable tax/legal wording and non-deterministic evidence review | Rejected |
| Translate JSON keys and enum values | Visually uniform | Mutates the canonical rule contract and risks invalid submissions | Rejected |
| Split the workbench into a separate localized application | Tailored UX | Duplicates authorization, validation, and protocol boundaries | Rejected |
| **Typed catalogs with presentation-only localization** | **Deterministic, accessible, protocol-safe, consistent with the shell** | **Maintains a bounded translation review surface** | **Selected** |

## Invariants and failure policy

- A locale change cannot alter a request body, rule digest, status value, or
  evidence payload.
- Missing catalog entries fail the test/build contract; they do not fall back
  to client-side translation or a fabricated status.
- Validation remains server-authoritative. Local workbench checks are guidance
  and never imply publication, legal compliance, or payment authorization.
- Arabic uses the existing document direction contract; code, JSON, digests,
  and protocol identifiers remain left-to-right.
- All loading, unavailable, error, retired, and simulation states retain their
  explicit fail-closed meaning in every supported locale.

## Verification

- `pnpm --dir apps/web test -- governanceMessages ruleWorkbenchMessages`
- `make ui-language-contract`
- `make web-check`
- `make web-browser-check`
- `make check`

Manual tax-owner, linguistic, screen-reader, RTL, and operator-usability review
remains a release gate.

## References

- [ADR-0316: Versioned rule-release provenance](0316-versioned-rule-release-provenance.md)
- [ADR-0340: Versioned shell and navigation localization](0340-versioned-shell-navigation-localization.md)
- [ADR-0341: Versioned shell chrome and session localization](0341-versioned-shell-chrome-and-session-localization.md)
