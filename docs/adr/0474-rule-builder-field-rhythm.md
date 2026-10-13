# ADR-0474: Use paired fields in the governed-rule builder

## Status

Accepted

## Context

The governed tax-rule authoring panel renders beside the review panel, but its
metadata and rule-component controls were still forced into one narrow column.
The result was a very tall form with the primary draft action far below the
fold, while the adjacent review surface appeared disproportionately empty.
This reduced scanability without improving the underlying validation or
evidence semantics.

## Decision

Use the authoring panel's own inline size as a container query. At 28rem or
wider, pair short metadata and component controls in two equal columns. Keep
effective dates, rule-editor mode, explanatory help, JSON, release evidence,
threshold explanations, and other `.full-field` content spanning the panel.
The existing one-column auto-fit behavior remains the fallback for narrower
panels and localized layouts.

## Alternatives considered

- Keep one column: rejected because the authoring workflow remains needlessly
  tall and hides the primary action.
- Use a viewport breakpoint: rejected because the persistent navigation rail
  changes the panel width independently of the viewport.
- Permanently use two columns: rejected because narrow panels and long
  localized labels need the existing single-column fallback.

## Consequences

- Common short fields can be scanned in pairs and the action becomes easier to
  reach.
- Long explanatory and evidence-bearing content retains full reading width.
- No DOM, validation, serialization, or financial semantics change.
- The rule builder remains usable at narrow widths and in RTL/localized views.

## Verification

- Material shell contract test for the container-aware field rhythm.
- Governance UI tests, frontend typecheck, and `git diff --check`.
- Docker browser gate covering responsive, localized, and accessibility flows.
