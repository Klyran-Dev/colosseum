# ADR-0398: Material shell landmark and role boundary

- Status: Accepted
- Date: 2026-10-05
- Owners: Web Platform and Design System

## Context

The product shell is the primary operator surface for a high-consequence,
multi-workspace application. Keyboard users need a reliable skip target, and
the shell must preserve Material 3 Expressive role pairing without allowing
component rules to introduce a second palette. The skip link referenced an
obsolete `workspace-content` identifier while the main landmark was
`main-content`. The stylesheet also had multiple historical shell layers, so
the final role-only application boundary needed to be explicit and tested.

## Decision

1. The skip link targets the existing `main-content` main landmark. The target
   is stable across every workspace and does not depend on lazy child content.
2. The shell's final application layer uses only semantic Material roles and
   approved role-derived tokens for shell surfaces, navigation, identity,
   popovers, and state containers. It does not introduce raw color literals,
   gradients, or data-dependent hues.
3. Forced-colors rules remain the last accessibility override and may replace
   Material roles with the user agent's system colors.
4. A rendered UI test and the repository color contract are release gates;
   changing the landmark or palette boundary requires updating both evidence
   and this ADR.

## Alternatives considered

### Add a second `workspace-content` landmark

Rejected. Duplicate or competing skip targets make keyboard navigation less
predictable and preserve the stale contract instead of correcting it.

### Remove the skip link

Rejected. The shell contains a persistent navigation rail and topbar; removing
the bypass would regress WCAG keyboard navigation.

### Keep component-local colors and rely on visual review

Rejected. Visual review cannot reliably detect a palette regression across all
locales, themes, forced-colors mode, and lazy workspaces.

### Use a runtime theme or arbitrary per-component colors

Rejected. It weakens reproducibility, complicates contrast evidence, and makes
state meaning ambiguous. The static SPA uses the versioned role matrix.

## Invariants

- The skip link resolves to exactly one visible main landmark with the stable
  `main-content` identifier.
- Navigation, identity, popover, and status surfaces use semantic roles and
  their intended `on-*` pairings.
- No raw color literal, gradient, or inline color enters production web
  source outside the token declaration boundary.
- Forced-colors, reduced-motion, keyboard, RTL, and responsive behavior remain
  effective after role-layer changes.
- Palette changes cannot influence authorization, signing, tax calculation,
  settlement bytes, or server-owned evidence.

## Verification

- Failing-first ProductShell test for the skip-link/main-landmark contract.
- Material 3 Expressive rendered contract and role-only color scan.
- Frontend unit/typecheck/build, browser journeys, accessibility/color
  contracts, and repository `make check`.

## Consequences

Keyboard navigation has a deterministic bypass into the active workspace, and
future shell styling has one auditable semantic boundary. The role matrix
remains a static baseline palette; dynamic color or additional themes require
a separate contrast and design-system decision.
