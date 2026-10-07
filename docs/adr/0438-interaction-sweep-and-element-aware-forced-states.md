# ADR-0438 — Interaction sweep and element-aware forced states

- Status: Accepted. Supersedes the "six controls" limit in ADR-0433.
- Date: 2026-10-07
- Decision owners: Principal UI Architecture

## Context

ADR-0433 proved forced `:hover`, `:active`, and `:focus-visible` snapshots equal
real interaction on six hand-picked controls. Sampling leaves every other control
kind unproven, and the first corrections already showed the forced states were
incomplete in ways a sample could miss.

## Decision

1. **Sweep instead of sample.** `e2e/interaction-sweep.spec.ts` (normal gate) takes
   each distinct kind of enabled, visible interactive element (tag, role, input type,
   and class set) across all 16 workspaces, closed and with popovers open, and
   requires real mouse hover and a real press to equal the forced states, and the
   style to return to rest afterwards. `e2e/style-fixtures.spec.ts` runs the same
   check over every captured unit-test state in snapshot mode, where the enabled
   primary buttons, fields, and links exist. Skips are explained (for example one
   element covered by another popover) and bounded; a kind inside a closed popover is
   retried when the popover is open rather than counted as covered.
2. **Element-aware forced states.** The sweep found what the six controls hid. A
   mouse press also focuses the control, and on text-entry controls and selects
   it matches `:focus-visible`, while a button press does not. The snapshot helper now
   forces `hover+active+focus` for a press, plus `focus-visible` for text entry and
   selects (`forcedClassesFor`, `TEXT_ENTRY`), and keyboard focus forces
   `focus+focus-visible`. The snapshot format is now 5.

## Evidence

- Signed-out workspaces: 15 distinct kinds verified, 1 explained skip.
- Captured states: 18 distinct kinds verified, 0 skipped, 0 mismatches.
- Before the corrections the sweep failed on checkboxes, date inputs, and selects
  (missing focus and focus-visible styling in the forced press).
- 425 unit tests and 78 real-browser tests pass.

## Limits

- A kind is identified by tag, role, type, and class set; two controls of the same
  kind in different contexts are assumed to share rules.
- `select` and text-entry focus-visible behaviour is measured in Chromium; WebKit
  and Firefox may differ and are checked separately (ADR-0439).
- Real touch and multi-pointer gestures are covered by ADR-0437, not by this sweep.
