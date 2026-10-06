# ADR-0423 — Canonical navigation and panel skin

- Status: Accepted. Active navigation fill amended by ADR-0425 (primary-container).
- Date: 2026-10-06
- Decision owners: Product Design, Principal UI Architecture

## Context

Navigation items, the sidebar, and panels were restyled by about eight
successive cascade layers. `.nav-item-active` was declared six times and
`.panel` eight, each layer adding or reversing the last. The result was hard to
audit, did not follow the Material 3 navigation drawer item (primary-container
fill, an inset bar, a bordered icon chip, uppercase tertiary group labels), and
made every palette or shape change a multi-layer edit.

A second finding: the forced-colors layer is documented as the final cascade
layer, but ADR-0421 and ADR-0422 appended rules after it, so those later rules
could override its system-colour mapping in high-contrast mode.

## Alternatives

| Option | Verdict |
| --- | --- |
| Append another override layer | Rejected: adds a ninth copy and worsens auditability. |
| Rewrite the whole stylesheet at once | Rejected: too large to verify; the compact rail, mobile shell and forced-colors rules depend on current geometry. |
| **Replace the repeated skin declarations with one canonical block; leave geometry rules in place** | **Selected.** |

## Decision

1. About 50 superseded colour/shape/typography declarations for the sidebar,
   navigation, and panels were removed and replaced by one block, ADR-0423.
2. **Navigation** follows the M3 drawer item: full-radius item, 56dp minimum
   height, `secondary-container` / `on-secondary-container` active indicator
   with a bold label, plain icons (no chip or border), group labels in the
   title-small role in sentence case, and hover/pressed state layers derived
   from `on-surface` at the documented opacities.
3. **Panels** are tonal: `surface-container-low`, no border or shadow,
   extra-large corners (metric cards: large; nested evidence/notice boxes:
   medium).
4. **Geometry is unchanged.** The block sets no `grid-template-columns` or
   padding on nav items, so the compact rail and mobile shell keep their
   existing layout rules.
5. **The forced-colors block is the last block in the stylesheet again.**
6. ADR-0422's button rules exclude `.navigation-compact-toggle`; that control
   is shell chrome and must not receive content-button padding or shape.

## Enforcement

`materialExpressiveShellContract.test.ts` checks the canonical block, that each
nav/panel skin is declared once, that no nav rule sets geometry, that the
forced-colors layer comes after every ADR layer, and that chrome controls are
excluded from the content-button rules. This supersedes the earlier pinned
assertions for primary-container active navigation (ADR-0419).

## Consequences

- The active navigation item is now a quiet secondary-container pill rather
  than the primary-container fill with an inset bar (ADR-0419 chose that look).
  This is a deliberate move to the M3 specification.
- Other legacy surfaces (toolbars, status pills, forms, tables, evidence
  views) still carry repeated declarations and should migrate the same way.
- Contrast for the new pairs is covered by the existing 4.5:1 role-pair
  contract (`secondary-container` / `on-secondary-container`).
