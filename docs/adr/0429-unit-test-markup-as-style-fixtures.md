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
   `e2e/unverified-surfaces.json` appears and again at test end.
   `scripts/web_style_fixtures.sh` produces 125 states.
2. **Snapshot it in the real browser.** `e2e/style-fixtures.spec.ts` (skipped
   unless `STYLE_FIXTURE_INPUT_DIR` and `STYLE_SNAPSHOT_DIR` are set) loads each
   state into the running app, with its real stylesheet inside a shell wrapper
   that includes the sidebar column so layout is representative, then reuses the
   ADR-0428 snapshot capture. `scripts/web_style_snapshot.sh` includes them when
   `test-results/style-fixtures` exists. 243 states are captured (118 journey
   states plus 125 fixtures).
3. **Close the gaps with real tests,** not test-only scaffolding. New unit tests
   render the replay gap queue (queued, dead-lettered, acknowledged), the
   governance decision trail, progressive rate bands, a successful document
   simulation, the applicability card in every lifecycle state, the split-plan
   mismatch and match states, and the persisted compact rail. Each is also
   ordinary behaviour coverage that did not exist.
4. **Migrate everything now verifiable.** 45 more rules (32 font sizes, 17
   corner radii) moved onto the type-scale and shape tokens. The permitted-literal
   list (`e2e/unverified-surfaces.json`) shrinks from 54 classes to 2.
5. The diff tool treats two time-dependent states as volatile (geometry and
   structure ignored; colour, type, and shape still compared), keeping identical
   CSS at 0 differences across two runs.

## Evidence

- Coverage: 228 of 230 styled classes render in at least one captured state.
- Token migration: 1,798 element-level differences over 243 states, all on font
  size, corner radius, and derived geometry; no colour, background, display,
  position, padding, or opacity change.
- 396 unit tests, 4 web contracts, 49 real-browser journeys with axe pass.

## Residual limits

- `split-review` and `split-review-row` still render only after a wallet-connected
  payment compile and keep their literals; they are the explicit remainder.
- Fixtures are a final or first-appearance DOM, not interactive states; hover,
  focus, and open-menu styling is covered by tests and manual checks.
- Fixture layout approximates the shell; absolute positions are representative,
  not pixel-identical to production.
