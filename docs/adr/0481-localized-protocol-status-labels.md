# ADR-0481: Render protocol statuses as localized operator labels

- Status: Accepted
- Date: 2026-10-10
- Owners: Web platform

## Context

The treasury release API correctly uses stable protocol values such as
`proposed`, `onchain_created`, and `unknown`. The release lifecycle surface was
rendering those values directly, which exposed implementation vocabulary to
operators and bypassed the existing locale catalog. That made the interface
look unfinished and reduced clarity for non-English operators.

## Decision

Keep protocol status values unchanged at the API and domain boundaries, but map
the validated `VaultReleaseStatus` union to localized lifecycle messages at the
rendering boundary. The mapping is exhaustive and has no raw-value fallback;
an unrecognized status is rejected by the response parser before rendering.

## Alternatives considered

1. Keep raw protocol values. Rejected because they are not operator-facing
   language and are inconsistent with the rest of the localized surface.
2. Format values mechanically by replacing underscores. Rejected because it
   still leaks implementation vocabulary and cannot express locale-specific
   terminology.
3. Translate status values in the API. Rejected because API contracts must
   remain locale-neutral and machine-stable.

## Consequences

- Lifecycle status pills and summaries are readable in all supported locales.
- API and audit evidence retain stable machine values.
- Adding a new status requires a parser/catalog/rendering update and a test.

## Verification

- A German UI test proves `proposed` renders as `Vorgeschlagen` and never as
  the raw protocol value.
- Existing frontend, typecheck, accessibility, and Docker browser checks
  remain required for the UI release gate.
