# Implementation plan

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
tokens, and expose no `posted` or payment-success control. Prove parsing,
failure states, accessibility, mobile/RTL layout, and the NGINX browser journey
before considering this UI slice complete.

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
- [x] Execute the default workload on 2026-10-01: 256/256 HTTP 200 responses
  at concurrency 32 on the pinned local arm64 image, with p50/p95/p99 of
  1.344/3.650/10.302 ms.
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
- [ ] Run and retain the Docker drill artifact on a Docker-enabled CI runner;
  the current macOS host has no reachable Docker daemon, so this evidence is
  not claimed locally yet.
- [x] Require finalized `AdjustmentRefundClaimCancelled` evidence before the
  claim projection becomes cancelled; preserve unknown provider outcomes and
  idempotent replay semantics.
- [x] Add production UI/UX for governance cancellation, wallet rejection,
  timeout, unknown, finalized cancellation, and reconciliation-required states
  with accessible explanations and no implied refund success.
- [ ] Extend the pinned Linux validator conformance suite with the complete
  cancellation lifecycle and unchanged tax-vault, funder, and payer balances.

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

## Phase 0A - Tier-1 product foundation

Deliverables:

- product coverage matrix and explicit launch jurisdictions/tax regimes;
- SLOs, error budgets, RTO/RPO, support model, on-call ownership, and incident severity definitions;
- tax-rule provenance and legal/tax review ownership model;
- financial operations for refunds, reversals, chargebacks, remittance evidence, and reconciliation;
- API/IDL/rule schema compatibility and deprecation policy;
- public-repository documentation, security disclosure process, contribution workflow, changelog, and licensing decision;
- dependency register, vendor/RPC exit plan, backup/restore design, and key-recovery plan.

Exit criteria: the Tier-1 product standard has an owner and evidence plan for every pillar; unsupported product/jurisdiction cases have explicit behavior.

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
and cancellation evidence. No API or browser key may authorize a vault
transfer.

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

## Phase 3A - Tier-1 tax engine

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

### Structured Tier-1 rule workbench (ADR-0143)

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
before launching and passes 15/15 test cases while auditing the covered
surfaces. This evidence uses synthetic API fixtures and does not claim live
wallet or chain finality.
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

Exit criteria: controlled production meets the Tier-1 product standard for the published scope; general availability is a separate approval, not an automatic consequence of deployment.

## Version and dependency governance

As of 2026-09-25, the documentation sources report Rust stable 1.98, Agave/Solana CLI 4.3.0, React 19.3, Node.js 24.21.0 LTS, Solidity 0.8.37, Anchor CLI 1.2.0, and NGINX stable 1.30.5. Confirm availability and Solana compatibility during bootstrap, then pin exact versions in source and CI. Do not use a floating `stable` or `latest` channel in production CI. The off-chain workspace remains on Rust 1.98.1; the on-chain package explicitly declares Rust 1.95 compatibility because Agave 4.3.0's platform-tools v1.57 SBF compiler reports rustc 1.95.0-dev. See [ADR-0067](../adr/0067-sbf-compiler-compatibility-boundary.md).

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
# Implementation plan
