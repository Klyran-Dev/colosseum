# Changelog

- Added ADR-0440: pinned overrides (`source-map-js` 1.2.2, `uuid` 11.1.1) cut `pnpm audit`
  findings for apps/web from 6 to 4, and a fail-closed audit gate enforces the AGENTS.md
  rule (fail on high/critical unless a time-bounded, approved exception). The remaining
  high finding (`bigint-buffer`, dev-only, no patch) is recorded as a proposed exception
  awaiting its owner's approval.

- Added ADR-0439: the product now runs in real Firefox and WebKit (`scripts/web_cross_browser_check.sh`,
  16 tests): CSS feature support, token resolution, spring transitions and reduced motion,
  and WCAG 2.2 AA plus 320px reflow in both themes on four workspaces.

- Added ADR-0438: real hover and press are now verified against the forced states for
  every distinct interactive kind (15 on the workspaces plus 18 in captured states,
  0 mismatches) instead of six samples. The sweep found that a press also focuses
  controls and that text fields and selects match `:focus-visible`; forced states are
  now element-aware (snapshot format 5).

- Added ADR-0437: hover styling now exists only under `@media (hover: hover)`, so a
  tap on a phone no longer leaves a control looking hovered. New touch-emulation tests
  cover taps, navigation, no stuck hover (proved with a negative control), and WCAG 2.2
  AA on the mobile layout in both themes. Desktop is unchanged (0 snapshot differences).

- Added ADR-0436: the non-custodial wallet contract no longer passes silently when
  ripgrep is missing (it now scans with `grep`, fails on scanner errors, and has a
  regression test run by `pnpm test`); the stylesheet has a size and rule-count
  budget; and the supported-browser baseline (Chrome/Edge 113+, Firefox 113+,
  Safari 17.2+) is pinned in the build and tested.

- Added ADR-0435: the real-browser gate now runs axe with WCAG 2.2 AA rules in both
  the dark and light themes (previously WCAG 2.0, dark only) and checks reflow at
  320px on all 16 workspaces. It found and we fixed topbar popovers that ran off the
  screen on phones and a rule-workbench grid that overflowed a 320px viewport.

- Added ADR-0434: the full M3 elevation scale (levels 0-5) and a 4px-grid spacing
  scale now replace the last hard-coded shadows and 344 ad-hoc padding, margin and
  gap declarations (37 distinct values). Tests reject raw shadows and spacing;
  snapshot runs can run journeys in parallel (about 3 minutes for all variants).

- Added ADR-0433: a real-browser test now proves forced hover/press/focus states
  equal real mouse and keyboard interaction (and that springs drive transitions;
  reduced motion removes them). It found that pressed and keyboard-focus snapshots
  were missing the implied hover and `:focus` states, now fixed. Snapshots are
  versioned (format 3) and unsettled captures are reported.

- Added ADR-0432: style snapshots wait for the page to settle, which removed the
  two geometry-exempt states (they were capture races); the diff tool now
  compares every state and has its own tests. `STYLE_SNAPSHOT_WORKERS` runs the
  fixture states in parallel (serial and parallel runs are identical).

- Added ADR-0431: style snapshots now also record forced hover, keyboard-focus
  and pressed states and every disclosure opened (1,182 files per run, 0
  differences between identical runs). The workbench base-share fraction is
  grouped so it can no longer be split across lines.

- Added ADR-0430: the base button transition uses motion tokens (no hover lift),
  21 invisible 1-3% tints became `transparent`, and the rule-workbench source
  row wraps so its fields are no longer clipped. Tests reject each regression.

- Added ADR-0429: markup captured from unit tests is loaded into the real
  browser as style fixtures (125 states; 243 snapshots in total). Added unit
  tests for replay gaps, the governance decision trail, rate bands, document
  simulation results, applicability states, split-plan mismatch, and the
  compact rail, and the compiled split-plan review. Every styled class now
  renders in at least one of 245 snapshot states, and the stylesheet has no
  hard-coded font sizes or corner radii.

- Added ADR-0428: opt-in computed-style snapshots inside the real-browser
  journeys (118 states, both themes) with a diff tool, so CSS refactors of
  data-dependent surfaces can be proven. Used it to remove 8 dead selectors,
  guard against future ones, and move 73 more rules onto the M3 type-scale and
  shape tokens. 54 classes no journey renders yet remain an explicit,
  test-enforced list.

- Added ADR-0427: access-control permission rows, risk/decision chips, policy
  captions, technical details, empty states and release evidence use the type
  scale and tonal cards, so decision-bearing text is no longer 9-11px. The
  authorization posture strip wraps instead of truncating its values, and the
  access-control layout now follows workspace width. Surfaces that only render
  with live data are explicitly not migrated yet.

- Added ADR-0426: form fields are M3 outlined text fields with hover, invalid
  and disabled states; labels, supporting text, notices and legends use the
  type scale, so form text no longer falls below 11px. A test rejects `rem`
  font sizes and radii on form surfaces.

- Added ADR-0425: the active navigation item is a vivid primary-container
  pill; the topbar status controls are M3 chips with tonal states; menus are
  elevated borderless surfaces; status pills use sentence-case labels. Fixed
  seven undefined `--app-*` aliases that left panel, card, sidebar and menu
  backgrounds transparent, and added a test that rejects unresolved `var()`
  references.

- Added ADR-0424: removed 341 provably dead CSS declarations (same-selector
  and `.product-shell`-prefixed overrides). A before/after computed-style
  snapshot of all 16 routes in both themes shows no visual change. The colour
  contract now checks the supporting-role aliases instead of a dead rule.

- Added ADR-0423: navigation and panels now use one canonical M3 skin (pill
  items, secondary-container active indicator, plain icons, tonal cards)
  instead of about eight repeated override layers. The forced-colors block is
  the final stylesheet layer again, and the compact sidebar toggle no longer
  receives content-button padding.

- Added ADR-0422: the theme now uses the Material baseline palette (seed
  `#6750A4`, TonalSpot) generated by a pinned `material-color-utilities` and
  verified by test, replacing the Aurora mint/teal palette. Added the full M3
  Expressive shape scale, six spring motion tokens, type scale with emphasized
  variants, and state layers, applied to content-area controls.

- Added ADR-0421 workspace container queries: multi-pane layouts and form grids
  now size from the available workspace width instead of the viewport, fixing
  truncated field labels/placeholders at tablet and collapsed-rail widths.

- Added ADR-0403 to remove repeated account identity from the persistent
  navigation rail. Access state remains visible; verified tenant and subject
  details stay in the account menu and page context.

- Added ADR-0402 to remove duplicate tax component traces from the aggregate
  result workspace. The single progressive “Why this tax?” trace preserves
  explainability while reducing scan cost; a failing-first UI regression now
  proves one rendered trace per validated component.

- Corrected public review references to the actual `apps/web/src/authorization.ts`
  and `apps/web/src/wallet.ts` modules, and added a contract preventing stale
  authorization or wallet-module links.

- Improved migration checksum-drift diagnostics with explicit forward-only
  recovery guidance. Applied migrations remain immutable; operators are told to
  restore the matching artifact or add a new migration. The diagnostic has a
  test-first regression contract.

- Added ADR-0384 authoritative atomic scale metadata to tax facts/results,
  canonical evidence, browser rendering, and quote-to-route settlement
  validation. Missing or mismatched precision now fails closed.

- Added ADR-0383 explicit atomic display-scale enforcement so browser money
  formatting cannot silently assume precision; route-bound checkout remains
  tied to finalized server-owned mint decimals.

- Added ADR-0382's explicit Tax Engine golden-vector coverage manifest and
  weighted-component cascade vector. Rust tests now reject missing, duplicate,
  or unexpected v1/v2 scenario IDs while retaining exact decimal-string
  arithmetic and component assertions.
- Added repeated-distinct-digest single-flight cleanup coverage under
  ADR-0381; 128 sequential misses prove coordination entries do not leak.

## 2026-10-05

- Added ADR-0381 bounded single-flight Tax Engine compilation. Same-digest
  misses now share one immutable graph compilation per service instance while
  unrelated rules retain independent cache work. The service suite and Clippy
  pass; the post-change local Docker run returned 256/256 HTTP 200 responses at
  concurrency 32 with p50/p95/p99 of 1.466/3.256/5.705 ms. This remains local
  synthetic evidence.

- Re-ran `make check` after the Tax Engine service cache-integrity and
  single-flight compilation changes: workspace Clippy/tests, 74 frontend test
  files, production build, security scan, Docker/observability/configuration
  contracts, Tax Engine runtime/mTLS, EVM adapter, and public-repository checks
  all passed. The retained arm64-only Alertmanager image warning and external
  Linux/validator, staging, custody, legal, and production-SLO gates remain
  explicit.

- Added service-level LRU cache integrity coverage for the warm Tax Engine.
  The regression test proves one queue entry per compiled-rule digest, a
  single recency move on cache hits, and eviction of the least-recently-used
  digest without changing calculation authority.

- Re-ran the bounded warm Tax Engine service workload against the rebuilt
  pinned image: 256/256 authenticated HTTP 200 responses at concurrency 32,
  with p50/p95/p99 of 1.373/4.987/11.360 ms on the native arm64 Docker runtime;
  a matched repeat returned 2.020/8.538/17.459 ms. Both image digests and the
  retained tail-variance investigation are recorded in ADR-0232; Linux/amd64,
  regional, and production SLO evidence remain separate gates.

- Preserved the Tier-1 tax engine's integer fast path for legacy additive tax
  bases while routing weighted and bounded v2 bases through exact rational
  evaluation. The focused classifier test and 66-test tax-engine suite pass;
  the matched native arm64 Criterion rerun improved all four retained local
  hot-path baselines. Linux/service/production SLO evidence remains separate.

- Added ADR-0380 to make policy-issuer authority, stablecoin admission, and
  external issuer/off-ramp dependencies explicit. The typed authority matrix,
  bounded evidence envelope, dependency lifecycle, fail-closed unknown state,
  and replacement requirements are now documented without treating protocol
  admission as redemption, legal, or tax-compliance approval.

- Added ADR-0379 and bounded taxable-base floor/cap operations to the Tier-1
  tax DSL. Exact linear bases may now be clamped before thresholds and rates,
  with v1 canonical compatibility preserved, v2 golden-vector coverage,
  property tests, an explicit capped-base vector, and localized rule-workbench
  controls; jurisdiction publication and tax-owner review remain open.

- Added ADR-0375 and the bounded linear tax-base v2 envelope. The Tier-1 tax
  engine now supports explicit exact weighted and subtractive base terms with
  dependency ordering, integral-atomic admission, v2 golden vectors, replay
  coverage, and structured localized authoring controls while preserving v1
  additive-rule bytes.

- Added ADR-0373 and a tenant-scoped evidence-access explorer. The API now
  exposes bounded, read-only audit metadata for authorized operators, and the
  React shell provides localized loading, empty, failure, and progressive
  technical-disclosure states without exposing raw facts or credentials.

- Added ADR-0374 and an expanded-by-default, session-only compact Material 3
  navigation rail. Compact mode reduces desktop shell density while retaining
  localized accessible names and the same server-owned authorization decisions.

- Added ADR-0370 and extended the CI toolchain contract to record resolved
  Node.js 24.21.0 and pnpm 12.3.4 versions before frontend dependency install.

- Added ADR-0369 and an executable CI toolchain contract. Rust CI and
  release-candidate workflows now install and export Rust 1.98.1 explicitly,
  record compiler versions, and fail `make check` if the pinned bootstrap
  drifts.

- Revalidated the pinned production toolchain against upstream release
  channels. Rust 1.98.1, React 19.3.0, PostgreSQL 18.6, Solidity 0.8.37, and
  Anchor 1.2.0 remain compatible; Node 24 remains the LTS build boundary and
  Agave 4.3.0 remains the stable validator boundary. No version change was
  authorized without the ADR-0163 evidence-gated upgrade process.

- Added ADR-0368 to keep non-urgent deployment, network, and service warnings
  inside the expandable system-status disclosure as ordinary contextual text.
  Actionable financial failures remain assertive reviewed alerts, reducing
  competing screen-reader announcements without changing operational authority.

- Revalidated the authenticated Docker integration gate after the financial
  workspace error-boundary hardening. PostgreSQL 18, two-replica idempotency
  fencing, ambiguous submission recovery, process-crash reconciliation,
  immutable rule rollback, route/asset verification, NATS outbox publication,
  and adjustment evidence `[200,409]` submission all passed; external Linux
  validator, live-provider, staging, custody, legal, and production-SLO gates
  remain open.

- Added ADR-0367 and extended the typed financial-workspace error boundary to
  aggregate/document checkout, refund claim/cancellation/execution, tax-document
  simulation, rule governance, document quote, treasury/vault,
  reconciliation, settlement adjustment, replay operations, revenue streaming,
  and the application shell. API payload messages no longer become browser
  presentation authority; reviewed fallback copy and validated UUIDv7 support
  references are preserved while ordinary provider/parser errors are redacted.
  The shell-level provider-diagnostic regression is now covered by the 294-pass
  web suite (10 intentional skips).

- Added ADR-0366 and redacted arbitrary injected-wallet provider exceptions from
  the primary wallet surface. Rejections now use localized safe copy while
  typed multi-account selection remains available; no wallet authorization or
  automatic retry behavior changed.

- Added ADR-0365 and moved capability-snapshot recovery from raw parser errors
  to localized plain-language copy. Expired, missing-expiry, and generic access
  failures remain fail-closed while the shell explains the next safe step;
  technical diagnostics are not rendered in the operator surface.

- Extended the pinned real-browser evidence to 47 journeys with an expired
  server capability-snapshot scenario. The browser now proves that invalid
  authorization freshness leaves protected navigation in a needs-check state,
  renders the bounded expiry explanation, and passes axe; live identity and
  manual assistive-technology evidence remain separate release gates.

- Extended the pinned real-browser evidence to 46 journeys with an explicit
  reconciliation-timeout scenario. The browser now proves that a bounded
  client deadline leaves balance evidence unloaded, exposes an accessible
  failure, and does not fabricate a target or observation; live-provider and
  manual assistive-technology evidence remain separate release gates.

- Added ADR-0364 and collapsed the secondary saved-rule simulation by default
  in the Tax Rules workspace. Governed draft authoring remains the primary
  visible task, while the server-owned what-if simulation remains discoverable
  through an accessible native disclosure without changing its safeguards.

- Added ADR-0360 and typed localization for the governance document-simulation
  workbench. English, German, Arabic RTL, and Japanese labels, actions,
  result states, and fail-closed recovery copy are now consistent while rule
  values, jurisdiction identifiers, digests, statuses, and server evidence
  remain unchanged.

- Added ADR-0359 and an explicit fee-payer binding check across every signed
  wallet-submission profile. The coordinator now verifies the first static
  message account is the first compiled account, a signer, and writable before
  profile-specific comparison; a sponsored fee payer is rejected without an
  RPC call.

- Added ADR-0358 and localized lazy-workspace loading and error recovery states
  through the shell-operational catalog. Recovery remains fail-closed and never
  exposes exception details or infers financial, wallet, settlement, or
  evidence state.

- Added ADR-0357 and typed shell-operational localization for system status,
  account context, protection details, and footer copy in English, German,
  Arabic RTL, and Japanese. Runtime environment values, account identity, and
  authorization evidence remain unchanged.

- Added ADR-0356 and typed access-control workspace localization. Authorization
  explanations, policy labels, decision states, groups, and risk labels now
  support English, German, Arabic RTL, and Japanese while capability IDs,
  reason codes, scope names, and server-owned authorization values remain
  unchanged.

- Added ADR-0355 and typed localization for the governed partner-payout
  workspace. Balances, lifecycle actions, setup references, release preparation,
  and finalized-evidence boundaries now support English, German, Arabic RTL,
  and Japanese without translating protocol values or authority inputs.

- Added ADR-0354 and typed localization for the high-consequence tax-rule
  governance and structured authoring flows. Labels, lifecycle guidance,
  provenance evidence, simulation states, and validation messages now support
  English, German, Arabic RTL, and Japanese without translating canonical rule
  values, status discriminators, evidence, digests, or API payloads.

- Added ADR-0352 and typed vault-reconciliation localization at the
  release-stop boundary for read-only balance checks, verified source
  selection, evidence detail, stale/mismatch/error states, and append-only
  history. Atomic balances, slots, identities, statuses, error codes, and
  release authority remain unchanged.

- Added ADR-0351 and typed replay-operations localization at the recovery
  boundary for replay-gap review, bounded requeue, targeted finalized-evidence
  recovery, route-verification evidence, and explicit non-authority language.
  Stream keys, statuses, slots, signatures, addresses, notes, action values,
  and server-owned evidence remain unchanged.

- Added ADR-0350 and typed settlement-adjustment localization at the correction
  boundary for proposal, approval, recovery, wallet-bound record evidence,
  provider uncertainty, and finalized evidence. Financial values, adjustment
  kinds, identifiers, signed bytes, and server-owned statuses remain unchanged.

- Added ADR-0346 and a typed Treasury and vault-release localization boundary
  for governed proposal, verified-route, lifecycle, transition, evidence, and
  authority copy. German, Arabic RTL, and Japanese presentation is bounded to
  operator language; routes, accounts, amounts, digests, transition values,
  and server-owned claim statuses remain unchanged.

- Added ADR-0347 and typed refund-claim localization at the wallet boundary
  for refund
  preparation, immutable artifact review, two-wallet authorization, recovery,
  finalized evidence, and reconciliation. German, Arabic RTL, and Japanese
  presentation remains separate from claim IDs, amounts, destinations,
  signatures, transaction bytes, and server-owned statuses.

- Added ADR-0348 and typed refund-execution localization at the final release
  boundary for final approval, two-wallet authorization, recovery,
  proof-pending, and finalized movement evidence. Release IDs, amounts,
  destinations, signatures, transaction bytes, and server-owned statuses
  remain unchanged.

- Added ADR-0349 and typed refund-cancellation localization at the governance
  boundary for controlled cancellation, approval-wallet signing, recovery,
  provider uncertainty, and finalized cancellation evidence. Claim references,
  authority accounts, signatures, transaction bytes, and server-owned statuses
  remain unchanged.

- Added ADR-0344 and a typed document-checkout localization boundary for
  invoice evidence, route review, payment scope, signing, settlement status,
  recovery, and withholding/legal language. The reusable wallet picker now
  uses its own bounded catalog; wallets, routes, mints, amounts, signatures,
  and server-owned evidence remain unchanged. German and Arabic RTL browser
  journeys include axe coverage.

- Added ADR-0345 and a typed aggregate-checkout localization boundary for
  route review, recipient plans, safeguards, amount review, wallet approval,
  and settlement recovery. Financial/protocol values remain unchanged, with
  German and Arabic RTL browser journeys covered by axe.

- Added ADR-0343 and a typed document-quote catalog for invoice tax entry,
  line items, location evidence, result review, and checkout handoff. German,
  Arabic, and Japanese presentation is bounded to operator copy; document IDs,
  line IDs, financial values, rules, digests, and server evidence remain intact.

- Added ADR-0342 and a typed quote-workspace catalog for customer quotes and
  scenario exploration. Form, retry, location, result, and evidence language
  now follows the selected locale while amounts, rule identifiers, jurisdiction
  values, fingerprints, and server-owned explanations remain unchanged.

- Added ADR-0341 for shell chrome and session localization. The bounded locale
  catalog now covers navigation disclosure, display settings, theme options,
  timezone warnings, and local/OIDC sign-in labels now follow the selected
  locale without translating credentials, server authority, or financial data.

- Added ADR-0340 and a typed, bounded shell localization catalog for German,
  Arabic, and Japanese alongside the canonical English fallback. Navigation
  groups, workspace labels/descriptions, risk context, and the polite workspace
  status now follow the selected locale; workspace panels remain an explicitly
  tracked follow-up and no authority or financial data is translated.

- Added ADR-0339 and a polite, bounded workspace navigation status for
  screen-reader context. The live region contains only the plain-language
  workspace label and never exposes tenant, authorization, financial, wallet,
  or technical evidence data.

- Added ADR-0338 and a forced-colors accessibility layer that maps semantic
  Material roles to operating-system system colors with no second palette or
  product preference. Static and Playwright forced-colors contracts preserve
  focus, error, disabled, and active-navigation visibility.

- Added ADR-0337 and made the Material 3 navigation disclosure explicitly
  single-open. Switching work areas closes the previous group, preserving a
  calm task-oriented sidebar without hiding routes or changing authorization.

- Added ADR-0336 and a bounded, read-only route-verification evidence view in
  the existing system-operator recovery workspace. The API validates UUIDv7
  identities, lifecycle/counter invariants, and approval binding before the
  React disclosure renders durable run evidence; the browser loads it only on
  explicit action and keeps route activation/finalized settlement authority
  separate.

- Added the public [architecture review guide](docs/review/architecture-review-guide.md),
  mapping tax, authorization, route admission, settlement, wallet, evidence,
  and UI boundaries to their source, invariants, local gates, and remaining
  release evidence. It keeps Docker fixture proof separate from Linux,
  staging, legal, audit, and production approval.

- Revalidated the pinned toolchain baseline against official release channels;
  the matrix remains on stable/LTS-compatible Rust 1.98.1, Agave 4.3.0,
  React 19.3, Node 24.21 LTS, Anchor 1.2.0, Solidity 0.8.37, and NGINX
  1.30.5 rather than adopting a newer Current or pre-release channel.

- Added ADR-0335 and a quieter Material 3 Expressive state hierarchy: the
  selected route now uses a tonal surface with a primary rail, while an
  unconfigured sign-in state remains visible as a compact warning disclosure.
  Semantic state aliases keep component colors bound to the Solana Aurora role
  matrix.

- Added ADR-0334 and a bounded capability-snapshot expiry boundary. Protected
  UI affordances become indeterminate at expiry and the shell refreshes the
  server-owned snapshot once, without allowing stale browser state to authorize
  an operation.
- Re-ran the authenticated Docker integration gate after the authorization
  boundary update; concurrent idempotency, ambiguous submission recovery,
  process-crash recovery, rule rollback, and finalized adjustment evidence
  passed on the pinned PostgreSQL 18 topology.
- Added ADR-0332 to make browser authorization affordances server-decision-only:
  a missing capability decision now remains indeterminate even when copied
  roles, scopes, tenant data, or attributes appear sufficient. Protected API
  operations continue to re-authorize on the server.
- Added ADR-0333 to reject contradictory capability decision/reason pairs at
  the browser boundary, keeping authorization status copy fail closed.

- Added ADR-0331 and a canonical versioned RBAC/PBAC/ABAC authorization
  catalog shared by the API capability snapshot and React access-review
  affordances, preventing policy metadata drift while keeping the server
  authoritative. The API now also rejects unknown groups, workspaces, risks,
  malformed scope/role tokens, duplicate requirements, and unsupported ABAC
  attributes before publishing a snapshot, and validates the catalog before
  binding its listener.

## 2026-10-04
- Added ADR-0330 and a server-owned, route-bound invoice evidence status flow;
  document checkout no longer asks operators to paste signed attestation JSON.
  The API re-verifies stored evidence in the tenant and route context, while
  the UI presents plain-language status with collapsed technical details.
- Tightened the Linux conformance evidence postcondition so successful
  release artifacts must include stablecoin account-topology assertions,
  disabled/substituted-asset rollback, non-stable classification rejection,
  and TransferFeeConfig rejection; a build without those cases cannot satisfy
  the validator gate.
- Synchronized the real-validator TypeScript conformance harness with the
  stablecoin `AssetConfig` account contract. Legacy SPL and extension-free
  Token-2022 setup now provisions the governed PDA, every movement builder
  supplies it in canonical order, disabled assets prove no balance drift, and
  TransferFeeConfig mints fail before activation.
- Added ADR-0322 and an expand/contract migration for settlement-route
  evidence. Existing active routes without finalized v2 asset evidence are
  suspended without rewriting or deleting historical records; only routes
  re-verified through the bounded finalized verifier can become active again.
- Added ADR-0321 and bound finalized settlement-route activation to the
  mint-scoped on-chain `AssetConfig` PDA. The verifier now requests five
  accounts, proves the exact stablecoin layout and enabled state, stores five
  bounded evidence records, and uses a v2 topology digest so pre-admission
  route evidence cannot be replayed. Existing route backfill and Linux
  validator evidence remain required before non-local signing.
- Added ADR-0320 and froze the on-chain stablecoin admission boundary: every
  v1 token movement will require a governed, mint-scoped `AssetConfig` PDA,
  while route configuration remains only an admission aid. The ABI, route
  backfill, validator, audit, and governance work is intentionally still open;
  missing asset evidence cannot enable signing.
- Added the first ADR-0320 implementation slice: the Solana movement contexts,
  settlement-client, coordinator-facing artifact, and browser signing envelope
  now carry and verify the bounded asset-admission account. Focused tests pass;
  this does not enable paid-state movement until route evidence is backfilled
  and retained Linux validator and governance gates are complete.
- Added ADR-0319 and bounded browser-facing API errors at the shared response
  boundary: untrusted messages are normalized, control characters are removed,
  text is capped at 512 characters, whitespace-only messages use the operation
  fallback, and only canonical UUIDv7 support references are rendered.
- Added ADR-0318 and routed document quote-to-payment handoffs through one
  App-owned typed History API boundary; the visible payment-review workspace
  and canonical browser URL now change together without placing evidence,
  credentials, wallet material, or financial facts in browser history. The
  Docker browser journey also proves that a refreshed destination remains
  fail-closed until its authenticated calculation context is re-established.
- Added ADR-0317 and completed the next Material 3 Expressive shell pass:
  the fixed Solana Aurora palette now includes explicit lowest/dim/bright
  surface roles, while the sidebar, top bar, menus, panels, and navigation use
  a deterministic tonal hierarchy with lower visual scan cost. No workspace,
  authorization state, or production workflow was hidden or weakened.
- Added ADR-0312 and applied outcome-first language to document payment review;
  finance and operations users now see invoice evidence, payment route, wallet
  approval, and finalized-settlement boundaries without losing technical
  evidence or fail-closed behavior.
- Added ADR-0313 and consolidated repeated shell security messaging into one
  compact, keyboard-operable protection disclosure while preserving complete
  access, approval, decision-record, and wallet-custody explanations.
- Added ADR-0314 and an executable non-custodial wallet source contract to
  reject seed phrases, mnemonics, private-key fields, and client-side keypair
  construction at the browser/API boundary.
- Added ADR-0315 and extended the executable UI language contract so tax
  calculation, attestation, settlement evidence, filing, and legal
  responsibility remain distinct; positive compliance claims now fail the
  repository check.
- Added ADR-0316 and implemented a versioned rule-release provenance envelope;
  Rust, PostgreSQL, integration, and structured governance-workbench gates now
  fail closed at publication until source, ownership, review, and jurisdiction
  evidence are complete.
- Added ADR-0311 and named the fixed Solana Aurora Material 3 theme family;
  light/dark roles now use documented Solana navy, Aurora Mint, and accessible
  blue tertiary values with exact palette-contract enforcement.
- Added ADR-0310 and removed decorative gradients from production selectors;
  shell, navigation, cards, actions, totals, and review states now use flat
  Material tonal roles with a fail-closed gradient regression contract.
- Expanded the public tax coverage matrix with an explicit engine-semantics
  evidence table for simple, progressive, cascading, inclusive/exclusive,
  threshold, exemption, credit, min/max, truncation, rounding, residual, and
  replay/conservation behavior. Jurisdiction support remains unpublished until
  provenance and legal ownership are approved.
- Added ADR-0309 and consolidated deployment, network, service, and sign-in
  diagnostics into calm, accessible Material 3 disclosures. Full detail and
  safe next steps remain available without obscuring the operator's primary
  task.
- Removed dormant legacy shell-status styles and added a contract preventing
  the retired persistent badge pattern from returning.
- Added ADR-0308 and exact-release authenticated Docker integration evidence
  binding for release candidates.
- Added ADR-0303 exact-commit browser UI evidence to the release-candidate
  workflow, including revision binding, checksum inclusion, and failure
  artifact retention.
- Added ADR-0304's dependency-free Material role contrast contract to the
  standard web and repository checks; both light and dark semantic palettes
  must maintain a 4.5:1 text contrast ratio.
- Added ADR-0305 and `make tier1-local-check` as the canonical ordered local
  evidence gate across repository checks, authenticated Docker integration,
  real-browser UI journeys, and validator-backed Solana conformance.
- Added ADR-0302 and a versioned presentation-only preference store for locale,
  time zone, and Material theme. Preferences restore across reloads, while
  credentials, authorization state, financial facts, and wallet artifacts do
  not enter browser persistence.
- Refined the local development sign-in affordance into an accessible closed
  disclosure, keeping the credential field and its safety warning available
  without competing with the primary workspace task.
- Added ADR-0301 and a real pinned-validator revenue-stream movement case:
  finalized accruals, one-use claim execution, duplicate rejection, and
  cancellation preserve token balances on rejected paths; plan pause also
  rejects new claims without creating claim state. Linux/amd64 execution and
  retained release evidence remain required before enablement.
- Refined the Material navigation shell so access decisions are quiet semantic
  marks instead of repeated “Not available” sentences; accessible descriptions,
  tooltips, and the page-level access card retain the complete authorization
  context.
- Added a public requirements-traceability and evidence matrix that maps
  `AGENTS.md` mandates to source, local Docker, Linux CI, staging, and external
  production evidence without overstating readiness.
- Added a real-browser Material theme regression that switches light/dark
  schemes and verifies the selected semantic surface across every workspace.
- Added ADR-0299 and fail-closed release-bundle source-revision binding so
  validator evidence from another commit cannot be combined with packaged
  binaries and configuration.
- Added ADR-0300 and closed the Material color-token boundary across the SPA:
  component rules now use semantic roles and role-derived alpha values, while
  the web contract rejects raw color literals outside the token declarations.
- Extended the Solana workflow contract to protect release-bundle integrity
  verification ordering before final bundle creation.
- Added ADR-0298 and a final Material 3 Expressive precedence layer for the
  operational shell. Light, dark, and system themes now consume semantic roles
  for action, surface, status, risk, and navigation states; the color, CSP, and
  plain-language contracts also run in the pull-request web CI job.
- Added ADR-0297 for a production account/session menu with safe identity
  disclosure, access-review navigation, and a real sign-out boundary.
- Added ADR-0295 and server-owned UUIDv7 revenue-plan reference selection so
  the browser cannot submit payout topology or authority digests.
- Added ADR-0296 to gate indexer chain readers on bounded validator health and
  updated the local/bootstrap documentation for the fail-closed startup path.
- Added a static API metric-registry contract that prevents endpoint
  observations from drifting from the fixed-cardinality production catalog.
- Added ADR-0285 and the pure `revenue-streaming` domain slice: finalized
  settlement accruals, four-eyes stream lifecycle, bounded single-use release
  claims, replay/conflict rejection, cap/conservation checks, corrections, and
  cancellation/completion semantics. No unfinished API or UI surface is
  exposed.
- Added ADR-0282 for plain-language primary workflows with progressive
  technical disclosure. Invoice quote, rule review, and access workspaces now
  keep operator tasks prominent while retaining exact audit references behind
  accessible record or administrator details.
- Refined the guided rule builder so tax authors work with rate bands, taxable
  amounts, thresholds, credits, and rounding decisions in task language while
  preserving the exact validated rule document.
- Grouped manual provider-signed location records under an explicit “Location
  verification” disclosure and marked the raw exchange as advanced, without
  weakening server-side evidence validation or tenant binding.
- Added browser assertions for the updated labels and refreshed the real
  browser journeys for verified location records and approved tax rules.
- Revalidated the current worktree with `make check`, `make integration-check`,
  and `make web-browser-check`. Local Docker evidence remains green; Linux
  validator, live provider, staging identity, custody, legal, and regional SLO
  release gates remain explicitly open.
- Added ADR-0283 to make the product boundary explicit: the first settlement
  scope is an atomic per-transaction split, while continuous partner streaming
  is a separate future bounded context and is not implied by “real-time” copy.
- Added ADR-0284 and optimized the Tier-1 tax hot path to borrow validated facts
  when no FX conversion is present. The full tax-engine suite passes, and the
  current local benchmark shows improved compiled and document calculation
  timings; production SLO claims remain gated on Linux/amd64 evidence.

## 2026-10-03

- Added ADR-0278 for addressable static-SPA workspace navigation, including
  safe deep links and browser back/forward behavior without putting credentials
  or financial facts in URLs.
- Added ADR-0277 and implemented the complete server-owned capability catalog
  for every production workspace, including document payments, reconciliation,
  adjustments, and refund lifecycle actions. The updated local benchmark
  validated all fourteen decisions at p50/p95/p99 of 1.052/1.915/3.114 ms.
- Added ADR-0276 and a real-browser two-tenant capability-session journey. It
  proves that replacing an opaque session removes the previous account
  reference, subject, and capability decisions before the next tenant is
  rendered; identity-provider and live multi-tenant API evidence remain open.
- Added ADR-0275 and `make benchmark-authorization-capabilities`, a bounded
  two-replica authenticated HTTP workload for the server-owned capability
  snapshot. The default local run validated 256/256 semantic responses at
  concurrency 32 with p50/p95/p99 of 0.992/1.624/3.094 ms; regional staging
  and production SLO evidence remain separate gates.
- Added ADR-0270 and a bounded browser OIDC ID-token validation boundary. The
  SPA now verifies signed ID tokens against same-origin provider JWKS, issuer,
  client audience, nonce, subject, and time claims before establishing a
  session; access-token authorization remains API-owned.
- Extended ADR-0263's product-language contract to adjustment review and
  refund cancellation. The operator sees the decision and next step first;
  exact request references, authorization evidence, and recovery data remain
  available for controlled review.
- Extended the same operator-first language to invoice tax, rule review,
  payment review, balance checks, and exception recovery. Technical values
  remain available for controlled review without defining the primary journey.
- Added ADR-0271 and an executable UI language contract to the repository
  quality gate. Primary labels are checked for task language while exact
  protocol evidence remains available through progressive disclosure.
- Added ADR-0272 and refined payment review into a production operator flow:
  amount, tax, route, recipient distribution, expiry, and wallet approval are
  primary; exact account, policy, compute, and intent evidence is available in
  explicit accessible disclosures.
- Added ADR-0273 and removed example financial values from quote, invoice,
  document-payment, and persisted-rule simulation workspaces. Operators now
  enter amounts, line facts, rules, jurisdictions, and categories explicitly;
  only the generated retry reference is prefilled.
- Added ADR-0274 and removed the hidden USD assumption from aggregate quotes
  and scenarios. Currency is now explicit and normalized before the server
  validates supported tax and settlement semantics.
- Applied ADR-0263's plain-language money-movement contract to refund requests,
  refund releases, and treasury release requests. Exact amounts, addresses, and
  evidence remain reviewable, while primary labels and actions describe the
  operator's decision instead of protocol internals.
- Added ADR-0267 and removed fixed account, operator, environment, and network
  labels from the product shell. The shell now uses the verified account
  snapshot and explicit immutable build context, and visibly reports missing
  deployment configuration instead of presenting a plausible false context.
- Added ADR-0268 to replace production bearer-credential entry with an OIDC
  Authorization Code + PKCE session boundary. Implementation and identity-
  provider evidence remain tracked release work.
- Added ADR-0269 to bind the configured identity-provider origin into the
  immutable NGINX CSP and reject cross-origin OIDC discovery endpoints.
- Added ADR-0265 and a responsive, labeled navigation disclosure so narrow-screen
  users can discover the complete task catalog without relying on icon memory.
- Added ADR-0266 and preserved the bounded Tax quotes → Payment review handoff
  so expiry and signing safeguards remain visible instead of losing context.

## 2026-10-02

- Added ADR-0261's vendor-neutral FX issuer process bootstrap. Local
  development can compose bounded HTTP provider and signer adapters once at
  startup through strict server-owned configuration; partial, malformed, and
  non-local activation remains fail-closed.
- Added ADR-0260's authenticated server-owned FX quote composition boundary:
  source currency is an explicit request fact, while provider policy, evidence,
  and signing remain server-owned and non-local activation remains fail-closed.
- Added stable FX quote request identity: pre-resolution facts and normalized
  source currency are idempotent inputs, while provider quote metadata remains
  immutable evidence rather than a caller request digest.
- Added the public Tier-1 decision register required by ADR-0251. Provider,
  jurisdiction, legal ownership, staging, SLO, retention, rollback, custody,
  and license choices remain explicitly open until accountable owners attach
  evidence; no external decision is inferred from a missing value.
- Completed registry-bound asynchronous FX issuer construction. The active
  policy snapshot now gates both synchronous and HSM/KMS-compatible async
  issuer creation, with missing-digest and snapshot-isolation coverage; the
  concrete custody adapter remains an external release gate.
- Added ADR-0244 for the exact foreign-exchange evidence boundary required by
  global taxation. The pure engine now normalizes exact evidence once and
  preserves it for replay; API entry points reject caller-supplied FX until a
  separately governed provider adapter can attach verified evidence. Provider
  freshness and legal rate acceptance remain outside the tax hot path.
- Added ADR-0245 for a server-owned signed FX evidence bounded context. The
  provider authenticity and key-rotation boundary is designed before wiring a
  production provider adapter; freshness, legal acceptance, and custody remain
  explicit release gates.
- Added ADR-0246 for a vendor-neutral FX provider policy and issuance boundary.
  Provider acquisition, exact policy evaluation, and HSM/KMS signing remain
  replaceable ports outside the deterministic tax-engine hot path.
- Added ADR-0247 for an atomic digest-keyed FX provider policy snapshot. Invalid
  policy replacements fail closed and retain the last known-good snapshot.
- Added ADR-0248 for registry-bound FX evidence issuance. Production adapters
  must resolve the exact active policy digest before constructing an issuer;
  missing or zero digests fail closed before provider or signer work.
- Added ADR-0249 for policy-scoped FX provider identity authorization. Quotes
  from unapproved provider identities now fail before signing; provider
  rotation requires a new immutable policy version.
- Added ADR-0250 for deterministic FX provider agreement. Multi-provider
  policies require exact economic agreement and select by policy priority;
  disagreement and insufficient provider evidence fail closed.
- Added ADR-0251 for an accelerated Tier-1 delivery model. Three named
  workstreams may run in parallel after a Day 2 decision freeze; the 3–4 week
  production-candidate and 5–7 week controlled-production estimates remain
  conditional, and no release gate, audit, legal approval, or evidence
  requirement is waived.
- Added ADR-0252 for bounded concurrent FX provider acquisition. Provider
  fan-out now has an explicit design boundary for source identity, concurrency,
  per-provider deadlines, aggregate cancellation, and partial-failure evidence
  before any real vendor adapter is enabled. The vendor-neutral Rust boundary
  and focused concurrency/cancellation tests are implemented; provider,
  regional, legal, custody, and production-SLO gates remain open.
- Added ADR-0253 for a bounded async FX signing boundary. HSM/KMS-compatible
  adapters can sign without blocking the Tokio executor, subject to an explicit
  deadline and mandatory local envelope verification. The vendor-neutral Rust
  path and focused async signing tests are implemented; custody, rotation,
  recovery, and production HSM/KMS evidence remain open.
- Added ADR-0254 for deterministic FX acquisition outcome order. Concurrent
  provider candidates and bounded failure classifications are normalized by
  policy priority after task completion, so response timing cannot change
  replayable evidence order; the focused test suite now has 18 passing tests.
- Added ADR-0255 for a bounded vendor-neutral HTTP FX provider adapter. The
  transport contract will enforce endpoint, credential, timeout, body-size,
  strict-decoding, exact-decimal, provider-identity, and raw-response evidence
  controls without selecting a concrete provider or weakening release gates.
- Added ADR-0256 for a bounded vendor-neutral HTTP FX signer adapter. The
  signer transport preserves exact payload bytes, server-owned key identity,
  strict signature shape, bounded timeout/body behavior, and no duplicate
  signing retries while leaving HSM/KMS custody decisions open.
- Added ADR-0257 for the server-owned FX issuance coordinator. It composes one
  registry snapshot, bounded provider acquisition, deterministic agreement,
  async signing, and local envelope verification without accepting caller-owned
  FX authority or inventing provider/legal/custody decisions.
- Added ADR-0258 for historical FX-envelope integrity verification and shared
  async signer ownership without requiring private-key cloning.
- Added ADR-0259 for atomic durable FX-envelope binding in quote evidence and
  tenant-scoped historical replay.
- Added ADR-0243 and a program pause-coverage contract. CI and normal checks
  now discover every public `transfer_checked` entry point and fail closed if
  movement is outside the explicit allowlist, if an allowlisted instruction
  loses its protocol pause boundary, or if vault-release loses its PDA-bound
  pause state. Runtime validator and staging incident evidence remain separate
  gates.
- Added ADR-0242 to make the on-chain emergency pause cover governed vault
  release execution. The release instruction now requires the protocol-config
  PDA and rejects before CPI while paused; localnet conformance also checks
  vault/destination balance conservation. Linux validator and staging
  multisig/timelock evidence remain release gates.
- Added ADR-0241 and an executable conformance workflow contract. Normal
  checks now fail closed if CI or release-candidate workflows lose the pinned
  Linux/amd64 assertion, Dockerized validator command, evidence postcondition,
  or required retained-artifact handling. This protects workflow shape; a
  successful Linux validator run remains a separate release gate.
- Added ADR-0240 and migration `0059` to make refund-claim cancellation
  reconciliation leases compatible with the database lifecycle trigger. A
  same-status update may change only lease coordination fields; signature,
  commitment, slot, error, authority, and artifact fields remain fenced. The
  local two-replica Docker drill exposed and verified this regression on the
  arm64 host; Linux/amd64 and production evidence remain release gates.
- Re-ran the bounded warm Tax Engine service workload on the pinned local arm64
  image: 256/256 HTTP 200 responses at concurrency 32 with p50/p95/p99 of
  1.205/2.788/5.791 ms. This remains local synthetic evidence, not a regional
  or production SLO claim.

## 2026-10-01

- Added ADR-0239 and migration `0058` to close UUIDv7 schema drift. The
  executable PostgreSQL contract now discovers every public `uuidv7()` default
  from catalog metadata and requires a native UUID type plus a validated
  version-7 check constraint. Existing rows fail the migration if invalid;
  clean-bootstrap, upgraded-schema, and Linux backup/restore evidence remain
  release gates.
- Pinned the web package manager to pnpm 12.3.4 in the package manifest and
  compatibility matrix so local, Docker, and CI installs share one resolver
  version.
- Added the ADR-0238 automatic cancellation reconciler. The indexer now claims
  submitted cancellation intents with a short PostgreSQL UUIDv7 lease, performs
  bounded finalized recovery without holding database locks across RPC, and
  commits only with a token-fenced atomic claim/cancellation transition. Added
  migration `0057`, shared immutable evidence derivation, stale-lease safety,
  and fixed-cardinality cancellation backlog telemetry. Linux validator,
  provider-load, governance-key, and production operational evidence remain
  release gates.
- Added ADR-0238 and the first governed refund-claim cancellation domain
  slice. Rust now compiles a fixed five-account, amount-free cancellation
  artifact, validates UUIDv7/server-owned identity, and verifies exact wallet-
  signed bytes without address lookup tables or extra instructions. PostgreSQL
  migration `0054` and the storage adapter now persist a separate immutable
  UUIDv7 cancellation intent with lifecycle events and outbox publication.
  The API now exposes tenant-scoped prepare/submit/read routes with finalized
  registry governance binding and exact signed-byte verification. The UI now
  supports wallet review, transport submission, and finalized reconciliation;
  typed cancellation-event evidence atomically converges the original claim to
  `cancelled`. Linux validator evidence and production governance-key,
  provider-load, and operational gates remain explicitly open.
- Added ADR-0237 and a fail-closed Tax Engine service-token rotation contract:
  inline and file-backed credentials are mutually exclusive, file reads are
  bounded, and Unix `SIGHUP` atomically replaces the token while retaining the
  last known-good runtime configuration after invalid input.
- Added fixed-cardinality `runtime_reload` telemetry and a critical Prometheus
  alert for rejected Tax Engine token/TLS replacements, with a real-container
  assertion that credential material never enters metrics.
- Extended the validator conformance contract for governed refund movement to
  require payer-bound claim execution, governance cancellation, and unchanged
  balances after cancelled or substituted-topology attempts.
- Added ADR-0236 and a fail-closed Tax Engine mTLS client-identity boundary:
  non-local deployments require an exact bounded `spiffe://` URI-SAN
  allow-list, enforced during the Rustls handshake before HTTP authentication.

- Added ADR-0216 and migration `0053` to persist finalized verifier-derived
  mint precision on settlement routes. Checkout now renders zero-, six-, and
  nine-decimal assets using explicit `BigInt` scaling and fails closed when the
  route precision is missing or invalid; route precision remains immutable per
  verified version.
- Added ADR-0217 and CI retention for the authenticated Docker integration
  stream. Successful and failed runs now retain bounded public-safe evidence
  with `pipefail`, while synthetic integration output remains distinct from
  Linux validator, live-provider, and production-SLO evidence.
- Added ADR-0218 and a release-bundle integrity contract. Release manifests now
  name the SHA-256 hashes of the final Solana program binary and resolved
  Compose configuration, and both files are included in the common checksum
  set.
- Added ADR-0219 to align staging/production deployment preflight with the API's
  supported structured `API_AUTH_JWKS_JSON` representation while preserving
  fail-closed mutual exclusion and local-key rejection.
- Added ADR-0220 to align staging/production deployment preflight with the API's
  structured `API_JURISDICTION_PROVIDER_KEYS_JSON` representation while
  preserving fail-closed mutual exclusion and local-provider rejection.
- Added ADR-0221 to make the Rust migration runner own its PostgreSQL driver
  and migration futures under structured concurrency, failing closed on driver
  errors or premature connection termination.
- Tightened ADR-0217's CI integration evidence upload so a missing capture
  directory fails the job instead of producing a warning-only artifact gap.
- Added ADR-0222 to make API, worker, indexer, and route-verifier
  shutdown-signal listener failures fail closed instead of being silently
  ignored.
- Added a checked-in service-lifecycle contract and `make check` gate covering
  shutdown-future ownership and error propagation across those services.
- Added ADR-0223 and the shared bounded `observe_or_log` metrics boundary.
  Unexpected fixed-operation registration failures are now structured-log
  visible across API, worker, indexer, and route-verifier paths without making
  non-authoritative telemetry a dependency of financial or reconciliation work.
- Added ADR-0224 and a shared calculation-attestation golden vector consumed by
  Rust and TypeScript. Canonical payload bytes, SHA-256 digest, decimal-string
  atomic values, and explicit little-endian encoding now have executable
  cross-language parity evidence without granting authority to the browser.
- Added ADR-0225 and a dedicated warm Rust Tax Engine service adapter. The
  service shares the pure domain engine, authenticates bounded calculation
  requests, caches immutable rule digests, limits blocking concurrency, and is
  included in pinned Docker/Compose bootstrap, metrics, and lifecycle gates;
  the API settlement hot path remains in-process. Its private metrics plane is
  now scrapeable by Prometheus without exposing calculation credentials, and
  CI builds the pinned service image.
- Added ADR-0226 to extend the staging/production deployment preflight with
  Tax Engine environment parity and local-token rejection without exposing
  secret values or changing local Compose defaults.
- Added ADR-0227 to make the Tax Engine listener direct Rustls mTLS outside
  local development, with explicit client-CA verification, bounded ALPN, and
  fail-closed certificate configuration. The service now explicitly selects
  the AWS-LC Rustls provider when multiple providers are present and has
  deterministic trusted-client and untrusted-client handshake coverage.
- Added ADR-0228 for bounded Unix `SIGHUP` Tax Engine mTLS rotation. Complete
  replacement material is validated off the async executor before an atomic
  Rustls snapshot swap; malformed replacements retain the last known-good
  configuration, with deterministic valid/invalid reload coverage.
- Added the public Tax Engine mTLS rotation runbook covering atomic secret
  delivery, controlled reload, trusted-client verification, fail-closed
  rollback, CA overlap, incident escalation, and evidence redaction.
- Added ADR-0229 and aligned Tax Engine tracing with the existing canonical
  `tax_engine` metrics identity, preventing startup failure and telemetry
  dashboard/alert fragmentation.
- Added ADR-0230 and a Docker runtime contract that exercises the real pinned
  Tax Engine image for health, authentication, exact calculation, body limits,
  cleanup, and fixed-cardinality metrics.
- Added ADR-0231 and explicit Tax Engine request-rejection semantics: malformed
  JSON is `400 Bad Request` while the bounded request body remains `413
  Payload Too Large`, with focused tests and pinned-container verification.
- Added ADR-0232 and `make benchmark-tax-engine-service`, a bounded warm-image
  concurrency contract that validates every response and reports local
  p50/p95/p99 latency with workload metadata.
- Added ADR-0233 and `make tax-engine-mtls-runtime-contract`, proving the real
  pinned image's non-local Rustls mTLS boundary, ALPN, exact calculation,
  trusted/untrusted clients, and atomic reload behavior with synthetic keys.
- Added ADR-0234 and an API JWKS rotation contract. The API now supports an
  optional bounded file-backed JWKS source, lock-free immutable key snapshots,
  Unix `SIGHUP` rotation, and last-known-good retention after invalid
  replacements; a real container contract proves old-key rejection, new-key
  acceptance, and invalid-replacement safety.
- Added ADR-0235 and a jurisdiction-provider key rotation contract. The API now
  supports an optional bounded file-backed provider registry, lock-free
  immutable snapshots, Unix `SIGHUP` rotation, and last-known-good retention
  after invalid replacements; a real container contract proves old-key
  rejection, new-key acceptance, and malformed-replacement safety.
- Tightened the Linux validator-conformance workflow so evidence verification
  runs after both successful and failed attempts and missing retained evidence
  fails the artifact step instead of being warning-only.
- Applied the same always-verify and immediate failure-evidence retention
  contract to release-candidate validator runs.

## 2026-09-30

- Added ADR-0213's typed finalized refund-claim evidence boundary. Finalized
  claim responses now carry server-derived, signature/slot/artifact-bound event
  evidence with canonical u64 strings; strict browser parsing and the wallet
  workspace expose the proof progressively while distinguishing movement
  finality from legal or tax compliance. Live-validator and production-provider
  evidence remain release gates.
- Extended the pinned NGINX/Playwright suite with finalized refund proof
  disclosure, WCAG axe validation, reload, and durable intent recovery. The
  web-e2e image now receives only the checked-in shared vectors needed by its
  deterministic browser contract.
- Added ADR-0214 and a validator-client case for extension-free Token-2022
  two-recipient Split Plan settlement. The implementation asserts finalized
  0/25/45/30 balances while retaining unsupported-extension rejection; Linux
  validator execution remains the release evidence gate.
- Added ADR-0215 bounded Docker JSON resource-profile capture to aggregate and
  document load evidence, with CI/release artifact retention and a successful
  local profile-enabled aggregate run. Regional and production SLO evidence
  remain separate gates.
- Extended the pinned Solana refund conformance path with an explicit
  destination-substitution attempt and finalized balance-drift assertion before
  legitimate execution. Linux validator execution remains the release gate.
- Added ADR-0212's production Rust panic-safety contract. Every Rust crate
  denies `unwrap`, `expect`, and explicit panic paths in non-test builds, while
  test fixtures retain isolated assertions. The fixed RPC-fixture program ID
  now uses checked-in bytes with a compatibility test, and jurisdiction digest
  parsing no longer relies on a debug-only assertion.
- Added ADR-0210's Docker full-topology bootstrap. `make bootstrap` builds
  the pinned SBF artifact through Docker, starts the bounded long-running
  local topology, waits on Compose health checks, excludes one-shot test jobs,
  and fails closed on unavailable runtime capabilities. Its dry-run contract
  is included in `make check`; a successful Linux/amd64 validator run remains
  a release gate.
- Added ADR-0211's explicit Compose host-port override policy. Validator RPC
  and faucet bindings now follow the same deterministic environment override
  contract as the other published services, allowing parallel local
  topologies without random port allocation or silent rebinding.
- Added pinned real-browser document quote and checkout degraded-state
  evidence. Unavailable document quote and settlement-route APIs now have
  browser assertions that preserve empty/evidence states, keep compilation and
  signing disabled, and pass WCAG 2.2 A/AA axe scans.
- Implemented ADR-0201's shared bounded OTLP trace-export bootstrap for API,
  worker, indexer, and route-verifier, including endpoint security policy,
  bounded exporter settings, sensitive-field exclusions, and graceful flush.
  Added `make otel-trace-drill`, which proves local API request-span receipt
  and UUIDv7 correlation through the Docker collector; CI and release
  candidates now retain that regression evidence. Authenticated production
  telemetry, retention, alert routing, and SLO evidence remain release gates.
- Implemented ADR-0202's explicit tax rounding scope. Settlement-scope rules
  now round independent component totals once and allocate residual atomic
  units deterministically; mixed, dependent, or mixed-mode settlement rules
  fail closed. The structured rule workbench exposes the scope and preserves
  component-scope compatibility. Line/invoice facts remain a separate
  versioned contract.

## 2026-09-29

- Added ADR-0189's two-replica automated indexer replay recovery contract and
  `make indexer-replay-drill`, proving PostgreSQL 18 lease-fenced completion,
  checkpoint isolation, and fixed-cardinality replay metrics.
- Added ADR-0190's retry-exhaustion failure contract and
  `make indexer-replay-dead-letter-drill`, proving exactly eight bounded
  provider-failure attempts, lease release, explicit dead-letter state, and
  unchanged forward checkpoint. Production alert delivery and live recovery
  SLO evidence remain open.
- Added ADR-0191's pinned Alertmanager delivery contract and
  `make alert-delivery-drill`, proving a real isolated Prometheus availability
  alert reaches Alertmanager and a bounded Rust receiver. Production receiver
  authentication, escalation, and on-call response remain open.
- Added ADR-0192's real-browser vault-reconciliation evidence contract,
  covering server-shaped mismatch/history rendering and unauthorized
  fail-closed behavior through the built NGINX-served SPA. Live-provider,
  validator, wallet, and production identity evidence remain separate gates.
- Added ADR-0193's server-owned reconciliation freshness contract. The API
  now exposes a database-clock stale bit using a fixed 90-second policy, and
  the UI treats stale observations as an operational stop state.
- Added ADR-0194's wallet-bound adjustment finality drill. The isolated Docker
  path now serializes and signs the exact record-only Solana instruction,
  proves concurrent [200,409] submission with one RPC send, observes an
  explicitly finalized fixture status, projects the typed event, and replays
  the server-owned proof envelope. Live wallet/provider, validator, Linux,
  load/SLO, and production accessibility evidence remain release gates.
- Added ADR-0195's transaction-bound refund proof drill. The real Docker
  indexer now resolves the synthetic finalized claim PDA, parses the exact
  refund event and pre/post token balances, persists one immutable proof,
  finalizes the execute intent/outbox once, and rejects mutation/replay
  duplication. Live RPC-provider, validator, load/SLO, and paid-state evidence
  remain release gates.
- Tightened ADR-0168 conformance diagnostics so Agave's canonical
  `io_uring NOT supported` startup failure is retained as the typed
  `validator_io_uring_unsupported` result reason while preserving non-zero
  fail-closed behavior.
- Added ADR-0196's exact-second client quote-expiry guard. Expired quotes no
  longer expose compile or wallet-sign actions; the API remains the expiry
  authority. The pinned NGINX browser suite now covers the accessible expired
  and wallet-rejection states with synthetic route fixtures.
- Added ADR-0197 strict browser parsing for tax calculation and replay evidence;
  malformed or numerically unsafe financial payloads now fail closed before
  entering React state.
- Added ADR-0198 strict vault-release lifecycle response parsing, canonical
  Rust `u64` wire strings, and authoritative post-transition claim refreshes.
- Added real component interaction coverage for the claim and execute wallet
  workspaces, including sequential role signing, unchanged-byte submission, and
  proof-pending UX. Live wallet/provider/finality evidence remains gated.
- Added ADR-0199's authoritative Linux validator evidence contract. CI and
  release candidates now verify Linux/amd64 execution, pinned Agave startup,
  exact SBF identity, container metadata, and the complete real-RPC conformance
  matrix; Docker Desktop `io_uring` failures remain typed non-zero evidence.
- Implemented ADR-0200's Wallet Standard discovery, capability filtering,
  explicit multi-wallet and multi-account selection, account/signature
  binding, selected-account visibility in refund workspaces, and legacy
  fallback while preserving the non-custodial server-owned signed-byte
  boundary. Live provider/account matrix and production enablement evidence
  remain release gates.
- Extended the pinned NGINX Playwright evidence to exercise Wallet Standard
  provider registration, mandatory multi-account selection, and reload-state
  recovery in a real browser. This is synthetic provider evidence only; live
  wallet/provider matrix evidence remains open.
- Added a pinned document checkout success journey covering authenticated quote
  evidence, issuer-attestation verification, server-owned authorization, and
  the reviewed-artifact signing gate. It deliberately stops before signing or
  settlement and keeps live-chain evidence as a release gate.
## 2026-09-28

- Added ADR-0148's pinned Docker Playwright browser journey gate through the
  real NGINX edge, covering keyboard/focus, lazy workspaces, RTL/mobile
  overflow, reduced motion, and bounded chunk-failure recovery.
- Added ADR-0149's CSPRNG-only client idempotency identity helper so the SPA
  also boots correctly in the local HTTP browser context without weakening
  server-owned UUIDv7 entity semantics.
- Added ADR-0150 and the `settlement-adjustments` Rust bounded context for
  immutable, conserving, UUIDv7-bound refund, reversal, correction, chargeback,
  and remittance-adjustment proposals. The domain slice proves exact signed
  deltas, lifecycle fencing, evidence digests, idempotent posting, and
  cumulative bounds; durable API/storage, on-chain posting, and reconciliation
  remain explicitly gated.
- Added ADR-0151's tenant-scoped PostgreSQL proposal/read/approval/rejection
  workflow with append-only lifecycle evidence, transactional outbox records,
  four-eyes approval, and Docker integration coverage. Token movement and
  `posted` status remain unavailable until the governed on-chain posting slice.
- Added ADR-0152's accessible Settlement adjustments workspace with exact
  decimal-string validation, server-owned lifecycle review, bounded error and
  retry states, component/axe coverage, and a real NGINX Playwright journey.
- Added ADR-0154 design for a bounded, atomic Refund movement contract with
  explicit tax-vault and merchant-funder sources; implementation and paid-state
  evidence remain gated.
- Added ADR-0155's finalized-only `AdjustmentRefundExecuted` indexer decoder,
  raw-evidence binding, replay-safe typed projection, and PostgreSQL migration
  0032. This is evidence only; independent balance reconciliation and paid
  state remain gated.
- Added ADR-0156's pure finalized three-account refund balance proof with
  same-slot enforcement and exact checked-u64 delta validation, plus a
  bounded finalized multi-account RPC reader with expected-authority/mint/
  layout and executed-claim checks plus deadline-aware provider failover.
  This reader is diagnostic only; ADR-0157 now requires transaction-bound
  finalized pre/post token balances for a real proof. Durable proof, execution
  signature/event binding, validator evidence, and paid state remain gated.
- Added ADR-0158 for server-owned refund-proof orchestration: pending indexed
  executions are resolved against finalized claim topology inside a complete
  provider attempt before pure checked proof and immutable persistence. No
  browser-supplied topology or paid lifecycle state is accepted.

All notable changes to this project will be documented here. The project follows a documentation-first, evidence-based release process.

## [Unreleased]

- Added ADR-0153 for a separate, record-only on-chain adjustment evidence PDA;
  token movement and paid-state semantics remain explicitly gated.
- Extended the ADR-0212 contract checker to every `src/bin` and Solana program
  crate root, and added the missing production lint contract to the indexer
  projection-rebuild binary.
- Added a container and workflow pinning contract that rejects floating Docker
  image digests and mutable GitHub Action references in CI.
- Closed the ADR-0212 crate-root lint gap in the warm Tax Engine library and
  added a checked-in contract that verifies every Rust library/binary crate
  applies the non-test `expect`/`panic`/`unwrap` denial policy. CI now runs the
  contract explicitly, and release-candidate evidence runs the two-replica
  cancellation reconciliation drill.
- Added the executable two-replica Docker drill for ADR-0238 refund-claim
  cancellation reconciliation. It exercises synthetic finalized RPC recovery,
  UUIDv7 lease fencing, immutable raw evidence, atomic claim cancellation, and
  idempotent outbox publication. The drill is evidence tooling, not a claim of
  production readiness; it requires a reachable Docker daemon.

### Added

- ADR-0129 adds release-candidate provenance automation, grouped dependency
  maintenance policy, and a checksum/SBOM evidence bundle contract.
- ADR-0130 adds fail-closed API deployment-environment validation and explicit
  local Compose labeling; staging and production provisioning remain separate.
- ADR-0135 publishes the user-journey SLO and error-budget policy, explicit
  RTO/RPO and ownership rules, incident severity/containment runbook, and a
  provider-neutral dependency register with replacement paths. Live owner,
  receiver, regional measurement, and failover evidence remain deployment
  gates.
- ADR-0136 adds a scheduled vault-reconciliation drill that runs the real
  indexer against a bounded finalized-account RPC fixture and proves matched,
  append-only mismatch, and fixed-cardinality alert-input behavior.
- ADR-0137 adds an opt-in two-provider RPC failover mode to that drill. The
  real indexer is pointed at a deterministic failing fixture followed by a
  healthy fixture; the drill proves both providers were attempted and that
  reconciliation remains correct. Live-provider capacity, authentication,
  regional failover SLOs, and staging evidence remain deployment gates.
- ADR-0138 adds the versioned Rust database migration runner with advisory
  locking, immutable SHA-256 checksums, ordered application, and fail-closed
  historical drift detection. Compose now gates application startup on the
  runner and isolates synthetic local seeds. Encrypted backup policy and
  rollback rehearsal remain deployment gates.
- ADR-0139 adds a repository-side staging/production configuration preflight
  that rejects local databases, insecure RPC/NATS/identity defaults, unsafe
  mode flags, and synthetic seed directories without printing secret values.
  Secret validity, provider health, deployment approval, and live environment
  evidence remain release gates.
- ADR-0140 extends the validator conformance design with a real legacy-SPL
  Split Plan fan-out, finalized conservation assertions, duplicate-receipt
  rejection, and wrong-account-order atomic-failure evidence. A successful
  Linux validator/CI run remains required before the readiness gate closes.
- ADR-0141 adds validator conformance for governed policy revocation and proves
  that a revoked attestation cannot move tokens; successful Linux validator/CI
  execution remains required before the readiness gate closes.
- ADR-0142 adds SPA skip navigation, active workspace semantics, and tested
  focus management after lazy workspace transitions; full WCAG 2.2 AA evidence
  remains open.

- ADR-0127 defines the finalized vault-balance reconciliation bounded
  context: explicit opening balances, finalized receipt/release deltas,
  verified token-account reads, append-only run evidence, and fail-closed
  mismatch handling.
- ADR-0128 adds tenant-scoped authenticated reconciliation target/run read APIs
  and a runtime-validated operator workspace; browser target creation and
  opening-balance editing are explicitly prohibited.

- ADR-0125 adds authenticated, bounded targeted finalized-signature recovery:
  the API and operator UI fetch only the exact finalized transaction, persist
  projection evidence without advancing checkpoints, and append immutable
  requested/recovered/failed audit rows. Live provider drills, crash
  reconciliation alerting, and fleet recovery SLO evidence remain open.
- ADR-0126 adds the fixed-cardinality stale targeted-recovery audit gauge,
  Prometheus alert, indexed PostgreSQL aggregate, and disposable integration
  verification for requested/recovered/failed and append-only semantics.
- ADR-0097 adds an executable public-repository documentation and release
  contract check, with an explicit pending-license gate.
- ADR-0098 closes the browser durable-intent recovery boundary: status payloads
  are validated at runtime and uncertain submission outcomes cannot return the
  user to a second-signing path.
- ADR-0099 adds bounded runtime validation for settlement routes and compiled
  Solana instruction artifacts before browser review or wallet handoff.
- ADR-0100 adds quote-bound, authenticated calculation-attestation ingestion;
  the API derives binding and time from server-owned evidence, persists the
  signed document atomically with its attested status and outbox event, and
  never holds issuer private keys. The clean-stack integration gate now adds an
  isolated synthetic external issuer fixture and proves exact replay,
  conflict/tenant rejection, and non-simulation single/Split Plan compilation.
- ADR-0079 defines tenant-owned rule releases, authenticated tenant-bound
  governance and quote resolution, and fail-closed ownership migration
  semantics.
- ADR-0080 adds SHA-256-verified acquisition and caching for the pinned SBF
  platform-tools browser asset without weakening validator conformance gates.
- ADR-0084 adds a versioned bounded Prometheus alert contract for service
  availability, error ratio, quote/compile latency, and cache pressure, with
  pinned-image `promtool` validation in `make check` and CI. Production
  receiver delivery remains an explicit release gate.
- ADR-0085 makes validator conformance reviewable by retaining a bounded,
  public-safe evidence bundle with source/runtime metadata, SBF hash, Compose
  configuration, validator logs, and localnet test output on success or
  failure; CI uploads it with finite retention.
- ADR-0086 adds authenticated system-level replay-gap operations with exact
  scope/role authorization, bounded acknowledgement notes, atomic requeue
  semantics, and an immutable PostgreSQL operator-action ledger. Production
  operator identity governance, alert delivery, and replay-drill evidence
  remain release gates.
- ADR-0087 adds a lazy-loaded, typed Replay operations workspace with
  in-memory token handling, bounded notes, conflict-aware state transitions,
  and explicit acknowledgement-versus-repair UX.
- ADR-0088 adds canonical raw-event decoding and a bounded keyset-paginated
  normalized-receipt projection rebuild primitive; clean-database drill
  evidence now runs as a dedicated Docker CI job. Merchant, vault, and
  reporting projection rebuilds remain separate release gates.
- Adds public-safe policy-compromise, upgrade-rollback, vault-release,
  RPC-outage, indexer-gap, and backup/restore runbooks with explicit
  fail-closed stop conditions.
- ADR-0089 adds a pinned Docker PostgreSQL contention drill proving bounded
  lock/statement cancellation, transaction rollback behavior, migration
  ordering, and required evidence/replay indexes; production-like saturation
  remains a separate gate.
- ADR-0090 extends quote-service load evidence with bounded concurrent
  same-idempotency-key retries and one-calculation/one-outbox convergence
  assertions.
- ADR-0091 defines the bounded immutable Split Plan context. The first
  executable increment adds the Rust integer allocator, canonical plan digest,
  shared Rust/browser vectors, the immutable Split Plan PDA, additive split
  instruction, deterministic compiler artifact, and property tests; persisted
  API activation and live settlement evidence remain explicit gates.
- ADR-0092 adds PostgreSQL 18 tenant-scoped immutable Split Plan storage,
  canonical digest revalidation, and the authenticated quote-bound
  `/v1/settlement/split/compile-from-quote` endpoint. Docker integration now
  proves active-plan resolution without accepting caller-supplied recipients.
- ADR-0093 defines the finalized typed `SplitSettlementExecuted` evidence
  projection so split-plan version and digest binding survive indexing and
  replay.
- ADR-0094 defines the authenticated bounded Split Plan checkout discovery and
  review contract; browser recipients and amounts remain non-authoritative.
- ADR-0095 defines one tested browser API deadline/cancellation boundary with
  explicit health, ordinary API, and signed-submission budgets. Timeout paths
  fail closed and never retry settlement side effects in the browser.
- ADR-0096 adds authenticated, idempotent tenant-scoped rule-draft authoring.
  The Rust API validates and canonicalizes the full RuleSet, commits immutable
  creation evidence and a transactional outbox event, and the governance UI
  loads the resulting server-owned snapshot before lifecycle actions.
- The Docker integration evidence now seeds an immutable expired quote fixture
  and proves quote-bound settlement compilation rejects it with HTTP 409. The
  checkout review also shows the server-owned published rule release, rule
  digest, and attestation state before wallet signing.
- CI now runs the bounded quote-service workload and uses the verified
  Dockerized SBF builder for validator conformance.

- Tier-1 product-readiness standard and launch gates.
- Public repository documentation, security policy, contribution guide, and code of conduct.
- Pinned Rust workspace with deterministic tax-engine arithmetic, API request limits, and UUIDv7 calculation identifiers.
- React/Vite SPA, NGINX edge, PostgreSQL 18 migration baseline, Docker Compose topology, and pinned CI workflow.
- Anchor 1.2.0-compatible legacy-SPL settlement vertical slice with conservation checks, receipt PDA, and settlement event.
- Canonical Ed25519 calculation-attestation verification boundary and lossless decimal-string financial JSON contract.
- Pinned Agave 4.3.0 validator image/profile with explicit Linux `io_uring` runtime prerequisites.
- Hardened the NGINX edge to run as a non-root user with a read-only
  filesystem, bounded temporary storage, dropped capabilities, and a pinned
  Trivy security gate.
- Added a deterministic, non-custodial settlement instruction compiler with
  fixed program-domain validation, derived PDAs, canonical account metas, and
  wallet-handoff-only `prepared_unsubmitted` output.
- Added ADR-0056's bounded non-custodial settlement coordinator with canonical
  signed-transaction validation, immutable retry bytes, explicit preflight,
  bounded provider failover, and commitment evidence mapping.
- Added ADR-0057 and PostgreSQL migration 0010 for UUIDv7 settlement intents,
  idempotency digest fencing, and append-only lifecycle events. Submission is
  not called production-ready until live RPC/localnet finality evidence exists.
- Persisted published-rule quote path with PostgreSQL evidence, UUIDv7 calculation IDs, and merchant-scoped idempotency.
- Transactional outbox lease migration and bounded Rust/NATS JetStream dispatcher with integration coverage.
- Merchant/mint-scoped on-chain policy issuer registry with separate governance and issuer authorization tests.
- Finalized indexer/reconciliation domain slice with bounded checkpoints, conflict-safe receipt merges, and exact balance comparison.
- PostgreSQL indexer adapter with serializable finalized-page persistence, checkpoint fencing, receipt evidence, and append-only reconciliation runs.
- Finalized provider-page port and coordinator with exact range, completeness, finality, and receipt-identity validation before persistence.
- Bounded Rustls Solana JSON-RPC transport for finalized slot/block retrieval with response-size, timeout, endpoint, and JSON-RPC envelope validation.
- Deadline-bounded three-provider failover wrapper that preserves finalized commitment semantics.
- Atomic provider health scores, stable adaptive ordering, and capped cooldowns for degraded RPC endpoints.
- Bounded replay-gap queue with typed causes, adjacent-range coalescing, bounded work splitting, and explicit retry exhaustion.
- PostgreSQL UUIDv7 replay-gap ledger with bounded lease claims, stale-claim recovery, dead-lettering, and operator requeue state.
- Independently deployable finalized `indexer-service` with bounded configuration, health endpoints, provider-to-storage orchestration, and replay-gap recording.
- Lease-fenced bounded replay execution with idempotent historical projection repair and durable suffix advancement.
- Bounded immutable raw Solana transaction evidence for finalized projection rebuilds.
- Digest-keyed bounded compiled tax-rule caching with LRU eviction and cache statistics at the quote adapter boundary.
- Profiled Docker-local Redis, Prometheus, Grafana, and OpenTelemetry Collector topology with pinned digests, provisioned dashboards, and bounded service metrics.
- Pinned Anchor settlement-event decoder with conservation validation, domain-separated evidence digests, and bounded raw event persistence.
- Immutable raw chain evidence migration with serializable duplicate/conflict handling beside normalized receipts.
- Explicit JWT scope/role parsing and `tax:quote` authorization at the persisted quote boundary.
- Mandatory on-chain `ProtocolConfig` PDA with governance-controlled settlement pause/unpause and explicit pause events.
- Evidence-bound threshold modes, exemption codes, cumulative-period snapshots, and bounded validation in the Tier-1 tax engine.
- Tenant-scoped settlement route registry with immutable route versions,
  authenticated active-route lookup, deterministic payer ATA derivation, and
  quote-bound compilation that rejects free-form merchant-side account fields.
- ADR-0062 finalized settlement-route verifier service with bounded
  multi-provider `getMultipleAccounts`, exact account-layout and vault-PDA
  checks, append-only evidence, atomic activation, and Docker health wiring.
- ADR-0063 shared Rust/TypeScript settlement golden vectors covering
  instruction bytes, PDA derivation, account order, signer flags, and writable
  flags.
- ADR-0064 browser-side settlement-envelope validation before blockhash lookup
  or wallet signing.
- ADR-0065 validator-backed settlement conformance covering legacy SPL,
  extension-free Token-2022, duplicate nonce balance conservation, and
  rejection of a real Token-2022 TransferFeeConfig mint.
- An opt-in `make solana-conformance` command and Dockerized
  `cargo-build-sbf` fallback; ordinary unit/UI checks do not silently treat an
  unavailable validator as a passing release gate.
- ADR-0066 and `test-vectors/tax-engine-v1.json` add a versioned Tier-1 tax
  golden-vector contract covering global-tax arithmetic families and an exact
  cumulative evidence-digest regression.
- ADR-0067 isolates the Agave platform-tools v1.57 SBF compiler compatibility
  floor to the on-chain package, keeps off-chain crates on Rust 1.98.1, and
  hardens the amd64 Docker builder's non-root cargo/rustup cache path.
- ADR-0068 adds full-domain `u64` property coverage for on-chain amount
  validation while keeping live CPI/token-balance evidence as a separate
  validator-backed release gate.
- ADR-0069 binds tax calculation results to `engine_version` and adds complete
  fail-closed replay verification for component traces and evidence digests.
- ADR-0070 persists bounded normalized tax facts with each new quote and adds
  tenant-scoped, fail-closed PostgreSQL replay verification. Legacy rows without
  replay inputs remain explicitly non-replayable.
- ADR-0071 adds a Docker-isolated PostgreSQL 18 logical backup/restore drill
  that verifies restored replay-era schema and UUIDv7 support.
- The migration runner now fails closed on the first failed migration instead
  of allowing a later successful iteration to mask it.
- ADR-0072 adds exact and property-based tax arithmetic boundary evidence for
  zero, `u64::MAX`, rates, caps, floors, rounding ties, and conservation.
- ADR-0073 exports bounded fixed-label compiled-rule cache metrics from the API
  Prometheus endpoint.
- Added explicit compiled-tax-graph tests for dependency ordering and duplicate
  component-ID rejection; the corresponding readiness gate is now evidenced.
- ADR-0074 adds bounded NGINX per-source-IP request and connection admission
  with immutable image-build configuration validation.
- The Docker integration harness now proves PostgreSQL rejects overlapping
  published tax-rule windows in addition to published-rule mutation.
- ADR-0076 adds authenticated tax-rule author/reviewer/approver transitions,
  serializable governance revision fencing, append-only transition evidence,
  exact scope/role gates, and Docker integration coverage for the full
  publication lifecycle.
- ADR-0077 adds a repeatable-read, tenant-scoped governance projection and
  production React operator workspace with real authorization, transition
  evidence, retry/failure states, and bounded governance-read metrics.
- ADR-0078 adds a bounded Docker quote-service load harness that records
  authenticated status/error results, idempotent persistence counts, and
  p50/p95/p99 latency without making a production-performance claim.
- The persisted quote transaction now retries only PostgreSQL serialization
  and deadlock conflicts within a bounded attempt/backoff budget; the default
  200-request local workload passes with exact evidence counts.
- ADR-0081 tightens the NGINX CSP by removing `unsafe-inline` and adds Docker
  integration evidence for required security headers, unauthenticated `401`
  behavior, and deny-by-default CORS.
- Tenant rule ownership is now immutable in every lifecycle state, and replay
  joins include an explicit merchant predicate in addition to the composite
  database foreign key.
- ADR-0082 adds API-owned UUIDv7 request correlation: every response receives
  a bounded application-generated support ID, and caller/edge-supplied values
  cannot spoof the service correlation boundary.
- The React quote, governance, and checkout surfaces now render the validated
  API UUIDv7 support reference on service failures, without trusting arbitrary
  response payloads or client-provided IDs.
- ADR-0083 adds an executable PostgreSQL UUIDv7 catalog contract to clean-stack
  integration and the logical restore drill, preventing migration drift in
  generated sortable identifiers.
- The contract exposed and fixed a missing UUIDv7 version check on
  `settlement_route_verifications.id`; migration `0013` now applies the
  constraint safely to existing databases as well as new installs.

### Status

- ADR-0144 adds a server-owned persisted-rule simulation boundary. Governance
  simulation now loads the tenant-scoped immutable rule, verifies identity and
  canonical digest, rejects client jurisdiction evidence, uses the warm Rust
  compiled-rule cache, and returns an explicitly unattested trace. The SPA
  validates and renders the real create → reload → simulate path; this is not a
  quote, attestation, publication, or settlement authorization.
- The Docker integration harness now proves the ADR-0144 endpoint against
  PostgreSQL 18 with cross-tenant 404 isolation, jurisdiction/unknown-field
  rejection, tamper fail-closed behavior, canonical recovery, and retired-rule
  rejection.
- ADR-0145 adds a facts-only, two-replica Docker latency harness for persisted
  rule simulation. It records p50/p95/p99, pool pressure, response-binding
  invariants, and bounded read contention; the local p95 budget is not a
  regional production SLO.
- ADR-0146 centralizes exact locale-aware atomic/timestamp formatting, bounded
  language and IANA time-zone settings, and RTL document direction handling in
  the SPA. Full translation, real-browser, and assistive-technology evidence
  remain open.
- ADR-0147 adds redacted error containment around lazy workspaces so rejected
  chunks and render failures preserve global navigation and expose an explicit
  accessible reload state without implying financial or chain success.

- ADR-0143 adds the structured Tier-1 rule workbench contract: bounded
  component/dependency controls, progressive brackets, thresholds, exemptions,
  clamps, rounding, canonical JSON preview, and an explicit advanced JSON mode.

- ADR-0131 adds finalized `ProtocolPauseChanged` evidence projection,
  restart-safe fixed-cardinality pause/unknown alerts, a governed emergency
  pause runbook, and validator-backed localnet assertions that paused
  settlement cannot mutate token or receipt balances. Staging governance,
  alert-delivery, and incident evidence remain release gates.

- Architecture and implementation foundation phase; the simulation API and first settlement contract are not production deployment claims.
- No production deployment, audit completion, legal/tax certification, or mainnet support is claimed.
## Unreleased

- Hardened ADR-0403's navigation cleanup by removing the obsolete duplicate
  sidebar-account CSS contract and adding a failing-first Material shell
  regression that prevents the presentation surface from returning. The
  Docker/Nginx browser suite remains green with no authorization or financial
  behavior changes.

- Added ADR-0401 and an executable authorization-benchmark contract. The
  two-replica PBAC/ABAC latency benchmark now validates all nineteen
  server-owned capability policies, including revenue-streaming deny outcomes
  and resource-context indeterminate outcomes, instead of an incomplete subset.

- Added ADR-0378 and the optional Solidity 0.8.37 receipt-mirror adapter. The
  separately governed EVM package is non-custodial, conservation-checked,
  idempotent, Docker-tested, and cannot authorize or move Solana funds.

- Added ADR-0377 and a dedicated manually/tag-triggered Solana conformance
  evidence workflow. Blockchain owners can now run the exact commit on the
  pinned Linux/amd64 path and retain the verified validator artifact without
  waiting for the complete release-candidate bundle.

- Added ADR-0376 and a shared Linux/amd64 Solana conformance host preflight.
  Local ARM Docker runs now fail early with retained topology diagnostics, and
  authoritative evidence requires the versioned preflight to pass before
  validator startup.

- Added ADR-0371 and extended the Material 3 Expressive color contract to
  reject raw colors and gradients in production TypeScript/TSX source, closing
  the inline/CSS-in-JS palette bypass while preserving test fixtures.

- Added ADR-0372 and expanded the read-only Grafana service dashboard with
  bounded pause, replay, database-pool, tax-cache, and provider-health panels,
  guarded by a fixed-cardinality dashboard contract.

- Added ADR-0361 and a fail-closed Tier-1 Tax Engine rule compiler check that
  rejects repeated tax-base sources while retaining explicit additive and
  tax-on-tax bases.

- Added ADR-0362 and the versioned `GrossExcess` inclusive-threshold equation,
  with a golden vector and fail-closed rejection for undeclared threshold
  semantics.

- Added ADR-0353 and a Material 3 Expressive Aurora v2 shell pass: a
  seed-derived neutral tonal surface hierarchy, explicit semantic state
  containers, and a calmer active-navigation treatment replace competing
  blue-heavy layers without changing workspace, authorization, or settlement
  behavior. Added failing-first stylesheet contract coverage.

- Added ADR-0306 and collapsed advanced provider-location evidence by default
  in quote workspaces. Required evidence remains available through an
  accessible native disclosure, keeping the primary financial task readable
  without weakening server-side validation.
- Added ADR-0307 and an explicit `make reset-local` recovery path for stale
  synthetic Docker volumes. The command validates the Compose project name,
  requires confirmation, and never performs broad host deletion.

- Refined the SaaS navigation and visible product language around operator
  tasks, approvals, money movement, and evidence. Engineering terminology is
  now progressive disclosure rather than the primary journey; security and
  settlement boundaries remain explicit.
- Add ADR-0262 and a production-shaped React SaaS control-plane shell with
  grouped navigation, tenant/environment context, explicit risk affordances,
  and a dedicated RBAC/PBAC/ABAC access-control workspace. The browser never
  decodes JWT claims or treats an in-memory token as authorization; the API
  capability snapshot remains a required production gate.
- Added the server-owned `GET /v1/authorization/capabilities` contract with
  bounded UUIDv7 snapshots, exact scope/role decisions, fail-closed resource
  context handling, and React integration tests. The snapshot is advisory UI
  state; protected API and settlement operations still re-authorize.
