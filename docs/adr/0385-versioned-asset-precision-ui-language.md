# ADR-0385: Versioned asset-precision UI language

## Status

Accepted — 2026-10-05

## Context

ADR-0384 makes asset decimal scale an explicit, authoritative part of the
quote and settlement contract. The SPA still repeated the English label
“Asset decimal places” and an English validation error in several workspaces.
That creates inconsistent operator language, weakens the localization
boundary, and makes a safety-critical financial input look like an internal
implementation detail.

## Decision

Create one small, versioned `assetScaleMessagesFor(locale)` catalog owned by
the browser presentation layer. Quote, document quote, document simulation,
and rule-governance workspaces consume the catalog for the label, help text,
and validation copy. The catalog supports every existing product locale and
falls back to the canonical English catalog for unknown locales.

The UI calls the value “Asset precision” and explains that it is the number of
decimal places used by the asset, bounded to 0–9. The numeric value remains an
explicit user/API value; the catalog never changes calculation, formatting,
route binding, canonical digests, or settlement authority.

## Alternatives considered

| Alternative | Consistency | Localization | Financial safety | Decision |
| --- | --- | --- | --- | --- |
| Keep repeated English strings | Fail | Fail | Partial | Rejected |
| Add the same field independently to every workspace catalog | Partial | Pass | Pass | Rejected; duplicates semantic copy |
| Put presentation language in the Rust/API contract | Partial | Partial | Risk of coupling | Rejected |
| **Use one browser-owned versioned semantic catalog** | **Pass** | **Pass** | **Pass** | **Selected** |

## Invariants and acceptance criteria

- Every supported locale has non-empty label, help, and validation copy.
- Unknown locales resolve to the exact English catalog.
- Production JSX contains no repeated English asset-precision label or error.
- The input remains bounded to an integer from 0 through 9 and is required.
- No request, digest, result, route, or settlement representation changes.
- Existing component, localization, typecheck, build, browser, and accessibility
  gates remain green.

## Consequences

The control is easier to understand and review across locales, and future
copy changes have one auditable ownership boundary. Adding a locale requires
updating one catalog and its catalog test. The catalog is intentionally not a
general translation framework and does not move domain semantics into UI code.

