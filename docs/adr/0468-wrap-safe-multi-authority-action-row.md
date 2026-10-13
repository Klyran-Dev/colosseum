# ADR-0468: Use a wrap-safe multi-authority action row

- Status: Accepted
- Date: 2026-10-10
- Owners: Product engineering, accessibility, and operations

## Context

Refund claims, refund execution, and refund cancellation expose separate
signing/submission authority steps inside an `.evidence-actions` container. The
markup grouped the controls in `.button-row`, but no shared style existed for
that class. The global button rule therefore made each nested button a full
grid-width action, obscuring the relationship between the steps and creating a
poor scan path on desktop panels.

## Decision

Style `.button-row` as a flex row that wraps. Each button participates in the
row with a bounded flexible basis and can shrink below intrinsic label width.
The row remains usable at narrow widths by wrapping buttons rather than
allowing horizontal overflow. Button semantics, ordering, and disabled states
are unchanged.

## Alternatives considered

1. **Leave buttons as independent full-width actions.** Rejected because the
   authority sequence is visually fragmented and consumes unnecessary height.
2. **Force a permanent two-column grid.** Rejected because localized labels and
   narrow panels can overflow or become unreadable.
3. **Use a flexible wrapping action row.** Selected because it preserves the
   relationship between authority steps and remains safe at narrow widths.

## Invariants and acceptance criteria

- Signer and submitter controls remain separate buttons with their existing
  handlers and disabled conditions.
- Buttons are grouped visually and wrap without page-level horizontal scroll.
- The action row remains keyboard navigable in DOM order.
- No authority or financial state is inferred from presentation styling.

## Verification

- The stylesheet contract requires the shared row and button flex rules.
- Frontend component tests retain the refund action markup and handlers.
- The Docker browser suite covers 320px reflow, keyboard interaction, themes,
  localization, and accessibility.
