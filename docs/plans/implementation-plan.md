# Implementation plan

## ADR-0360 versioned document-simulation localization

- [x] Add immutable typed catalogs for English, German, Arabic RTL, and
  Japanese document-simulation presentation copy.
- [x] Localize form labels, line actions, unavailable/error/result states, and
  accessible region labels without translating protocol or server-owned values.
- [x] Add catalog and component coverage for German rendering and fail-closed
  service unavailability.
- [ ] Retain manual linguistic, RTL, assistive-technology, and tax-owner review
  evidence before a production UI claim.

See [ADR-0360](../adr/0360-versioned-document-simulation-localization.md).

## ADR-0359 explicit fee-payer binding

- [x] Verify the first static signed-message account is the first compiled
  account, a signer, and writable as the Solana fee payer.
- [x] Apply the bounded check across settlement, adjustment, refund,
  cancellation, and revenue-stream wallet-submission profiles.
- [x] Add a real two-key regression test that rejects a sponsored fee payer
  even when the compiled payer signs the instruction.
- [ ] Retain Linux validator, staging wallet, and independent security-review
  evidence before enabling production submission.

See [ADR-0359](../adr/0359-explicit-fee-payer-binding.md).

## ADR-0358 versioned lazy-workspace and recovery-state localization

- [x] Localize lazy workspace fallbacks through the shell-operational catalog
  without translating workspace identities or protocol values.
- [x] Localize the error boundary recovery state while redacting exception
  details and preserving the fail-closed no-authority behavior.
- [x] Add German recovery-state coverage and preserve the full browser
  failure-path gate.
- [ ] Retain manual assistive-technology, linguistic, and incident-response
  review evidence for degraded frontend states.

See [ADR-0358](../adr/0358-versioned-lazy-workspace-recovery-localization.md).

## ADR-0357 versioned shell operational localization

- [x] Add immutable typed catalogs for system status, account context,
  protection disclosures, and footer presentation in English, German, Arabic
  RTL, and Japanese.
- [x] Preserve build/runtime labels, network values, configuration warnings,
  subjects, tenant IDs, capability decisions, and server-owned reason text.
- [x] Add catalog tests and retain the shell's existing localized browser,
  RTL, responsive, and accessibility journeys.
- [ ] Retain manual operator, screen-reader, high-contrast, linguistic, and
  security review evidence before a production shell claim.

See [ADR-0357](../adr/0357-versioned-shell-operational-localization.md).

## ADR-0356 versioned access-control workspace localization

- [x] Add immutable typed catalogs for access explanations, authorization
  states, policy labels, groups, decisions, and risk labels in English,
  German, Arabic RTL, and Japanese.
- [x] Preserve capability IDs, decision kinds, reason codes, scope names,
  role names, tenant IDs, subjects, and server-owned snapshots exactly.
- [x] Pass the shell locale into the access-control workspace and cover
  catalog completeness, German component rendering, and Arabic RTL browser
  accessibility.
- [ ] Retain manual security-operator, linguistic, screen-reader, RTL, and
  multi-tenant review evidence before a production authorization claim.

See [ADR-0356](../adr/0356-versioned-access-control-localization.md).

## ADR-0355 versioned revenue-streaming operator localization

- [x] Add immutable typed catalogs for partner-payout balances, governed plan
  lifecycle, setup references, claim preparation, and release-stop language in
  English, German, Arabic RTL, and Japanese.
- [x] Preserve plan IDs, status values, route/reference IDs, slots, atomic
  amounts, Solana addresses, digests, capability decisions, and request bodies
  while localizing presentation copy.
- [x] Pass the active shell locale into the lazy revenue-streaming workspace and
  retain locale-aware integer display.
- [x] Add component catalog/flow tests and a German/Arabic RTL browser journey
  with axe checks.
- [ ] Retain manual payout-operator, tax-owner, linguistic, and assistive
  technology review evidence before a production UI claim.

See [ADR-0355](../adr/0355-versioned-revenue-streaming-localization.md).

## ADR-0354 versioned rule-governance and authoring localization

- [x] Add immutable typed catalogs for the governance workspace and structured
  rule workbench in English, German, Arabic RTL, and Japanese.
- [x] Localize authoring, provenance, lifecycle, simulation, loading, empty,
  and fail-closed validation states while preserving canonical rule values,
  status discriminators, evidence, digests, and request bodies.
- [x] Pass the active shell locale into the workbench and cover exact enum
  values with catalog and component tests.
- [x] Add German and Arabic RTL browser coverage with axe checks for the
  high-consequence tax-rule authoring path.
- [ ] Retain manual tax-owner, linguistic, screen-reader, RTL, and operator
  usability review evidence before a production UI claim.

See [ADR-0354](../adr/0354-versioned-rule-governance-authoring-localization.md).

## ADR-0353 Material 3 Expressive Aurora v2 shell

- [x] Replace the blue-heavy v1 surface matrix with a versioned, neutral-tonal
  Aurora v2 role matrix derived from the single approved seed.
- [x] Add explicit semantic success, attention, blocked, and supporting
  containers so state presentation cannot drift through component-local blends.
- [x] Apply one calm active-navigation treatment, lower the shell's competing
  surface emphasis, and preserve all grouped workspaces, responsive behavior,
  RTL, forced-colors, and reduced-motion semantics.
- [x] Add failing-first stylesheet coverage and retain the palette/color
  contracts as release gates.
- [ ] Retain manual wide/narrow operator, screen-reader, high-contrast, and
  linguistic review evidence before any production UI claim.

See [ADR-0353](../adr/0353-material-3-expressive-aurora-v2-shell.md).

## ADR-0352 versioned vault-reconciliation localization

- [x] Add a typed immutable catalog for read-only balance checks, active source
  selection, evidence detail, stale/mismatch/error release-stop states, and
  append-only history in English, German, Arabic RTL, and Japanese.
- [x] Preserve exact atomic balances, slots, account/mint identities, route and
  run IDs, statuses, error codes, and release authority while localizing only
  presentation copy.
- [x] Keep amount and timestamp formatting on the validated locale/time-zone
  path without changing persisted or server-owned values.
- [x] Add failing-first catalog/component tests and German/Arabic RTL browser
  coverage with axe checks.
- [ ] Retain manual treasury-operator, linguistic, assistive-technology, and
  incident-response review evidence.

## ADR-0351 versioned replay-operations localization

- [x] Add a typed immutable catalog for replay-gap review, bounded requeue,
  targeted finalized-evidence recovery, route-verification evidence, and
  explicit non-authority language in English, German, Arabic RTL, and Japanese.
- [x] Preserve stream keys, statuses, slots, signatures, addresses, operator
  notes, action discriminators, route evidence IDs, and API payloads exactly.
- [x] Keep locale/time-zone presentation separate from recovery and indexer
  state, including localized status labels and durable timestamp formatting.
- [x] Add failing-first catalog/component tests and German/Arabic RTL browser
  coverage with axe checks.
- [ ] Retain manual incident-response, linguistic, assistive-technology, and
  operator-usability review evidence.

## ADR-0350 versioned settlement-adjustment localization

- [x] Add a typed immutable catalog for correction proposal, approval, recovery,
  wallet-bound record evidence, provider uncertainty, and finalized evidence in
  English, German, Arabic RTL, and Japanese.
- [x] Keep financial values, adjustment kinds, UUIDv7 identifiers, evidence
  digests, signed bytes, request payloads, and server-owned statuses unchanged;
  localize only presentation labels and user guidance.
- [x] Pass the active locale through the correction workspace and reusable
  wallet picker without adding work to the signing or settlement path.
- [x] Add failing-first catalog/component coverage and a German/Arabic RTL real
  browser journey with axe checks.
- [ ] Retain manual linguistic, assistive-technology, operator-usability, and
  legal/merchant-term review evidence for correction workflows.

## ADR-0335 Material 3 Expressive quiet state hierarchy

- [x] Add failing-first coverage for the compact sign-in configuration state
  while preserving the expanded error alert.
- [x] Use governed semantic state aliases and a tonal active-route treatment;
  no new component color literals or gradients were introduced.
- [x] Record the hierarchy, alternatives, invariants, and verification in
  [ADR-0335](../adr/0335-material-expressive-quiet-state-hierarchy.md).
- [ ] Retain manual operator-usability, screen-reader, high-contrast, and
  production theme review evidence.

## ADR-0334 capability snapshot expiry boundary

- [x] Add failing-first tests for render-time expiry, missing expiry, and
  bounded refresh scheduling.
- [x] Make stale and missing capability snapshots indeterminate and trigger a
  bounded capability refresh at the server-provided expiry.
- [x] Record alternatives, invariants, latency boundaries, and verification in
  [ADR-0334](../adr/0334-capability-snapshot-expiry-boundary.md).
- [ ] Retain staging identity-provider, clock-skew, screen-reader,
  multi-tenant, and production authorization evidence.

## UI language and navigation quality gate

The product shell is organized around operator tasks: daily work, rules and
access, money movement, and records and issues. Primary labels must stay in
plain language; exact rule references, fingerprints, engine versions, and
protocol details belong in accessible progressive disclosures or administrator
surfaces. Every new workspace must define its breadcrumb, stable path, access
state, empty/loading/error/reconciliation states, safe next action, and
keyboard/mobile navigation behavior before implementation. See ADR-0282.

The attached evaluation's “streaming payouts” language is explicitly bounded by
ADR-0283. The first production settlement authority is a bounded atomic
per-transaction split: tax withholding, vault deposit, and recipient allocation
must succeed or fail together. Continuous streaming is a later bounded context
and must consume finalized, attested accruals without bypassing tax, vault,
pause, authorization, or evidence controls.

### ADR-0285 — Finalized revenue-streaming claim domain

- [x] Add the pure `revenue-streaming` domain crate for finalized accruals and
  single-use release claims.
- [x] Add storage/API adapters with tenant-scoped authorization and durable
  idempotency after the domain contract is accepted.
- [x] Add the bounded Solana plan, finalized-accrual reference, single-use
  claim, cancellation, and vault-CPI instructions with explicit account
  bindings and checked caps. The program does not claim to verify RPC
  finality; the operator signature remains downstream of the proof adapter.
- [x] Add deterministic settlement-client PDA and instruction compilers with
  account-order, wire-shape, authority, digest, window, and aliasing tests.
- [x] Add the partner payout UI and real-browser authorized/denied journeys;
  it still keeps chain signing and finalized evidence behind the explicit
  production boundary.
- [x] Bind each plan to exactly one server-resolved active settlement route;
  browser requests cannot choose merchant-side accounts or route topology.
- [x] Persist the named payout operator subject and extend the wallet-proof
  boundary with context-qualified revenue-stream roles before exposing any
  revenue-stream compiler or wallet submission endpoint; see ADR-0289. The
  compiler and wallet submission endpoint remain intentionally gated.
- [x] Add the pure finalized proof adapter and typed finalized split projection;
  see ADR-0290.
- [x] Add the bounded application reconciler that resolves server-owned split
  allocations and records idempotent accruals; see ADR-0291.
- [x] Persist finalized revenue reconciliation jobs in the same transaction as
  indexed split evidence and run the leased worker; see ADR-0292.
- [x] Add the finalized signed-message verifier; it rejects extra instructions,
  lookup tables, account substitution, and signer/writable drift. The design is
  recorded in ADR-0293.
- [x] Add the server-owned revenue payout-destination registry, immutable
  versions, tenant-scoped active resolution, and finalized chain verification;
  see ADR-0294. Browser requests must never supply destination token accounts.
- [x] Bind new plans to the resolved destination route version and evidence
  digest; legacy plans remain readable but claim compilation stays blocked.
- [x] Replace raw revenue-plan digest/topology inputs with the bounded
  server-owned UUIDv7 reference catalog and selectable production UI; see
  ADR-0295.
- [x] Add the real-browser journey for authenticated reference loading,
  business-record selection, ID-only proposal submission, technical-input
  redaction, and browser axe validation; `make web-browser-check` now covers
  the complete local flow.
- [x] Add the real validator-backed revenue-stream movement case and release
  evidence requirement; see ADR-0301. Execute it on Linux/amd64 before
  enabling the revenue-stream movement path.

### ADR-0297 — production account and session menu

- [x] Replace the inert top-bar avatar with a server-context account/session
  disclosure that exposes safe identity status, links to access review, and
  signs out through the existing session boundary; see ADR-0297.
- [x] Add anonymous, access-error, keyboard, accessibility, and real-browser
  coverage without rendering credentials or raw identity-provider claims.

### ADR-0298 — Material 3 Expressive visual system

- [x] Define one semantic Material role palette for light, dark, and system
  themes; enforce a final role-token cascade for the rendered shell while
  enforcing the repository-wide semantic color-token contract from ADR-0300.
- [x] Reduce navigation and top-bar repetition so task, state, and next action
  have a clear visual hierarchy.
- [x] Add theme preference controls that only change presentation state and
  preserve authorization, wallet, and settlement behavior.
- [x] Verify contrast, reduced motion, RTL, responsive layout, component tests,
  real-browser accessibility, and public design documentation.
- [x] Add a real-browser cross-workspace theme regression covering all product
  navigation entries and proving light/dark surface separation.
- [x] Enforce the CSP, semantic color-role, and plain-language contracts in the
  pull-request web CI job so visual and content regressions block review.

### ADR-0302 — versioned local display preferences

- [x] Persist only validated locale, IANA time zone, and Material theme
  preferences in a versioned local schema; malformed, oversized, or
  unavailable browser storage falls back to safe defaults.
- [x] Prove the preference store cannot restore credentials, authorization
  snapshots, financial facts, signed bytes, or wallet material.
- [x] Add real-browser reload evidence for locale, RTL direction, time zone,
  and theme restoration while proving the local credential remains empty.

### ADR-0300 — semantic color-token migration

- [x] Add a failing-first stylesheet contract that rejects raw color literals
  outside the Material token declaration blocks.
- [x] Migrate all component and workspace color declarations to semantic roles,
  role-derived alpha values, and shared elevation/action tokens.
- [x] Re-run component, browser, contrast, responsive, RTL, reduced-motion,
  typecheck, and production-build evidence after the migration.

### ADR-0310 — tonal surfaces and no decorative gradients

- [x] Add a failing-first web contract that rejects component gradients outside
  the Material token declaration blocks.
- [x] Replace decorative gradients in the brand mark, sidebar, selected
  navigation, cards, primary actions, calculation totals, and review actions
  with explicit Material surface, container, action, and state roles.
- [x] Preserve expressive hierarchy through semantic palette roles, shared
  elevation, shape, typography, responsive layout, and reduced-motion rules.
- [ ] Re-run the complete manual operator usability and assistive-technology
  review across every production tenant workspace before making UX claims.

### ADR-0311 — Solana Aurora Material 3 role palette

- [x] Name the repository theme family and document the fixed `#8DF0CE`
  palette seed and role ownership.
- [x] Replace the generic lavender/pink role matrix with deterministic Solana
  navy, Aurora Mint, and accessible blue tertiary light/dark roles.
- [x] Extend the dependency-free palette contract to pin exact role values and
  contrast, preventing undocumented/random palette drift.
- [ ] Complete manual visual, operator, localization, and assistive-technology
  review across supported tenant roles before making production UX claims.

### ADR-0317 — Material 3 Expressive surface hierarchy and navigation density

- [x] Extend the fixed Solana Aurora matrix with the M3 surface-container
  hierarchy and semantic aliases for shell layers.
- [x] Reduce shell scan cost through compact navigation spacing and tonal
  separation without removing any production workspace or authorization state.
- [x] Add palette, component, browser, responsive, RTL, and reduced-motion
  evidence for the hierarchy; manual operator and assistive-technology review
  remains a release gate.

### ADR-0323 — Material 3 Expressive operational workspace composition

- [x] Add native accessible disclosure groups for navigation, keeping Daily
  work open by default and opening the active group automatically.
- [x] Preserve every production route, server-owned capability decision,
  `aria-current` marking, keyboard reachability, and responsive mobile
  navigation behavior.
- [x] Update real-browser journeys to exercise the production group
  interaction before activating a lower-frequency route.
- [x] Verify the composition with the current 62-file web suite (255 passing
  tests, 10 intentionally skipped), the semantic color and Material palette
  contracts, and the current Docker-backed browser journeys.
- [ ] Complete manual wide/narrow, screen-reader, keyboard-only, RTL,
  reduced-motion, and operator usability review before making production UX
  claims.

### ADR-0324 — Server-owned vault-release route selection

- [x] Add the vault-proposer route-list endpoint using the existing bounded,
  tenant-scoped, finalized-verified settlement-route registry.
- [x] Remove browser-supplied mint, token-program, and tax-vault fields from
  vault-release proposal requests; accept only a UUIDv7 route ID and the
  beneficiary account.
- [x] Resolve and validate the route inside the serializable persistence
  transaction, bind the route ID and resolved account snapshot into the v2
  request digest, and record route identity in creation evidence.
- [x] Replace free-form merchant-side account inputs with a Material 3
  route selector and read-only route evidence, with fail-closed loading and
  empty-state behavior.
- [x] Preserve exact idempotent replay of an accepted claim after route
  suspension while rejecting new proposals; verify the no-duplicate-event and
  no-duplicate-outbox invariant in the Docker integration harness.
- [ ] Add production staging evidence for route suspension during proposal
  creation and retain a successful Linux/amd64 integration artifact.

### ADR-0325 — typed asset-admission projection

- [x] Add typed `AssetConfig` identity, classification, enabled state, and
  PDA-bump columns to the settlement route projection with expand/contract
  constraints.
- [x] Bind route activation and active-route reads to the typed projection;
  preserve raw immutable evidence for replay and forensics.
- [x] Add migration, fixture, route-verifier, storage, and direct-SQL negative
  tests before implementation.
- [ ] Backfill and re-verify all non-local routes, retaining counts and
  rollback evidence before enabling paid-state movement.

### ADR-0326 — Material 3 Expressive semantic state hierarchy

- [x] Record the visual/accessibility decision and compare repeated badges,
  repeated availability text, hidden state, and accessible row context.
- [x] Add a failing-first UI contract for removing persistent navigation state
  badges while retaining access and action-risk semantics.
- [x] Implement the semantic row context and remove redundant visual badges
  from wide, narrow, and mobile navigation rules.
- [x] Replace first-letter navigation avatars with a fixed monochrome semantic
  glyph set; glyph identity never communicates authorization, risk, tenant, or
  service state.
- [ ] Complete manual screen-reader, keyboard-only, RTL, reduced-motion, and
  light/dark visual evidence for the release candidate.

### ADR-0327 — fail-closed public release claims

- [x] Compare reviewer-only prose, blocking all release evidence, automatic
  licensing, and an executable negative contract before implementation.
- [x] Add baseline and mutation tests for unsupported production claims,
  unapproved license files, and release-workflow deployment authority.
- [x] Add the contract to `make check` and the public-repository contract.
- [ ] Obtain repository-owner license approval, independent release review,
  and all external production-readiness evidence before public source or
  production release.

### ADR-0328 — durable route-verification run evidence

- [x] Compare logs/metrics, a mutable singleton, per-route audit rows, and a
  bounded append-only run ledger before implementation.
- [x] Add UUIDv7 run persistence with bounded counters, explicit running,
  completed, completed-with-failures, and failed lifecycle states.
- [x] Wire non-empty scheduled route-verification passes to durable summary
  completion; idle polls create no evidence rows.
- [x] Add direct-SQL negative coverage for counter conservation and terminal
  immutability, plus a public operational runbook.
- [ ] Execute an approved backfill against a representative non-local
  snapshot and retain the exact run IDs, route evidence, and independent
  review before paid-state enablement.

### ADR-0329 — approval-bound route-verification backfill mode

- [x] Compare daemon timing, an unauthenticated run-once flag, a separate
  service, and an approval-bound one-shot mode before implementation.
- [x] Add UUIDv7 approval binding to backfill run evidence and reject missing,
  malformed, or mode-inconsistent approval configuration before any listener,
  database connection, or RPC transport is created.
- [x] Execute exactly one bounded settlement-route batch in backfill mode and
  exit without running unrelated verifier loops or the health listener.
- [x] Add configuration, storage, direct-SQL, and executable one-shot contract
  coverage, then update the operator runbook and public documentation index.
- [ ] Execute an approved backfill against a representative non-local
  snapshot and retain the approval ID, run ID, exact route evidence, and
  independent review before paid-state enablement.

### ADR-0336 — bounded route-verification operator evidence view

- [x] Add failing-first storage/API parser tests for bounded run evidence,
  UUIDv7 identity, terminal count conservation, and approval binding.
- [x] Add an authenticated read-only endpoint using the existing system
  operator boundary; no merchant identifier or route account is accepted.
- [x] Add an explicit disclosure in the recovery workspace with loading,
  empty, unauthorized, unavailable, malformed, and read-only evidence states.
- [x] Add real-browser coverage proving the evidence is loaded only after an
  operator action and never implies route activation or finalized settlement.
- [x] Add authenticated Docker integration coverage against PostgreSQL for a
  synthetic terminal run, bounded response parsing, failure classification,
  and invalid-limit rejection.
- [ ] Retain non-local backfill, Linux/amd64 validator, audit, and production
  operator evidence separately from this local UI/API proof.

### ADR-0337 — single-open Material 3 navigation group

- [x] Add a failing-first shell test proving that opening a second group closes
  the previous group while preserving the active-workspace context.
- [x] Replace accumulating disclosure state with one explicit open group and
  keep route changes synchronized with the current workspace group.
- [x] Record the alternatives, presentation/authorization invariants, and
  latency boundary in [ADR-0337](../adr/0337-single-open-material-navigation-group.md).
- [ ] Retain manual wide/narrow, keyboard-only, screen-reader, RTL, and
  production theme review evidence.

### ADR-0338 — forced-colors accessibility layer

- [x] Add a failing-first static contract requiring system colors,
  `forced-color-adjust: auto`, and no palette literals in the accessibility
  layer.
- [x] Map Material semantic roles to operating-system system colors without
  adding a second persisted product theme or touching authorization/financial
  behavior.
- [x] Add forced-colors Playwright emulation and WCAG browser coverage.
- [x] Record alternatives, invariants, and resource impact in
  [ADR-0338](../adr/0338-forced-colors-accessibility-layer.md).
- [ ] Retain manual OS high-contrast, screen-reader, and operator usability
  review evidence before production release.

### ADR-0339 — polite workspace navigation announcement

- [x] Add a failing-first shell test for a bounded `role="status"` workspace
  announcement that contains no authorization, financial, or technical data.
- [x] Implement the non-interruptive live region without adding a network,
  timer, dependency, or financial-path operation.
- [x] Add real-browser and axe coverage for direct routes and normal workspace
  navigation, then update the public evidence matrix.
- [ ] Retain manual screen-reader and operator usability review evidence before
  production release.

### ADR-0340 — versioned shell and navigation localization

- [x] Add failing-first catalog tests for every supported locale and workspace
  with deterministic English fallback.
- [x] Localize shell navigation labels, descriptions, groups, risk context, and workspace
  status announcements using a typed bounded catalog.
- [x] Add UI/browser RTL and axe coverage without changing authority or
  financial data; keep full panel translation as a separately tracked slice.
- [ ] Retain linguistic, legal/tax terminology, screen-reader, and operator
  review evidence before claiming full localization.

### ADR-0341 — versioned shell chrome and session localization

- [x] Add failing-first catalog coverage for bounded shell controls, display
  preferences, session actions, dynamic timezone help, and deterministic
  English fallback across all supported locales.
- [x] Localize navigation disclosure, display settings, theme options, time
  zone warnings, and the local/OIDC session-control labels without passing or
  rendering credentials through the catalog.
- [x] Update accessibility and real-browser journeys for translated labels,
  RTL behavior, credential non-persistence, and axe coverage through NGINX.
- [ ] Complete workspace-panel catalogs and retain linguistic, legal/tax,
  screen-reader, and operator review evidence before claiming full
  localization.

### ADR-0342 — versioned quote workspace localization

- [x] Add failing-first catalog coverage for quote, simulator, location,
  retry-protection, result, evidence, and deterministic fallback copy in every
  supported locale.
- [x] Localize the customer quote and scenario workspaces without translating
  amounts, currencies, rule identifiers, jurisdiction values, fingerprints,
  component explanations, or server-owned evidence.
- [x] Add component and real-browser German/Arabic coverage with RTL and axe
  assertions through the NGINX-served SPA.
- [ ] Retain linguistic, legal/tax terminology, screen-reader, and operator
  review evidence before claiming full workspace localization.

### ADR-0343 — versioned document-quote localization

- [x] Add failing-first catalog coverage for invoice quote form, line items,
  location evidence, retry protection, result, notices, and deterministic
  fallback in every supported locale.
- [x] Localize the invoice quote workspace without translating document IDs,
  line IDs, amounts, currencies, jurisdictions, rule references, digests, or
  server-owned evidence values.
- [x] Add component and real-browser German/Arabic coverage with RTL and axe
  assertions through the NGINX-served SPA.
- [ ] Retain linguistic, legal/tax terminology, screen-reader, and operator
  review evidence before claiming full workspace localization.

### ADR-0344 — versioned document-checkout localization

- [x] Add typed `en-US`, `de-DE`, `ar-EG`, and `ja-JP` catalogs for invoice
  evidence, route selection, payment scope, review, signing, settlement
  status, recovery, and withholding/legal language boundaries.
- [x] Localize the reusable wallet picker with a separate bounded catalog;
  wallet addresses, route IDs, mints, amounts, digests, signatures, and
  server-owned status values remain unchanged.
- [x] Add failing-first catalog/component tests and real-browser German,
  Arabic RTL, and axe coverage through the production route.
- [ ] Retain linguistic, legal/tax terminology, screen-reader, and operator
  review evidence before claiming full workspace localization.

### ADR-0345 — versioned aggregate-checkout localization

- [x] Add typed `en-US`, `de-DE`, `ar-EG`, and `ja-JP` catalogs for aggregate
  route review, recipient-plan selection, safeguards, quote expiry, amount
  review, technical disclosure, wallet approval, and settlement recovery.
- [x] Preserve financial values, route/account identifiers, mints, basis
  points, digests, intent IDs, signatures, server statuses, and request
  payloads exactly while translating only presentation copy.
- [x] Add failing-first catalog/component tests and real-browser German,
  Arabic RTL, and axe coverage through the NGINX-served payment-review route.
- [ ] Retain linguistic, legal/tax terminology, screen-reader, and operator
  review evidence before claiming full workspace localization.

### ADR-0346 — versioned Treasury and vault-release localization

- [x] Add a typed `en-US`, `de-DE`, `ar-EG`, and `ja-JP` catalog for governed
  release proposal, verified-route, lifecycle, transition, evidence, and
  authority-boundary copy.
- [x] Preserve route IDs, destination accounts, mints, token programs, nonces,
  amounts, timestamps, digests, transaction references, slots, request
  payloads, and server-owned claim statuses exactly while translating only
  presentation copy.
- [x] Add failing-first catalog/component coverage and real-browser German,
  Arabic RTL, and axe coverage through the NGINX-served Treasury route.
- [ ] Retain linguistic, legal/tax terminology, screen-reader, and operator
  review evidence before claiming full workspace localization.

### ADR-0347 — versioned refund-claim localization at the wallet boundary

- [x] Add a typed `en-US`, `de-DE`, `ar-EG`, and `ja-JP` catalog for refund
  preparation, immutable artifact review, wallet roles, recovery, finalized
  evidence, and reconciliation copy.
- [x] Preserve claim/correction identifiers, amounts, destinations, account
  keys, signatures, transaction bytes, status values, evidence digests, and
  request payloads exactly while translating only presentation copy.
- [x] Add failing-first catalog/component coverage and pass the existing
  unchanged-byte wallet/state-machine tests.
- [x] Add real-browser German, Arabic RTL, and axe coverage through the
  NGINX-served refund-request route while preserving the existing finalized
  evidence journey.
- [ ] Retain linguistic, legal/tax, screen-reader, and operator review
  evidence before claiming full refund-workspace localization.

### ADR-0348 — versioned refund-execution localization at the final release boundary

- [x] Add a typed `en-US`, `de-DE`, `ar-EG`, and `ja-JP` catalog for final
  refund-release preparation, immutable artifact review, two-wallet approval,
  recovery, proof-pending, and finalized movement evidence.
- [x] Preserve release/request identifiers, amounts, mints, destinations,
  signatures, transaction bytes, status values, evidence digests, and request
  payloads exactly while translating only presentation copy.
- [x] Add failing-first catalog/component coverage and preserve the existing
  unchanged-byte refund execution state-machine tests.
- [x] Add real-browser German, Arabic RTL, and axe coverage through the
  NGINX-served refund-payment route.
- [ ] Retain linguistic, legal/tax, screen-reader, and operator review
  evidence before claiming full refund-workspace localization.

### ADR-0349 — versioned refund-cancellation localization at the governance boundary

- [x] Add a typed `en-US`, `de-DE`, `ar-EG`, and `ja-JP` catalog for controlled
  cancellation, immutable artifact review, governance-wallet signing,
  recovery, provider uncertainty, and finalized cancellation evidence.
- [x] Preserve claim/request identifiers, authority accounts, signatures,
  transaction bytes, status values, evidence digests, and request payloads
  exactly while translating only presentation copy.
- [x] Pass the active locale into the shared wallet picker and keep recovery
  data memory-only and byte-for-byte unchanged.
- [x] Add failing-first catalog/component coverage and real-browser German,
  Arabic RTL, and axe coverage through the NGINX-served cancellation route.
- [ ] Retain linguistic, legal/tax, screen-reader, and operator review
  evidence before claiming full refund-workspace localization.

### ADR-0330 — server-owned document evidence status for checkout

- [x] Compare raw JSON paste, raw evidence reads, browser-selected evidence,
  and a server-owned route-bound status contract before implementation.
- [x] Add the authenticated route-bound evidence status endpoint while keeping
  issuer ingestion and settlement compilation as separate authority boundaries.
- [x] Replace the visible attestation JSON textarea with a plain-language
  status card and collapsed technical details; keep all failure states
  fail-closed and accessible.
- [x] Add API, parser, browser, accessibility, and public-documentation
  contract coverage before reviewing this payment flow as production-grade.

### ADR-0331 — canonical authorization policy catalog

- [x] Compare independent catalogs, browser-fetched metadata, generated
  projections, and a versioned shared contract before implementation.
- [x] Make the versioned catalog the single metadata source for API capability
  evaluation and React access-review/navigation affordances.
- [x] Validate bounded shape, uniqueness, scope/role/attribute semantics, and
  fail-closed behavior before any capability snapshot is issued.
- [x] Add Docker build-context, API, frontend, browser, and public-document
  contract coverage and verify no RBAC/PBAC/ABAC drift remains.

### ADR-0332 — server-decision-only browser authorization affordances

- [x] Add a failing-first regression proving copied browser roles, scopes, and
  attributes cannot authorize an action without the exact server decision.
- [x] Make missing policy decisions remain indeterminate while preserving
  explicit server allow/deny/indeterminate outcomes.
- [x] Record the authority boundary, alternatives, latency impact, and
  verification gates in ADR-0332.
- [ ] Retain staging identity-provider, screen-reader, multi-tenant, and
  production authorization evidence.

### ADR-0333 — capability decision and reason consistency

- [x] Add failing-first parser cases for unknown and contradictory capability
  decision/reason pairs.
- [x] Enforce the explicit allow/deny/indeterminate reason matrix before a
  capability snapshot enters React state.
- [x] Record alternatives, invariants, latency impact, and verification in
  ADR-0333.
- [ ] Retain staging identity-provider, screen-reader, multi-tenant, and
  production authorization evidence.

### ADR-0318 — workflow handoff URL integrity

- [x] Route every user-initiated workspace change, including child-workspace
  handoffs, through one App-owned typed History API boundary.
- [x] Keep the document quote-to-payment calculation reference in memory while
  updating the canonical `/operations/document-payment` URL exactly once.
- [x] Keep credentials, wallet material, tenant identifiers, financial facts,
  evidence, signatures, and authorization decisions outside browser history.
- [x] Add a failing-first real-browser regression for the handoff URL and direct
  destination reload, then verify the NGINX browser gate.
- [ ] Complete manual support-link, screen-reader, localization, and operator
  review across all handoff paths before making production UX claims.

### ADR-0319 — bounded browser error disclosure

- [x] Add a pure shared browser normalizer for untrusted API error messages;
  control characters are removed, whitespace is collapsed, and messages are
  bounded to 512 characters before entering React state.
- [x] Preserve actionable operation-specific fallbacks for empty or malformed
  server messages and append support references only for canonical UUIDv7 IDs.
- [x] Add failing-first API-boundary tests for long messages, control
  characters, whitespace-only messages, and invalid support references.
- [ ] Complete staging support-artifact redaction, localization, and manual
  operator review before making production supportability claims.

### ADR-0320 — on-chain stablecoin asset admission

- [x] Freeze the mint-scoped `AssetConfig` PDA decision and compare the
  off-chain-only, hard-coded-mint, metadata-inference, and per-merchant-list
  alternatives before changing the settlement ABI.
- [x] Implement the first reviewable on-chain slice: immutable mint-scoped
  configuration, governed enable/disable, and fail-closed admission checks on
  every current token movement context; add the thirteenth aggregate/document
  account and fixed Split Plan account order to the client/browser contracts.
- [x] Add focused Rust, settlement-client, API, coordinator, and browser tests
  for the new PDA, account order, and browser-side substitution rejection.
- [x] Add failing-first domain tests for enabled/disabled state, wrong
  mint/program/decimals, and non-stable classification; account-context
  enforcement is covered by the compiled instruction and integration gates.
- [x] Add the bounded initializer/disable instructions and require the enabled
  stablecoin configuration in aggregate, document, split, vault-release,
  revenue-stream, and adjustment-refund token movement contexts.
- [x] Add the additive settlement-client, coordinator, browser-envelope, and
  cross-language account-order vectors; reject pre-0320 prepared artifacts.
- [x] Add PostgreSQL asset projection and route-verifier evidence with an
  expand/contract migration; active routes without v2 evidence are suspended,
  and active reads require finalized asset proof. Signing must never receive a
  silent stablecoin default.
- [x] Add real-validator transaction builders for legacy-SPL and extension-free
  Token-2022 positive movement, disabled-asset rollback, and TransferFeeConfig
  rejection; the builders now use the post-0320 `AssetConfig` account order.
- [x] Add non-stable, non-conserving amount, and substituted-account validator
  transaction negatives, plus authenticated Docker tenant-isolation rejection
  for aggregate compile; successful Linux/amd64 execution and retained
  evidence remain open.
- [ ] Retain the successful Linux/amd64 execution evidence for the complete
  stablecoin validator matrix.
- [ ] Retain Linux/amd64 validator, audit, governance, and route-backfill
  evidence before enabling paid-state movement. Until then, missing asset
  evidence remains an explicit unavailable or re-verification-required state.

### ADR-0321 — finalized asset-configuration route evidence

- [x] Extend finalized settlement-route verification to request and validate
  the deterministic five-account topology, including the enabled stablecoin
  `AssetConfig` PDA, exact discriminator/layout, mint, token program, decimals,
  asset kind, and PDA bump.
- [x] Require five bounded account evidence records for route activation and
  version the route-verification digest to `v2`, preventing four-account
  pre-admission evidence from being replayed as current evidence.
- [x] Add failing-first indexer coverage for five-account requests and disabled
  asset configurations; preserve provider agreement and finalized commitment.
- [ ] Run the route verifier against every existing active route and retain
  backfill/re-verification evidence before enabling signing in a non-local
  environment.
- [ ] Add Linux/amd64 validator vectors for initialized, enabled, disabled,
  non-stable, and substituted asset configurations, followed by independent
  audit and governance review.

### ADR-0322 — route-evidence version migration

- [x] Add the expand/contract migration with explicit legacy/v2 schema
  metadata, preserve append-only historical evidence, and suspend active rows
  that cannot prove the v2 five-account topology.
- [x] Require v2 evidence in route activation and every active-route read, so
  stale services and direct callers cannot advertise legacy routes.
- [x] Exercise migration/bootstrap, concurrent submission, ambiguous outcome,
  process-crash reconciliation, and rule-rollback integration scenarios in the
  Docker PostgreSQL 18 stack.
- [ ] Run the migration against a representative production snapshot, retain
  suspended-row and re-verification counts, and rehearse rollback/restore.
- [ ] Re-verify every non-local route at finalized commitment before enabling
  signing; complete the Linux/amd64 validator, audit, governance, and legal
  gates.

### ADR-0312 — outcome-first document payment language

- [x] Replace protocol-first document checkout labels with plain-language
  review, evidence, wallet, route, and approval language.
- [x] Preserve server-owned evidence, exact workflow states, accessible labels,
  and the explicit boundary between wallet approval and finalized settlement.
- [x] Add component, real-browser, and executable language-contract coverage;
  complete manual operator, localization, and assistive-technology review
  before making production UX claims.

### ADR-0313 — compact protection context

- [x] Remove the repeated full-width shell security strip and keep one
  page-context access summary as the primary location for protection context.
- [x] Preserve the complete assurance copy behind a closed, keyboard-operable
  disclosure without changing server authorization or custody semantics.
- [x] Add component, real-browser, language-contract, and accessibility
  coverage; manual operator and assistive-technology review remain release
  gates.

### ADR-0314 — non-custodial wallet source contract

- [x] Add a scoped, failing-first source contract for browser and API request
  boundaries that rejects seed phrases, mnemonics, private-key fields, and
  client-side keypair construction while allowing public keys and server-owned
  signer identifiers.
- [x] Run the contract in `make check` and retain the existing Wallet Standard,
  signed-message, and exact-artifact tests as the runtime defense in depth.
- [ ] Complete live wallet-provider matrix, Linux validator, independent
  security, and production custody review before enabling production movement.

### ADR-0315 — executable tax-responsibility language boundary

- [x] Add failing-first checks for automated withholding/evidence language and
  explicit calculated, attested, settlement, and legal-responsibility states.
- [x] Reject positive UI claims that imply guaranteed compliance, completed
  filing, or legal compliance; keep the check scoped to production UI source.
- [ ] Complete counsel, merchant-terms, jurisdiction-owner, localization, and
  assistive-technology review before production language claims.

### ADR-0316 — versioned rule-release provenance readiness

- [x] Add the failing-first storage and migration tests for the exact
  `tax-rule-provenance/v1` publication envelope.
- [x] Enforce source reference/digest, legal-tax owner, review record, and
  jurisdiction binding at both Rust storage and PostgreSQL publication
  boundaries; keep incomplete drafts authorable.
- [x] Update the governance workbench with structured release-evidence fields,
  local integration fixtures, executable repository contracts, and the public
  evidence matrix; legal/tax owner approval and source verification remain
  external release gates.

## ADR-0284 — zero-copy tax-fact normalization

- [x] Return borrowed validated facts for the common no-FX calculation path and
  allocate normalized facts only when server-owned FX evidence is present.
- [x] Add failing-first ownership invariant tests for no-FX and FX paths.
- [x] Re-run the full tax-engine suite and Criterion hot-path benchmark;
  current native arm64 evidence is retained in ADR-0075 and remains separate
  from Linux/amd64 service and regional SLO evidence.

## ADR-0153 — On-chain adjustment evidence boundary

The adjustment slice is a separate bounded `record_adjustment` Solana
instruction and an active merchant/mint-scoped authority registry. It records an approved signed-delta adjustment in a deterministic
PDA without token CPI or mutation of the original settlement receipt. The
compiler, canonical payload, account constraints, cross-language vector, and
Linux validator evidence are required before any API state can say chain
evidence was recorded. The Docker conformance contract includes success,
duplicate, wrong-receipt, non-conserving, and no-token-movement assertions;
Linux validator execution remains open. A separate ADR is required before token movement: the
funding signer, beneficiary, tax-vault authority, legal evidence, and
Token-2022 compatibility cannot be inferred from a negative adjustment.

## ADR-0164 — canonical settlement-receipt identity projection

Enrich the finalized generic receipt projection with the immutable receipt
PDA, payer, merchant, mint, nonce, and policy digest decoded from raw
`SettlementExecuted`/`SplitSettlementExecuted` events. Use the additive
PostgreSQL migration and raw-evidence rebuild to backfill legacy rows; reject
incomplete rows at authority-sensitive boundaries. This closes the unsafe
caller-supplied receipt-account gap required before exposing any adjustment
evidence compiler endpoint. Projection rebuild, migration, and Linux
validator evidence remain release gates.

## ADR-0165 — server-owned settlement-intent receipt binding

Resolve `original_settlement_id` only by joining the authenticated tenant's
finalized settlement intent to the complete `indexed_receipts` projection on
signature, finalized slot, and asset mint. Collapse identical multi-stream
duplicates and reject missing, incomplete, stale, or conflicting evidence.
The lookup is the required server boundary before exposing an adjustment
evidence compiler; it does not authorize token movement or a posted state.

## ADR-0166 — wallet-bound adjustment authorities and durable compiler intents

Bind authenticated proposer and approver subjects to Solana public keys only
through one-time Ed25519 wallet proofs over tenant/role/expiry-bound challenge
messages. Persist a server-generated adjustment-evidence intent before
compilation so retries reuse one nonce, one receipt identity snapshot, and one
canonical request digest. The compiler remains unsigned and unsubmitted until
on-chain adjustment-registry verification and validator-backed finality gates
are complete.

## ADR-0170 — wallet-bound adjustment submission and finalized recording

- [x] Add a separate mutable chain-intent lifecycle for wallet submission;
  keep the compiler/evidence intent immutable.
- [x] Verify canonical signed `record_adjustment` bytes, exact account metas,
  authority, and server-bound instruction digest before submission.
- [x] Project `SettlementAdjustmentRecorded` from finalized raw event evidence
  and advance only after an exact intent/event match. Submission acknowledgement
  must never imply recording, payment, filing, or legal compliance.

## ADR-0171 — wallet-bound adjustment browser workflow

- [x] Drive server-owned adjustment compilation from the operator workspace
  without constructing financial values in the browser.
- [x] Reuse the shared injected-wallet boundary and require the connected
  approver key to match the compiled signer account before the wallet prompt.
- [x] Submit unchanged signed bytes through the durable chain intent and make
  timeout, wallet rejection, unknown, provider disagreement, and indexer-lag
  states explicit and accessible.
- [x] Allow `finalized` to render only after the API confirms the exact
  finalized typed event; never imply payment, filing, or legal compliance.
  ADR-0184 adds the persisted event-index/evidence-digest binding, typed API
  proof envelope, strict browser parser, and proof-backed UI state. Live-wallet,
  live-provider, and production accessibility evidence remain release gates.

## ADR-0172 — exact decimal wire encoding for Solana `u64` values

- [x] Serialize adjustment nonces, registry slots, and refund claim nonces as
  canonical decimal strings at API/browser boundaries.
- [x] Reject numeric, non-canonical, and out-of-range counters in the browser;
  preserve Rust `u64` semantics and legacy vector deserialization.
- [x] Add cross-language serialization tests and update the API contract before
  enabling high-range wallet-bound protocol evidence.

## ADR-0173 — server-owned refund-claim wallet orchestration

Design the refund movement bounded context before implementation. Add an
explicit merchant/mint funding configuration, durable claim and execute
intents, server-owned finalized topology verification, shared non-custodial
wallet orchestration, and transaction-bound proof gating. The browser must not
provide financial values or token-account topology, and no state may say paid
from a wallet signature, provider acknowledgement, or indexed event alone.
- [x] Add the tenant-scoped versioned funding-configuration schema, UUIDv7
  identity, idempotent create/read operations, four-eyes approval, bounded
  suspension/retirement transitions, append-only events, and API scope/role
  boundaries. The configuration is evidence only and cannot become active
  without finalized Solana verification.
- [x] Add durable claim/execute intents, exact signed-message verification,
  claim-account topology verification, shared non-custodial wallet
  orchestration, and proof-gated lifecycle transitions. Finalized claim-event
  reconciliation and transaction-bound execute balance proof are implemented;
  operational release gates remain below.
- [x] Complete bounded concurrent idempotency/submission evidence through the
  authenticated Docker API and synthetic wallet transport drill.
- [ ] Complete Linux validator conformance, production load/SLO evidence, and
  production enablement.

## ADR-0174 — finalized refund-funding topology verification

Add the server-owned finalized account-set verifier required before an
approved funding configuration can become active. Derive the vault-authority
PDA, verify the merchant, mint, tax-vault, net-source, and refund-funder
accounts at one finalized slot, compare complete provider attempts, and bind
the resulting digest to immutable activation evidence. The first activation
contract is legacy SPL Token only; Token-2022 requires a separate compatibility
decision and validator evidence.
- [x] Add the bounded provider/parser port and failing-first topology vectors.
- [x] Add immutable verification evidence and the approval-to-active state
  machine; browser status/evidence must never activate a configuration.
- [x] Add API/operator activation flow, metrics, UI failure states, and
  provider-disagreement/restart tests.
- [ ] Add Linux validator topology-rotation and claim-admission conformance.

## ADR-0181 — lease-fenced refund-funding topology verification

The finalized topology verifier now uses a short PostgreSQL UUIDv7 lease for
cross-replica ownership. It commits the claim before RPC work, skips unexpired
leases, reclaims stale pending rows after 60 seconds, releases only the
matching token on provider failure, and requires the exact token for activation.
Migration `0047` and the schema verification contract are implemented; the
concurrent verifier Docker drill, production lease telemetry, and Linux
validator evidence remain release gates.
- [x] Record alternatives, invariants, TTL, stale-worker fencing, and retry
  policy in ADR-0181.
- [x] Add UUIDv7 lease columns, pair constraint, eligibility index, claim/release
  storage operations, and token-fenced activation.
- [x] Add two-replica verifier contention/crash evidence with
  `make refund-funding-verifier-drill`: two replicas converge to one finalized
  read and activation, and a killed claimant is reclaimed exactly once after
  the bounded lease is aged past its TTL. This is Docker/synthetic evidence;
  Ubuntu CI retains its fixed-cardinality `result.env` artifact; Linux
  validator and production RPC evidence remain separate gates.
- [x] Add fixed-cardinality backlog, active-lease, and expired-lease metrics
  with bounded Prometheus alerts; retained scrape/delivery evidence remains a
  production gate.

## ADR-0175 — durable refund claim and execute intents

Create the Refund Movement bounded context with separate server-owned claim and
execute intents. Load finalized receipt evidence and active verified funding
configuration server-side, bind an explicit refund-funder subject to the
configured key, allocate claim nonces transactionally, persist unsigned wallet
artifacts before signing, and verify exact signed transaction bytes before any
RPC submission. Only finalized transaction-bound event and balance proof may
advance a movement to a successful state.

- [x] Record the authority, schema, state-machine, alternative-analysis, and
  failure-policy decision in ADR-0175.
- [x] Add the pure `refund-workflow` domain boundary for server-owned payer
  destination derivation, claim/execute compilation, conservation checks, and
  tenant/configuration/receipt-bound artifact digests.
- [x] Add the `refund_claim_intents`, `refund_execute_intents`, nonce allocator,
  append-only lifecycle evidence, and outbox schema. Claim and execute
  persistence, UUIDv7 allocation, and first outbox publication are implemented.
- [x] Add the server-owned claim artifact builder and preparation API, exact
  signed-message verification, bounded submission/reconciliation, and execute
  intent API contracts.
- [x] Add ADR-0186 bounded storage-local retries for PostgreSQL serialization
  and deadlock conflicts during refund nonce, claim-intent, and execute-intent
  transactions; preserve the same digest and idempotency key across attempts.
- [x] Add the authenticated Docker refund-execute API contention drill: 16
  concurrent same-key callers converge to one durable intent, lifecycle event,
  outbox publication, and response digest; fix replay authority validation so
  idempotent replays bind both the compiled approver key and authenticated
  adjustment approver subject.
- [x] Record the replay identity-domain correction and bounded authorization
  lookup in ADR-0187.
- [x] Add ADR-0188 bounded p50/p95/p99 latency evidence to the authenticated
  execute-preparation contention drill; retain production and regional SLO
  gates as separate evidence requirements.
- [x] Extend the same Docker drill through the real signed-submission boundary
  with synthetic test-only wallet keypairs: concurrent claim and execute
  submissions each produce one fixture RPC send, persist a `submitted` intent,
  and reject the competing replay without changing signed bytes. Finalized
  chain evidence and live-wallet/provider coverage remain separate gates.
- [x] Add ADR-0189 and `make indexer-replay-drill`: two real indexer service
  replicas automatically claim and complete one persisted replay gap through
  PostgreSQL and the bounded RPC fixture, while the drill proves the forward
  checkpoint is unchanged and fixed-cardinality replay metrics are exported.
- [x] Add ADR-0190 and `make indexer-replay-dead-letter-drill`: two real
  replicas exhaust exactly eight deterministic provider-failure attempts,
  release the lease, persist bounded dead-letter state, and leave the forward
  checkpoint unchanged.
- [x] Add ADR-0191, pinned Alertmanager configuration, and
  `make alert-delivery-drill`: an isolated worker scrape outage fires the real
  Prometheus availability rule and proves delivery through Alertmanager to a
  bounded Rust receiver fixture; production authentication, escalation, and
  on-call response remain separate gates.
- [x] Add ADR-0192 and extend the real-browser SPA journey with server-shaped
  vault-reconciliation mismatch, history, and unauthorized fail-closed
  evidence; live-provider, wallet, validator, and production identity gates
  remain separate.
- [x] Add ADR-0193 with a server-owned 90-second database-clock freshness
  contract for reconciliation reads, including exact API/parser semantics and
  stale stop-state evidence; production scheduler SLO and live-provider
  timing remain separate gates.
- [x] Add ADR-0194 and extend `make integration-check` through the real
  wallet-byte adjustment boundary: exact single-instruction serialization,
  concurrent [200,409] idempotency, one fixture RPC send, explicit finalized
  observation, typed event/raw evidence projection, and proof-envelope replay.
  Live wallet/provider, Linux validator, load/SLO, and production accessibility
  evidence remain separate gates.
- [x] Add ADR-0195 and `make refund-proof-integration-drill`: the real Docker
  indexer resolves the synthetic finalized claim PDA, parses the exact
  transaction-bound refund event and pre/post token balances, persists one
  immutable balance proof, finalizes the execute intent/outbox once, and
  rejects proof mutation and replay duplication. Live RPC-provider, validator,
  production load/SLO, and paid-state enablement remain separate gates.
- [x] Add ADR-0196's client quote-expiry boundary: the checkout refreshes the
  server-declared expiry observation once per second, prevents compile/sign
  actions at the exact expiry second, keeps API expiry validation as the
  authority, and has a 10/10 pinned NGINX browser regression for the rendered
  disabled state. Live-provider evidence remains a separate gate.
- [x] Add ADR-0197's strict browser tax-evidence response parser. Calculation
  and replay payloads now validate UUIDv7 identity, bounded complete component
  traces, fixed digests, expiry, statuses, and signed i128 decimal amounts
  before entering React state; malformed responses fail closed.
- [x] Add ADR-0198's strict vault-release lifecycle response contract.
  Vault-release `u64` amounts, nonces, and slots serialize as canonical decimal
  strings; the SPA validates claim/transition evidence and refreshes the
  server-owned claim after every successful transition instead of merging form
  values into financial state.
- [ ] Add wallet workspace evidence against a live injected wallet, Linux
  validator conformance, load/SLO evidence, and production enablement gates.

## ADR-0200 — Solana Wallet Standard signing boundary

- [x] Add Wallet Standard discovery and explicit multi-provider selection while
  retaining the legacy injected-wallet fallback only when no standard wallet
  is registered.
- [x] Bind the selected standard account and returned signed bytes to the
  existing server-compiled transaction and durable intent contract.
- [x] Add an accessible account selector for providers exposing multiple
  signing accounts; refuse implicit first-account selection.
- [x] Render the selected public key in claim and execute wallet workspaces so
  operators can compare the active signer with the immutable artifact.
- [x] Add failing-first capability, rejection, mismatch, browser, and
  accessibility tests before enabling the adapter in production flows. The
  pinned browser journey now also covers synthetic Wallet Standard registration,
  explicit multi-account selection, and reload behavior.
- [ ] Add live Wallet Standard provider/account matrix evidence, Linux
  validator/provider conformance, load/SLO evidence, and production
  enablement gates.

## ADR-0201 — bounded OTLP trace-export boundary

- [x] Add the shared observability bootstrap with opt-in OTLP gRPC traces,
  bounded queue/batch/timeout configuration, service-name allow-listing, and
  TLS/insecure endpoint policy.
- [x] Replace duplicated Rust binary tracing initialization with the shared
  bootstrap and add bounded graceful shutdown flush handling.
- [x] Add failing-first configuration tests and a Docker collector receipt
  drill that proves request correlation without financial payload leakage.
- [x] Run the collector receipt drill in CI and retain its evidence in the
  release-candidate bundle.
- [ ] Keep production collector authentication, TLS, retention, sampling,
  dashboards, alert routing, and on-call evidence as release gates.

## ADR-0202 — explicit tax rounding scope and residual allocation

- [x] Add backward-compatible `Component` and explicit `Settlement` rounding
  scope to the versioned Rust DSL.
- [x] Reject mixed scopes, dependent settlement-scope graphs, and mixed
  settlement rounding modes before compilation.
- [x] Add deterministic largest-remainder allocation, canonical-ID ties,
  golden vectors, replay/digest coverage, workbench controls, and exact
  positive atomic-quantum truncation/rounding.
- [x] Add the separately versioned line/invoice facts contract, bounded
  document digest, explicit scope separation, deterministic allocation,
  replay/property/golden coverage, and a simulation-only document endpoint;
  approved jurisdiction coverage and production authorization remain separate
  release gates.

## ADR-0203 — versioned line and invoice tax facts

- [x] Add bounded `TaxDocumentFacts`/`TaxLineFacts` schemas and canonical
  document digest domain.
- [x] Add explicit `Line`/`Invoice` rounding scopes with deterministic
  quantum-sized residual allocation and aggregate/document API separation.
- [x] Add Rust tests, property tests, golden vectors, replay coverage, and
  benchmark evidence for bounded document calculations.
- [x] Add separately authorized API/storage/UI adapters after the pure domain
  contract was verified through ADR-0204–ADR-0208; aggregate `TaxFacts` is not
  reinterpreted as a document. Production identity, legal ownership, and
  finalized live-chain evidence remain separate release gates.

## ADR-0204 — authoritative document quote boundary

- [x] Add the dedicated PostgreSQL 18 document-calculation schema with
  encrypted facts evidence, immutable result storage, status constraints, and
  indexes scoped by merchant/calculation/document digest.
- [x] Add the Rust storage adapter with published-rule/provider resolution,
  UUIDv7 idempotency, transactional outbox creation, encrypted evidence, and
  exact replay verification.
- [x] Add authenticated document quote/replay routes; document-specific
  attestation binding remains a separate gate, and aggregate settlement routes
  remain aggregate-only until a separate payment-mapping ADR exists.
- [x] Add document-specific attestation binding and explicit cross-context
  rejection tests for aggregate attestation/settlement endpoints.
- [x] Add Docker two-replica contention, tenant isolation, tamper/replay,
  migration, and operational evidence before enabling production document
  quotes.

## ADR-0205 — dedicated document-calculation attestation

- [x] Record a separate signed contract and persistence/route decision; do not
  reuse aggregate calculation attestation bytes.
- [x] Implement the policy contract, PostgreSQL table, authenticated route,
  encrypted/replay-safe persistence, and document-specific outbox event.
- [x] Prove aggregate attestation and settlement paths reject document IDs,
  with PostgreSQL integration, two-replica contention, tenant-isolation,
  replay/conflict, and outbox-publication evidence.

## ADR-0206 — document-to-payment binding

- [x] Record document-payment alternatives, invariants, bounded scope
  semantics, and the partial-line rejection rule before implementation.
- [x] Implement the pure Rust document-payment domain, authorization
  persistence, authenticated compiler, typed wallet-submit verification, and
  browser-side provenance checks.
- [x] Implement and unit-test the additive Solana `settle_document`
  instruction and strict document receipt/event decoding. Real validator
  conformance is now represented by a validator-backed Docker test case;
  successful Linux/amd64 execution remains an explicit release gate.
- [x] Add conformance assertions for document receipt provenance bytes,
  conservation, and duplicate document-receipt rejection; retain the typed
  `io_uring` startup failure when the local runtime cannot execute Agave.

## ADR-0207 — document-bound checkout workspace

- [x] Record the UI authority boundary, alternatives, evidence states, and
  accessibility/performance budgets before implementation.
- [x] Implement the lazy document attestation, document-scope compilation,
  wallet submission, and reconciliation workspace with browser contract tests.
- [x] Add real-browser accessibility and degraded-state evidence for the
  document workflow. The pinned NGINX/Playwright journey now covers document
  quote API unavailability, settlement-route failure, and successful
  evidence-to-reviewed-artifact handoff. Failure states keep signing
  unavailable; the success state enables signing only after server
  attestation verification and durable compilation. All pass WCAG 2.2 A/AA
  axe checks; live provider and finalized-chain evidence remain separate gates.

## ADR-0208 — authenticated document-quote workspace

- [x] Record the production quote versus simulation boundary and evidence
  ownership before implementation.
- [x] Implement authenticated document facts submission, idempotent replay
  rendering, jurisdiction-evidence failure states, and handoff to document
  checkout.
- [x] Add lazy-workspace and WCAG browser contract coverage for the document
  quote surface. Docker integration now covers missing/expired authentication,
  tenant isolation, replay, and idempotency conflict failure journeys.
- [x] Prove full-document and whole-line subset compilation, replay/conflict,
  tenant isolation, aggregate/document separation, and durable authorization
  evidence in Docker/PostgreSQL integration. Finalized live-chain evidence
  remains open pending the validator conformance gate.

## ADR-0209 — bounded document-quote latency evidence

- [x] Record the document-specific workload, alternatives, safety invariants,
  and explicit distinction from aggregate quote and production SLO evidence.
- [x] Add an explicit `BENCHMARK_MODE=document` path to the existing bounded
  Docker quote harness, using the two-line document rule, document endpoint,
  document calculation table, and document outbox aggregate while sharing the
  credential, percentile, timeout, replica, and cleanup logic.
- [x] Add `make benchmark-document-quote-service` and a dedicated CI evidence
  artifact for the document workload.
- [ ] Retain a successful CI run and production-like regional/resource-profile
  measurements before using document latency as a production SLO claim.

## ADR-0215 — bounded load-test resource-profile evidence

- [x] Record the bounded sampling scope, alternatives, privacy constraints, and
  diagnostic-only release boundary in ADR-0215.
- [x] Add bounded Docker JSON resource sampling to the aggregate/document load
  harness and retain the profile beside CI latency evidence.
- [x] Re-run the aggregate and document workloads locally with retained
  profiles; both preserved the declared latency, contention, and durable
  invariants. Production-like regional/resource-budget review remains open.
- [ ] Review retained CI and production-like regional CPU/memory/runtime
  evidence against the declared budgets before accepting production SLOs.

## ADR-0216 — server-owned mint precision at the settlement-route boundary

- [x] Persist finalized verifier-derived `mint_decimals` on settlement routes
  through migration `0053`, enforce the `0..=9` bound, and prevent mutation on
  active, suspended, or retired route versions.
- [x] Expose and strictly validate route precision in the authenticated API and
  browser checkout; amount presentation uses `BigInt` scaling and never
  converts atomic values through JavaScript `number` arithmetic.
- [x] Add zero-, six-, and nine-decimal formatter vectors plus missing,
  fractional, and out-of-range route rejection tests.
- [ ] Retain Docker/PostgreSQL migration evidence and Linux validator-backed
  route activation evidence before treating precision support as a production
  release gate.

## ADR-0217 — retained Docker integration evidence

- [x] Record the public-safe evidence boundary, retention period, alternatives,
  and failure semantics in ADR-0217.
- [x] Retain the authoritative Docker integration stream for successful and
  failed CI runs with `pipefail` and bounded artifact retention.
- [x] Fail the CI evidence-upload step when the expected capture directory is
  absent, keeping missing evidence distinct from a retained failed run.
- [ ] Review retained Linux CI artifacts for the exact release commit before
  using synthetic integration evidence in a release decision.

## ADR-0218 — explicit release integrity hashes

- [x] Require and copy the resolved Compose configuration into the release
  bundle and include it in the common checksum set.
- [x] Add named machine-readable SHA-256 fields for the final Solana program
  binary and effective configuration, with a failing-first contract test.
- [ ] Retain a successful Linux release-candidate artifact and independently
  review its program/configuration hashes before production enablement.

### ADR-0299 — release evidence source-revision binding

- [x] Require validator-conformance metadata to name the exact Git revision
  packaged by the release bundle.
- [x] Reject mismatched evidence before copying or reporting a complete bundle,
  with a negative integrity-contract test.
- [ ] Retain and independently review a successful Linux release-candidate
  bundle for the exact release commit.

### ADR-0303 — exact-commit release browser evidence

- [x] Run the pinned NGINX/Playwright browser journeys on the exact release
  candidate revision before bundle creation.
- [x] Copy the result and a full Git revision binding into the release evidence
  input, and reject missing or mismatched browser evidence.
- [x] Include the browser result in the release artifact checksum set and
  machine-readable manifest.
- [ ] Retain and independently review a successful Linux release-candidate
  browser artifact; automated synthetic journeys do not replace live-wallet,
  assistive-technology, legal/tax, or production SLO evidence.

### ADR-0304 — Material role contrast contract

- [x] Add a dependency-free WCAG contrast verifier for the dark and light
  Material role pairs used by the product shell.
- [x] Run the verifier through `make material-palette-contract`, `make check`,
  and the web CI job before UI tests and build.
- [ ] Complete manual assistive-technology and operator usability review for
  the full production tenant surface; the static and automated gates do not
  constitute that external evidence.

### ADR-0305 — one-command Tax Engine local gate orchestration

- [x] Add `make tax-engine-local-check` with ordered Rust/frontend/contracts,
  authenticated Docker integration, real-browser, and validator-backed gates.
- [x] Add a failing-first Makefile contract that prevents command omission or
  reordering and preserves fail-fast behavior.
- [ ] Execute the complete gate on the retained Linux/amd64 runner and review
  its artifacts; a Mac Docker capability failure must remain an explicit
  conformance blocker rather than a skipped result.

### ADR-0306 — collapsed advanced evidence disclosures

- [x] Collapse provider-signed location evidence by default in aggregate and
  document quote workspaces while keeping the required input available through
  native accessible disclosure.
- [x] Add component and real-browser coverage proving the disclosure is closed
  initially and can be opened before submission.
- [ ] Complete manual operator usability review across supported tenant roles,
  mobile widths, and assistive technologies.

### ADR-0307 — project-scoped local reset

- [x] Add an explicit `make reset-local` command that removes only the
  validated Compose project's synthetic volumes and orphans.
- [x] Add a shell contract proving project-name validation, warning text,
  exact Compose scoping, and the absence of broad deletion commands.
- [x] Verify fresh PostgreSQL 18 migration and API/NGINX startup from a clean
  scoped Compose project on the current Docker runtime. Linux/amd64 full
  topology and validator evidence remain separate release gates.

### ADR-0308 — exact-commit authenticated integration evidence

- [x] Run the authenticated Docker integration gate in the release-candidate
  workflow after repository checks and before release bundle creation.
- [x] Bind the captured integration stream to the packaged Git revision and
  reject missing or mismatched evidence in the bundle creator.
- [x] Include the integration evidence in the common checksum set and release
  manifest, with a separate failure-evidence upload path.
- [ ] Retain and independently review a successful Linux release-candidate
  integration artifact; synthetic Docker evidence does not replace staging,
  live-provider, legal/tax, or regional production SLO evidence.

## ADR-0219 — JWKS JSON deployment-preflight parity

- [x] Align the staging/production preflight with the API's supported
  structured `API_AUTH_JWKS_JSON` or compatibility-key-pair contract.
- [x] Reject mixed, empty, malformed-shape, and known local-development key
  representations without printing secret material.
- [ ] Exercise provider-managed key rotation and HSM/KMS-backed identity
  provisioning in a controlled staging environment.

## ADR-0220 — jurisdiction-provider JSON deployment-preflight parity

- [x] Align the staging/production preflight with the API's supported
  structured `API_JURISDICTION_PROVIDER_KEYS_JSON` or compatibility-key-pair
  contract.
- [x] Reject mixed, empty, malformed-shape, and known local-development
  provider-key representations without printing key material.
- [ ] Exercise live provider-key rotation, provider signature verification, and
  HSM/KMS-backed jurisdiction evidence provisioning in controlled staging.

## ADR-0210 — Docker full-topology bootstrap

- [x] Record the explicit-service selection, Compose health-wait contract,
  one-shot exclusion, and fail-closed capability policy.
- [x] Add `make bootstrap` and a dry-run contract test that selects the full
  long-running local topology without starting browser or conformance jobs.
- [ ] Execute the full bootstrap successfully on a Linux/amd64 Docker runtime;
  the current macOS Docker runtime remains blocked by Agave's `io_uring`
  requirement and this failure must remain visible.

## ADR-0211 — Explicit Docker host-port overrides

- [x] Record the collision model, stable-default policy, and rejection of
  random host-port allocation.
- [x] Make validator RPC/faucet bindings environment-overridable, preserving
  fixed internal service ports and fail-closed Compose readiness.
- [x] Add a bootstrap contract assertion and document parallel-topology
  operation.
- [ ] Capture a successful full-topology run on a Linux/amd64 runtime with a
  recorded non-default port map where parallel stacks are required.

## ADR-0177 — exact signed refund-claim submission

Connect the existing bounded signed-transaction verifier to the durable claim
aggregate. Submission accepts only an intent ID and canonical signed bytes;
the API loads the immutable artifact, fences the lifecycle, submits unchanged
bytes, and records ambiguous provider outcomes as `unknown`.

- [x] Record the exact-verification, recovery, and ambiguity policy in
  ADR-0177.
- [x] Add claim recovery and signed-submission routes with durable CAS
  transitions.
- [x] Add RPC-fixture integration and contention evidence; the production worker
  now performs bounded automatic convergence, and the browser workspace tests
  cover role-separated signing and transport-state rendering.
- [x] Add the typed server-derived finalized claim-event response, strict
  browser parser, progressive-disclosure proof display, and browser contract
  coverage. Live injected-wallet, reload, and production-provider evidence
  remain separate release gates.

## ADR-0178 — two-signer refund-claim wallet workspace

Add a dedicated lazy React workspace for server-owned claim review, sequential
proposer/refund-funder wallet signing, explicit recovery input, and transport
versus finalized-proof state semantics.

- [x] Record the wallet-boundary alternatives, invariants, and recovery policy
  in ADR-0178.
- [x] Add strict browser parsing, two-role signing, submission gating, and
  accessible explicit failure states.
- [x] Add synthetic injected-wallet multi-account role-signing tests for claim
  preparation, exact submission, and proof-pending rendering.
- [x] Add synthetic injected-wallet multi-account finalized-proof rendering;
  the browser action and indexer worker both use the shared proof-driven
  finality boundary.
- [x] Add a pinned NGINX/Playwright journey for server-bound finalized claim
  proof disclosure, axe validation, reload, and durable intent recovery. The
  journey uses synthetic API evidence and does not claim live wallet or chain
  finality.
- [ ] Add real injected-wallet multi-account, reload, RPC-fixture, and live
  finalized-proof browser evidence.

## ADR-0179 — durable, proof-gated refund-execute intent

Add the second server-owned refund movement workflow after a finalized claim.
The execute bounded context must load finalized claim evidence and active
verified topology, compile the fixed fourteen-account execute instruction,
persist it before wallet signing, verify exact approver/refund-funder signed
bytes, and reconcile only from finalized transaction-bound event and balance
proof.

- [x] Record the execute-intent authority boundary, alternatives, invariants,
  API shape, and proof-gated release policy in ADR-0179.
- [x] Add failing-first execute-intent storage/domain tests, idempotent
  persistence, append-only outbox publication, and lifecycle CAS transitions.
- [x] Add prepare/recover/submit API routes with exact signed-message and
  recent-blockhash verification.
- [x] Add approver/refund-funder wallet workspace and explicit proof-pending,
  unknown, and finalized states.
- [x] Connect immutable transaction-bound balance-proof persistence to the
  execute-intent finalized transition and outbox evidence; align SQL unknown
  recovery triggers in migration `0045`.
- [x] Add server-owned claim-finality reconciliation from bounded finalized
  recovery and exact `AdjustmentRefundClaimCreated` event evidence; unlock
  execute preparation only after the CAS transition to `finalized`, and run
  the same bounded reconciliation automatically from the indexer worker.
- [x] Add migration `0046` short UUIDv7 reconciliation leases with
  `SKIP LOCKED`, stale-lease reclaim, lease-fenced finalization, and fixed
  backlog telemetry; no database lock is held across RPC recovery.
- [x] Add concurrent idempotency/submission tests through the authenticated API,
  RPC fixture, and synthetic two-wallet browser boundary.
- [ ] Add validator conformance, end-to-end finalized reconciliation fixtures,
  load evidence, and production enablement gates.

## ADR-0176 — server-owned refund claim preparation API

Expose only a server-owned, tenant-scoped claim-preparation request containing
an approved adjustment ID, idempotency key, and explicit confirmation. The API
loads approved adjustment, finalized receipt, active verified funding, and
wallet bindings; derives all financial and account inputs; allocates the claim
nonce; and persists the unsigned artifact idempotently before any wallet prompt.

- [x] Record the request contract, alternatives, invariants, and release
  boundary in ADR-0176.
- [x] Add server-side loading and claim artifact preparation endpoint; the
  endpoint stops at `prepared_unsubmitted` and does not submit or sign.
- [x] Add API contract tests, concurrent idempotency tests, and accessible
  wallet-review states.

Migration `0043` hardens the intent mutation guards and requires a funder
subject for reviewed funding configurations; it is a prerequisite for the
storage implementation rather than evidence that movement is enabled.

Migration `0044` adds the explicit `refund_funder` wallet-proof role and
funding activation verifies that the active binding matches the configured
funder public key exactly.

## ADR-0167 — finalized server-owned adjustment-registry verification

Add a bounded finalized `getAccountInfo` verifier to the settlement RPC
coordinator. It derives the adjustment-registry PDA from the fixed program and
server-owned merchant/mint, validates owner, discriminator, exact layout,
active state, bump, and both wallet-bound authority keys, and fails closed on
successful-provider disagreement. Persist the registry address, finalized
slot, commitment, and account digest in the immutable compiler intent so a
registry rotation cannot change a retry. Only after parser/coordinator/API
tests and validator-backed registry rotation evidence pass may non-simulation
adjustment compilation be enabled.

## ADR-0168/0169 — reproducible conformance and SBF stack safety

The validator conformance client runs inside a pinned non-root Docker image,
and the CI validator job no longer bootstraps host Node.js or pnpm, so
`make solana-conformance` has no host JavaScript-toolchain dependency. The
adjustment-refund Anchor contexts box their large account values, and the
Docker/local SBF build wrapper fails closed on compiler stack-frame safety
diagnostics. The program artifact now builds without the previously observed
4 KiB frame overflows. Validator health probes are bounded and classify an
early Agave exit in retained `result.env` evidence, including the explicit
`validator_io_uring_unsupported` capability reason; Linux validator execution
remains an independent release gate.

## ADR-0199 — authoritative Linux validator evidence

Add a postcondition verifier for the validator evidence bundle. The Ubuntu CI
and release-candidate workflows assert the Linux/amd64 Docker runtime and
verify the versioned result, pinned Agave log, exact SBF hash, container
identity, and all named real-RPC test cases. Local `io_uring` failures remain
fail-closed diagnostics; only retained successful Linux evidence can close the
validator release gate.

## ADR-0241 — executable Linux conformance workflow contract

- [x] Add a static contract over CI and release-candidate workflows requiring
  the pinned Ubuntu runner, Linux/amd64 Docker assertions, the Dockerized
  conformance command, postcondition verification, and fail-closed retained
  evidence handling.
- [x] Run the contract from normal repository checks and the public CI job.
- [ ] Retain a successful Linux/amd64 validator artifact for the exact release
  commit; the workflow contract protects the gate but cannot create validator,
  provider, identity, or production-operational evidence.

## ADR-0376 — Linux/amd64 conformance host preflight

- [x] Add a shared fail-closed host preflight for local, CI, and release
  conformance execution; retain a versioned host record with OS, architecture,
  Docker engine, and failure reason.
- [x] Require the preflight before SBF build/validator startup and require it
  again in the retained evidence verifier; preserve validator startup as the
  authority for kernel capabilities such as `io_uring`.
- [x] Add workflow-order and public-repository contracts, with actionable
  local ARM diagnostics.
- [ ] Retain successful Linux/amd64 validator evidence for the exact release
  revision; preflight makes the gate explicit but cannot manufacture external
  runner evidence.

## ADR-0377 — Dedicated Solana conformance evidence workflow

- [x] Add a manually or tag-triggered dedicated workflow with pinned runner,
  actions, Dockerized SBF build, validator suite, host preflight, evidence
  verifier, and fail-closed artifact retention.
- [x] Bind artifact naming and retained metadata to the exact source SHA and
  add a public workflow-shape contract.
- [ ] Execute the workflow on native Linux/amd64 and independently review the
  successful artifact before checking the validator release gate.

## ADR-0378 — Non-custodial Solidity receipt mirror

- [x] Add a separate Solidity 0.8.37 package that mirrors finalized Solana
  receipt evidence without custody, tax recomputation, external calls, or
  settlement authority.
- [x] Add exact amount-conservation, issuer/governance, pause, idempotency,
  conflict, two-step governance, and bounded amount-conservation fuzz tests.
- [x] Run the adapter tests through a pinned Docker Foundry image and add the
  package to public documentation and the repository check contract.
- [ ] Complete an independent EVM audit and approve a chain-specific deployment
  before accepting any real receipt data; this adapter is not production
  enabled by code alone.

## ADR-0379 — Bounded tax-base clamps

- [x] Add exact optional floor/cap bounds to the v2 `TaxBase` DSL while
  preserving v1 canonical bytes for unbounded legacy rules.
- [x] Add failing-first golden/property/replay tests for floor, cap, both
  bounds, negative adjusted bases, and boundary equality.
- [x] Add a public tax-coverage row and localized rule-workbench controls only
  after the domain/API contract is stable.
- [ ] Retain tax-owner/legal review and jurisdiction-specific provenance before
  treating any bounded-base regime as production-supported.

See [ADR-0379](../adr/0379-bounded-tax-base-clamps.md).

## ADR-0242 — protocol pause gates vault-release execution

- [x] Add the protocol-config PDA to the versioned vault-release execution
  account set and reject paused execution before any token CPI or claim
  mutation.
- [x] Add a failing-first Rust invariant test and extend the real localnet
  release case to prove paused balances remain unchanged.
- [ ] Retain successful Linux/amd64 validator evidence and staging
  multisig/timelock pause-ceremony evidence before enabling production release
  movement.

## ADR-0243 — executable program pause-coverage contract

- [x] Add an explicit static allowlist contract covering every current
  authority-bearing token-movement instruction, discover future public
  `transfer_checked` entry points, and require the protocol pause boundary in
  CI and normal checks.
- [x] Bind the vault-release pause-PDA account and pause-state validation into
  the same contract.
- [ ] Keep Linux validator, staging governance, and production incident-drill
  evidence as separate release gates.

## ADR-0244 — exact foreign-exchange evidence boundary

- [x] Add typed source/target currency, exact rational rate, provider quote,
  observation, digest, quantum, and rounding evidence to aggregate and
  document facts.
- [x] Normalize gross, net, line, and cumulative amounts exactly once before
  rule evaluation and expose the immutable envelope in calculation results.
- [x] Add failing-first Rust/TypeScript vectors for ties, overflow, malformed
  evidence, replay, and double-conversion prevention.
- [x] Reject caller-supplied FX evidence at simulation and persisted quote API
  boundaries; reserve server-side provider attachment for a separately governed
  integration.
- [ ] Keep provider freshness, legal rate acceptance, and production FX
  availability as separately governed release gates.

## ADR-0245 — server-owned signed FX evidence bounded context

- [x] Add a pure `fx-evidence` crate for canonical signed provider envelopes,
  exact tax-engine mapping, freshness-window validation, and bounded failures.
- [x] Add failing-first vectors for signature/payload tampering, merchant and
  facts substitution, expiry/future observation, unknown keys, malformed
  bounds, and atomic provider-key rotation.
- [x] Keep public quote payloads unable to supply FX authority; attach evidence
  only through a separately authenticated server-owned provider adapter.
- [ ] Keep provider freshness, legal acceptance, HSM/KMS custody, provider
  availability, and Linux/production SLO evidence as release gates.

## ADR-0246 — server-owned FX provider policy and issuance boundary

- [x] Add vendor-neutral source, policy, and HSM/KMS signer ports before
  choosing a concrete FX provider.
- [x] Enforce exact pair, freshness, future-skew, lifetime, legal-acceptance,
  and reference-rate deviation policy with failing-first vectors.
- [x] Issue only server-bound signed FX envelopes and preserve the existing
  no-client-authority API boundary.
- [ ] Approve a concrete provider, regional routing, legal owner, custody
  ceremony, production SLO, and cross-currency enablement evidence.

## ADR-0247 — atomic FX provider policy snapshot reload

- [x] Add a bounded digest-keyed policy registry with atomic replacement and
  lock-free request-path reads.
- [x] Reject malformed, empty, duplicate, oversized, unknown-field, and
  semantically invalid replacements while retaining the last good snapshot.
- [ ] Keep policy publication, legal approval, file provenance, permissions,
  reload timing, and production activation evidence as separate gates.

## ADR-0248 — registry-bound FX evidence issuance

- [x] Require production composition to resolve the exact policy digest from
  the active immutable registry before constructing an FX evidence issuer.
- [x] Provide the same fail-closed exact-digest binding for synchronous and
  asynchronous HSM/KMS-compatible issuer construction, with snapshot-isolation
  coverage.
- [x] Fail closed for missing or zero policy digests before provider or signer
  work, and isolate in-flight issuers from later snapshot replacement.
- [ ] Migrate a concrete production provider adapter and HSM/KMS signer to the
  bound API after provider, legal, custody, and staging decisions are approved.

## ADR-0249 — FX provider identity allowlist

- [x] Add a bounded, unique, exact-match provider allowlist to every FX policy.
- [x] Reject unapproved provider candidates before signing and preserve the
  existing server-owned evidence boundary.
- [ ] Select and exercise concrete providers, regional routing, quorum policy,
  legal ownership, and provider failover evidence.

## ADR-0250 — deterministic FX provider agreement

- [x] Add a policy-defined minimum provider agreement count and bounded pure
  candidate-set selector.
- [x] Require exact economic agreement, distinct approved identities, and
  policy-priority selection; fail closed on disagreement or insufficient data.
- [ ] Exercise bounded concurrent provider acquisition, regional failover,
  provider SLOs, legal ownership, and production agreement evidence.

## ADR-0251 — accelerated Tax Engine delivery and decision freeze

Acceleration changes sequencing and ownership, not the Tax Engine definition of
done. The planning estimate is a 3–4 week production candidate and a 5–7 week
controlled production launch, conditional on external approvals and evidence.

- [x] Create and link the public decision register with explicit open states,
  accountable roles, evidence requirements, and revisit triggers.

- [ ] **Days 1–2 — freeze decisions:** record the initial FX provider and
  backup/allowlist, supported jurisdictions and assets, legal/tax owner,
  staging environment and regions, RPC providers, SLOs, evidence retention,
  rollback authority, and incident commander.
- [ ] **Workstream A — blockchain/conformance:** produce the Linux/amd64
  validator artifact, wallet-signing matrix, legacy SPL and extension-free
  Token-2022 evidence, failover/reconciliation evidence, and finalized
  account/amount-conservation proofs.
- [ ] **Workstream B — platform/security:** provision HSM/KMS controls and key
  ceremony, Linux/amd64 CI, pinned image/dependency/SBOM scans, observability,
  threat-model review, backup/restore, rollback, and incident rehearsals.
- [ ] **Workstream C — provider/tax/legal:** integrate the approved provider
  ports, establish provider SLOs and regional failover, publish the supported
  jurisdiction/asset matrix, obtain rule/legal ownership, and produce signed
  FX/rule provenance evidence.
- [ ] **Week 1 integration readiness:** all three workstreams have passing
  interface contracts, named artifact owners, production-like dashboards, and
  no unresolved authority boundary.
- [ ] **Weeks 2–3 evidence sprint:** run wallet, validator, Token-2022,
  provider failover, load, reconciliation, and regional latency tests in
  parallel. Record workload, region, RPC, commitment, resource profile, and
  p50/p95/p99 results.
- [ ] **Weeks 3–4 readiness:** close independent program/security review,
  threat-model findings, backup/restore, rollback, and incident-rehearsal
  evidence. Critical/high findings block the candidate unless an explicit,
  time-bound exception is approved.
- [ ] **Weeks 5–6 canary and launch review:** run the staging canary, review the
  complete evidence packet, obtain provider/tax/legal approval, verify signed
  release artifacts/SBOM/provenance, and record a staffed go/no-go decision.
- [ ] **Controlled-production gates:** exact Linux validator evidence, live
  wallet and Token-2022 matrix, measured regional SLOs, HSM/KMS rotation and
  recovery, failover/reconciliation, independent audit disposition,
  restore/rollback/incident evidence, and public-safe documentation are all
  green. No schedule target can waive a gate.

The authoritative decision record is
[ADR-0251](../adr/0251-accelerated-tax-engine-delivery-and-decision-freeze.md).
The decision register must be updated whenever a frozen choice changes.

## ADR-0252 — bounded concurrent FX provider acquisition

- [x] Add the object-safe async source port with server-owned provider identity.
- [x] Reject invalid requests, duplicate/unapproved sources, identity
  substitution, and unbounded topology before any provider call.
- [x] Implement bounded Tokio fan-out with maximum concurrency, per-provider
  deadlines, aggregate cancellation, and bounded failure classifications.
- [x] Prove maximum in-flight calls, timeout/cancellation behavior, partial
  failure, and deterministic handoff to ADR-0250 admission with tests.
- [x] Normalize candidates and failure classifications by policy priority so
  response completion order cannot change replayable acquisition evidence.
- [ ] Integrate approved regional adapters only after provider selection,
  legal ownership, HSM/KMS, and production SLO evidence are approved.

## ADR-0253 — bounded async FX signing boundary

- [x] Add the object-safe async HSM/KMS-compatible signer port and bounded
  signing-timeout configuration.
- [x] Share canonical payload preparation and final envelope verification with
  the synchronous local path; never accept unverified signer bytes.
- [x] Add tests for successful async issuance, timeout, signer failure,
  malformed signature, public-key mismatch, and rejection before signer use.
- [ ] Integrate an approved HSM/KMS adapter only after key ceremony,
  rotation/revocation, recovery, regional latency, and custody evidence.

## ADR-0254 — deterministic FX acquisition outcome order

- [x] Normalize concurrent provider candidates and bounded failure outcomes by
  the immutable policy provider priority.
- [x] Add a reversed-completion test proving stable candidate and failure order.
- [ ] Preserve the ordering contract in the approved provider adapter's
  production evidence and any future canonical acquisition serialization.

## ADR-0255 — bounded HTTP FX provider adapter

- [x] Implement the vendor-neutral bounded HTTP source with strict endpoint,
  credential, timeout, request, response, and body-size controls.
- [x] Prove transport failure, identity binding, exact decimal parsing,
  evidence hashing, cancellation, and credential-redaction behavior with local
  fixture tests.
- [ ] Keep provider selection, jurisdiction/assets, legal owner, custody,
  regional SLO, and production enablement in the decision register.
- [ ] Enable a concrete provider only after independent security, legal,
  HSM/KMS, failover, and regional latency evidence is approved.

## ADR-0256 — bounded HTTP FX signer adapter

- [x] Implement the vendor-neutral bounded HTTPS signer with exact payload,
  configured key identity, strict signature decoding, and no hidden retry.
- [x] Prove endpoint policy, payload binding, body limits, credential
  redaction, redirect rejection, timeout, cancellation, and no-retry behavior
  with local fixture tests.
- [ ] Complete HSM/KMS non-exportability, mTLS/service identity, key ceremony,
  rotation/revocation, recovery, audit, and regional latency evidence before
  enabling production issuance.

## ADR-0257 — server-owned FX issuance coordinator

- [x] Compose the exact registry snapshot, bounded provider acquisition,
  deterministic policy agreement, async signing, and final envelope verification
  into one server-owned coordinator.
- [x] Prove successful issuance, missing-policy short circuit, snapshot
  isolation, provider disagreement, signer failure/timeout, and cancellation.
- [x] Expose the coordinator through ADR-0260's authenticated tenant-scoped
  API port; caller-owned FX evidence remains forbidden.
- [ ] Enable the authenticated path for non-local traffic only after durable
  evidence/outbox and provider/legal/custody release decisions are explicit.

## ADR-0258 — FX evidence replay integrity and shared signer ownership

- [x] Separate historical signed-envelope integrity verification from current
  issuance freshness and future-skew checks.
- [x] Allow the async signer port to be shared through `Arc` without requiring
  adapter cloning or private-key export.
- [x] Persist the complete typed FX envelope with the quote, bind it to the
  pre-FX facts digest, audit access, and verify it during authenticated replay.
- [x] Integrate the persisted envelope into the authenticated quote composition
  boundary with server-owned issuance and fail-closed missing-issuer behavior.
- [ ] Enable the authenticated path for non-local production traffic only after
  provider, legal, custody, and evidence-retention decisions are frozen.

## ADR-0259 — durable FX envelope quote binding

- [x] Persist the complete typed FX envelope in immutable quote evidence
  metadata in the same transaction as encrypted facts and the first outbox
  record.
- [x] Bind the envelope to the pre-resolution facts digest and verify its
  projected conversion against normalized tax facts.
- [x] Revalidate the envelope and projection during tenant-scoped historical
  replay without applying current issuance expiry.
- [x] Supply this storage boundary from the authenticated server-owned
  composition boundary; storage still rejects caller-owned evidence.
- [ ] Enable a concrete provider/signer deployment only after provider, legal,
  custody, and evidence-retention decisions are frozen.

## ADR-0260 — authenticated server-owned FX quote composition

- [x] Define the server-only asynchronous issuance port and keep provider,
  policy, reference-rate, signer, and custody inputs outside browser payloads.
- [x] Add optional source-currency input with same-currency bypass and bounded
  fail-closed behavior when no issuer is configured.
- [ ] Wire an approved provider policy registry and HSM/KMS-backed signer into
  the process bootstrap after DEC-0251-01/03/09 are frozen.
- [ ] Retain provider agreement, legal acceptance, regional latency, and
  production FX enablement evidence before enabling non-local traffic.

## ADR-0261 — vendor-neutral FX issuer process bootstrap

- [x] Add a strict, bounded process configuration contract for the policy
  registry, provider sources, signer identity, and secret indirection.
- [x] Construct the existing provider/signer adapters once at startup and
  inject the warm server-owned issuance port into `AppState`.
- [x] Prove partial-configuration, duplicate-topology, invalid-identity,
  local-construction, and non-local fail-closed behavior before listener bind.
- [ ] Enable staging/production only after DEC-0251-01/03/04/06/07/09 are
  frozen with retained provider, legal, custody, residency, and SLO evidence.

## ADR-0154 — governed adjustment refund movement

Implement only the first explicit movement contract: a negative `Refund` claim
with a tax-vault source, merchant-funder source, and payer-bound destination.
The on-chain claim/execute/cancel instructions, bounded Rust compilers, and
cross-language claim vector are implemented. The claim, two checked CPIs,
cancellation, and finalized evidence are implemented in the pinned client
contract; the client now attempts a substituted destination, proves no balance
drift before the legitimate execution, and exercises governance cancellation
of a pending claim. Linux validator execution, finalized reconciliation, and
unknown transaction state still require retained evidence before any paid-state
API is introduced.

## ADR-0155 — finalized refund execution evidence projection

Add the finalized-only indexer projection for `AdjustmentRefundExecuted`.
The fixed-width decoder, identity/conservation/finality checks, raw-evidence
binding, idempotent replay merge, and PostgreSQL migration `0032` are
implemented. The projection remains evidence only; validator execution,
independent finalized token-account reconciliation, and a future paid-state
transition remain release gates.

## ADR-0156 — finalized refund balance proof

Add the pure `settlement-adjustments` proof for one finalized three-account
snapshot: tax-vault decrease by tax, merchant-funder decrease by net, and
payer-destination increase by gross at one non-zero finalized slot. The domain
function and boundary tests are implemented. The bounded finalized RPC
multi-account topology reader and deadline-aware failover are implemented with
exact token-account authority/mint/layout checks. Its post-state reader is
diagnostic only and is not a before/after proof source.

## ADR-0157 — transaction-bound refund balance proof

ADR-0158 adds the server-owned proof worker: it selects one unproved indexed
refund execution, resolves the finalized claim PDA within a complete provider
attempt, invokes the pure checked delta proof, and persists immutable evidence.
It never accepts topology from the browser or creates a paid lifecycle state.

Use finalized `getTransaction` pre/post token balances, exact account indexes,
and the execution signature/event projection to prove the three refund legs at
one transaction slot. Add a separate finalized claim-account topology check,
then persist the immutable proof transactionally with the indexed event and raw
evidence. Do not add `posted`, `chain_executed`, or `paid` until signature/event
binding, Linux validator evidence, replay/idempotency, and operator
reconciliation gates pass.

## ADR-0158 — server-owned refund-proof orchestration

The bounded worker and its narrow ports are implemented in the indexer and
wired into the indexer service. `RefundProofWorker` claims at most one
server-owned candidate, derives all missing account bindings from the finalized
claim PDA through the complete provider port, calls the pure checked proof, and
persists only immutable evidence. ADR-0185 implements the UUIDv7 PostgreSQL
lease, proof-exclusion query, concurrent ownership fencing, idempotent proof
path, raw-event transactional adapter, and three-attempt dead-letter handling.
Focused tests plus the Docker lease drill cover the bounded and contention
behavior. Keep lifecycle/payment state out of this slice.

## ADR-0185 — lease-fenced refund-proof candidate ownership

Add a 60-second PostgreSQL UUIDv7 lease to the indexed refund execution
projection. `SKIP LOCKED` claims one candidate before RPC work, stale leases
are reclaimable, and serializable persistence requires the exact lease token
while rechecking raw event identity. Wire the canonical `RefundProofWorker`
through PostgreSQL source/sink adapters and remove the legacy unleased proof
cursor. The lease is coordination only and cannot authorize payment.

## ADR-0186 — bounded refund-intent serialization retries

Refund nonce allocation and claim/execute intent creation retain their
`SERIALIZABLE` authority boundary while applying a three-attempt, storage-local
retry only for PostgreSQL `40001` and `40P01`. The request digest,
idempotency key, immutable artifact, first lifecycle event, and first outbox
publication remain unchanged across attempts. `make postgres-contention-drill`
proves the database contract for 16 concurrent nonce allocations and same-key
claim creation. `make refund-execute-api-contention-drill` additionally drives
the authenticated API and proves 16 same-key execute-preparation callers
converge to one intent, lifecycle event, outbox publication, and response
digest; retry telemetry, live submission, and production load/SLO evidence
remain release evidence.

## ADR-0159 — refund-proof operational observability

Export a fixed-cardinality `refund_proof` worker operation and the durable
`stp_indexer_refund_proofs_pending` backlog gauge. Alerting is evidence of
unproved finalized executions only; it never marks a refund paid or authorizes
release. Scrape authorization, Alertmanager delivery, and retained load/SLO
evidence remain production gates.

## ADR-0150 — immutable settlement adjustments

The `settlement-adjustments` domain bounded context is implemented before
refund, reversal, correction, chargeback, or remittance-adjustment APIs. The
first slice validates UUIDv7 identity, signed i128 atomic deltas, gross/tax/net
conservation, adjustment-kind sign semantics, canonical evidence digests,
monotonic lifecycle transitions, and idempotent cumulative bounds without
touching the original receipt or authorizing token movement. Subsequent slices
must add PostgreSQL append-only evidence and outbox persistence, tenant-scoped
four-eyes approval, UI review and explanation, a governed on-chain posting
plan, and finalized reconciliation. Until those slices are evidenced, the
product must continue to describe refund/correction support as planned rather
than available.

## ADR-0151 — durable settlement-adjustment workflow

Persist tenant-scoped adjustment proposals with UUIDv7 identity, complete
request/domain digests, exact signed deltas, immutable financial fields,
append-only lifecycle events, and a same-transaction outbox publication.
Expose bounded authenticated propose/read/approve/reject operations with
four-eyes subject separation and idempotent retry recovery. This
proposal/approval slice is implemented and verified by migration and
full-stack Docker integration tests. Do not expose a `posted` transition until
a separate on-chain posting and finalized reconciliation design exists.

## ADR-0152 — settlement-adjustment operator workspace

Add the dedicated lazy-loaded React workspace for proposal, tenant-scoped
lookup, evidence review, and governed approve/reject/void operations. Preserve
decimal strings and server-owned lifecycle state, use memory-only bearer
tokens, and expose no `posted` or payment-success control. The proposal form
starts empty for all settlement, financial, asset, and evidence values; test
fixtures provide synthetic inputs explicitly. Prove parsing, failure states,
accessibility, mobile/RTL layout, and the NGINX browser journey before
considering this UI slice complete.

## ADR-0132 — explicit worker deployment-environment safety

The finalized indexer and settlement-route verifier now require explicit
`INDEXER_ENV` and `ROUTE_VERIFIER_ENV` values (`local`, `staging`, or
`production`) before constructing transports or opening durable connections.
Insecure local RPC remains local-only. Compose keeps explicit local defaults,
and the environment-contract gate verifies both worker contracts. This closes
the remaining ambiguity in the worker startup boundary while preserving the
separation between environment labeling and real staging/production release
approval.

## ADR-0134 — multi-replica settlement submission-fence evidence

The local-stack settlement drill routes two identical verified signed requests
to independent API processes sharing PostgreSQL, NATS, and the RPC fixture. On
2026-09-27 it produced one `200`, one `409`, one validating RPC send, and one
durable native signature, then completed the independent ambiguous and crash
reconciliation drills. This is evidence for the database compare-and-set
fence, not a production provider or regional failover claim.

## ADR-0133 — multi-replica quote idempotency evidence

The quote-service Docker load harness runs two independent API containers
against the same PostgreSQL/NATS services. Unique-key requests alternate
between replicas, and bounded same-key contention also crosses replicas. The
drill requires one durable idempotency record, calculation, and outbox event;
it does not use a process-local or Redis lock as correctness authority. CI
uploads the bounded output as a public-safe evidence artifact; regional and
production-volume evidence remain separate release gates.

## ADR-0135 — SLO, error-budget, incident, and dependency controls

Publish the user-journey SLO and error-budget policy, incident severity and
containment runbook, and provider-neutral dependency/exit register. The
artifacts define explicit latency, availability, finality, reconciliation,
RTO/RPO, ownership, burn-rate, and zero-tolerance financial-integrity rules.
The public-documentation contract verifies their presence and links. Named
production owners, alert-receiver delivery, measured regional SLOs, and
restore/failover evidence remain deployment gates.

## ADR-0136 — scheduled vault-reconciliation drill

Extend the isolated RPC fixture with one bounded finalized legacy-SPL account
response and run the real indexer service against it. The drill inserts a
server-owned target, proves a scheduled `matched` run, changes only the
fixture's synthetic amount, and proves an append-only `mismatch` run plus
fixed-cardinality alert input. It does not substitute for live validator,
multi-provider, staging, or Alertmanager evidence.

## ADR-0137 — multi-provider RPC failover drill

Add a bounded method-failure mode to the isolated RPC fixture and an opt-in
two-provider mode to the scheduled vault-reconciliation drill. The real
indexer must attempt the degraded first endpoint, fail over to the healthy
second endpoint, and still persist finalized matched/mismatch evidence. This
closes process-level endpoint-wiring evidence while leaving live provider,
regional capacity, and production SLO gates open.

## ADR-0138 — versioned database migration runner

Add a Rust migration runner with lexical version validation, PostgreSQL
advisory locking, immutable SHA-256 checksums, and a durable applied-migration
ledger. Compose runs it as a one-shot dependency before application services;
local synthetic fixtures are an explicit separate seed phase. The runner is
exercised concurrently against an isolated PostgreSQL 18 instance and then
against an intentionally changed historical file to prove that drift fails
closed. ADR-0182 adds the synthetic encrypted-artifact contract and ADR-0183
adds the isolated expand/contract rollback rehearsal; production KMS/HSM,
off-site recovery, staging approval, and live rollback authorization remain
release gates.

## ADR-0221 — structured migration-connection lifecycle

- [x] Replace the detached migration-runner PostgreSQL driver task with an
  owned `tokio::select!` supervisor.
- [x] Fail closed when the connection driver errors or terminates before the
  migration operation completes; return promptly after migration success.
- [ ] Retain staging migration-service evidence and production rollback,
  backup/PITR, and operator-approval evidence.

## ADR-0222 — fail-closed shutdown-signal handling

- [x] Propagate shutdown-signal listener errors from the API, worker, indexer,
  and route-verifier lifecycles instead of converting them to a silent success.
- [x] Keep signal ownership inside each existing structured `select!` loop or
  Axum graceful-shutdown boundary and preserve orderly telemetry shutdown on a
  valid signal.
- [ ] Capture production supervisor restart, signal delivery, and graceful
  termination evidence.

## ADR-0223 — observable non-authoritative metrics failures

- [x] Add a shared bounded `observe_or_log` adapter so fixed-operation metric
  registration failures are visible without making telemetry authoritative for
  financial or reconciliation work.
- [x] Keep the API metric operation catalog single-sourced between production
  bootstrap and test state so new endpoint observations cannot drift silently;
  the authenticated revenue-reference integration flow exercises the new
  operations.
- [x] Add the static API metric registry contract to `make check`; it compares
  every API `observe_or_log` call with the fixed operation catalog and rejects
  both missing and stale registrations.
- [x] Migrate API, worker, indexer, and route-verifier production call sites
  away from silently discarded metric-observation errors.
- [ ] Retain production scrape, alert-delivery, and on-call response evidence;
  local logs and Prometheus-compatible output do not prove those live gates.

## ADR-0224 — cross-language calculation-attestation vector

- [x] Add the versioned calculation-attestation vector with canonical payload,
  digest, issuer key, and signature material.
- [x] Consume the vector from Rust policy verification and the browser
  compatibility helper; preserve decimal-string atomic values and explicit
  little-endian field encoding.
- [ ] Add production issuer-key rotation, HSM/KMS ceremony, and finalized
  chain evidence; a compatibility vector does not prove deployment authority.

## ADR-0225 — dedicated warm Tax Engine service adapter

- [x] Add a separately deployable Rust/Tokio adapter that shares the pure
  `tax-engine` domain library and exposes authenticated calculation and
  document-calculation endpoints for simulation, batch, and future worker
  workloads.
- [x] Enforce bounded request bodies, cache capacity, blocking-concurrency
  permits and queue wait, cache-key immutability, service authentication, and
  fail-closed environment configuration.
- [x] Add a pinned non-root Docker image, Compose health check, Prometheus
  scrape target, bootstrap inclusion, lifecycle contract, and executable
  deterministic/cache/body-limit tests.
- [ ] Add staging/production service identity, TLS/mTLS, token rotation,
  network policy, redacted audit evidence, and regional SLO/load evidence.
- [ ] Keep the API's synchronous wallet-signing and settlement compilation
  path in-process unless measured evidence and a new ADR prove a service call
  is safe for the documented latency and availability budgets.

## ADR-0226 — Tax Engine deployment preflight boundary

- [x] Extend the shared staging/production preflight with exact
  `TAX_ENGINE_ENV` parity, non-local token, and complete mTLS path
  requirements.
- [x] Add positive and negative executable deployment-contract cases without
  printing secret values or changing local Compose defaults.
- [ ] Complete production service identity, TLS/mTLS termination, secret
  rotation, network policy, and regional SLO evidence.

## ADR-0227 — Rustls mTLS transport for the Tax Engine service

- [x] Add direct Rustls mTLS serving for staging/production with explicit
  certificate, private-key, client-CA, and ALPN configuration. The service
  explicitly selects the AWS-LC provider when the workspace exposes multiple
  Rustls providers, and deterministic in-memory handshakes prove both trusted
  client acceptance and untrusted client rejection.
- [x] Reject partial/malformed TLS material and never downgrade non-local
  startup to plaintext or local configuration.
- [ ] Exercise production certificate provisioning, rotation/reload, CA
  compromise response, regional mTLS load, and service SLO evidence.

## ADR-0228 — atomic Tax Engine mTLS certificate reload

- [x] Add explicit Unix `SIGHUP` handling with one sequential reload cycle per
  delivered signal and structured ownership with server shutdown.
- [x] Validate replacement certificate, key, client CA, verifier, and ALPN in
  a blocking task before atomically swapping the Rustls configuration.
- [x] Retain the last known-good configuration on malformed replacement
  material and cover valid/invalid reloads with deterministic tests.
- [ ] Prove production secret-manager atomic file delivery, permissions,
  certificate/key rotation, CA overlap and compromise response, regional
  reload latency, connection behavior, and alert/on-call evidence.

## ADR-0229 — canonical Tax Engine observability identity

- [x] Use the single `tax_engine` identity across tracing initialization,
  fixed-cardinality metrics, telemetry allow-lists, dashboards, and alerts.
- [x] Add a regression test proving the identity is accepted and dynamic or
  alternate service names remain rejected.
- [ ] Retain production dashboard/alert query evidence and review any future
  metric-identity migration as an explicit compatibility change.

## ADR-0230 — executable Tax Engine container runtime contract

- [x] Add a bounded isolated Docker runtime probe for health, authenticated
  readiness, exact aggregate calculation, authentication failure, body limits,
  and canonical fixed-cardinality metrics.
- [x] Keep synthetic credentials/facts local to the probe and clean only its
  own Compose project state.
- [x] Include the runtime contract in the standard repository check and public
  documentation/ADR indexes.
- [ ] Retain Linux/amd64 CI runtime evidence and production resource-profile,
  mTLS, regional SLO, and consumer-authorization evidence.

## ADR-0231 — explicit Tax Engine request-rejection semantics

- [x] Normalize malformed JSON and JSON shape/content-type failures to bounded
  `400 Bad Request` responses at both calculation endpoints.
- [x] Preserve the request-body admission layer's `413 Payload Too Large`
  response instead of masking it as a JSON error.
- [x] Cover both status boundaries with focused tests and the real pinned
  Docker runtime contract.
- [ ] Retain production gateway/client retry telemetry and confirm that
  deployment-specific proxies preserve the documented status contract.

## ADR-0232 — warm Tax Engine service concurrency evidence

- [x] Add the bounded Docker benchmark with warm-up, exact response checks,
  bounded request timeout, scoped cleanup, and p50/p95/p99 calculation.
- [x] Execute the default workload on 2026-10-03: 256/256 HTTP 200 responses
  at concurrency 32 on the pinned local arm64 image, with p50/p95/p99 of
  1.214/2.717/3.826 ms.
- [ ] Retain Linux/amd64 CI evidence, resource-profile samples, regional
  measurements, mTLS/client-identity evidence, and production SLO approval.

## ADR-0233 — real-container Tax Engine mTLS contract

- [x] Add the pinned-image mTLS probe with ephemeral CA/client/server
  certificates, trusted and untrusted-client checks, bearer authorization,
  exact calculation, ALPN negotiation, and bounded cleanup.
- [x] Exercise invalid and valid atomic CA replacement through the real
  process `SIGHUP` path; retain the last known-good configuration after an
  invalid replacement.
- [ ] Retain Linux/amd64 CI transport evidence, secret-manager delivery,
  certificate issuance/revocation, CA overlap, regional reload latency,
  identity issuance/rotation, and production alert/on-call evidence.

## ADR-0236 — Tax Engine mTLS client identity binding

- [x] Compare bearer-only, CA-only, subject/CN, exact URI-SAN, and external
  introspection alternatives; select the bounded Rustls URI-SAN verifier.
- [x] Require a unique, bounded `spiffe://` URI allow-list in non-local
  configuration and reject malformed or missing identity policy before bind.
- [x] Enforce identity at the TLS handshake, before bearer authorization and
  tax calculation, with no request-path network or database dependency.
- [x] Add focused handshake/configuration tests and wrong-identity real-image
  mTLS contract coverage.
- [ ] Retain production identity issuance, rotation, CA overlap/revocation,
  regional handshake latency, and alert/on-call evidence.

## ADR-0237 — Tax Engine service-token rotation

- [x] Support exactly one bounded service-token source: inline
  `TAX_ENGINE_AUTH_TOKEN` or file-backed `TAX_ENGINE_AUTH_TOKEN_FILE`.
- [x] Validate file size, UTF-8, trimming, token bounds, and non-local
  local-token rejection before listener bind.
- [x] Add lock-free immutable token snapshots and sequential Unix `SIGHUP`
  reload; validate token and TLS replacements together and retain the complete
  last-known-good runtime configuration after any failure.
- [x] Cover source exclusivity, bounded files, router rotation, deployment
  preflight, and real pinned-container rotation.
- [x] Run the real-container token-rotation contract as a dedicated required
  Ubuntu CI job, separate from the mTLS transport job.
- [x] Export the fixed-cardinality `runtime_reload` operation and alert on
  rejected token/TLS replacements without exposing credential material.
- [ ] Prove production secret-manager atomic delivery, file permissions,
  overlap/revocation ceremony, compromise response, regional reload latency,
  and alert/on-call evidence.

## ADR-0238 — governed refund-claim cancellation workflow

- [x] Record the alternatives, invariants, fixed account shape, non-custodial
  signing boundary, finalized evidence rule, and diagnostic latency budget in
  ADR-0238.
- [x] Add the pure Rust cancellation compiler with canonical discriminator,
  claim PDA/registry derivation, exact five-account order, positive nonce and
  duplicate-account validation, and wire-budget tests.
- [x] Add exact signed-message cancellation verification with no address lookup
  tables, extra instructions, account substitution, or fee-payer aliasing.
- [x] Add a separate UUIDv7 cancellation intent aggregate, immutable artifact
  validation, lifecycle events, outbox record, and PostgreSQL migration;
  never mutate the immutable claim artifact to represent cancellation.
- [x] Add governance-scope API prepare/submit/read routes. Browser input may
  select a bound governance wallet and provide signed bytes only; it may not
  provide financial values, program IDs, registry addresses, claim PDAs, or
  nonces. Submission remains proof-pending until finalized reconciliation.
- [x] Add the finalized cancellation-event reconciliation route and atomic
  claim/cancellation transition.
- [x] Add the bounded automatic cancellation reconciler: UUIDv7 PostgreSQL
  lease, `SKIP LOCKED` ownership, stale-lease reclaim, shared immutable
  evidence derivation, token-fenced final CAS, and fixed-cardinality backlog
  telemetry. The operator route remains available but cannot steal an active
  worker lease.
- [x] Add the executable Docker drill and public contract for the reconciler:
  two real indexer replicas, synthetic finalized RPC recovery, stale lease
  reclaim, exact raw-evidence digest, atomic claim/cancellation convergence,
  idempotent outbox cardinality, and zero pending telemetry.
- [x] Run the Docker drill locally: on 2026-10-02 the arm64 host produced
  `finalized|cancelled`, one cancellation event, one cancelled-claim event,
  two outbox events, one raw transaction/evidence record, and zero backlog.
  Linux/amd64 CI retention, validator/provider, and production evidence remain
  release gates.
- [x] Require finalized `AdjustmentRefundClaimCancelled` evidence before the
  claim projection becomes cancelled; preserve unknown provider outcomes and
  idempotent replay semantics.
- [x] Add production UI/UX for governance cancellation, wallet rejection,
  timeout, unknown, finalized cancellation, and reconciliation-required states
  with accessible explanations and no implied refund success.
- [ ] Extend the pinned Linux validator conformance suite with the complete
  cancellation lifecycle and unchanged tax-vault, funder, and payer balances.

## ADR-0239 — catalog-driven UUIDv7 invariant closure

- [x] Compare manual allow-lists, migration-source scans, triggers, and live
  PostgreSQL catalog discovery with explicit drift and latency criteria.
- [x] Replace the hand-written UUIDv7 verification list with catalog discovery
  of every public `uuidv7()` default and require native UUID type, default
  preservation, and a validated version-7 check constraint.
- [x] Add migration `0058` to close the invariant for existing generated
  identifiers using deterministic bounded constraint names and fail-closed
  validation.
- [x] Clean bootstrap, upgraded-schema, invalid-row, and backup/restore
  contracts pass locally against Docker PostgreSQL 18.6 on arm64; Linux
  PostgreSQL 18 evidence remains a release gate.

## ADR-0240 — lease-safe refund-claim cancellation transition trigger

- [x] Add migration `0059` so cancellation reconciliation can acquire a
  UUIDv7 lease without changing status; same-status lifecycle-field mutation
  remains rejected.
- [x] Add a failing-then-passing signature-page regression test for complete
  initial pages, preserving non-null cursor-progress fail-closed behavior.
- [x] Pass the two-replica Docker cancellation drill and retain the exact
  finalized evidence/outbox/cardinality result locally.
- [ ] Retain Linux/amd64 migration, validator, provider-failover, and
  production operational evidence before enabling the workflow.

## ADR-0234 — atomic API JWKS reload for bounded identity rotation

- [x] Add an optional file-backed JWKS source that is mutually exclusive with
  inline key configuration and is fully validated before startup.
- [x] Add lock-free immutable key-snapshot replacement on Unix `SIGHUP`; read
  and parse the bounded file in a blocking task outside the request executor,
  and retain the last known-good snapshot after every invalid replacement.
- [x] Add focused rotation/rejection tests and a real API runtime contract for
  valid replacement, removed-key rejection, and invalid-replacement retention.
- [ ] Retain production identity-provider overlap/revocation ceremony,
  secret-manager atomic delivery and permissions, regional reload timing, and
  alert/on-call evidence before production activation.

## ADR-0235 — atomic jurisdiction-provider key registry reload

- [x] Add an optional file-backed provider registry that is mutually exclusive
  with structured inline and compatibility-pair configuration and is fully
  validated before startup.
- [x] Add lock-free immutable provider-key snapshot replacement on Unix
  `SIGHUP`; parse bounded files outside the request executor and retain the
  last known-good registry after invalid replacements.
- [x] Add focused provider evidence rotation tests and a real API runtime
  contract for valid replacement, removed-key rejection, and malformed-
  replacement retention.
- [ ] Retain live provider-key overlap/revocation, HSM/KMS custody, atomic
  secret delivery, regional reload timing, and legal/tax owner evidence before
  production activation.

## ADR-0160 — clean-stack Docker drill migration gate

Every PostgreSQL-backed operational drill now waits for PostgreSQL health and
runs the pinned one-shot migration service before accessing application
tables. This keeps projection-rebuild and contention evidence reproducible
from a clean checkout and prevents a stale or pre-migrated local volume from
masking migration failures. The drills continue to use isolated project
names, bounded synthetic fixtures, and scoped teardown; live production
migration rollback remains a release gate.

## ADR-0161 — governed v0+ALT settlement admission

The signed-message boundary now supports a bounded v0 path through a
server-owned lookup resolver. It reads only the lookup tables referenced by
the signed message at finalized commitment, requires canonical ALT ownership
and active state, caps tables and loaded addresses, and compares the expanded
account domain, signer/writable privileges, program, instruction data, and
wire budget with the server-created settlement artifact. Legacy messages
retain the existing path; v1 remains rejected until separately governed.
Linux target-cluster conformance, compute-unit/priority-fee evidence, and
regional RPC SLOs remain release gates.

## ADR-0162 — server-owned compute-budget and priority-fee policy

The settlement compile artifact now carries a bounded versioned policy. Browser
transactions serialize the exact `SetComputeUnitLimit` and
`SetComputeUnitPrice` pair before settlement, while the API verifies their
order, data, and program identity against the server artifact. Missing or
mutated policies fail closed for both legacy and v0+ALT messages. The initial
v1 values are deterministic local/test defaults; Linux validator calibration,
target-cluster fee acceptance, and regional latency evidence remain release
gates.

## ADR-0127 — finalized vault-balance reconciliation

Add immutable route/stream reconciliation targets with operator-approved
opening-balance evidence. The scheduled indexer loop will derive an exact
checked-u64 expected balance from finalized receipt tax inflows and finalized
executed vault-release outflows, read the verified token account at finalized
commitment, append a run ledger, and expose fixed-cardinality mismatch/error
telemetry. Unsupported Token-2022 extensions fail closed and remain a
separate compatibility decision.

## ADR-0128 — tenant-scoped vault-reconciliation operator read model

Expose bounded authenticated target/run projections to the merchant-scoped
operator workspace. Return atomic values and slots as decimal strings, validate
all responses in the browser, and keep target provisioning outside the browser
until the independent opening-balance ceremony and system-operator governance
are implemented.

## Phase 0 - decisions and legal boundary

Deliverables:

- approve the scope and ADRs;
- identify initial stablecoin(s), jurisdictions, product tax codes, and policy issuer governance;
- define legal/custody/licensing review with counsel;
- define the target cluster, RPC providers, regions, and SLOs;
- create the repository policy for version pinning, security disclosure, and upgrades.

Exit criteria: signed scope document, threat model accepted, and no unresolved contradiction between tax business rules and the on-chain invariant model.

## Phase 0A - Tax Engine product foundation

Deliverables:

- product coverage matrix and explicit launch jurisdictions/tax regimes;
- SLOs, error budgets, RTO/RPO, support model, on-call ownership, and incident severity definitions;
- tax-rule provenance and legal/tax review ownership model;
- financial operations for refunds, reversals, chargebacks, remittance evidence, and reconciliation;
- API/IDL/rule schema compatibility and deprecation policy;
- public-repository documentation, security disclosure process, contribution workflow, changelog, and licensing decision;
- dependency register, vendor/RPC exit plan, backup/restore design, and key-recovery plan.

Exit criteria: the Tax Engine product standard has an owner and evidence plan for every pillar; unsupported product/jurisdiction cases have explicit behavior.

## Phase 1 - workspace and contracts

Deliverables:

- Rust workspace with pinned toolchain and locked dependencies;
- PostgreSQL 18.x container and migration baseline with UUIDv7 ID policy;
- Anchor program workspace using the selected Anchor v1 template;
- TypeScript React/Vite workspace with generated client types;
- canonical policy, split-plan, asset, quote, receipt, and error schemas;
- localnet/devnet CI and deterministic fixture generation;
- NGINX container with SPA fallback, compression, caching, TLS headers, and API proxy configuration.

Exit criteria: clean build from a fresh machine/CI image; schema changes require compatibility review; no floating dependency ranges in production manifests.

Current evidence: the repository now has a pinned Rust workspace, Anchor 1.2.0
settlement program boundary, PostgreSQL 18.6 migration baseline with UUIDv7,
React/Vite/NGINX containers, and a strict GitHub CI workflow. The program slice
is intentionally legacy-SPL-only and is not a mainnet deployment claim.

Tenant rule-ownership isolation is defined in ADR-0079. Rule releases are
merchant-owned, quote resolution and governance predicates are tenant-bound,
and migration 0016 fails closed when legacy ownership is unknown. The
clean-stack integration and backup/restore drill now prove the migration,
cross-tenant API boundaries, duplicate-key coexistence, and composite foreign
key invariant.

ADR-0080 adds a SHA-256-verified, pinned platform-tools browser-asset fallback
for Dockerized SBF compilation. It does not relax the independent validator
startup or live transaction conformance gate.

The CI workflow now runs the bounded quote-service workload as a dedicated
job and routes validator conformance through the same Dockerized SBF builder
used locally.

## Phase 2 - on-chain vertical slice

Implement in this order:

1. Asset registry with legacy SPL Token and Token-2022 owner checks.
2. Merchant config and versioned split plan.
3. Policy attestation account and canonical digest.
4. Stablecoin tax vault and PDA authority.
5. Settlement receipt and client nonce replay protection.
6. Atomic tax + net split CPI flow.
7. Structured events and explicit error codes.
8. Pause, revoke, and upgrade authority controls.

The implemented first slice is the bounded token-interface `settle` instruction
with a cached `PolicyAttestation` PDA: conserving tax/net transfers,
merchant/mint/sequence/digest/amount binding, expiry/revocation checks,
payer/source and vault-PDA authority checks, distinct token-account checks,
deterministic payer/merchant/nonce receipt PDA, and a structured event. ADR-0034
adds the merchant/mint-scoped governed issuer registry used to authorize
attestation registration and revocation. ADR-0035 accepts extension-free
Token-2022 through the same interface and rejects unsupported extensions.
Pause controls and localnet transaction tests remain mandatory follow-on
slices. ADR-0065 adds the validator-backed conformance harness: it builds the
pinned SBF artifact, loads it under the fixed program ID, and executes legacy
SPL, extension-free Token-2022, duplicate-nonce, and real Token-2022 extension
rejection cases. The suite is opt-in through `make solana-conformance` so the
ordinary unit/UI checks remain deterministic while the release gate fails when
the validator, artifact, or RPC evidence is unavailable. ADR-0068 adds
full-domain property coverage for the on-chain `u64` amount validator; live
CPI balance conservation and atomic rollback remain separate validator gates.

Exit criteria: localnet property tests demonstrate amount conservation, recipient correctness, policy freshness, replay resistance, and atomic rollback.

### Split Plan implementation gate

The current enabled API workflow has one destination. ADR-0091 now provides the
pure integer allocator, canonical Rust/browser vectors, immutable Split Plan
PDA, additive split instruction, on-chain invariants, and deterministic
compiler artifact. The remaining gate is authenticated persisted-plan/API
resolution, route activation, indexer/evidence decoding, browser review,
validator-backed CPI evidence, and measured eight-recipient budgets. No
arbitrary recipient list may be accepted as a shortcut, and the repository must
not claim production revenue-split support until that full gate is green.

ADR-0092 is the next executable gate: persist plans under authenticated tenant
scope, load only active/effective plans by ID, bind the complete server-owned
plan document into quote compilation and intent evidence, and keep the existing
single-recipient endpoint compatible until route activation is governed.

ADR-0093 is the following evidence gate: decode and persist
`SplitSettlementExecuted` as a typed finalized projection, retaining the
split-plan version and digest through forward indexing and replay.

ADR-0094 closes the checkout review gap: expose only authenticated active
Split Plans and make the browser explicitly review and bind the selected plan
through the existing quote-bound compiler. The browser never owns recipients,
amounts, or merchant scope.

ADR-0095 closes the browser request-boundary gap: all SPA API calls use one
tested fetch wrapper with explicit health, ordinary API, and signed-submission
deadlines. Caller cancellation remains available for route changes and
unmounts; timeouts fail closed and never trigger browser retries or fabricated
settlement state.

ADR-0096 closes the rule-authoring gap: an authenticated author can create an
 idempotent tenant-owned draft through the Rust API and governance workspace.
 The server validates the complete RuleSet, records canonical digest,
 provenance, immutable creation evidence, and the first outbox record in one
 PostgreSQL transaction; only then may the existing four-eyes lifecycle
 advance it.

ADR-0098 closes the browser durable-intent recovery gap: settlement status
responses are validated as bounded UUIDv7 lifecycle records, an unsuccessful
submit reloads the server-owned intent, and a failed reload leaves the checkout
in an `unknown` no-second-signing state instead of clearing the intent. Live
provider and finalized-chain evidence remain production gates.

ADR-0099 closes the remaining browser settlement-response trust gap: active
routes and compiled instruction artifacts are parsed with bounded Solana,
UUIDv7, digest, base64, account-order, token-program, and discriminator
invariants before they reach the review or wallet handoff state. The API still
repeats all authority-sensitive checks against the signed message.

ADR-0100 closes the production attestation authority gap: an authenticated
quote-bound endpoint derives the merchant, route mint, calculation digests,
amounts, and verification time from server-owned evidence; verifies an
externally signed attestation; persists the immutable document, `attested`
calculation status, and first outbox event transactionally; and requires the
persisted payload digest before non-simulation single or Split Plan settlement
compilation. Issuer private keys remain outside the API. Generic attestation
verification remains an isolated utility and cannot authorize settlement.

ADR-0101 adds a pure Rust jurisdiction-resolution bounded context with
versioned source precedence, explicit conflict modes, and canonical evidence
digests. ADR-0102 binds the server-owned resolution envelope into normalized
tax facts and calculation results while preserving legacy replay bytes. ADR-0103
adds the production boundary: provider-signed evidence is verified against the
authenticated tenant and pre-resolution facts digest, then resolved against an
immutable tenant policy bound to the published rule release. The local Compose
integration path now proves this end to end with an ephemeral non-root issuer;
production provider registry, HSM/KMS key rotation, privacy controls, and
jurisdiction-specific legal/tax approvals remain release gates. A
caller-supplied jurisdiction string or digest is never sufficient for a new
globally supported quote.

ADR-0104 adds the authenticated jurisdiction-policy control plane. Policy
drafts are tenant-scoped, idempotent, canonically digested, and bound to a
rule release; distinct author/reviewer/approver subjects promote them through
an append-only, transactional lifecycle. Docker integration proves draft
replay, four-eyes review/approval/publication, governance reads, and mutation
rejection. Production identity provisioning, legal/tax owner approval, and
governance UI/accessibility evidence remain release gates.

ADR-0105 adds the governed vault-release bounded context. A merchant/mint
registry separates governance, proposal, and approval authorities; immutable
claim PDAs bind amount, destination, reference digest, nonce, and bounded
execution window; the vault PDA performs one checked CPI atomically with claim
consumption. The next vertical slice adds the authenticated durable claim API,
outbox/evidence projection, treasury review UI, and validator-backed release
and cancellation evidence. The treasury UI now starts without invented
evidence, nonce, or amount values and fails form validation until approved
inputs are supplied; the browser still cannot authorize a vault transfer. No
API or browser key may authorize a vault transfer.

ADR-0107 closes the settlement-intent tenant boundary. Submit, status, and
reconciliation reads now carry the authenticated merchant into PostgreSQL, and
state transitions fence both intent UUID and merchant ID in one transaction.
Cross-tenant IDs are intentionally indistinguishable from missing IDs. This is
a security hardening slice and does not by itself close live RPC or validator
settlement evidence gates.

ADR-0108 implements the privacy/security slice: retain only normalized replay
facts, encrypt them before PostgreSQL persistence with an injected
KMS/HSM-compatible AEAD key provider, discard raw jurisdiction envelopes after
in-memory verification, and append an access audit event for every replay
attempt. The expand/contract migrations preserve legacy replay rows while
making new quote writes fail closed without an evidence key. Production KMS/
HSM key rotation, retention/deletion policy, and backup-recovery evidence
remain release gates.

ADR-0109 adds the concrete key-provider port and bounded current/previous-key
ring. The local adapter supports deterministic Docker verification and the API
accepts a bounded ring document for rotation rehearsal; production must supply
an approved KMS/HSM-backed provider and complete the re-encryption, revocation,
recovery, and coverage drills before enabling durable evidence at scale.

## Phase 3 - policy issuer and transaction compiler

Deliverables:

- Rust policy-signing adapter using HSM/KMS-backed keys;
- policy registry publisher and revocation workflow;
- authenticated quote-bound attestation ingestion with immutable evidence and
  issuer/HSM/KMS integration;
- API quote and transaction-compilation endpoints;
- versioned transaction compiler with v1/v0 selection;
- simulation, compute-budget, priority-fee, blockhash, and bounded retry policies;
- idempotency and status endpoints.

Exit criteria: API holds no user private keys; an invalid or stale policy cannot compile a valid settlement; retries do not duplicate a settlement.

## Phase 3A - tax engine

Deliverables:

- pure Rust tax-engine domain crate with no network, database, clock, or wallet dependencies;
- canonical tax-rule DSL and compiled rule graph;
- normalized transaction facts and jurisdiction-resolution pipeline;
- support for simple, progressive, thresholded, tax-on-tax, cascading, inclusive/exclusive, min/max, exemptions, credits, and explicit rounding/truncation rules;
- rule provenance, effective dating, approval workflow, semantic versioning, and immutable content hashes;
- calculation trace explaining every component, base, bracket, threshold, clamp, and rounding step;
- golden-vector, property, fuzz, metamorphic, and differential tests;
- calculation attestation publisher bound to merchant, rule version, jurisdiction, asset, facts digest, and expiry;
- simulation endpoint and preview fixture set for the UI.

Current evidence: the `policy` crate implements canonical payload encoding,
Ed25519 signing/verification, digest validation, validity windows, amount
conservation, and calculation binding. The API exposes a verification endpoint
whose status is explicitly `verified_unsettled`. The settlement program now
requires a cached, non-revoked, unexpired `PolicyAttestation` PDA whose exact
merchant, mint, sequence, digest, and gross/tax/net tuple matches the
instruction. The governed issuer registry now authorizes registration and
revocation through a merchant/mint-scoped PDA with separate governance and
issuer roles; fresh on-chain Ed25519 verification and multisig-backed
deployment remain open gates.

ADR-0066 adds the versioned `test-vectors/tax-engine-v1.json` contract. The
Rust engine now executes exact golden cases for simple, progressive,
tax-on-tax, cascading, inclusive/exclusive, threshold, cumulative, exemption,
credit, min/max, truncation, and rounding semantics, plus an exact cumulative
facts-digest regression. Property, fuzz, differential, legal-coverage, and
replay-evidence gates remain separate requirements.

The public [tax coverage matrix](../product/tax-coverage-matrix.md) now maps
each engine capability to its vector and domain-test evidence while explicitly
separating calculational capability from jurisdiction approval. It remains
forbidden to publish a jurisdiction without provenance, effective dating,
legal/tax ownership, required facts, and an approved rule release.

ADR-0037 adds explicit JWT scope authorization at the service boundary. ADR-0028 defines the bounded PostgreSQL adapter and transactional production
quote endpoint backed only by published immutable rule releases. ADR-0029 adds
merchant authentication and ADR-0032 adds the transactional outbox. These are
implemented foundation slices, not a claim that the simulation endpoint is a
production substitute.

Current evidence: `crates/storage` now provides that adapter, `/v1/tax/quote`
selects an effective published release, revalidates its canonical digest, and
commits the calculation plus idempotency response transactionally. The API
also verifies a bounded OIDC-compatible EdDSA JWT against a configured JWKS
set and derives the merchant scope from the authenticated principal. The quote
operation additionally requires the exact `tax:quote` scope; missing
authorization is a fail-closed 403. Docker
smoke tests prove first-write, exact replay, NGINX forwarding, authenticated
scope, one transactional outbox event, immutable published-rule rejection,
and `409` conflict behavior. ADR-0033 now provides the first bounded delivery
slice: PostgreSQL lease claims, stale-claim fencing, retry/dead-letter bounds,
and a Rust worker that waits for NATS JetStream acknowledgement before marking
an event published. Live JWKS refresh/revocation policy, production
observability, and operational outbox alerting remain separate gates; policy
attestation and settlement authority are covered by the chain slice below.

ADR-0048 now provides an immutable compiled tax-rule graph. It removes repeat
validation, digesting, and dependency sorting from callers that reuse a
published rule while preserving byte-equivalent calculation results. A bounded
service-level cache, invalidation metrics, and representative p99 load evidence
remain required before this is treated as a production latency claim.

The graph boundary now has executable evidence for cycle rejection, duplicate
component-ID rejection, dependency-before-dependent evaluation, and compiled
versus uncached result equivalence. These tests prove domain ordering and
validation behavior; they do not replace load, invalidation, or operational
cache evidence.

ADR-0049 now adds the first bounded application cache: the PostgreSQL quote
adapter reuses immutable compiled rules by verified canonical digest, with a
256-entry LRU and hit/miss/eviction counters. The database still performs the
published/effective rule lookup on every request. Metrics export, invalidation
drills, and concurrent p99 evidence remain open production gates.

ADR-0069 binds every new result to the semantic engine version and compares the
complete component trace during replay. ADR-0070 persists the bounded normalized
`TaxFacts` document in the same transaction as the calculation and outbox event,
and adds the authenticated `GET /v1/tax/quotes/{calculation_id}/replay`
operation. Focused storage tests and the Docker integration harness together
prove facts persistence, tenant scoping, digest/rule/result verification, and
fail-closed legacy behavior;
backup/restore, retention, and migration rollback drills remain production
gates.

ADR-0071 adds `make backup-restore-drill`, which initializes a clean pinned
PostgreSQL 18 source from every migration and synthetic fixture, performs a
custom-format logical dump, restores it into a separate disposable target, and
checks the replay-era schema and UUIDv7 contract. ADR-0182 adds
`make encrypted-backup-restore-drill`, which verifies an encrypted artifact and
rejects tampered ciphertext before restore. Production KMS/HSM envelope
encryption, PITR, encrypted off-site retention, RPO/RTO measurement, and
live rollback authorization remain open; ADR-0183 adds the isolated recovery
rehearsal contract below.

## ADR-0183 — expand/contract migration rollback rehearsal

Add `make migration-rollback-rehearsal` as a bounded Docker recovery contract.
The drill captures a pre-deployment PostgreSQL 18 image, applies a synthetic
additive migration, restores the pre-deployment image into an isolated target,
proves the old-schema read and sentinel-data contract, and reapplies the
forward migration idempotently. It does not claim production RPO/RTO,
KMS/HSM key recovery, off-site retention, or live rollback authorization.

## ADR-0184 — typed finalized adjustment proof envelope

Persist the exact finalized adjustment event index and evidence digest on the
chain intent, then expose a server-owned typed proof envelope in the API only
after complete indexed-event identity matching. The browser rejects a
status-only `finalized` response and renders finality evidence separately from
payment, filing, or legal-compliance claims. Migration `0048` is additive;
legacy finalized rows without the proof binding fail closed. The clean Docker
integration gate proves the migration/schema contract, adjustment lifecycle,
and proof-bound API/browser behavior; live wallet, provider, validator, and
production accessibility evidence remain open.

ADR-0076 adds the authenticated tax-rule governance control plane. The
`draft → simulated → reviewed → approved → published → retired` workflow
requires exact JWT scopes and roles, distinct author/reviewer/approver
subjects, serializable revision fencing, and an append-only governance event
in the same PostgreSQL transaction. The isolated integration harness proves
the complete promotion-and-retirement path, same-subject review rejection,
event immutability, and the corrective-release rollback contract: version 1 is
retired and version 2 is independently promoted and selected for calculation.
Identity-provider role lifecycle, legal/tax owner attestation, and production
ceremony rehearsal remain release gates.

ADR-0077 adds the governance read model and production operator workspace.
`GET /v1/tax/rules/{rule_set_id}/governance` returns a repeatable-read,
tenant-scoped snapshot of rule identity, duty assignments, effective window,
current revision, and ordered transition evidence. The React SPA renders real
loading, authorization, stale/offline, retry, transition, and retired states;
browser role checks are presentation-only and every mutation is reauthorized by
the API. Read-model integration, browser accessibility, and production identity
provider exercises remain release gates.

ADR-0074 adds bounded NGINX per-source-IP request-rate and concurrent-connection
admission before API proxying, with `429` behavior and `nginx -t` validation in
the immutable web image. This is edge backpressure evidence only; production
source-IP modeling, tenant quotas, load tuning, and alerting remain open.

ADR-0075 adds pinned Criterion microbenchmarks for tax-rule compilation,
uncached calculation, and immutable compiled-graph calculation. The explicit
`make bench-tax-engine` target provides hot-path evidence; service-level
concurrency and regional p50/p95/p99 evidence remain separate release gates.

ADR-0232 adds `make benchmark-tax-engine-service`, a bounded Docker workload
against the real warm Tax Engine image. It warms one immutable rule digest,
drives 256 authenticated requests at concurrency 32 by default, validates
exact tax/net outputs for every response, and reports p50/p95/p99 with image,
architecture, timeout, and service-limit metadata. This is local synthetic
service evidence; regional, resource-profile, mTLS, consumer-authorization,
and production SLO evidence remain separate gates.

ADR-0381 adds bounded process-local single-flight compilation keyed by the
canonical rule digest. Same-digest cache misses now share one immutable graph
compilation and one typed failure while unrelated digests retain independent
cache work. The semaphore bounds the in-flight map; a concurrent TDD contract
proves one compilation, result sharing, cleanup, and subsequent LRU reuse.
Cross-replica coalescing, invalidation, regional resource profiles, and
production SLO evidence remain separate gates.

ADR-0078 adds `make benchmark-quote-service`, a bounded Docker-isolated HTTP
workload with ephemeral EdDSA credentials, unique idempotency keys, explicit
status/error accounting, PostgreSQL duplicate-prevention checks, and p50/p95/p99
output. It is local service evidence only; production-region SLO acceptance,
representative data volume, and resource-profile evidence remain release gates.
ADR-0090 extends the quote load with bounded identical-key concurrency and
asserts one idempotency record, calculation, and outbox event. This closes
the deterministic retry-contention case; production-volume fan-out and
regional database evidence remain separate gates.
ADR-0089 adds `make postgres-contention-drill`, which runs against the pinned
PostgreSQL 18 image, proves the 250 ms lock and 900 ms statement boundaries,
checks rollback after a timed-out contender, and asserts the required evidence
and replay indexes. Production-like pool saturation, account-lock contention,
partition/retention, and regional database evidence remain separate gates.
The quote adapter now retries only SQLSTATE `40001`/`40P01` around the complete
transaction with three total attempts and bounded backoff after the load gate
exposed serialization contention.

ADR-0081 hardens the static edge with a strict same-origin CSP and executable
header, authentication, and deny-by-default CORS assertions in the Docker
integration harness. Production TLS/HSTS ownership, WAF policy, and browser
compatibility evidence remain separate release gates.

ADR-0082 makes request correlation an API-owned boundary. Rust middleware
generates a UUIDv7 for every request, overwrites forged or edge-generated
values, attaches it to the tracing span, and returns it on direct and NGINX
responses. Router error tests and the Docker integration harness prove the
behavior; cross-service propagation and production trace-export evidence
remain operational release gates.
The shared React API-error boundary validates the returned UUIDv7 and displays
it as a support reference across quote, governance, and checkout failure
states.

ADR-0110 adds the stable-Rust adversarial account-meta gate. The settlement
coordinator property suite constructs actual payer-signed transactions and
substitutes every compiled account position and writable privilege; the
verifier must reject each mutation before provider submission. The dedicated
CI job runs this bounded gate on every change. Validator-backed CPI, Token-2022
extension, rollback, and independent program-audit evidence remain open.

ADR-0111 adds the first executable frontend accessibility gate. The primary
quote and all lazy operational workspaces are rendered with Testing Library in
the pinned happy-dom environment and audited by axe-core for WCAG 2.0 A/AA
rules. The normal web CI job also runs the focused contract explicitly. Full
browser keyboard/focus, responsive, RTL, localization, and
assistive-technology evidence remain required before the broad WCAG 2.2 AA
gate closes.

ADR-0112 adds fixed-cardinality PostgreSQL pool gauges sourced from the
deadpool status snapshot without acquiring a connection. The local capacity
drill will constrain the pool, exercise quote/idempotency contention, and
sample waiters during load. The 2026-09-27 run observed peak waiting=3 with a
one-connection pool and zero errors; production pool sizing and regional
data-volume, retention, and index-plan evidence remain open.

ADR-0113 adds the PostgreSQL evidence index-plan drill. It seeds bounded
synthetic raw-chain, receipt, settlement-intent, and evidence-audit rows, then
asserts the normal planner selects the declared scoped indexes for production
query shapes. The 2026-09-27 run passed with 25,000 rows in each evidence
family, 25,000 tenant intents, and 10,000 lifecycle events; all six declared
indexes were selected. It deliberately does not delete evidence or claim a
retention policy; partition/archive/PITR ownership and production-volume plans
remain open.

ADR-0114 adds `make postgres-pitr-drill`, which starts the pinned PostgreSQL 18
image with WAL archiving, takes a physical base backup, records an exact target
LSN between two synthetic marker transactions, and restores to that LSN in a
disposable target. The target marker must exist while the post-target marker
must be absent. This proves local PostgreSQL recovery mechanics only; encrypted
off-site WAL, archive monitoring, measured RPO/RTO, retention ownership, and
production rollback authorization remain production gates. ADR-0183 separately
exercises the pre-deploy restore and forward-recovery migration sequence.
The 2026-09-27 run passed at target LSN `0/4006470` with six archived WAL files
and 1 second of local recovery time; this is not a production RTO claim.

ADR-0115 unifies the indexer and settlement provider-routing health contract.
The bounded process-local score/cooldown component orders available settlement
providers by health, quarantines degraded endpoints for a capped interval, and
preserves fail-closed stale-blockhash handling and immutable signed bytes.
Live multi-region failover and provider SLO evidence remain separate gates.

ADR-0116 adds a deterministic legacy settlement wire budget: the compiler
enforces a 976-byte admission ceiling beneath Solana's 1,232-byte packet
maximum, preserving a 256-byte margin, and the coordinator rejects unused
static account keys. ADR-0162 now governs the compute-unit/priority-fee
prefix; validator calibration, v1, and v0+ALT target-cluster evidence remain
release gates.

ADR-0117 exposes fixed-cardinality settlement provider-health gauges from the
API metrics endpoint. Only bounded provider indices, scores, consecutive
failures, and cooldown milliseconds are emitted; endpoint URLs and transaction
identity are deliberately redacted. Production scrape policy, alert delivery,
and SLO evidence remain operational gates.

ADR-0083 adds `db/verification/uuidv7_contract.sql`, an executable catalog
contract for every application-generated sortable UUID. Clean-stack
integration and the logical restore drill now fail closed on missing native
UUID types, `uuidv7()` defaults, or version-7 constraints; CSPRNG claim tokens,
external identity claims, hashes, and Solana-native identifiers remain
explicitly excluded.
The first restore-drill run found that `settlement_route_verifications.id`
was missing its version check; migration `0013` now adds that constraint
idempotently, and both restore and clean-stack evidence pass.

ADR-0084 adds the bounded observability alert contract. Prometheus rules for
service availability, aggregate request error ratio, quote/compile p95
latency, and cache pressure are versioned beside the pinned Compose
configuration. `make observability-config` runs `promtool check config`,
`promtool check rules`, and `amtool check-config` against the exact images used
by the local profile and CI. ADR-0191 and `make alert-delivery-drill` prove
the local Prometheus-to-Alertmanager-to-receiver path; production receiver
authentication, notification delivery, and load-derived SLO evidence remain
open gates.

ADR-0085 strengthens the validator-backed release gate with a bounded,
public-safe evidence bundle. The conformance runner records the exact source
revision, runtime and Compose metadata, SBF hash, validator logs, Compose
configuration, and browser test output before isolated cleanup; CI retains
the bundle on both success and failure, always runs the postcondition verifier,
and treats missing evidence as a hard upload error. The release-candidate
workflow retains the validator bundle immediately on failure before later
release steps can be skipped. A successful retained bundle is still required
before live settlement correctness gates can be closed.

ADR-0086 adds the Indexer Operations bounded context. Operators inspect
bounded replay-gap projections and acknowledge or requeue dead-lettered work
through exact scope/role-protected API routes. PostgreSQL records each action
in an immutable UUIDv7 ledger, and requeue is atomic with retry-state reset.
The Docker integration exercise proves state conflicts, acknowledgement versus
repair semantics, action immutability, and the requirement for fresh review
after requeue; production identity governance, alert delivery, and replay
drills remain release gates.

ADR-0087 adds the corresponding lazy-loaded Replay operations workspace. The
UI validates server responses, keeps the bearer token in memory, requires
bounded notes, exposes only server-authorized actions, and reloads the
projection after mutations. Parser tests cover unsafe IDs, slots, statuses,
timestamps, and acknowledgement fields; browser state never implies that
projection repair equals finalized chain settlement.

ADR-0030's database-enforced published-rule immutability and half-open window
exclusion now have integration evidence: the Docker harness rejects both a
published document mutation and an overlapping published release. Workflow
separation and production governance remain open.

ADR-0072 adds exact financial-boundary vectors and generated `u64` amount
properties for rates, floors, caps, rounding ties, and gross conservation.
These are off-chain arithmetic evidence; validator CPI and Token-2022 runtime
evidence remain separate gates.

ADR-0073 exposes the immutable compiled-rule cache's hit, miss, eviction, and
entry counters through fixed-label API Prometheus metrics. It does not turn the
cache into an authority; concurrent-load, invalidation, and p99 evidence remain
performance gates.

ADR-0050 adds the first bounded metrics boundary. The API records fixed-series
`tax_calculate`, `tax_document_calculate`, and `tax_quote` request counts,
errors, and latency histograms at `/metrics`; unknown operation names cannot
create metric series. Worker and indexer instrumentation, scrape access policy,
OpenTelemetry export, dashboards, alert routing, and load-derived percentile
evidence remain follow-on gates.

ADR-0051 now defines the profiled Docker-local topology for Redis, Prometheus,
Grafana, and OpenTelemetry Collector. The Compose files pin stable image tags
and manifest digests, provision a service dashboard, and keep telemetry out of
the tax authority path. Production scrape authentication, alert routing,
retention, image scanning, and representative load evidence remain release
gates.

ADR-0052 adds the next protocol safety slice: a mandatory singleton
`ProtocolConfig` PDA with a governance-controlled pause bit checked before any
settlement CPI. Pause/unpause remains separate from program upgrade authority;
policy revocation and governance recovery remain available while paused. The
localnet transaction drill and multisig/timelock deployment evidence remain
open release gates.

ADR-0131 adds the operational evidence half of the pause control. The
finalized indexer decodes and durably projects `ProtocolPauseChanged` events,
exposes fixed-cardinality active/unknown state signals, pages through the
emergency-pause runbook, and the validator conformance harness proves that
paused settlement leaves token balances and receipt state unchanged. Staging
multisig/timelock rehearsal, independent RPC verification, alert delivery, and
incident-record evidence remain open deployment gates.

The clean PostgreSQL bootstrap now mounts migration `0030` and runs an
automated migration-bootstrap contract. The protocol-pause projection has
database-level update/delete guards, and the Docker integration drill verifies
both table presence and append-only behavior before exercising the service
flows.

ADR-0053 closes the previously documented-but-unimplemented tax semantics for
thresholds, exemption codes, and cumulative-period snapshots. The engine now
requires explicit threshold basis and activation mode, binds cumulative inputs
to a supplied evidence digest, and fails closed when required evidence is
missing. Inclusive-threshold equations and jurisdiction-specific legal rule
publication remain separate governance work.

ADR-0055 adds the first deterministic settlement compiler boundary. It derives
the fixed PDA set and canonical Anchor account order, validates amount
conservation and token-program scope, and returns a wallet-reviewable
`prepared_unsubmitted` instruction without RPC, signing, or confirmation. A
live transaction coordinator remains a separate release slice.

ADR-0056 adds the bounded non-custodial transaction coordinator boundary. It
validates canonical wallet-signed bytes, preserves immutable retry bytes,
submits with explicit preflight policy, and maps provider commitment evidence
without overclaiming finality. ADR-0057 adds UUIDv7 durable settlement intents
and append-only lifecycle events; the guarded storage/API submit path now
exists. ADR-0058 adds cryptographic binding of the exact signed Solana message
to the compiled instruction: signature verification, exact program/data/account
comparison, no unresolved address-table lookups, and a one-instruction policy
run before any durable transition or RPC call. Deployment enablement remains a
release gate until blockhash policy, localnet, and staging evidence exist.

ADR-0118 hardens the transaction coordinator's concurrency boundary. A verified
wallet-signed message derives its native Solana signature before the RPC call;
the durable intent must win a `signed -> submitting` compare-and-set before
`sendTransaction` is allowed. Provider responses must match that signature,
ambiguous outcomes become visible `unknown` evidence, and reconciliation uses
the persisted signature without resubmitting automatically. Validator-backed
and multi-replica submission drills remain release evidence. The Docker
local-stack drill now proves one concurrent `200`/one `409`, exactly one
validating fixture `sendTransaction`, and durable `submitted` readback; crash,
live-validator, multi-replica, and production RPC-failover evidence remain
open. ADR-0119 extends that drill with an accepted-but-ambiguous provider
response: the API persists `unknown`, retains the exact native signature, and
reconciles to `confirmed` without resubmitting. A real process-crash drill
is now implemented under ADR-0120: the API is killed after the durable
`submitting` claim, restarted, and reconciles the retained signature without a
second send. Live validator, multi-replica, and production RPC-failover
evidence remain open. ADR-0121 removes detached health-server tasks from the
worker, indexer, and route-verifier lifecycles: each health server is a pinned
future owned by the structured service loop, unexpected termination fails the
process, and normal shutdown cancels it with the rest of the service. This is
service-lifecycle evidence; supervisor, orchestration, and multi-replica
failure evidence remain open.

ADR-0060 now binds compiler input to an authenticated merchant's persisted,
expiring quote. The API owns gross/tax/net amounts at compilation time and
rejects stale quotes; the checkout UI displays the expiry and must restart the
quote flow when it elapses. Merchant Solana account registry resolution is
still an explicit pre-mainnet gate.

ADR-0029 is the security gate for this slice. A caller-supplied merchant UUID
is not accepted by the persisted quote contract. Local Compose uses a
synthetic public key only; no private key is committed, and the integration
harness creates an ephemeral signing key solely for its test process.

Exit criteria: the same rule version and normalized facts produce byte-identical results across repeated runs; every tax total is explainable; unsupported or ambiguous rules fail closed; a published calculation can be independently replayed from its evidence bundle.

ADR-0122 closes the tax-rule document compatibility boundary. New rule
publications persist a `solana-tax-protocol/tax-rule/v1` `RuleSetDocument`
envelope while preserving the canonical financial rule digest. Replay accepts
only this envelope for new evidence and a bounded legacy bare-document branch
for immutable historical rows; unknown schemas fail closed.

ADR-0123 adds fixed-cardinality indexer replay gauges sourced from the
configured stream's durable PostgreSQL gap ledger. Queued, leased,
dead-lettered, and leased-slot-span metrics feed validated Prometheus alerts
without exposing stream keys as labels. Lease-expiry/claim-loss counters,
receiver delivery, and production on-call evidence remain follow-on gates.

ADR-0124 adds a bounded finalized signature-pagination adapter for targeted
recovery. It is discovery-only and cannot mutate projections; the next
indexer recovery slice is a separately authorized signature-to-evidence worker
with a fixed page budget and finalized backfill contract.

ADR-0125 implements that bounded recovery slice: the authenticated API accepts
one operator-selected signature, discovers it within a fixed page budget,
fetches its finalized transaction with the versioned RPC contract, verifies the
response identity, and persists raw and normalized evidence through the
existing replay projection boundary. The endpoint appends immutable
requested/recovered/failed audit evidence. Live provider/failover drills,
crash reconciliation alerting, and fleet SLO evidence remain production gates.

ADR-0126 closes the observability implementation gap for that two-phase audit:
the indexer refresh loop exports a fixed-cardinality count of requested rows
older than five minutes without a terminal outcome, and Prometheus alerts on a
non-zero sustained value. The metric never carries target identity or mutates
chain/projection state; production notification delivery and live
reconciliation drills remain gates.

## Phase 4 - indexer and reconciliation

ADR-0036 and ADR-0040 define and `crates/indexer` implements the bounded domain
and coordinator slice:
finalized checkpoint planning, bounded gap/backfill semantics,
duplicate/conflict handling, exact atomic-unit reconciliation, finalized
provider-page validation, a bounded Rustls HTTP JSON-RPC adapter for finalized
`getSlot`, `getBlocks`, and `getBlock` calls, and a three-provider
deadline-bounded failover wrapper. The storage adapter now
persists finalized pages, checkpoints, normalized receipt evidence, bounded raw
Anchor event bytes, and append-only reconciliation runs atomically. ADR-0041
defines the pinned `SettlementExecuted` decoder, domain-separated evidence
digest, conservation checks, and one-event-per-signature compatibility boundary.
ADR-0042 adds process-local atomic provider health scoring and capped cooldowns
to the failover pool without changing finalized-only validation. Signature
pagination and durable gap leases remain follow-on implementation work. ADR-0043
now provides the pure bounded replay-gap queue: typed reasons, range coalescing,
bounded splitting, capacity limits, and explicit retry exhaustion. Replay/rebuild
operator tooling now has the PostgreSQL lease/state adapter defined by ADR-0044;
ADR-0045 adds a separately deployable `indexer-service` that connects finalized
RPC, the pinned settlement decoder, health-scored failover, serializable page
persistence, and replay-gap creation. ADR-0046 adds lease-fenced replay
execution: one bounded chunk per tick, idempotent historical projection writes,
durable suffix advancement, and no forward-checkpoint mutation. Raw
full-transaction retention is now defined by ADR-0047 and persisted in a
bounded evidence table. Signature pagination and localnet transaction tests
remain follow-on implementation work. ADR-0088 adds the deterministic
raw-evidence decoder, bounded projection rebuild primitive, Docker drill, and
CI job. The receipt-projection rebuild gate is evidenced; merchant, vault, and
reporting projection rebuilds require separate raw-evidence contracts.

Deliverables:

- subscription consumer and slot-based backfill;
- append-only raw event store;
- receipt, vault, and merchant read models;
- checkpoint, gap detection, replay, and balance reconciliation;
- operator tooling for a signature, receipt, and vault audit trail.

Exit criteria: delete and rebuild projections from chain evidence with identical results; injected missed events are detected and repaired.

## Phase 5 - React merchant and checkout experience

ADR-0059 establishes the non-custodial checkout wallet handoff. The first
implementation slice is a real transaction-review surface: it constructs one
compiled settlement instruction, requests a wallet signature through a narrow
browser port, submits unchanged signed bytes with the compiled artifact, and
surfaces durable intent/reconciliation states. It does not fabricate success,
custody keys, or add unapproved instructions. Tenant-to-Solana-account
configuration, quote expiry, wallet matrix, and localnet evidence remain
release gates. ADR-0061 adds the tenant-scoped settlement route registry:
quote-bound compilation resolves merchant/mint/vault/destination/token
configuration from authenticated tenant state and derives the payer source
ATA without RPC or ATA creation in the hot path.
ADR-0062 adds a separate finalized route-verifier service. It validates the
merchant system account, mint, extension-free token accounts, token-program
owners, mint bindings, and deterministic vault authority through bounded
multi-provider RPC before atomically activating a route and recording
append-only evidence. The service is never called from checkout or settlement
submission.

ADR-0063 adds the shared `test-vectors/settlement-instruction-v1.json`
contract. Rust compiler tests and TypeScript wallet-boundary tests consume the
same versioned artifact and compare the complete instruction, not only its
data. This closes cross-language serialization drift for the first settlement
contract; localnet execution and broader API/attestation vectors remain open.
ADR-0064 adds a bounded browser-side envelope check before blockhash lookup or
wallet signing. It is defense in depth only; the API still verifies the actual
signed message before submission.

Deliverables:

- wallet connection and transaction review;
- quote expiry and policy version display;
- merchant split-plan and asset configuration;
- settlement status with commitment-level explanation;
- vault balance and release-claim views;
- tax breakdown, “why this tax?” explanation, and calculation-evidence views;
- tax rule authoring, simulator, review, approval, publishing, rollback, and audit screens;
- accessible, responsive SPA served by NGINX.

Exit criteria: wallet rejection, expiry, timeout, duplicate, and reconciliation-required states are understandable and recoverable.

## Phase 6 - production hardening

Deliverables:

- independent program audit and remediation;
- fuzz, property, load, chaos, and RPC-failover tests;
- key ceremony, recovery, rotation, and compromise runbook;
- canary deployment and rollback rehearsal;
- SBOM, signed artifacts, dependency policy, and vulnerability management;
- mainnet readiness review.
- public-safe operational runbooks for policy compromise, upgrade rollback,
  vault release incidents, RPC outages, indexer gaps, and backup/restore.

Exit criteria: all production-readiness gates in [production-readiness.md](production-readiness.md) are green and legal sign-off is recorded.

## Phase 7 - controlled production and public release

### Release integrity boundary (ADR-0129)

The repository now has a release-candidate workflow and Dependabot policy that
keep dependency maintenance visible and produce a bounded evidence bundle for
each requested tag/manual run. The bundle records the source revision,
toolchain, checksums, SBOM, security result, Docker metadata, web/Rust/SBF
artifacts, and validator evidence. It does not publish images, deploy funds,
or select a license. A successful workflow run and independent release review
are still required before any public source or production release claim.

### Deployment environment safety boundary (ADR-0130)

API startup now requires an explicit `API_ENV` of `local`, `staging`, or
`production`, rejects simulation and insecure RPC flags outside local, and
uses the validated result for all RPC construction. Compose is explicitly
labeled local. This prevents unsafe configuration combinations but does not
replace staging/production secret provisioning, launch approval, key
ceremonies, or mainnet readiness evidence.

### Non-local deployment configuration contract (ADR-0139)

The repository now has a fail-closed preflight for staging and production
manifests. It requires matching non-local service labels, explicit disabled
simulation/insecure-RPC flags, TLS-verified PostgreSQL URLs, HTTPS RPC
providers, non-local identity/evidence/jurisdiction inputs, and no synthetic
seed directory. `make deployment-config-contract` exercises positive and
negative cases. This is a configuration safety boundary; actual secret
provisioning, provider health, launch approval, and live deployment evidence
remain release gates.

### Validator-backed Split Plan conformance (ADR-0140)

The validator conformance suite now includes the bounded two-recipient Split
Plan path. It initializes a real immutable plan, executes legacy-SPL fan-out,
checks finalized source/vault/recipient conservation, rejects duplicate receipt
nonces, and rejects wrong remaining-account order without balance movement.
This is local validator evidence only; program audit, target-cluster compute
and priority-fee measurements, staging governance, and production approval
remain separate gates.

### Extension-free Token-2022 Split Plan conformance (ADR-0214)

- [x] Record the compatibility boundary, alternatives, invariants, and
  fail-closed extension policy in ADR-0214.
- [x] Add a real extension-free Token-2022 two-recipient Split Plan case to
  the pinned validator client, including finalized 0/25/45/30 balance
  assertions.
- [ ] Retain successful Linux/amd64 validator evidence for this case before
  marking Token-2022 Split Plan compatibility as a release-complete gate.

### Validator-backed policy revocation conformance (ADR-0141)

The conformance suite also executes the governed revocation path against a
fresh legacy-SPL mint, then proves that settlement with the revoked attestation
fails before token movement. The authority, PDA binding, and unchanged balance
assertions are runtime evidence; Linux validator execution, program audit,
staging governance, and production approval remain separate gates.

### SPA workspace focus management (ADR-0142)

The React shell now exposes a skip link and stable workspace landmark, marks the
active workspace control, and moves focus to the committed workspace boundary
after user navigation. The initial render does not steal focus. This closes a
keyboard-context defect while full browser, RTL, localization, and assistive-
technology evidence remain separate WCAG readiness gates.

### Structured Tax Engine rule workbench (ADR-0143)

The governance workspace uses bounded schema-driven controls for the complete
supported rule DSL: component dependencies, flat/progressive rates, threshold
semantics, exemptions, price mode, credit, floor/cap, truncation/rounding, and
generated canonical JSON. Advanced JSON remains an explicit expert mode for
forward-compatible interchange. The browser constructs and previews documents;
the Rust service remains the only validation, canonicalization, governance,
and publication authority. Workbench tests must cover valid and invalid rule
shapes before the UI slice is considered complete.

### Server-owned persisted-rule simulation (ADR-0144)

Governance simulation posts only facts to
`/v1/tax/rules/{rule_set_id}/simulate`. The service derives tenant scope from
JWT claims, loads the persisted rule, verifies the versioned document identity
and canonical digest, rejects retired rules and client jurisdiction evidence,
and uses the warm compiled-rule cache. The response is parsed by a bounded SPA
validator and is explicitly unattested; it cannot be used as a quote,
attestation, publication, or settlement input.

`make integration-check` now proves the endpoint against the pinned PostgreSQL
18/API Docker stack: successful server-owned calculation, cross-tenant 404,
jurisdiction mismatch 422, nested unknown-field 400, tampered-document 500,
canonical recovery 200, and retired-rule 409. The next evidence slice is
covered by ADR-0145: `make benchmark-rule-simulation` runs a facts-only,
two-replica Docker workload with response-binding checks, read contention, and
p50/p95/p99 output. It enforces the declared local p95 diagnostic budget unless
an explicit exception is set. The default Docker run on 2026-09-27 measured
1.958/3.001/6.125 ms at p50/p95/p99 with zero errors, exact digest binding,
64/64 successful read-contention responses, and zero observed pool waiters.
Browser
WCAG/RTL/localization and regional latency measurements. Live validator,
independent audit, legal/tax ownership, and production key/approval gates
remain unchanged.

### Centralized locale, time-zone, and RTL display contract (ADR-0146)

The SPA now owns explicit display settings for bounded locales (`en-US`,
`de-DE`, `ar-EG`, and `ja-JP`) and an operator-selected IANA time zone. A
dependency-free formatter module keeps atomic values as `BigInt`, localizes
grouping/decimal separators, formats evidence timestamps consistently, and
derives document direction for RTL locales. Lazy workspaces receive the
context through props so they do not depend on ambient browser defaults.
Invalid time zones are visible and render with a UTC fallback. Formatter,
settings, direction, and rendered axe-core tests pass; human screen-reader,
real-browser responsive, full translation-catalog, and legal-copy review
remain release gates.

### Lazy workspace error containment (ADR-0147)

The selected code-split workspace is now wrapped in a bounded React error
boundary. Rejected dynamic imports and render-time workspace exceptions leave
the global shell and navigation available, expose an accessible reload state,
and never render raw exception details or imply a financial/chain outcome.
Boundary tests prove redacted failure rendering; real-browser chunk-failure,
CSP/cache, and assistive-technology evidence remain release gates.

### Reproducible real-browser UI journeys (ADR-0148)

The web platform now includes an opt-in Docker Compose `browser` profile with
an official Playwright 1.63.0 runner pinned by immutable image digest. It
executes against the built NGINX edge and proves keyboard skip/focus behavior,
real lazy workspace loading, narrow RTL overflow, reduced-motion behavior,
redacted dynamic-import recovery, and the server-bound finalized refund-proof
disclosure/reload journey. It also injects the locked dev-only `axe-core`
source into the real browser and audits the quote, API-unavailable state,
every lazy workspace, and the refund-proof surface against WCAG A/AA tags. Use
`make web-browser-check`; the current Docker run type-checks the e2e project
before launching and passes 48/48 test cases, including expired capability
snapshot and bounded reconciliation timeout handling, while auditing the
covered surfaces. This evidence uses
synthetic API fixtures and does not claim live wallet or chain finality.
CI retains failure traces, screenshots, videos,
and JSON results under the ignored `test-results/web-e2e` path. This automated
contract complements the fast axe-core baseline and does not claim
screen-reader, legal/tax, live RPC, or mainnet evidence.

### Secure browser idempotency identity (ADR-0149)

Real-browser evidence exposed that `crypto.randomUUID()` is not available in
the local HTTP secure-context boundary. The SPA now uses one CSPRNG-only
client identity helper: native `randomUUID()` where available and a canonical
UUIDv4 `getRandomValues()` fallback otherwise. The helper is limited to opaque
client idempotency keys; all authoritative server entity identifiers remain
UUIDv7. Missing CSPRNG support fails closed and cannot blank the application
shell silently.

Deliverables:

- controlled mainnet launch with allow-listed merchants, assets, and published jurisdictions;
- live SLO/error-budget dashboards and staffed incident response;
- completed restore, key-recovery, RPC-failover, indexer-replay, and rule-rollback drills;
- signed release artifacts, SBOM, provenance, migration notes, and public changelog;
- public coverage matrix, support policy, security disclosure workflow, and release notes;
- post-launch review against latency, correctness, tax coverage, UX, and operational evidence.

Exit criteria: controlled production meets the Tax Engine product standard for the published scope; general availability is a separate approval, not an automatic consequence of deployment.

## ADR-0262 UI and authorization delivery status

- [x] Replace the single tab strip with a responsive SaaS control-plane shell
  grouped by operation, governance, money movement, and observation.
- [x] Add a typed capability catalog with explicit action risk, required
  scopes, roles, and workspace ownership.
- [x] Add a dedicated RBAC/PBAC/ABAC access-control workspace with
  fail-closed, server-authority copy and a reviewable permission matrix.
- [x] Preserve real lazy domain panels, focus management, error containment,
  locale/time-zone/RTL handling, and WCAG axe coverage.
- [x] Implement `GET /v1/authorization/capabilities` as a server-owned,
  tenant-scoped, expiring capability snapshot and contract-test it against API
  authorization constants. The browser remains indeterminate until this
  contract is available and staging authorization evidence is retained; it
  never personalizes protected controls from decoded JWT claims.
- [ ] Add real-browser visual/responsive, screen-reader, CSP, and multi-tenant
  authorization evidence before any production UI authorization claim.

## ADR-0309 contextual status disclosure

- [x] Consolidate deployment, Solana-network, service, and unconfigured
  sign-in diagnostics into native Material 3 disclosures while preserving
  complete accessible explanations and fail-closed behavior.
- [x] Remove retired persistent shell-status selectors and add a stylesheet
  contract preventing the former noisy badge pattern from returning.
- [ ] Complete manual assistive-technology and operator usability review for
  the compact status summaries across supported roles, themes, narrow layouts,
  and RTL.

## ADR-0276 multi-tenant capability session reset evidence

- [x] Add a real-browser two-tenant capability-response journey that proves the
  account reference and capability snapshot are replaced when the opaque
  session changes, with no stale tenant reference rendered afterward.
- [ ] Retain staging identity-provider, real multi-tenant API, screen-reader,
  and production authorization evidence.

## ADR-0277 complete workspace capability catalog

- [x] Add the complete server-owned capability catalog for every production
  workspace, including document payments, reconciliation, adjustments, and
  refund lifecycle actions.
- [x] Surface each server decision in navigation without turning the browser
  into an authorization authority; retain exact handler re-authorization.
- [x] Validate the complete catalog in Rust, React, Docker benchmark, and
  real-browser accessibility journeys.
- [ ] Retain staging identity-provider, real multi-tenant, screen-reader,
  regional latency, and production authorization evidence.

## ADR-0278 addressable workspace navigation

- [x] Add an allow-listed typed pathname mapping for every workspace and safe
  root/unknown-path canonicalization.
- [x] Preserve push/back/forward behavior without encoding credentials,
  financial facts, tenant identity, or authorization state in the URL.
- [x] Verify direct deep links and browser history in the Docker browser gate,
  alongside focus and accessibility checks.
- [ ] Retain staging support-link, localization, screen-reader, and release
  manifest evidence.

The UI shell is reviewable and production-shaped, but this slice does not
claim that a browser token is proof of authorization or that the product is
ready for a live tenant without the backend capability contract and release
gates above.

## ADR-0263 task-oriented product language and navigation

- [x] Organize the primary journey around Daily work, Rules & access, Money
  movement, and Records & issues.
- [x] Use outcome-oriented labels for navigation, page titles, statuses,
  buttons, empty states, and failure copy.
- [x] Keep RBAC/PBAC/ABAC names, scopes, digests, and raw evidence behind
  progressive disclosure for administrators and auditors.
- [x] Preserve explicit boundaries: the service decides access, the wallet
  owner signs, finalized chain evidence proves settlement, and tax withholding
  is not filing or legal advice.
- [x] Add focused UI tests for task language, technical disclosure, keyboard
  navigation, RTL layout, and fail-closed access presentation.
- [x] Apply the copy contract to the first high-consequence money-movement
  slice: refund requests, refund releases, and treasury release requests. The
  UI keeps exact financial/address/evidence values reviewable, but presents
  their meaning in operator language and labels recovery/evidence inputs as
  advanced.
- [x] Apply the copy contract to adjustment review and refund cancellation;
  primary labels now describe the operator decision and next step while
  preserving exact evidence and recovery data for controlled review.
- [x] Add ADR-0271's executable UI language contract to the repository quality
  gate so primary navigation and high-consequence workspace labels cannot
  regress into protocol terminology.
- [x] Add ADR-0272's payment-review progressive disclosure. The primary
  payment path now explains the wallet authorization, amount, tax, route, and
  recipient decision in operator language while keeping exact account,
  compute, policy, and intent evidence in explicit accessible disclosures.
- [x] Add ADR-0273's explicit financial-input boundary. Quote, invoice,
  document-payment, and persisted-rule simulation workspaces no longer open
  with example amounts, line facts, tax rules, jurisdictions, or categories;
  only a generated non-secret retry reference is prefilled.
- [x] Add ADR-0274's explicit currency boundary. Aggregate quotes and
  scenarios no longer silently submit USD; currency is entered and normalized
  at the browser boundary while server-returned currency controls display.
- [x] Add ADR-0282's progressive technical disclosure to invoice quoting,
  access review, and guided rule authoring. Tax authors see task language for
  taxable amounts, rate bands, thresholds, credits, and rounding; exact rule
  serialization and audit fingerprints remain available in controlled detail
  views.
- [ ] Review every domain workspace with finance/tax operators and complete
  real-browser, screen-reader, localization, and legal-copy evidence before
  making production UX claims.

## ADR-0264 server-owned capability snapshot

- [x] Add the versioned `GET /v1/authorization/capabilities` contract with
  UUIDv7 snapshot identity, bounded expiry, exact scope/role decisions, and
  fail-closed indeterminate results for missing resource context.
- [x] Consume the snapshot in the React shell without decoding JWT claims or
  making client-side authorization decisions.
- [x] Preserve API re-authorization on every protected operation; the shell
  snapshot is an affordance and explanation contract only.

## ADR-0275 capability snapshot latency evidence

- [x] Add a bounded two-replica Docker HTTP benchmark for the capability
  snapshot. It verifies authenticated semantic responses, bounded concurrent
  load, and p50/p95/p99 output; regional staging SLO, identity-provider, and
  production evidence remain separate gates.
- [x] Execute the default local workload on 2026-10-05: 256/256 HTTP 200
  responses at concurrency 32 on the pinned local arm64 image with the complete
  nineteen-capability catalog, with p50/p95/p99 of 1.141/3.276/4.943 ms; the
  25 ms local p95 diagnostic budget was met. The earlier 2026-10-03
  fourteen-capability result remains historical context only.
- [ ] Retain Linux/amd64 CI evidence, resource-profile samples, regional
  measurements, identity-provider integration, and production SLO approval.

## ADR-0265 responsive navigation delivery status

- [x] Replace the narrow-screen icon-only strip with an explicit, labeled
  navigation disclosure.
- [x] Preserve native navigation semantics, accessible expanded state, close on
  workspace selection, and existing workspace focus management.
- [ ] Retain real-browser responsive layout and assistive-technology evidence
  for the complete production tenant surface.

## ADR-0266 quote-to-payment handoff delivery status

- [x] Preserve the server response across the aggregate quote/payment review
  handoff so expiry and authorization states remain visible.
- [x] Clear aggregate quote state when leaving the bounded handoff cluster and
  keep document quote/payment state isolated.
- [x] Prove the expired-quote handoff and disabled compilation in the pinned
  real-browser journey.

## ADR-0267 honest runtime and account context delivery status

- [x] Remove fixed tenant, operator, environment, and network labels from the
  product shell.
- [x] Derive account context only from the verified server capability snapshot
  and show a bounded opaque reference.
- [x] Parse explicit build-time deployment and Solana-cluster context with
  visible `Not configured` failure states and no plausible defaults.
- [x] Pass explicit local context from the Docker edge build and preserve the
  values as release configuration inputs.
- [ ] Retain staging/production release-manifest evidence proving the displayed
  context matches the approved deployment and cluster.

## ADR-0268 browser OIDC PKCE session delivery status

- [x] Add strict public-client configuration parsing for issuer, client ID,
  redirect URI, scopes, and local-only authentication mode.
- [x] Implement bounded PKCE transaction creation, provider discovery, callback
  validation, and one-time authorization-code exchange.
- [x] Keep access tokens in memory only and make all protected workspaces use
  one session boundary.
- [x] Hide synthetic credential entry from every staging/production build and
  fail closed when OIDC configuration is absent or invalid.
- [ ] Retain staging identity-provider evidence for redirect registration,
  issuer/audience mapping, key rotation, revocation, logout, and outage.

## ADR-0269 OIDC origin and CSP delivery status

- [x] Bind the configured issuer origin into the immutable NGINX CSP without
  permitting wildcard HTTPS browser egress.
- [x] Reject discovery metadata whose authorization or token endpoint is not
  on the configured issuer origin.
- [x] Prove local same-origin CSP, configured-provider CSP, and generated NGINX
  configuration with Docker checks.
- [ ] Retain staging evidence for provider-origin configuration, CORS, and
  provider endpoint rotation.

## ADR-0270 browser OIDC ID-token validation delivery status

- [x] Add bounded provider JWKS discovery and strict ID-token validation before
  establishing a browser session.
- [x] Require the returned ID token to bind issuer, client audience, nonce,
  subject, and bounded time claims; reject unsupported algorithms and key types.
- [x] Add key-rotation retry, malformed-JWKS, oversized-response, and invalid
  ID-token session tests.
- [x] Load the validator only during callback completion so JOSE verification
  does not inflate the initial product bundle or settlement-facing workspaces.
- [ ] Retain staging evidence for provider JWKS rotation and algorithm policy.

## ADR-0280 browser credential presentation delivery status

- [x] Remove bearer-token rendering from domain workspaces while preserving
  authenticated API requests through the in-memory session; the shared shell
  authentication control is the sole credential-entry surface.
- [x] Replace example-oriented governance copy with an explicit controlled
  simulation action that cannot be mistaken for a live quote or settlement.
- [x] Add component, accessibility, typecheck, and UI-language evidence for the
  session-status presentation boundary.
- [ ] Retain staging identity-provider, logout, expiry, revocation, and support-
  artifact redaction evidence.

## ADR-0281 automatic quote retry-reference delivery status

- [x] Remove editable idempotency UUID fields from aggregate and document quote
  workspaces.
- [x] Keep the server API contract unchanged while generating a fresh in-memory
  CSPRNG reference whenever semantic quote inputs change.
- [x] Preserve the same reference across retries when the input set is
  unchanged, with plain-language UI explaining the protection.
- [ ] Retain staging retry/timeout evidence and support-runbook confirmation
  that operators do not copy implementation identifiers into quote forms.

## Version and dependency governance

As of 2026-10-06, the official release channels still support the approved
baseline: Rust stable 1.98, Agave/Solana 4.3.0, React 19.3, Node.js 24.21.0
LTS, pnpm 12.3.4, Solidity 0.8.37, Anchor CLI 1.2.0, and NGINX stable 1.30.5.
Node 26 is current but is not selected because the production policy requires
an active or maintenance LTS runtime; Agave 4.4/4.5 are not selected while
their release channels remain non-stable. Confirm availability and Solana
compatibility during bootstrap, then pin exact versions in source and CI. Do
not use a floating `stable` or `latest` channel in production CI. The
off-chain workspace remains on Rust 1.98.1; the on-chain package explicitly
declares Rust 1.95 compatibility because Agave 4.3.0's platform-tools v1.57
SBF compiler reports rustc 1.95.0-dev. See
[ADR-0067](../adr/0067-sbf-compiler-compatibility-boundary.md) and the
[toolchain compatibility matrix](../architecture/toolchain-compatibility.md).

References:

- [Rust Forge current releases](https://forge.rust-lang.org/)
- [React versions](https://react.dev/versions)
- [Node.js release schedule](https://nodejs.org/en/about/previous-releases)
- [Anchor installation and version management](https://www.anchor-lang.com/docs/installation)
- [Anchor changelog](https://www.anchor-lang.com/docs/updates/changelog)
- [Solidity releases](https://www.soliditylang.org/blog/category/releases/)
- [NGINX news and stable releases](https://nginx.org/news.html)
- [Solana core concepts](https://solana.com/docs/core)
- [Agave releases](https://github.com/anza-xyz/agave/releases)
- [Solana fees and compute budget](https://solana.com/docs/core/fees)
- [Solana Token-2022 extensions](https://solana.com/docs/tokens/extensions)
- [PostgreSQL 18 documentation](https://www.postgresql.org/docs/18/)
- [PostgreSQL UUID functions](https://www.postgresql.org/docs/18/functions-uuid.html)
- [RFC 9562 UUIDs](https://www.rfc-editor.org/rfc/rfc9562.html)

## ADR-0361 distinct tax-base validation hardening

- Reject repeated `Gross`, `Net`, or component references in additive tax
  bases during rule compilation; see ADR-0361.
- Preserve distinct multi-source bases for explicit tax-on-tax composition.
- Keep weighted, subtractive, and general expression bases out of v1 until a
  separately bounded DSL and golden-vector decision exists.

## ADR-0362 explicit inclusive threshold equation

- Add the versioned `GrossExcess` equation for inclusive thresholds.
- Preserve fail-closed rejection for undeclared and unsupported threshold
  meanings.
- Add executable unit and golden-vector evidence before exposing the choice in
  rule-authoring UI.

## ADR-0364 collapsed secondary rule simulation

- Keep governed-draft authoring open as the primary Tax Rules task.
- Collapse the saved-rule simulation by default and expose it through a native,
  keyboard-operable “Try the saved rule” disclosure.
- Preserve the existing authenticated, persisted-rule, server-owned simulation
  boundary; disclosure state is presentation-only.
- Verify the density decision with the governance component test and the full
  real-browser accessibility journey.

## ADR-0365 plain-language authorization recovery copy

- [x] Keep parser diagnostics typed and independent of React/localization.
- [x] Map expiry, missing-expiry, and generic capability failures through the
  localized shell catalog without rendering raw technical errors.
- [x] Add parser, catalog, component, and real-browser accessibility coverage
  for the expired capability state.
- [ ] Retain staging identity-provider, clock-skew, screen-reader, and support
  evidence before a production authorization claim.

See [ADR-0365](../adr/0365-plain-language-authorization-recovery-copy.md).

## ADR-0366 localized wallet-provider error redaction

- [x] Keep arbitrary provider exception details out of the primary wallet UI.
- [x] Preserve the typed multi-account recovery path and do not retry or infer
  wallet authorization in the browser.
- [x] Add component and real-browser rejection coverage with localized copy and
  accessibility assertions.
- [ ] Retain live injected-wallet, screen-reader, provider-matrix, and staging
  support evidence before a production wallet claim.

See [ADR-0366](../adr/0366-wallet-provider-error-redaction.md).

## ADR-0367 financial-workspace error redaction boundary

- [x] Add the typed operator-error boundary and fail-closed fallback mapping
  without changing settlement, wallet, or reconciliation authority.
- [x] Apply it to aggregate/document checkout and refund claim, cancellation,
  and execution first, with unit and browser evidence for API and provider
  diagnostics.
- [x] Extend the boundary to tax-document simulation, rule governance, and
  document quote workspaces with localized operator copy and failing-first
  provider/transport exception tests.
- [x] Extend the boundary to treasury/vault release and reconciliation,
  settlement adjustment, replay operations, revenue streaming, and the
  application shell with localized operator copy and failing-first exception
  tests.
- [ ] Retain live-provider, staging, manual assistive-technology, and support
  evidence before making a production UX claim.

See [ADR-0367](../adr/0367-financial-workspace-error-redaction.md).

## ADR-0368 non-interruptive system-status disclosure

- [x] Keep system configuration and service warnings inside the expandable
  status disclosure without presenting them as assertive financial alerts.
- [x] Preserve actionable workspace failures as reviewed `role="alert"`
  content and keep system context available as ordinary disclosure text.
- [x] Add failing-first shell coverage for the status-role boundary.
- [ ] Retain manual screen-reader and operator-usability evidence before a
  production accessibility claim.

See [ADR-0368](../adr/0368-non-interruptive-system-status-disclosure.md).

## ADR-0369 explicit CI toolchain bootstrap

- [x] Add a failing-closed workflow contract for the exact Rust 1.98.1
  installation, `RUSTUP_TOOLCHAIN` export, and compiler-version evidence.
- [x] Install and select the pinned toolchain before host-side Cargo commands
  in the Rust, adversarial settlement, and release-candidate workflows.
- [x] Include `make ci-toolchain-contract` in `make check` so workflow drift is
  detected locally and in CI.
- [ ] Retain a successful Linux release-candidate run and independently review
  its complete evidence bundle before production enablement.

See [ADR-0369](../adr/0369-explicit-ci-toolchain-bootstrap.md).

## ADR-0370 explicit frontend CI toolchain evidence

- [x] Record resolved Node.js and pnpm versions after pinned setup and before
  dependency installation in CI and release-candidate workflows.
- [x] Extend `make ci-toolchain-contract` to reject missing Node/pnpm pins or
  missing version evidence.
- [ ] Retain a successful Linux release-candidate run and independently review
  its complete evidence bundle before production enablement.

See [ADR-0370](../adr/0370-explicit-frontend-ci-toolchain-evidence.md).

## ADR-0371 production-source Material color boundary

- [x] Extend the Material color contract beyond CSS to production TypeScript
  and TSX source, rejecting raw color functions, literals, and gradients while
  leaving test expectations available for contract coverage.
- [x] Prove the contract fails on a temporary production-source color probe,
  then remove the probe and re-run the clean contract.
- [ ] Retain manual visual, screen-reader, localization, and forced-colors
  review evidence before making a production UX claim.

See [ADR-0371](../adr/0371-production-source-material-color-boundary.md).

## ADR-0372 bounded operational observability dashboard

- [x] Add pause, replay, database-pool, tax-cache, and settlement-provider
  signals to the repository-provisioned read-only Grafana dashboard.
- [x] Add a fixed-cardinality dashboard contract that rejects variables and
  sensitive identity labels, and include it in `make check`.
- [ ] Retain authenticated production scrape, dashboard retention, alert
  delivery, ownership, and load-derived SLO evidence.

See [ADR-0372](../adr/0372-bounded-operational-observability-dashboard.md).

## ADR-0373 tenant-scoped evidence-access explorer

- [x] Add the authenticated, tenant-derived `GET /v1/audit/evidence-access`
  contract with bounded latest-N retrieval and optional UUIDv7 calculation
  filtering.
- [x] Add the read-only Evidence & audit workspace, localized catalogs,
  progressive technical disclosure, and explicit empty/error/unauthorized
  states to the Records & issues navigation group.
- [x] Add TDD, tenant-isolation boundary, serialization, accessibility, and
  real-browser journey coverage in the repository.
- [ ] Retain staging identity, privacy, retention, and manual audit-operator
  review before a production evidence claim.

See [ADR-0373](../adr/0373-tenant-scoped-evidence-access-explorer.md).

## ADR-0374 collapsible Material 3 navigation rail

- [x] Add an expanded-by-default desktop rail with a session-only compact preference.
- [x] Keep localized accessible names, active-workspace orientation, and server-owned authorization unchanged in compact mode.
- [x] Verify keyboard, forced-colors, reduced-motion, typecheck, build, and real-browser behavior.
- [ ] Retain manual RTL, reduced-motion, and assistive-technology review before a production UX claim.

See [ADR-0374](../adr/0374-collapsible-material-navigation-rail.md).

## ADR-0375 bounded linear tax-base operations

- [x] Add the v2 rule envelope and explicit signed weighted base-reference
  representation while preserving legacy additive v1 canonical bytes.
- [x] Add exact rational base evaluation, dependency validation, and
  fail-closed non-integral/overflow behavior.
- [x] Add golden vectors, replay/serialization tests, API contract coverage,
  and structured rule-workbench controls for weighted/subtractive terms.
- [ ] Retain legal/tax-owner review of coefficient semantics and jurisdiction
  releases before claiming production jurisdiction support.

See [ADR-0375](../adr/0375-bounded-linear-tax-base-operations.md).

## ADR-0381 bounded single-flight Tax Engine rule compilation

- [x] Add a failing-first same-digest concurrency test with bounded barrier,
  one-compilation instrumentation, shared result, cleanup, and LRU reuse.
- [x] Add a bounded repeated-distinct-digest test proving completed single-flight
  entries are removed and cannot accumulate under cache churn.
- [x] Add the bounded process-local single-flight map without holding the LRU
  mutex across compilation or introducing a network dependency.
- [x] Run the complete Tax Engine service suite, workspace checks, and the
  bounded Docker service benchmark; retain synthetic scope and tail variance.

See [ADR-0381](../adr/0381-single-flight-tax-rule-compilation.md).

## ADR-0382 Tax Engine v2 golden-vector coverage manifest

- [x] Add the failing-first required-ID manifest and duplicate/unknown-ID
  rejection to the versioned Rust vector tests.
- [x] Add the v2 weighted-component cascade vector with exact aggregate and
  component expectations.
- [x] Update the public tax coverage matrix and run the full workspace/public
  documentation gates.

See [ADR-0382](../adr/0382-tax-engine-v2-golden-vector-coverage-manifest.md).

## ADR-0383 explicit atomic display scale at the browser boundary

- [x] Add the failing-first formatter contract that rejects omitted or invalid
  display precision without converting atomic values through JavaScript
  `Number`.
- [x] Require every production formatter call site to pass an explicit scale;
  keep route-bound checkout values on finalized server-owned mint precision.
- [x] Carry quote/document result scale from the versioned facts contract;
  route-bound checkout remains on finalized server-owned mint precision.
- [x] Run the browser, palette, and full repository gates.

See [ADR-0383](../adr/0383-explicit-atomic-display-scale.md).

## ADR-0384 authoritative atomic scale in the quote contract

- [x] Add required bounded `atomic_decimals` to tax facts and document facts,
  bind it into canonical evidence, and carry it into calculation results.
- [x] Reject missing or invalid scale at Rust and browser boundaries; remove
  the interim two-decimal quote/document convention.
- [x] Compare quote/document result scale with the finalized server-owned route
  before every settlement compiler can create an instruction or intent.
- [x] Add failing-first vectors, replay/serialization/API/browser coverage and
  run the full repository gates.

See [ADR-0384](../adr/0384-authoritative-atomic-scale-in-quote-contract.md).

## ADR-0385 versioned asset-precision UI language

- [x] Add one localized semantic catalog for the explicit asset precision
  control and use it in quote, document quote, document simulation, and rule
  governance workspaces.
- [x] Prove every supported locale, unknown-locale fallback, and absence of
  repeated hard-coded production copy before implementation is accepted.
- [x] Re-run frontend tests, typecheck, build, browser journeys, and
  accessibility/color contracts.

See [ADR-0385](../adr/0385-versioned-asset-precision-ui-language.md).

## ADR-0386 versioned financial-workspace error language

- [x] Add reviewed error catalogs for document quote and aggregate checkout in
  every supported locale.
- [x] Replace inline financial-workspace error copy with typed catalog values;
  preserve redaction, bounded dynamic values, and server-owned state.
- [x] Prove the catalog and fallback contract with frontend tests, typecheck,
  production build, accessibility/color contracts, and real-browser journeys.

See [ADR-0386](../adr/0386-versioned-financial-error-language.md).

## ADR-0387 versioned document-settlement error language

- [x] Add reviewed document-checkout error catalogs for every supported
  locale, including route, evidence, compilation, wallet, submission,
  status-refresh, and reconciliation recovery states.
- [x] Replace inline document-checkout operator copy with typed catalog
  values; bound HTTP status and durable-intent interpolation and preserve
  fail-closed repeat-signing behavior.
- [x] Prove the catalog and component contract with failing-first tests,
  typecheck, production build, accessibility/color contracts, and real-browser
  journeys.

See [ADR-0387](../adr/0387-versioned-document-settlement-error-language.md).

## ADR-0388 versioned refund-execution recovery language

- [x] Add reviewed refund-execution error and safety-recovery catalogs for all
  supported locales, including two-person signing and proof-pending states.
- [x] Replace inline refund-execution authorization, preparation, recovery,
  signing, and submission copy with typed catalog values and bounded HTTP
  status interpolation.
- [x] Preserve unknown-outcome fail-closed behavior and prove the catalog,
  typecheck, production build, accessibility/color contracts, and browser
  journey behavior.

See [ADR-0388](../adr/0388-versioned-refund-execution-recovery-language.md).

## ADR-0389 versioned refund-claim recovery language

- [x] Add reviewed refund-claim error and safety-recovery catalogs for all
  supported locales, including two-person signing and finalized reconciliation.
- [x] Replace inline refund-claim authorization, preparation, recovery,
  signing, submission, and reconciliation copy with typed catalog values and
  bounded HTTP status interpolation.
- [x] Preserve unknown-outcome fail-closed behavior and prove the catalog,
  typecheck, production build, accessibility/color contracts, and browser
  journey behavior.

See [ADR-0389](../adr/0389-versioned-refund-claim-recovery-language.md).

## ADR-0390 versioned vault-release recovery language

- [x] Add reviewed vault-release error and recovery catalogs for all supported
  locales, including route loading, malformed response, proposal, lookup,
  transition, mismatch, and refresh-reconciliation states.
- [x] Replace inline vault-release authorization, route, proposal, lookup, and
  transition copy with typed catalog values and bounded HTTP status
  interpolation.
- [x] Preserve server-owned route selection, two-person governance, and
  fail-closed malformed-response behavior; prove the catalog, UI tests,
  typecheck, production build, accessibility/color contracts, and browser
  journey behavior.

See [ADR-0390](../adr/0390-versioned-vault-release-recovery-language.md).

## ADR-0391 versioned replay-operations recovery language

- [x] Add reviewed replay-operations messages for bounded HTTP status,
  page-budget validation, action failure, projection reload recovery, and
  malformed route-verification evidence in every supported locale.
- [x] Replace inline replay recovery copy with typed catalog values while
  preserving redaction, bounded retry semantics, explicit reconciliation
  states, and server-owned route-verification evidence.
- [x] Prove the catalog, focused UI behavior, typecheck, production build,
  accessibility/color contracts, and browser journey behavior.

See [ADR-0391](../adr/0391-versioned-replay-operations-recovery-language.md).

## ADR-0392 versioned settlement-adjustment recovery language

- [x] Add reviewed validation, authentication, transport, and reconciliation
  messages for every supported locale.
- [x] Replace inline Settlement Adjustments recovery copy and unbounded HTTP
  status interpolation with typed catalog values and bounded display status.
- [x] Preserve tenant authorization, server-owned adjustment state, exact
  wallet evidence instructions, unknown-outcome fail-closed behavior, and
  finalized reconciliation semantics.
- [x] Prove the catalog and panel contract with failing-first tests and
  typecheck; run the full frontend, browser, accessibility, and repository
  gates.

See [ADR-0392](../adr/0392-versioned-settlement-adjustment-recovery-language.md).

## ADR-0393 versioned revenue-streaming recovery language

- [x] Add reviewed recovery/status and secondary evidence-label messages for
  every supported locale.
- [x] Replace inline Revenue Streaming HTTP status, unavailable-address,
  amount-placeholder, and setup-evidence copy with typed catalog values.
- [x] Preserve server-owned reference selection, finalized-balance authority,
  authorization, idempotency, claim bounds, and fail-closed payout semantics.
- [x] Prove the catalog and component contract with failing-first tests and
  typecheck; run the full frontend, browser, accessibility, and repository
  gates.

See [ADR-0393](../adr/0393-versioned-revenue-streaming-recovery-language.md).

## ADR-0394 versioned vault-reconciliation recovery language

- [x] Add reviewed bounded HTTP recovery messages for every supported locale.
- [x] Replace inline Vault Reconciliation status interpolation with a typed
  catalog formatter bounded to valid HTTP status values.
- [x] Preserve release-stop semantics, finalized observation authority,
  evidence-derived balances, tenant authorization, and read-only behavior.
- [x] Prove the catalog and panel contract with failing-first tests and
  typecheck; run the full frontend, browser, accessibility, and repository
  gates.

See [ADR-0394](../adr/0394-versioned-vault-reconciliation-recovery-language.md).

## ADR-0395 versioned governance recovery language

- [x] Add reviewed bounded HTTP recovery messages for every supported locale.
- [x] Replace inline Governance HTTP status interpolation across lookup, draft,
  transition, and simulation paths with the typed catalog formatter.
- [x] Preserve tenant authorization, idempotent immutable drafts, provenance,
  lifecycle approval gates, server-owned simulation, and redacted diagnostics.
- [x] Prove the catalog and component contract with failing-first tests and
  typecheck; run the full frontend, browser, accessibility, and repository
  gates.

See [ADR-0395](../adr/0395-versioned-governance-recovery-language.md).

## ADR-0396 versioned evidence-access recovery language

- [x] Add reviewed bounded HTTP recovery messages for every supported locale.
- [x] Replace inline Evidence Access HTTP status interpolation with a typed
  catalog formatter bounded to valid HTTP status values.
- [x] Preserve tenant-scoped authorization, bounded filters, UUIDv7 identity
  validation, read-only behavior, exact audit evidence, and redacted errors.
- [x] Prove the catalog and panel contract with failing-first tests and
  typecheck; run the full frontend, browser, accessibility, and repository
  gates.

See [ADR-0396](../adr/0396-versioned-evidence-access-recovery-language.md).

## ADR-0397 versioned refund-cancellation recovery language

- [x] Add reviewed bounded HTTP recovery messages for every supported locale.
- [x] Replace inline Refund Cancellation status interpolation across prepare,
  recovery, submit, and reconciliation with the typed catalog formatter.
- [x] Preserve unchanged signed bytes, explicit confirmation, governance-wallet
  authority, unknown outcomes, finalized event reconciliation, and redacted
  diagnostics.
- [x] Prove the catalog and panel contract with failing-first tests and
  typecheck; run the full frontend, browser, accessibility, and repository
  gates.

See [ADR-0397](../adr/0397-versioned-refund-cancellation-recovery-language.md).

## ADR-0398 Material shell landmark and role boundary

- [x] Correct the keyboard skip link to the stable main workspace landmark.
- [x] Add an executable rendered-shell contract for the skip-link/main
  landmark relationship.
- [x] Establish a final shell application layer that uses semantic Material 3
  roles for surfaces, navigation, identity, popovers, and state containers.
- [x] Keep raw color literals, gradients, arbitrary hues, and authorization or
  protocol state out of the role boundary.
- [ ] Retain manual keyboard, screen-reader, forced-colors, responsive, RTL,
  and design-system review evidence before a production UI claim.

See [ADR-0398](../adr/0398-material-shell-landmark-and-role-boundary.md).

## ADR-0399 versioned navigation access language

- [x] Add bounded access-status messages for English, German, Arabic RTL, and
  Japanese shell catalogs.
- [x] Replace hard-coded navigation access labels and the local unknown-policy
  fallback with the active locale catalog.
- [x] Preserve server-owned authorization reasons, reason codes, capability
  IDs, tenant IDs, roles, scopes, and risk classifications exactly.
- [x] Prove catalog completeness and German rendered status language with
  failing-first tests and typecheck.
- [ ] Retain manual linguistic, screen-reader, RTL, and authorization-operator
  review evidence before a production UI claim.

See [ADR-0399](../adr/0399-versioned-navigation-access-language.md).

## ADR-0400 no implicit asset precision in financial workspaces

- [x] Remove the implicit two-decimal initialization from aggregate quote,
  document quote, document simulation, and governed-rule simulation forms.
- [x] Add one shared exact parser that rejects blank/coerced, fractional,
  signed, multi-digit, and whitespace precision values while accepting explicit
  zero through nine.
- [x] Add failing-first input-boundary and parser tests; update submission
  fixtures to provide precision explicitly.
- [ ] Retain manual operator, localization, assistive-technology, route-asset,
  and live-wallet review evidence before a production financial-workspace claim.

See [ADR-0400](../adr/0400-no-implicit-asset-precision.md).

## ADR-0401 complete authorization benchmark catalog

- [x] Extend the capability benchmark's exact semantic key set to include all
  nineteen server-owned policies, including revenue-streaming decisions.
- [x] Preserve explicit `deny/scope_missing` and
  `indeterminate/resource_context_required` assertions for the synthetic
  principal; no browser or benchmark decision authorizes protected work.
- [x] Add `authorization-benchmark-contract` to `make check` so a future
  catalog addition cannot silently reduce benchmark coverage.
- [ ] Retain Linux/amd64, regional, identity-provider, resource-profile, and
  production SLO evidence under ADR-0275 before making a production latency
  claim.

See [ADR-0401](../adr/0401-complete-authorization-benchmark-catalog.md).

## ADR-0402 single tax-trace disclosure in the result workspace

- [x] Remove the duplicate aggregate result trace while preserving the
  server-owned component explanation and exact displayed values.
- [x] Add a failing-first UI regression proving one rendered trace per
  validated component.
- [x] Record the alternatives, progressive-disclosure invariant, and bounded
  render/resource impact in [ADR-0402](../adr/0402-single-tax-trace-disclosure.md).
- [ ] Retain manual operator, screen-reader, responsive, localization, and
  design-system review evidence before a production UI claim.

See [ADR-0402](../adr/0402-single-tax-trace-disclosure.md).

## ADR-0403 single persistent account context in the navigation shell

- [x] Remove the duplicate tenant/subject context from the persistent sidebar
  while retaining the compact access-state surface.
- [x] Keep verified identity details in the account menu and current access
  details in the page context; do not change server-owned authorization data.
- [x] Add failing-first rendered-shell coverage proving one persistent account
  identity presentation and preserve the browser/accessibility contracts.
- [x] Remove obsolete duplicate-account styling and enforce the Material shell
  stylesheet contract so the retired presentation surface cannot return.
- [ ] Retain manual keyboard, screen-reader, responsive, RTL, and
  authorization-operator review evidence before a production UI claim.

See [ADR-0403](../adr/0403-single-persistent-account-context.md).

## ADR-0404 typed localized authorization affordance reasons

- [x] Classify browser authorization states with bounded reason codes rather
  than rendering raw server reason strings in navigation or accessibility
  descriptions.
- [x] Cover all supported locales in the shell catalog and preserve the
  server-owned authorization decision and tenant boundary.
- [x] Prove localized rendering, raw-reason absence, typecheck, full frontend,
  accessibility, and Docker/NGINX browser behavior.
- [ ] Retain staging identity-provider, clock-skew, screen-reader, and
  authorization-operator review evidence before a production UX claim.

See [ADR-0404](../adr/0404-localized-authorization-affordance-reasons.md).

## ADR-0405 Material theme-color synchronization

- [x] Move browser chrome from the legacy literal to the two approved Material
  3 Aurora background-role values in the HTML metadata boundary.
- [x] Synchronize explicit light/dark preferences by changing only bounded
  media conditions; preserve native `prefers-color-scheme` behavior for
  `system` without introducing palette literals into TypeScript.
- [x] Add failing-first unit/source-contract coverage and real Docker/NGINX
  Playwright assertions for system, light, and dark metadata activation.
- [ ] Retain manual OS/browser chrome, screen-reader, forced-colors, RTL, and
  design-system review evidence before a production UI claim.

See [ADR-0405](../adr/0405-material-theme-color-synchronization.md).

## ADR-0406 Material 3 mobile shell action row

- [x] Add a failing-first Docker/NGINX Playwright regression at 568 CSS pixels
  proving the top bar uses a two-row identity/action composition, wrapped
  horizontal actions, and bounded mobile height.
- [x] Replace the inherited mobile flex-column action layout with a semantic
  Material action row while preserving DOM order, accessible names, popover
  anchoring, and all authorization/session behavior.
- [x] Record the alternatives, invariants, and presentation-only boundary in
  ADR-0406 and the requirements traceability matrix.
- [ ] Retain manual screen-reader, keyboard-only, RTL, reduced-motion, and
  operator-usability evidence before a production UI claim.

See [ADR-0406](../adr/0406-material-mobile-shell-action-row.md).

## ADR-0407 evidence-bound tax applicability context

- [x] Freeze the boundary between arithmetic tax evaluation, jurisdiction
  evidence, and product/tax applicability in [ADR-0407](../adr/0407-evidence-bound-tax-applicability-context.md).
- [x] Record alternatives, bounded latency constraints, treatment outcomes,
  historical replay compatibility, and legal/tax ownership requirements.
- [x] Define the versioned signed applicability-attestation schema and
  canonical domain in the policy boundary before changing quote or
  document-quote contracts.
- [ ] Define the complete server-owned API applicability-context schema and
  compatibility contract before changing quote or document-quote contracts.
- [x] Add the first pure Rust applicability slice in `crates/tax-applicability`
  with bounded classification facts, explicit treatment outcomes, precedence,
  stale-evidence rejection, ambiguity rejection, and a versioned golden vector.
- [x] Bind an optional server-owned `ApplicabilityDecision` into aggregate and
  document Tax Engine facts/results with a pre-applicability digest, additive
  canonical suffix, replay preservation, and fail-closed treatment gates.
- [x] Add the signed `ApplicabilityAttestation` verification kernel with
  evidence-window enforcement, jurisdiction-resolution binding, rule-release
  binding, and explicit API error mappings.
- [x] Implement the pure-engine arithmetic/reporting semantics for zero-rated,
  exempt, reverse-charge, withholding, and out-of-scope treatments under
  [ADR-0416](../adr/0416-applicability-treatment-settlement-semantics.md);
  unresolved treatment remains fail-closed and no path reinterprets it as zero
  tax.
- [x] Add the versioned v2 signed treatment-outcome digest contract for
  aggregate and document attestation verification, with legacy v1 byte
  compatibility and a shared golden vector under ADR-0417.
- [ ] Complete issuer rollout, production rejection telemetry, and independent
  security review for v2 before enabling non-`Taxable` settlement routes.
- [ ] Obtain jurisdiction-owner/legal approval for the treatment semantics and
  publish jurisdiction-specific provenance before enabling production routes.
- [x] Extend the signed kernel into the complete server-owned applicability
  context with bounded predicates, replay-compatible evidence, and policy
  persistence.
- [x] Add tenant-scoped PostgreSQL 18 policy/evidence persistence, UUIDv7 IDs,
  effective-window constraints, and append-only governance events.
- [x] Integrate server-owned applicability resolution into quote, document
  quote, attestation, and settlement binding paths.
- [ ] Add UI progressive disclosure for treatment, evidence, limitations, and
  review-required states across supported locales.
- [ ] Retain jurisdiction/product legal-owner, provenance, staging, and manual
  accessibility evidence before publishing production coverage.

The current repository deliberately remains arithmetic-capable but has no
production-supported jurisdiction or applicability treatment. The free-form
`regime` selector remains a rule-release identity and must not be treated as a
product category.

## ADR-0409 server-owned applicability-context persistence and quote binding

- [x] Freeze the server-owned context lifecycle, persistence invariants,
  bounded cache boundary, quote reference, and alternatives in
  [ADR-0409](../adr/0409-server-owned-applicability-context-persistence.md).
- [x] Add the PostgreSQL 18 immutable context projection, append-only
  governance events, UUIDv7 constraints, and verification SQL.
- [x] Add typed storage create/load/transition validation and idempotency
  boundaries with tenant-bound attestation checks before exposing a quote
  reference.
- [x] Add the server-owned context reference to quote/document-quote contracts
  and persist the exact attestation digest with each calculation. The context
  is verified as published, tenant-scoped, current, and evidence-live; the
  arithmetic-only compatibility path remains explicit until legal ownership
  enables governed treatment semantics.
- [x] Bind context and attestation digests into versioned settlement-client
  compilation and Solana applicability-binding PDAs/instruction invariants;
  finalized indexer evidence and production treatment gates remain open under
  [ADR-0411](../adr/0411-versioned-solana-applicability-settlement-binding.md).
- [x] Add the bounded finalized-account verifier port for the receipt-scoped
  applicability PDA, including deterministic PDA derivation, exact Anchor
  layout/discriminator, owner, field-substitution, response-size, and finalized
  commitment checks under [ADR-0412](../adr/0412-finalized-applicability-binding-verifier.md).
- [x] Add the serializable, idempotent PostgreSQL 18 verification projection
  and storage boundary that locks the raw event, compares the typed verifier
  result field-for-field, and rejects conflicting replays under
  [ADR-0413](../adr/0413-atomic-applicability-binding-projection.md).
- [x] Add the UUIDv7 lease-fenced verification job table and bounded worker
  lifecycle under [ADR-0414](../adr/0414-bounded-applicability-binding-verifier-worker.md).
- [x] Schedule the verifier worker outside quote, compile, signing, and submit
  hot paths; retain the authorized read contract as a separate gate.
- [x] Expose the authorized read contract
  before presenting an applicability binding as finalized; keep account RPC out
  of quote, compile, signing, and submit hot paths.
- [x] Add the tenant-scoped evidence read contract and explicit lifecycle
  statuses under [ADR-0415](../adr/0415-tenant-scoped-applicability-verification-read-model.md).
- [x] Add the localized, non-final applicability evidence card and strict
  response parser to the document-checkout flow; bind its stream selector at
  build time without making it an authorization boundary.
- [ ] Add localized UI treatment, evidence, limitation, and review-required
  disclosure review, including manual RTL, reduced-motion, screen-reader, and
  legal/tax-owner approval before production enablement.

## ADR-0408 quiet tonal Material 3 Expressive shell

- [x] Freeze the shell hierarchy around the audited Aurora role matrix: surface
  roles for structure, primary for active/high-emphasis actions, secondary for
  supporting controls, tertiary for limited expressive emphasis, and explicit
  warning/error/verified state aliases.
- [x] Add a failing-first shell contract for semantic final-layer rules, no
  decorative gradients/raw component colors, quiet tonal panels, bounded reading
  width, and compact system status.
- [x] Apply the quiet tonal shell: remove panel shadows, reduce redundant
  surface contrast, keep diagnostics and sign-in detail behind disclosures, and
  preserve keyboard/forced-colors/mobile behavior.
- [ ] Retain Docker/NGINX browser evidence at 1440px and 390px plus manual
  screen-reader, RTL, reduced-motion, and operator-usability review before a
  production UX claim.

See [ADR-0408](../adr/0408-material-expressive-quiet-tonal-shell.md).

## ADR-0410 Material state emphasis and border budget

- [x] Define neutral informational badges and explicit success, warning, and
  danger state classes.
- [x] Remove shared panel borders and elevation from the final shell layer;
  retain outlines for controls, focus, and dialogs.
- [x] Add the executable Material role contract and preserve forced-colors,
  reduced-motion, responsive, and keyboard behavior.
- [ ] Retain Docker/NGINX browser screenshots at 1440px and 390px plus manual
  assistive-technology and operator-usability evidence before a production UX
  claim.

See [ADR-0410](../adr/0410-material-state-emphasis-and-border-budget.md).

## ADR-0419 neutral risk taxonomy and calm active navigation

- [x] Separate workflow risk labels from operational outcomes so ordinary
  `read`, `write`, `financial`, and `governance` metadata stays neutral.
- [x] Keep verified, attention, blocked, and error treatments on explicit
  semantic state roles with paired content roles and non-color labels.
- [x] Remove the active navigation border/inset treatment and use one quiet
  primary-container selection surface.
- [ ] Retain wide/narrow browser screenshots plus manual operator, screen-reader,
  forced-colors, RTL, and reduced-motion review before a production UX claim.

See [ADR-0419](../adr/0419-neutral-risk-taxonomy-and-active-navigation.md).

## ADR-0418 applicability-context management API

- [x] Add strict authenticated create/read/transition routes over the
  server-owned applicability projection.
- [x] Derive tenant and actor identity from the authenticated principal and
  enforce the explicit author/reviewer/approver role matrix.
- [x] Add an isolated signed-attestation Docker fixture and exercise create,
  idempotent replay, tenant isolation, four-eyes publication, append-only
  lifecycle evidence, and published-row immutability through
  `make integration-check`.
- [ ] Add the production governance UI review/import workflow without making
  browser-authored treatment or attestation data authoritative; retain manual
  accessibility, legal/tax-owner, staging identity, and production evidence
  gates.

See [ADR-0418](../adr/0418-applicability-context-management-api.md).

## ADR-0420 applicability governance workspace

- [ ] Separate signed treatment-evidence governance from tax-rule authoring in
  the product navigation and workspace model.
- [ ] Keep the browser as an issuer-signed evidence importer and lifecycle
  operator only; all treatment, tenant, digest, and authorization decisions
  remain server-owned.
- [ ] Add localized, accessible create/read/transition states with bounded
  loading, timeout, retry, stale-data, and reconciliation-required language.
- [ ] Retain real-browser wide/narrow evidence and manual screen-reader, RTL,
  reduced-motion, legal/tax-owner, and staging identity review before a
  production treatment enablement claim.

See [ADR-0420](../adr/0420-applicability-governance-workspace.md).

## ADR-0530 capability context and tenant identity presentation

- [x] Resolve the API deployment environment as known process context while
  preserving indeterminate decisions for selected resource attributes.
- [x] Add regression coverage proving quote capabilities are not blocked by the
  environment attribute alone.
- [x] Replace the visible unnamed-workspace marker with an actionable naming
  label and clear the profile input until the administrator supplies a name.
- [ ] Retain authenticated Docker/browser evidence after deployment and have a
  tenant administrator save the tenant-owned workspace name.

See [ADR-0530](../adr/0530-capability-context-and-tenant-identity-presentation.md).
