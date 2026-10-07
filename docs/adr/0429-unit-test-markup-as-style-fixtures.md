# ADR-0429 — Unit-test markup as style fixtures, and near-complete token coverage

- Status: Accepted. Supersedes the coverage limit in ADR-0428.
- Date: 2026-10-07
- Decision owners: Principal UI Architecture, Product Design

## Context

ADR-0428 could snapshot only what the 49 real-browser journeys render, which
left 54 styled classes (governance timeline, replay gap cards, document result
lines, applicability states, bracket rows, split-plan warning, compact rail, ...)
unverifiable and on hard-coded `rem` sizes. Re-creating each of those API
payloads in Playwright would duplicate strictly validated fixtures that the
unit tests already hold.

## Decision

1. **Capture real markup from unit tests.** `src/styleFixtureCapture.setup.ts`
   (loaded only when `STYLE_FIXTURE_DIR` is set, with `--sequence.hooks=list`)
   saves the DOM a test renders the first time each class in
   `e2e/fixture-targets.json` (the classes no journey renders) appears and again
   at test end. `scripts/web_style_fixtures.sh` produces 127 states. The target
   list is deliberately separate from the permitted-literal list so that
   improving coverage never shrinks the capture.
2. **Snapshot it in the real browser.** `e2e/style-fixtures.spec.ts` (skipped
   unless `STYLE_FIXTURE_INPUT_DIR` and `STYLE_SNAPSHOT_DIR` are set) loads each
   state into the running app, with its real stylesheet inside a shell wrapper
   that includes the sidebar column so layout is representative, then reuses the
   ADR-0428 snapshot capture. `scripts/web_style_snapshot.sh` includes them when
   `test-results/style-fixtures` exists. 245 states are captured (118 journey
   states plus 127 fixtures).
3. **Close the gaps with real tests,** not test-only scaffolding. New unit tests
   render the replay gap queue (queued, dead-lettered, acknowledged), the
   governance decision trail, progressive rate bands, a successful document
   simulation, the applicability card in every lifecycle state, the split-plan
   mismatch and match states, the compiled split-plan review (wallet stubbed at
   the `WalletPicker` boundary), and the persisted compact rail. Each is also
   ordinary behaviour coverage that did not exist.
4. **Migrate everything.** 47 more rules (33 font sizes, 19 corner radii) moved
   onto the type-scale and shape tokens. The permitted-literal list
   (`e2e/unverified-surfaces.json`) is now empty, so the stylesheet contains no
   hard-coded font size or corner radius (only `50%` circles remain) and a test
   fails if one is reintroduced.
5. The diff tool treats two time-dependent states as volatile (geometry and
   structure ignored; colour, type, and shape still compared), keeping identical
   CSS at 0 differences across two runs.

## Evidence

- Coverage: 230 of 230 styled classes render in at least one of 245 captured
  states.
- Token migration: element-level differences are confined to font size, corner
  radius, and derived geometry (line height, size, grid columns, em-based letter
  spacing, resolved `auto` margins); no colour, background, display, position,
  padding, or opacity change. The final step (split review) changed exactly 12
  elements' corner radii.
- 397 unit tests, 4 web contracts, and 49 real-browser journeys with axe pass.

## Remaining limits

- Fixtures are a final or first-appearance DOM, not interactive states; hover,
  focus, and open-menu styling is covered by tests and manual checks.
- Fixture layout approximates the shell (a sidebar placeholder keeps the grid
  columns); absolute positions are representative, not pixel-identical.
- Two journeys with time-derived content are compared without geometry.
- Spacing, elevation, and colour-mix state mixes are not tokenized beyond what the
  palette and state-layer tokens already cover.
