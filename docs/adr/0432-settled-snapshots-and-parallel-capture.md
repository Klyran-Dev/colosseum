# ADR-0432 — Settled snapshots, no exempt states, and opt-in parallel capture

- Status: Accepted. Supersedes the "two journeys compared without geometry" limit
  in ADR-0428/0429 and the volatile-state exemption in the diff tool.
- Date: 2026-10-07
- Decision owners: Principal UI Architecture

## Context

Two snapshot states differed between identical runs: the vault reconciliation
mismatch state and the expired-quote checkout state. The diff tool exempted
their geometry and structure. Investigation showed the cause was a capture race,
not time-dependent content: the snapshot could land while an async list was still
loading or while a one-second tick re-rendered, so the page was caught mid-load.
A run also took about eight minutes because everything ran on one worker.

## Decision

1. **Settle before capturing.** Before each snapshot the helper waits until the
   shell has had no DOM mutations for 350 ms (capped at 4 s). This stabilizes
   every state, and the earlier snapshots of a few states (for example the
   "selected theme across every workspace" end state, 292 versus 346 elements)
   turned out to be incomplete pages.
2. **Remove the exemption.** `scripts/diff_style_snapshots.mjs` compares every
   state in full. Its behaviour (identical, changed property, expected-property
   filter, structural count mismatch, coverage listing) is covered by
   `src/diffStyleSnapshots.test.ts`.
3. **Opt-in parallelism.** `STYLE_SNAPSHOT_WORKERS=N` runs the independent
   fixture states concurrently; the default remains the repository's single
   worker. `STYLE_SNAPSHOT_NO_STATES=1` skips interactive variants.

## Evidence

- Two parallel resting-state runs: 0 differences across 245 states.
- A serial run and a parallel run: 0 differences across 245 states, so
  parallelism does not change results.
- A parallel resting-state run takes about 3.5 minutes (serial about 5.5; a full
  run with interactive variants about 8 serial).
- 406 unit tests, 4 web contracts, and 49 real-browser journeys with axe pass.

## Consequences and limits

- Snapshots taken before this change are not comparable for the formerly racy
  states; take a fresh baseline before diffing across this boundary.
- Settling assumes the page eventually stops mutating; a state with a continuous
  animation or live ticker would hit the 4 s cap and be captured as-is.
