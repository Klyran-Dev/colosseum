# ADR-0408: Quiet tonal shell for Material 3 Expressive operations

- Status: Accepted
- Date: 2026-10-06
- Decision owners: Principal UI Architect, Product Design, Accessibility
- Scope: React/Vite operations shell and shared workspace surfaces

## Context

The operations console carries high-risk payment, tax, authorization, and evidence workflows. The existing shell exposed too many equally loud borders, badges, translucent blends, and top-bar controls at once. That visual density made the current task, system health, access state, and technical diagnostics compete for attention.

Material 3 Expressive is not permission to assign a different color to every component. M3 color roles provide a semantic system: surface roles establish hierarchy, primary roles carry the strongest action and selection emphasis, secondary roles support lower-emphasis controls, tertiary roles provide limited expressive contrast, and error roles communicate risk. Containers are paired with their `on-*` roles and custom UI maps to the same tokens.

## Decision

Adopt a quiet tonal shell with one semantic Aurora palette for light, dark, and system themes:

1. The page background uses `surface`; navigation uses `surface-container-lowest`; the top bar uses `surface-container`; workspace panels use `surface-container-low`; popovers use `surface-container-high`. No component invents a new surface color.
2. Primary is reserved for the active navigation state, the single highest-emphasis action, focus indication, and positive/verified state containers where no dedicated success role exists.
3. Secondary is reserved for supporting controls and low-emphasis actions. Tertiary is reserved for small expressive accents and not used as a generic text color across the page.
4. Warning, error, and verified states use explicit semantic aliases and their paired `on-*` colors. They are never used for decoration or to make inactive navigation look important.
5. Tonal layering replaces stacked borders and shadows. Shell panels have no elevation shadow by default; elevation is reserved for anchored menus and dialogs.
6. Technical diagnostics, sign-in details, appearance controls, and access evidence remain available through disclosure/popover patterns but do not occupy the primary visual hierarchy when closed.
7. The shell targets a 1200px reading width on desktop, uses the active workspace as the main visual anchor, and keeps the navigation rail scannable by showing labels without repeating descriptions or access explanations in the closed state.
8. All new shell rules are semantic-token-only in the final precedence layer. Literal color values belong only in the audited theme token blocks and browser metadata contract.

## Alternatives considered

| Option | Rejected/selected | Reason |
| --- | --- | --- |
| Keep the current multi-layer CSS overrides | Rejected | It permits visual drift and makes it difficult to prove which surface or state role is active. |
| Add a unique accent to each bounded context | Rejected | It confuses product category with operational state and increases cognitive load in money-moving workflows. |
| Use a single flat neutral palette | Rejected | It reduces hierarchy and makes primary actions, access decisions, and warnings harder to scan. |
| Adopt semantic M3 roles with quiet tonal layering | Selected | It preserves expressive brand character while keeping color meaning, contrast, and accessibility auditable. |

## Invariants

- No final shell rule contains a literal hex/rgb color or decorative gradient.
- Each filled component uses the paired M3 `on-*` role for its content.
- Closed shell chrome exposes one page identity, one compact system state, and one account affordance; details are progressive disclosures.
- State meaning is never conveyed by hue alone; labels, icons, and accessible names remain present.
- Forced-colors, reduced-motion, keyboard, responsive, and WCAG 2.2 AA behavior remain supported.

## Verification

- `materialExpressiveShellContract.test.ts` proves semantic final-layer rules and rejects decorative gradients/raw colors.
- `verify_material_palette_contract.mjs` proves role completeness and contrast for both schemes.
- `make web-check` proves TypeScript, Vitest, accessibility assertions, and production build.
- Browser checks must review the desktop shell at 1440px and the mobile shell at 390px for hierarchy, overflow, focus order, and disclosure behavior.

