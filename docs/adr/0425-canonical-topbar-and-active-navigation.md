# ADR-0425 — Canonical topbar, status chips, menus, and active navigation

- Status: Accepted. Amends ADR-0423 (active navigation fill) and ADR-0419.
- Date: 2026-10-06
- Decision owners: Product Design, Principal UI Architecture

## Context

1. The active navigation item rendered as a grey `secondary-container` pill
   (ADR-0423). In review it was too quiet to read as "you are here".
2. The topbar (status chips, display settings, session control, account menu)
   was styled by about 60 rules across many layers, with 9-12px type, uppercase
   letter-spaced labels, bordered menus, and a ring around the attention chip.
3. Investigating the result exposed a latent defect that predates this work:
   seven `--app-*` aliases were referenced but never declared
   (`--app-surface-container-low` ×11, `-high`, `-lowest`, `--app-on-secondary`,
   `--app-on-secondary-container` ×4, `--app-on-tertiary`, `--app-on-warning`).
   An undefined `var()` makes the declaration invalid at computed-value time,
   so panel, card, sidebar and menu backgrounds silently rendered transparent.

## Decision

1. **Active navigation** uses `primary-container` / `on-primary-container`, a
   bold label, a plain icon, and an `on-primary-container` hover layer. It stays a
   full-radius pill with no inset bar. (A valid M3 role, vivid in both themes.)
2. **Topbar chips** follow M3 assist chips: 32px minimum height, small corner,
   label-medium type, outlined by default, a tonal container with no border for
   ready/attention, `secondary-container` while a disclosure is open, and a role
   state layer on hover. The status dot colour per state lives in the same block.
3. **Menus** are elevated tonal surfaces (`surface-container-high`, level-2
   elevation, large corner, no border); inner rows use `surface-container`.
4. **Account trigger** is a filled full-radius avatar (`primary-container`).
5. **Status pills** use sentence-case label-medium type instead of uppercase.
6. **Declare the missing aliases** and add a test that fails if any `var(--x)`
   references a custom property that is not declared (or has no fallback).
7. The contrast contract also covers `on-surface-variant` on
   `surface-container`, `-low`, `-high`, and `-highest` (34 pairs at 4.5:1),
   because panels and menus now render on those tones.

## Evidence

- Before/after computed-style and geometry snapshots (all 16 routes, real dark
  and light themes, transitions disabled) list only the intended changes.
- Harness corrections found along the way: the first light snapshot never
  switched theme (the tokens live on `<html>`), and transitions made snapshots
  read start values. Both were fixed and the ADR-0424 comparison re-run: 0
  differences in both themes.
- 371 unit tests, 4 web contracts, and 49 real-browser journeys with axe pass.

## Consequences

- Panels, sidebar and cards now show their real tonal fills; the page looks
  less flat and slightly different from earlier reviews.
- The topbar is about 8px taller and chips are wider (larger type). Narrow
  widths wrap through the existing mobile rules.
- Reviewers should expect further transparent-background fixes to be visible
  wherever an undefined alias previously hid a surface.
