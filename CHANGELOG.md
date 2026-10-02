# Changelog

## 2026-10-02

- Added ADR-0244 for the exact foreign-exchange evidence boundary required by
  global taxation. The pure engine now normalizes exact evidence once and
  preserves it for replay; API entry points reject caller-supplied FX until a
  separately governed provider adapter can attach verified evidence. Provider
  freshness and legal rate acceptance remain outside the tax hot path.
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
