# ADR-0310: Tonal surfaces and no decorative gradients in the operational shell

## Status

Accepted — implemented in the SPA stylesheet and verified by the web color
contract. Rendered browser and accessibility evidence remains required for the
full production tenant surface.

## Context

The shell already had a repository-owned Material 3 role palette, but several
base selectors still used gradients to blend primary, secondary, tertiary, and
surface colors. Those blends were not independently auditable as Material
roles, made card boundaries compete with the task hierarchy, and could make a
warning or selected state look decorative rather than meaningful. This was
especially visible in the sidebar, selected navigation, primary actions, and
calculation totals.

Material 3 Expressive permits stronger brand expression through color, shape,
adaptive components, typography, and purposeful motion. It does not require a
gradient-based palette. The product is a high-consequence operational surface,
so tonal elevation and semantic state meaning take precedence over ornamental
effects.

## Decision

Use a flat, role-based tonal surface hierarchy throughout the SPA:

- page and shell backgrounds use `background`/`surface` roles;
- cards and raised work areas use `surface-container` or
  `surface-container-high` plus the shared elevation tokens;
- selected navigation uses `primary-container` with `on-primary-container`;
- primary actions use `primary` with `on-primary`;
- calculation totals use `tertiary-container` with
  `on-tertiary-container`;
- review actions use the explicit `warning` role and `on-warning`;
- no component selector may use a CSS gradient outside the token declaration
  blocks.

This is a presentation-only decision. It cannot change authorization,
calculation, wallet, transaction, or settlement behavior. Expressiveness is
retained through the approved palette, shape tokens, responsive layout,
typography hierarchy, and bounded motion that remains disabled for reduced
motion users.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep component gradients | Visually energetic and familiar | Blends unrelated semantic roles, weakens state scanning, and is hard to audit across themes | Rejected |
| Use arbitrary component colors | Fast local styling | Reintroduces palette drift and inconsistent meaning | Rejected |
| Use a full third-party Material theme | Broad component defaults | Adds dependency/ownership surface and cannot encode settlement failure semantics | Rejected |
| **Use Material roles with flat tonal surfaces and shared elevation** | Deterministic, theme-safe, accessible, low runtime cost, clear operator hierarchy | Requires disciplined role selection and contrast review | **Selected** |

## Invariants and failure policy

- Component styles contain no gradient functions or raw color literals.
- A state uses the same semantic role in every workspace and theme.
- Surface hierarchy is expressed by Material surface roles and shared elevation,
  not by mixing unrelated accent colors.
- The static contract fails closed when a gradient is added outside the token
  declaration blocks.
- Light and dark role-pair contrast checks and rendered axe journeys remain
  mandatory; this ADR does not replace either form of evidence.

## Verification

- The color contract is written and run failing-first against the previous
  gradient selectors.
- Gradients were removed from the brand mark, shell/sidebar, selected
  navigation, cards, primary actions, calculation totals, and review action.
- `scripts/verify_web_color_contract.sh` passes.
- `make material-palette-contract`, component tests, typecheck, production
  build, and `make web-browser-check` remain required before review.

## References

- [Material Design 3](https://m3.material.io/)
- [Material foundations and design tokens](https://m3.material.io/foundations/)
- [ADR-0298: Material 3 Expressive visual system](0298-material-3-expressive-visual-system.md)
- [ADR-0300: Semantic color-token migration](0300-semantic-color-token-migration.md)
- [ADR-0304: Static contrast contract](0304-material-role-contrast-contract.md)
