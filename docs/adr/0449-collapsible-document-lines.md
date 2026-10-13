# ADR-0449: Collapsible document-line authoring

- Status: Accepted
- Date: 2026-10-09
- Owners: Product engineering and tax domain owners

## Context

Document quote and document simulation workflows support up to 256 lines. A
fully expanded line editor repeats identity, gross amount, net amount, and
exemption controls for every line, making larger documents hard to scan and
increasing accidental edits during review.

## Decision

Render each document line as an accessible disclosure section. The first line
is open initially, existing additional lines are closed, and a newly added
line opens automatically. The summary shows the localized line number, line
identifier when present, and a compact gross/net preview. The existing inputs,
validation, and canonical request serialization remain unchanged inside the
disclosure content.

Disclosure state is local presentation state only. It is never sent to the API
and cannot affect validation, totals, evidence, or authorization.

## Alternatives considered

1. Keep all lines expanded. Rejected because repeated controls overwhelm the
   document review surface.
2. Replace each line with a single opaque JSON editor. Rejected because it
   weakens field-level validation, localization, and accessibility.
3. Use a separate wizard step per line. Rejected because document-level totals
   and line relationships become harder to review together.

## Acceptance criteria

- Toggling a line does not change `TaxDocumentLineDraft` or the serialized
  request.
- The first line is open by default; newly added lines open immediately.
- Summaries remain useful when the line ID is empty and are localized through
  the existing message catalogs.
- The quote and simulation workflows preserve existing validation, error, and
  accessibility behavior.

