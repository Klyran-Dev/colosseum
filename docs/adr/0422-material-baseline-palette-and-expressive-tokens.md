# ADR-0422 — Material baseline palette and full Expressive token families

- Status: Accepted. Supersedes the palette decision in ADR-0311 (Solana Aurora);
  earlier ADRs remain as history.
- Date: 2026-10-06
- Decision owners: Product Design, Principal UI Architecture

## Context

The shell used a hand-tuned "Solana Aurora" mint/teal role matrix (seed
`#8DF0CE`). Product direction is to start from the **default Material palette**,
not a green/teal brand palette, and to comply with the Material 3 Expressive
token system. The existing theme lacked the full corner scale, spring motion,
type scale, state layers, and several colour roles (fixed roles, surface tint,
scrim).

## Alternatives

| Option | Verdict |
| --- | --- |
| Keep hand-tuned Aurora and add tokens | Rejected: contradicts product direction; hand-tuned values drift from the spec. |
| Runtime dynamic colour (wallpaper/user seed) | Rejected: non-deterministic evidence/screenshots, unaudited contrast, breaks pinned contract tests. |
| Hand-copy Material baseline hex values | Rejected: no provenance; cannot be regenerated or verified. |
| **Generate from the baseline seed with Google's `material-color-utilities`, commit the output, verify by test** | **Selected.** |

## Decision

1. **Seed `#6750A4`** (Material baseline), library default **TonalSpot**
   variant, standard contrast, `@material/material-color-utilities` **0.4.0**
   (Apache-2.0, dev dependency, pinned exactly, never shipped to browsers).
   Light primary `#65558F`, dark primary `#CFBDFE`.
2. Full role set: primary/secondary/tertiary/error families, background and
   the complete surface and surface-container ladder, outline, inverse roles,
   surface tint, scrim, shadow, and the primary/secondary/tertiary fixed
   roles.
3. **Warning** has no Material role. It is a custom amber source
   (`#FFB300`) harmonized toward the seed with the library's `Blend.harmonize`,
   using the standard tones (light 40/100/90/10, dark 80/20/30/90).
4. **Shape:** the ten-step scale (none, extra-small 4, small 8, medium 12,
   large 16, large-increased 20, extra-large 28, extra-large-increased 32,
   extra-extra-large 48, full). Buttons are full-radius and morph to medium
   while pressed.
5. **Motion:** six spring tokens (fast/default/slow × spatial/effects) using
   Material's damping/stiffness defaults (spatial 0.9, effects 1.0; stiffness
   1400/700/300 and 3800/1600/800). CSS has no native spring, so the closed-form
   unit-mass step response is sampled to `linear()` with its settle duration.
   Effects springs never overshoot; spatial springs overshoot by about 0.2%.
6. **Type scale:** the M3 baseline sizes, line heights, weights and tracking, plus
   emphasized weight 700 (the published emphasized title/label styles are Bold;
   700 is applied to the other roles by extension).
7. **State layers:** hover 8%, focus 10%, pressed 10%, dragged 16%, expressed as
   role-derived `color-mix`; disabled uses on-surface 38% content / 12% container.

## Enforcement

- `materialPaletteGeneration.test.ts` regenerates the palette and spring tokens
  from the pinned library and fails if any dark, light, or system-light token
  in `styles.css` differs, or if the primary family becomes green-dominant.
- `verify_material_palette_contract.mjs` pins every role value and checks 30
  text/background pairs at 4.5:1 (including the fixed roles).
- `materialExpressiveShellContract.test.ts` and the web colour contract keep
  components on roles and keep raw colour out of component rules.
- Updating the seed or the library version is an ADR change: regenerate, update
  the pinned values, run the contracts.

## Consequences and limits

- State aliases still map "success" to the primary container, which is now
  purple, not green. Status remains colour-independent (icon and text). A
  dedicated success role would be a separate decision.
- Disabled controls follow the Material spec (38% content), which the spec
  exempts from the 4.5:1 text requirement. Disabled state is also explained
  in adjacent text where an action is blocked.
- This slice applies the tokens to content-area controls, headings, and inputs.
  Remaining legacy rules still use `rem` literals and `color-mix` on roles and
  should be migrated incrementally.
- Contrast is verified statically on role pairs; no WCAG claim is made about
  composited `color-mix` states beyond the static pairs.
