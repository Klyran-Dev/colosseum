# ADR-0419 — Neutral risk taxonomy and calm active navigation

- Status: Accepted — implementation gate
- Date: 2026-10-06
- Decision owners: Principal UI Architecture, Product Design, Accessibility
- Scope: React/Vite product shell and shared semantic status components

## Context

The operator console contains two different kinds of information that must not
share the same visual language:

1. a workflow risk taxonomy (`read`, `write`, `financial`, `governance`); and
2. an operational outcome (`verified`, `attention`, `blocked`, or an error).

The previous shell mapped risk taxonomy to yellow, red, and tertiary fills.
That made ordinary navigation and workflow metadata look like live incidents.
The attached review image shows the resulting problem: many independent
accents compete with the current task, system state, and primary action.

## Decision

1. Risk taxonomy is neutral. Risk labels use `surface-container` and
   `on-surface-variant`, with accessible text remaining the authority for the
   meaning. Risk is not an incident state.
2. Outcome components alone may use semantic state roles: success for verified
   or allowed, attention for pending/indeterminate, and blocked/error for
   denied or failed.
3. The active navigation item uses the primary container as a quiet tonal
   selection surface. It does not add a border or inset shadow; its icon and
   label remain sufficient for focus and selection.
4. Neutral status pills are the default. A filled state pill requires an
   explicit success, attention, or danger class and paired `on-*` role.
5. No new hue, workspace accent, gradient, or inline color may be introduced
   to represent a bounded context or product area.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| One color per risk category | Rejected | Turns taxonomy into alert chrome and creates an unbounded palette. |
| Keep risk colors but reduce saturation | Rejected | Color still implies urgency and remains difficult to audit. |
| Remove risk labels | Rejected | Operators still need to understand the consequence of an action. |
| Neutral taxonomy plus semantic outcome colors | Selected | Preserves meaning, reduces clutter, and matches Material role semantics. |
| Bordered active navigation item | Rejected | Adds a second emphasis channel to an already selected item. |
| Tonal active navigation item without a border | Selected | Gives clear selection with lower visual noise and no loss of keyboard focus. |

## Invariants and verification

- Risk classes never consume warning, error, or tertiary state containers.
- Actual outcome classes continue to expose text and non-color state cues.
- The active navigation item has no border or inset shadow in the final shell
  layer.
- The stylesheet and production TypeScript/TSX remain semantic-token-only.
- `materialExpressiveShellContract.test.ts`, `make web-color-contract`,
  `make material-palette-contract`, `make web-check`, and real-browser checks
  remain release gates.

