# ADR-0430 — Motion tokens, meaningful tints, and wrapping rows

- Status: Accepted
- Date: 2026-10-07
- Decision owners: Principal UI Architecture, Product Design

## Context

With every styled class snapshotted (ADR-0429), the remaining legacy patterns
could be cleaned with proof: a base `button` rule with a raw `.16s ease` timing
and a hover lift that Material Expressive replaces with state layers; 21
`color-mix` values at 1-3% opacity that look like role tints but are invisible
(a ring around the status dot at 1%); and a rule-workbench source row whose
fields were squeezed to a few characters because the global field width and a
non-wrapping flex row left no room ("Gro" for "Gross").

## Decision

1. Express the base button transition with motion tokens (`--md-motion-short`)
   and drop the `translateY(-1px)` lift; filled and tonal buttons already carry
   hover, focus, and pressed state layers (ADR-0422).
2. Replace every `color-mix(... 0-3%, transparent)` with `transparent`. A role
   mixed in at 3% or less is not perceptible; borders stay as structural
   `solid transparent`, which forced-colors mode still renders as a system-colour
   border. Tints from about 4-8% upward are kept as deliberate state tints.
3. Let the workbench source row wrap (`flex-wrap`, `flex: 1 1 7rem`, `min-width: 0`
   for its labels).
4. Tests reject raw transition timings, `translateY` hover lifts, sub-4% tints,
   and a non-wrapping workbench row.

## Evidence

Snapshot diff against the previous state (245 states, both themes): changes are
limited to border and background colours and a box-shadow ring that went from 1%
opacity to none, plus geometry in the nine governance and tax-rule states where
the row wrapped. 400 unit tests, 4 web contracts, and 49 real-browser journeys
with axe pass.

## Consequences and limits

- Hover feedback for legacy buttons that rely only on the base rule is now their
  own specific rule (secondary, card, session actions) or the state layer.
- Wrapping places the "/" fraction separator at the start of a line when the row
  wraps; that is cosmetic and can be revisited with a dedicated fraction control.
