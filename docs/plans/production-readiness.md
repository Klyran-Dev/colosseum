# Production-readiness plan

Current evidence is intentionally partial: the repository now has authenticated
merchant-scoped quote persistence, database-enforced published-rule
immutability, a transactional quote outbox, bounded NATS JetStream delivery,
server-owned quote-bound calculation-attestation ingestion, and a cached
on-chain policy attestation binding. These are foundation gates, not completed
production readiness. Unchecked items remain release blockers.

ADR-0065 now provides an executable validator-backed conformance harness and a
Dockerized SBF build path. The harness is a release-gate implementation, not
evidence by itself: the localnet boxes below remain unchecked until the pinned
artifact is built, the validator run completes, and the resulting transaction
evidence is retained in CI or an equivalent controlled run. ADR-0085 now
retains a bounded public-safe evidence bundle, including the artifact hash,
toolchain/runtime metadata, validator logs, Compose configuration, and test
output on both success and failure. The pinned SBF artifact has now built
successfully in the Dockerized amd64 builder; the
validator run remains environment-gated here because Docker Desktop's VM does
not expose the `io_uring` capability required by Agave 4.3.0.
ADR-0080 additionally provides a SHA-256-verified browser-asset fallback for
the pinned platform-tools download; the current host successfully built the
SBF artifact through that path, while validator startup still fails at the
Agave `io_uring_supported()` assertion.

The 2026-09-27 conformance attempt built the pinned artifact with SHA-256
`ad61731b4f01c4e757133f71508db2de3c13c54996b568b2d92927a7119e675e`, then
captured the validator failure before RPC readiness: Agave logged
`io_uring NOT supported: Function not implemented` and exited on the
`io_uring_supported()` assertion. This is retained environment evidence, not
a validator conformance pass; the required Linux CI/runner execution remains
open.

The current guarded Docker SBF build after ADR-0169 produced artifact
SHA-256 `412a2d66c3edcc25c316377cc85a3bea85c8079d69bb4338d69119136e91457b`
without the prior 4 KiB stack-frame diagnostics. This is build evidence only;
the Linux validator and transaction evidence gates remain open.

ADR-0168 closes the local runner reproducibility gap: `make
solana-conformance` now executes the Vitest RPC client inside a pinned,
non-root Docker image and no longer depends on host Node.js or pnpm. This
improves bootstrap fidelity but does not close the independent Linux Agave
validator gate. Its health probes are now bounded and retain a typed failure
reason when Agave exits before RPC readiness, preventing a crashed validator
from hanging a developer or CI job.

## Correctness

- [x] Canonical settlement serialization has a shared versioned golden vector
  consumed by Rust and TypeScript; policy and broader API-schema vectors remain
  separate coverage work.
- [x] The browser validates the bounded settlement envelope before requesting a
  blockhash or wallet signature; the API remains the authoritative signed-
  message verifier.
- [x] On-chain amount validation is property-tested across arbitrary `u64`
  tax/net pairs, including zero and checked-add overflow behavior.
- [x] The bounded Split Plan domain contract has deterministic basis-point
  validation, largest-remainder allocation, canonical digest, shared Rust and
  browser vectors, and property coverage. The additive Solana instruction,
  immutable PDA lifecycle, live CPI evidence, and production fan-out budgets
  remain unchecked under ADR-0091.
- [ ] Validator-backed Split Plan fan-out proves finalized recipient balances,
  duplicate receipt rejection, and wrong-account-order atomic failure under
  ADR-0140; the conformance implementation is present but requires a successful
  Linux validator/CI execution before this gate can be checked.
- [ ] Validator-backed policy revocation proves a revoked attestation cannot
  authorize token movement under ADR-0141; the conformance implementation is
  present but requires successful Linux validator/CI execution.
- [ ] Live settlement CPI execution proves token-balance conservation and
  atomic rollback on the validator-backed localnet.
- [ ] `u64` overflow, zero amount, cap, rate, and rounding boundaries are covered.
- [ ] Legacy SPL Token and extension-free Token-2022 account ownership is checked in localnet transactions.
- [ ] Unsupported Token-2022 extensions are rejected or explicitly tested in localnet transactions.
- [ ] Policy issuer, sequence, expiry, revocation, merchant binding, and governed issuer-registry scope are enforced.
- [ ] Replay, duplicate nonce, stale blockhash, and duplicate submission are covered.
- [ ] Authenticated tenant configuration resolves merchant Solana accounts;
  settlement compilation does not rely on free-form caller-supplied merchant
  authority fields. ADR-0061 and its local integration slice provide the
  registry boundary. ADR-0062 provides the bounded verifier design and
  finalized account-proof implementation; production governance ceremonies and
  localnet activation evidence remain open.
- [x] ADR-0092 provides a tenant-scoped persisted Split Plan registry,
  canonical digest revalidation, active/effective lifecycle selection, and a
  quote-bound split compiler endpoint. On-chain initialization/activation,
  indexer evidence, and production governance remain open.
- [x] ADR-0093 split-event decoding, typed persistence, and replay-safe
  projection rebuild are implemented with Docker migration and Rust evidence.
  Validator-backed account-balance and recipient-transfer evidence remains
  open.
- [x] ADR-0094 governed Split Plan checkout discovery and review are
  implemented, accessible, and integrated with quote-bound compilation. The
  browser validates the exact bounded account envelope before wallet signing;
  validator-backed settlement evidence remains open.
- [x] Settlement compilation loads the authenticated tenant's persisted quote
  by `calculation_id`, verifies its server-owned expiry, and derives gross,
  tax, and net from that evidence; browser-provided amounts are forbidden.
  The Docker integration path proves both valid server-owned compilation and
  expired-quote rejection.
- [x] Settlement compiler golden vectors, PDA derivation, account ordering,
  wallet handoff, and browser artifact/route response validation are covered.
  Stale blockhash, duplicate submission, live RPC, and validator-backed
  evidence remain open.
- [ ] Durable UUIDv7 settlement intents fence idempotency and lifecycle
  transitions; `unknown` after possible provider acceptance requires signature
  reconciliation. ADR-0107 now makes every intent read and transition
  merchant-scoped at the storage boundary. ADR-0118 adds the durable
  `signed -> submitting` compare-and-set, derived-signature binding, and
  provider-response matching. The 2026-09-27 Docker local-stack drill proves
  one concurrent `200`/one `409`, exactly one validating RPC-fixture send,
  durable `submitted` readback, and an accepted-but-ambiguous response that
  persists `unknown` and later reconciles to `confirmed` without a second send.
  ADR-0120 now adds a real local Compose process-crash drill: the API is
  killed after the durable `submitting` claim, restarted, and reconciles the
  retained signature to `confirmed` without a second send. Live RPC,
  validator/localnet evidence, multi-replica fencing, and production failover
  remain open.
- [ ] The submit boundary cryptographically proves the signed message contains
  the compiled settlement instruction, expected program/account domain,
  payer authorization, and exact instruction policy before production
  enablement. ADR-0058 and unit tests cover the message-binding portion; fresh
  blockhash validity, durable nonce policy, localnet, and staging evidence
  remain open.
- [x] The durable settlement submission fence has been exercised across two
  independently running API replicas. ADR-0134's Docker drill returned one
  `200`, one `409`, exactly one validating RPC send, and one durable native
  signature; validator-backed, multi-region RPC failover, and production
  capacity evidence remain separate release gates.
- [ ] All failed paths prove no partial token movement.
- [ ] Governed vault-release registry and immutable claim lifecycle enforce
  separate proposal/approval duties, bounded windows, exact destination binding,
  cancellation, and finalized balance/event evidence. ADR-0105 defines the
  on-chain contract; the local integration gate exercises the full
  proposal/approval/on-chain-created/executed path and database invariants.
  API, indexer, key-ceremony, validator, and staging evidence remain release
  gates.
- [ ] Finalized vault-balance reconciliation is implemented under ADR-0127:
  immutable route/stream targets, operator-approved opening-balance evidence,
  checked-u64 receipt/release delta expectations, finalized token-account
  binding, append-only run evidence, and fixed-cardinality mismatch/error
  alerting. Opening-balance ceremony, live multi-provider evidence,
  Alertmanager delivery, scale/retention evidence, and supported Token-2022
  extension semantics remain release gates.
- [ ] ADR-0128 tenant-scoped reconciliation read APIs and the operator UI expose
  server-owned target/run evidence with explicit stale, mismatch, error,
  unauthorized, timeout, and empty states. Browser target creation and
  opening-balance editing remain prohibited; provisioning ceremony and a
  separately governed system-wide finance view remain release gates.
- [x] Tax-engine golden vectors cover simple, progressive, tax-on-tax,
  cascading, inclusive/exclusive, min/max, threshold, exemption, cumulative
  snapshot, credit, truncation, and rounding cases.
- [x] Tax-engine boundary evidence covers zero, exact `u64::MAX`, unit-rate,
  invalid-rate/clamp, all declared rounding ties, and generated bounded `u64`
  conservation through floor/cap logic.
- [x] The jurisdiction-resolution bounded context has versioned source
  precedence, strict conflict modes, canonical policy/evidence digests,
  replay verification, golden vectors, and permutation/property tests. Live
  provider authentication, jurisdiction-specific legal/tax approval, privacy
  controls, persistence, and binding into calculation attestations remain
  release gates under ADR-0101.
- [x] ADR-0102 binds an optional server-owned jurisdiction-resolution envelope
  into `TaxFacts` and `CalculationResult`, includes it in the canonical facts
  digest, and rejects replay after envelope tampering while preserving legacy
  facts' historical digest bytes.
- [x] New globally supported quotes require provider-signed jurisdiction
  evidence, server-side merchant/facts/time verification, an immutable
  tenant-scoped policy, persisted normalized resolution evidence, and replay
  binding. The clean Docker integration gate proves the synthetic path;
  production provider registry, key rotation, privacy controls, and legal/tax
  approvals remain open release gates under ADR-0103.
- [x] Jurisdiction-policy authoring and publication use an authenticated,
  tenant-scoped four-eyes lifecycle with UUIDv7 IDs, idempotent drafts,
  canonical policy digests, append-only events, transactional outbox records,
  effective-window binding to published rules, API role gates, and Docker
  integration evidence under ADR-0104. Production identity provisioning,
  legal/tax owner approval, and browser accessibility automation remain open.
- [x] Rule-graph cycle detection and dependency ordering are enforced with
  explicit compiled-graph tests; duplicate component identifiers fail before
  graph construction.
- [x] Published rule versions are immutable and cannot overlap within the same
  jurisdiction/tax regime without an explicit precedence decision. PostgreSQL
  triggers/exclusion constraints and the isolated integration harness prove
  mutation and overlap rejection.
- [x] New published rule documents use the versioned
  `solana-tax-protocol/tax-rule/v1` envelope from ADR-0122 without changing the
  canonical financial rule digest. Replay accepts the envelope and a bounded
  legacy branch for immutable historical bare documents; unsupported schemas
  fail closed. Storage and tax-engine tests cover round-trip, rejection, and
  digest stability.
- [x] Rule publication uses the authenticated author/reviewer/approver
  lifecycle with distinct subjects, serializable revision fencing, and
  append-only transition evidence; identity-provider lifecycle, legal/tax
  owner attestation, and production ceremony rehearsal remain open.
- [x] Rule releases are tenant-owned and every quote/governance query includes
  authenticated merchant scope; ADR-0079 defines the fail-closed migration and
  composite database invariant. Clean-stack integration proves same-key
  coexistence across tenants, immutable ownership in draft and published
  states, cross-tenant 404 boundaries, and mismatched calculation rejection.
- [x] Domain calculation results bind the engine version and replay the complete
  component trace from normalized facts and rule digest; version mismatch and
  tampering fail closed.
- [x] Persisted quote replay verifies the database evidence-reference envelope
  and tenant-scoped calculation record in the storage adapter and Docker
  PostgreSQL integration flow. Production backup/restore and migration-drill
  evidence remain separate release gates.
- [x] ADR-0100 implements authenticated quote-bound attestation ingestion:
  server-owned binding/time, immutable signed evidence, `attested` status,
  exact-document replay protection, and a transactional first outbox event.
  The clean-stack Docker harness now proves an isolated external synthetic
  issuer fixture, exact replay, conflict rejection, tenant isolation, and
  non-simulation single/Split Plan compilation. Production issuer/HSM/KMS
  integration, live issuer-registry evidence, and validator-backed
  policy-account registration remain release gates.
- [x] Compiled-rule cache hit/miss/eviction/entry metrics are exported with
  fixed labels and bounded cardinality; concurrent-load and invalidation
  performance evidence remains open.
- [x] PostgreSQL 18.x uses a pinned image digest and the clean-checkout
  Docker-isolated logical backup/restore drill passes against the complete
  migration and fixture set.
- [x] A Docker-isolated PostgreSQL 18 physical base-backup and WAL replay drill
  restores to an exact target LSN, retains the target marker, and excludes the
  post-target marker (`make postgres-pitr-drill`, ADR-0114). The
  2026-09-27 run used target LSN `0/4006470`, six archived WAL files, and 1
  second of local recovery time. This is local recovery-mechanics evidence
  only, not a production RTO claim.
- [ ] Production encrypted backup/PITR policy, measured RPO/RTO, and migration
  rollback rehearsal are complete. ADR-0138 adds the versioned, checksum-
  verified migration runner, Compose dependency gate, and isolated drift
  drill; encrypted production backup policy, staging evidence, and rollback
  rehearsal remain open.
- [x] ADR-0182 adds an encrypted logical-backup artifact drill that verifies
  integrity before restore and rejects tampered ciphertext in CI. Production
  KMS/HSM envelope encryption, off-site immutable retention, key recovery,
  measured RPO/RTO, and restore authorization remain open.
- [x] ADR-0183 adds a Docker-isolated expand/contract migration rollback
  rehearsal: it restores a pre-deploy PostgreSQL 18 image, proves the old
  schema/data read contract, removes expand-only objects, and reapplies the
  forward migration idempotently. Production staging approval, live rollback
  authorization, encrypted off-site recovery, and measured RPO/RTO remain
  open.
- [x] Internal IDs use native PostgreSQL `uuid` with UUIDv7 where sortable distributed IDs are appropriate; ADR-0083 executes a catalog-level contract after clean migrations and logical restore. Secrets, hashes, Solana IDs, and strict sequence values use the documented alternatives.
- [x] PostgreSQL lock and statement timeout contracts, migration ordering, and
  required evidence/replay indexes are exercised by the pinned Docker
  `make postgres-contention-drill` and CI job.
- [x] Bounded local pool pressure is measured through connection-free API
  gauges by `make postgres-pool-drill` and its CI job. On 2026-09-27, a
  one-connection pool handled 256 normal requests at concurrency 64 plus 256
  same-key retries with 0% errors, peak `waiting=3`, and exactly one
  contention idempotency/calculation/outbox result (ADR-0112). This is local
  capacity evidence only.
- [x] A Docker-isolated PostgreSQL 18 index-plan drill seeds 25,000 rows in
  each append-only evidence family, 25,000 tenant intents, and 10,000 intent
  lifecycle events. Normal planning selected all six declared scoped indexes
  for the production query shapes (`make postgres-index-plan-drill`,
  ADR-0113). This is local plan evidence only.
- [ ] Connection-pool saturation, account-lock contention at representative
  tenant volume, partition/retention policies, and index plans are load-tested
  against production-like data. Production-volume sizing, partition/retention,
  archive/PITR, and regional database evidence remain open; the local drills do
  not close this broader gate.

## Security

- [ ] Program audit complete with no unresolved critical/high findings.
- [ ] Upgrade authority, policy signer, and vault release keys are separated.
- [ ] HSM/KMS and multisig ceremonies are documented and rehearsed.
- [ ] Frontend/API do not receive seed phrases or user private keys.
- [ ] Dependency, container, SBOM, and secret scans pass in CI.
- [x] The local NGINX edge has a non-root UID, read-only root filesystem,
  dropped capabilities, bounded temporary storage, and a passing Trivy gate.
- [x] Bounded stable-Rust adversarial account-meta property tests run on every
  program change through the dedicated `program-adversarial` CI job. ADR-0110
  constructs actual payer-signed transactions and substitutes every compiled
  account position; local `cargo test --locked -p settlement-coordinator
  --all-targets` passes. Coverage-guided scheduled fuzzing and validator-backed
  on-chain account checks remain separate evidence.
- [x] Rate limits, authentication, authorization, CORS, CSP, and NGINX headers are tested.
  ADR-0074 adds bounded per-source-IP NGINX rate/connection admission and
  image-build config validation; ADR-0081 adds deny-by-default CSP/header
  assertions, unauthenticated `401` coverage, and CORS denial checks. The
  isolated integration harness proves a concurrent burst receives `429`;
  production load/tuning evidence remains open.
- [x] Rule publishing requires separation of author, reviewer, and approver
  duties at the API and PostgreSQL boundaries; deployment identity governance
  and operational ceremony evidence remain open.
- [x] Governance operators have a read model and React workspace showing rule
  status, duties, effective windows, immutable transition evidence, explicit
  failures, and server-authorized next actions; tenant ownership is separately
  gated by ADR-0079. Browser accessibility, identity-provider exercises, and
  production ceremony evidence remain open.
- [x] Sensitive jurisdiction evidence is minimized and encrypted before
  persistence, raw provider envelopes are discarded after verification, and
  replay attempts append tenant/actor/request/purpose/outcome audit records
  without storing decrypted facts (ADR-0108; migrations 0025-0026). Production
  KMS/HSM key rotation, retention/deletion policy, backup recovery, and privacy
  access-review evidence remain open.

## Performance

- [ ] p50/p95/p99 quote, compile, submit, processed, confirmed, and finalized latency is measured.
  ADR-0075 provides domain hot-path Criterion benchmarks and ADR-0078 provides
  bounded local HTTP quote-service evidence; representative contention,
  regional, and chain-finality measurements remain open. The default local
  200-request/concurrency-16 two-replica run completed on 2026-09-27 with 0%
  errors and exact persisted idempotency/calculation/outbox counts; its
  measured p50/p95/p99 was 3.856/13.393/27.985 ms. The 32-request same-key
  contention run crossed both replicas and converged to one idempotency record,
  calculation, and outbox event. This is local Docker evidence, not a
  production SLO claim. The same bounded workload is now a dedicated CI job;
  regional, resource-profile, and chain-finality measurements remain open.
- [x] Persisted-rule simulation has a separate facts-only two-replica latency
  workload with server-owned response-binding checks and bounded read
  contention (ADR-0145). Its local p95 diagnostic budget is evidence for the
  service path only; regional, resource-profile, browser, and production SLO
  acceptance remain open. The default Docker run on 2026-09-27 completed 200
  requests at concurrency 32 with 0% errors and exact rule-digest binding;
  p50/p95/p99 was 1.958/3.001/6.125 ms, with 64/64 contention responses and
  zero observed pool waiters.
- [x] The bounded quote workload includes same-key idempotency/account-row
  contention and asserts one calculation/outbox record under concurrent
  retries (ADR-0090). Production-volume account-lock and recipient fan-out
  evidence remain open.
- [x] The bounded quote workload has an executable two-replica drill: two
  independent API containers share the authoritative PostgreSQL fence, unique
  requests alternate between replicas, and same-key contention also crosses
  replicas. ADR-0133 and the CI quote-load artifact retain the evidence
  contract. Regional routing, production-like database contention, failover,
  and resource-profile evidence remain open.
- [x] Legacy settlement wire size is deterministically estimated and bounded
  at 937 bytes including the governed compute-budget pair for the maximum
  eight-recipient shape, leaving 295 bytes below
  Solana's 1,232-byte packet ceiling; extra static keys are rejected at the
  signed-message boundary (ADR-0116).
- [x] ADR-0162 now binds a versioned server-owned compute-unit limit and
  priority-fee bid into every single and Split Plan settlement artifact. The
  browser serializes exactly two compute-budget instructions before settlement,
  and the API rejects missing, reordered, duplicated, unknown, or mutated
  policy instructions. Validator-backed calibration and target-cluster fee
  acceptance remain open.
- [ ] Compute-unit budgets and priority-fee safety margins have validator-backed
  evidence.
- [ ] v1 and v0+ALT transaction paths are tested on target clusters/RPCs.
- [x] ADR-0161 implements the server-side bounded v0+ALT admission boundary:
  finalized RPC lookup resolution, canonical ALT owner/active checks, bounded
  table/address counts, exact expanded account and privilege matching, and
  fail-closed provider behavior are covered by Rust tests. Target-cluster
  execution evidence is still required for the preceding release gate. The
  TypeScript conformance suite now contains a real ALT create/extend/finalized
  fetch/v0 submit path with finalized conservation assertions; a successful
  Linux CI run is still required before the release checkbox can be checked.
- [x] Multi-provider failover and degraded RPC behavior are exercised by the opt-in two-fixture process drill in ADR-0137; live provider authentication, regional capacity, failover SLOs, and staging evidence remain open.
- [x] Settlement and indexer failover share a bounded process-local adaptive
  provider-health contract. Unit tests prove cooldown ordering, recovery,
  immutable signed-byte retries, and stale-blockhash no-failover behavior
  (ADR-0115). Live multi-region provider SLO and alert evidence remain open.

## Operations

Current evidence: the isolated Docker integration check exercises the worker's
PostgreSQL lease claim, NATS JetStream publish acknowledgement, and published
state transition. It does not yet constitute production observability,
multi-replica failure testing, dead-letter alerting, or an operational SLO.

The indexer domain, finalized provider-page coordinator, bounded Rustls JSON-RPC
transport, deadline-bounded three-provider failover wrapper, and PostgreSQL
adapter now prove bounded finalized checkpoint advancement, exact
range/completeness validation, transport response limits, transactionally
persisted receipt pages, duplicate delivery idempotency, conflicting evidence
rejection, exact atomic-unit reconciliation, and pinned Anchor
`SettlementExecuted` decoding with bounded raw event evidence. They do not yet
prove fleet-level provider budgets, live raw full-transaction capture, or an
authorized signature-to-evidence recovery worker. Process-local provider
health scoring and capped cooldown behavior are covered by deterministic unit
tests, and the pure replay-gap queue now bounds coalescing, splitting, and
retry exhaustion. PostgreSQL now contains lease-fenced replay-gap state, but
the deployable indexer runtime still requires live multi-region degradation,
replay-drill execution evidence, and broader merchant/vault projection rebuild
evidence. ADR-0046 now
provides the bounded replay executor and suffix advancement path; those runtime
and operational gates are intentionally not marked complete by unit tests.

- [ ] Structured logs, traces, metrics, and alert routing are live; ADR-0082
  now gives every API response an API-owned UUIDv7 correlation ID and rejects
  caller-controlled identity by overwrite. The API,
  worker, and indexer now expose bounded Prometheus-compatible metrics and
  Compose provisions Prometheus, Grafana, and an OTLP Collector. ADR-0084 now
  adds versioned alert rules and pinned-image `promtool` validation, but this
  gate remains open until scrape access policy, production exporters, receiver
  delivery, dashboards, alerts, and load-derived SLO evidence are verified.
- [x] ADR-0123 exposes fixed-cardinality queued, leased, dead-lettered, and
  leased-slot-span replay gauges from the configured indexer stream, with
  Prometheus rules for backlog and dead-letter alerts. Scrape authorization,
  receiver delivery, lease-expiry/claim-loss counters, and production alert
  response remain open.
- [x] ADR-0124 adds a bounded finalized `getSignaturesForAddress` adapter for
  targeted discovery: limits, response size, canonical cursors, duplicate
  signatures, and finalized status are validated. The adapter cannot advance
  a checkpoint or mutate a projection. Authorized worker integration, live
  provider pagination, and subsequent finalized transaction backfill remain
  open.
- [x] ADR-0125 adds bounded signature-to-evidence recovery primitives: the
  target must be discovered within a fixed page budget, `getTransaction` is
  requested at finalized commitment with a pinned supported version, the
  primary signature is rebound to the target, and evidence is persisted only
  through projection-only replay storage. The authenticated API requires the
  exact operator scope/role and appends immutable requested/recovered/failed
  audit rows. Live provider drills, crash-reconciliation alerting, and fleet
  recovery SLO evidence remain open.
- [x] ADR-0126 exports a fixed-cardinality stale-request gauge for targeted
  recovery audits and validates the corresponding Prometheus alert contract.
  Production scrape authorization, notification delivery, on-call response,
  and a live crash-reconciliation drill remain open.
- [x] Worker, indexer, and route-verifier health servers are owned by their
  structured service loops rather than detached tasks. ADR-0121 proves the
  lifecycle contract in code: unexpected health-server termination fails the
  process, while shutdown propagates cancellation and drops the listener.
  Supervisor restart, orchestration, and multi-replica failure evidence remain
  open.
- [x] Adaptive settlement provider health is exposed through bounded API gauges
  for score, consecutive failures, and cooldown milliseconds using only a
  fixed provider-index label (ADR-0117). Endpoint, merchant, request, and
  signature identity are excluded; production scrape and alert delivery remain
  open.
- [ ] Indexer backfill and replay are automated.
- [ ] Replay gaps have durable leases, dead-letter state, and retry exhaustion
  alerts. ADR-0086 now provides authenticated list/acknowledge/requeue
  operations plus immutable operator-action evidence; operator identity
  lifecycle, production alert delivery, and replay-drill evidence remain open.
- [x] Retained raw event evidence and normalized `indexed_receipts` rebuild from
  a clean projection with byte-identical results. ADR-0088 defines the
  canonical decoder and bounded keyset-paginated primitive; `make
  projection-rebuild-drill` proves digest equality and checkpoint invariance in
  Docker and CI. Merchant, vault, and reporting projections require separate
  raw-evidence contracts and remain open.
- [x] Vault balance reconciliation is scheduled and its matched/mismatch fixed-cardinality alert inputs are exercised through the real Docker indexer and synthetic finalized-account RPC boundary by ADR-0136; live provider, opening-balance ceremony, Alertmanager delivery, and Token-2022 extension evidence remain open.
- [x] Public-safe runbooks cover policy compromise, upgrade rollback, vault
  release incident, RPC outage, indexer gap, and backup/restore. Production
  role binding, on-call ownership, and measured drills remain deployment
  gates.
- [ ] Mainnet configuration is separate from staging and requires approval. ADR-0130 now enforces explicit `local`/`staging`/`production` startup environments and rejects simulation or insecure RPC outside local; separate secret provisioning, deployment approval, allow-listing, and mainnet evidence remain open.
- [x] Repository-side staging/production preflight rejects local databases, RPC/NATS/identity defaults, insecure flags, and synthetic seed fixtures under ADR-0139; actual secret validity, provider health, deployment approval, and live staging/production evidence remain open.
- [ ] Emergency pause/unpause is exercised on localnet and staging with
  multisig/timelock authority, event alerting, and a documented break-glass
  runbook. ADR-0131 now provides finalized event projection, fixed-cardinality
  active/unknown alerts, and localnet pause-blocking assertions; staging
  governance, delivery, and incident evidence remain required.
- [ ] Releases are reproducible and record program binary and configuration hashes.
- [x] SLOs, error budgets, RTO/RPO, support ownership, and incident severity policy are published under ADR-0135; named deployment owners, alert-receiver delivery, and measured regional evidence remain open.
- [ ] Backup restore, key recovery, RPC failover, and indexer replay drills
  pass in the required production-like environments. The Docker integration
  gate now rehearses governed rule rollback by retiring version 1, publishing
  corrected version 2 through the four-eyes workflow, and proving the
  corrected server-owned calculation; encrypted production backup, key
  recovery, live-provider, replay, and staging evidence remain open.
- [x] The provider-neutral dependency register and RPC/vendor exit paths are published under ADR-0135; contractual portability, approved provider bindings, and replacement drills remain deployment gates.

## Legal and product

- [ ] Counsel reviewed tax, custody, money transmission, reporting, privacy, and jurisdictional claims.
- [ ] Merchant terms identify who is responsible for tax registration and filing.
- [ ] Policy issuer authority and evidence provenance are documented.
- [ ] Stablecoin issuer and redemption/off-ramp dependencies are documented.
- [ ] User-facing copy says “automated withholding and evidence,” not “guaranteed compliance.”
- [ ] Each published rule has legal/tax owner, source provenance, effective period, review date, and jurisdiction scope.
- [ ] UI clearly distinguishes calculated tax, attested policy, and legal responsibility.

## User experience

- [x] Checkout explains gross, tax, net, governed distribution, published rule
  release, rule digest, policy state, quote expiry, wallet authorization, and
  durable intent before wallet signing.
- [x] Checkout constructs only the reviewed compiled instruction, keeps wallet
  keys client-side, submits unchanged signed bytes, and displays durable
  intent/commitment states without inferring finality from wallet signing.
  ADR-0098 adds runtime response validation and fail-closed recovery after an
  unsuccessful submit or status reload; live provider and finalized-chain
  evidence remain separate release gates.
- [x] “Why this tax?” exposes a progressive human-readable calculation trace,
  rule/facts digests, and explicit non-advice language without exposing private
  evidence.
- [x] ADR-0095 gives every SPA API call a tested finite deadline, preserves
  caller cancellation for stale/unmounted views, and keeps timeout outcomes in
  explicit error/reconciliation states. Browser accessibility and live
  provider evidence remain separate release gates.
- [x] Rule authoring now creates tenant-scoped, idempotent drafts through the
  authenticated API, validates/canonicalizes the complete RuleSet graph,
  records immutable creation evidence, and exposes the real authoring form in
  the governance workspace. Simulation, review, approval, publication, and
  retirement are server-governed; legal/tax owner attestation and rollback
  rehearsal remain release gates.
- [x] Replay operations has a lazy-loaded authenticated workspace with typed
  response validation, bounded incident notes, server-authorized
  acknowledge/requeue actions, and explicit acknowledgement-versus-repair
  state language. Production browser accessibility automation and identity
  provider exercises remain open.
- [ ] UI meets WCAG 2.2 AA, supports keyboard navigation, screen readers, responsive layouts, localization, currencies, time zones, and RTL.
- [x] SPA workspace navigation now has a skip link, active-state semantics,
  stable controlled landmark, and tested post-navigation focus management under
  ADR-0142; full WCAG 2.2 AA/browser/assistive-technology evidence remains open.
- [x] Governance rule authoring now has a bounded structured Tier-1 workbench
  with canonical JSON preview and an explicit advanced JSON mode under
  ADR-0143; server validation, legal/tax review, and full browser accessibility
  evidence remain release gates.
- [x] Governance simulation now runs only against the tenant-scoped persisted
  rule under ADR-0144, with identity/digest/jurisdiction checks, retired-rule
  rejection, warm-cache execution, bounded response parsing, explicit
  unattested/non-settlement semantics, and Docker-backed tenant/tamper,
  cross-tenant, recovery, and retired-rule evidence. ADR-0145 adds a
  two-replica, facts-only latency harness with exact digest binding and local
  p95 evidence. Full browser/live-provider evidence and production approval
  remain release gates.
- [x] The primary quote and lazy operational workspaces have a rendered
  axe-core WCAG A/AA regression contract in
  `apps/web/src/accessibility.test.tsx`, run explicitly in CI by ADR-0111.
  This is an automated baseline; full WCAG 2.2 AA evidence across real
  keyboard/focus journeys, RTL, localization, and assistive technologies
  remains open.
- [x] ADR-0146 centralizes exact `BigInt` atomic formatting, locale-aware
  timestamps, explicit IANA time-zone validation, bounded locale selection,
  and RTL direction handling. Formatter and rendered settings tests cover
  `en-US`, `de-DE`, `ar-EG`, invalid time zones, and large amounts; full
  translation catalogs, real-browser responsive review, and assistive-
  technology evidence remain open.
- [x] ADR-0147 contains lazy-import and workspace render failures behind an
  accessible, redacted recovery boundary that preserves the global shell and
  cannot imply financial or chain success. Real-browser chunk-failure,
  service-worker/cache, CSP, and assistive-technology evidence remain open.
- [x] ADR-0148 adds a pinned Docker Playwright runner through the real NGINX
  edge. The journey contract covers keyboard skip/focus behavior, lazy
  workspace chunks, narrow RTL overflow, reduced motion, and bounded dynamic
  import failure. It now also runs pinned `axe-core` WCAG A/AA audits against
  the quote and every lazy workspace in the real browser; the 4/4 Docker run
  passed. Manual assistive-technology review and live-provider journeys
  remain open.
- [x] ADR-0149 removes the secure-context-only `crypto.randomUUID()` startup
  dependency by using a CSPRNG `getRandomValues()` fallback for client-only
  idempotency keys. Server UUIDv7 entity ownership is unchanged.
- [x] ADR-0150 adds the pure `settlement-adjustments` domain contract for
  immutable refunds, reversals, corrections, chargebacks, and remittance
  adjustments: UUIDv7 identity, signed exact deltas, conservation, sign
  semantics, lifecycle, evidence digest, idempotent posting, and cumulative
  non-negative bounds. Durable API/storage, four-eyes approval, UI, governed
  on-chain posting, and finalized reconciliation remain open.
- [x] ADR-0151 durable adjustment proposal/approval workflow has PostgreSQL
  migration, tenant-scoped API authorization, append-only evidence, outbox
  atomicity, and Docker integration evidence. The domain/API slice still does
  not claim operational refund or correction support until on-chain posting,
  UI review, finalized reconciliation, and legal/tax operations are complete.
- [x] ADR-0152 adds the real lazy-loaded settlement-adjustment operator
  workspace with exact-string client validation, memory-only token use,
  server-owned lifecycle rendering, API conflict/support-reference states,
  accessibility coverage, and a 3/3 pinned NGINX Playwright browser gate.
  On-chain posting, wallet signing, and finalized reconciliation remain open.
- [ ] ADR-0153 on-chain adjustment evidence contract is implemented and
  vectorized. The Docker conformance contract now covers successful recording,
  duplicate rejection, wrong receipt binding, non-conserving deltas, and
  unchanged token balances; the gate remains unchecked until the pinned Linux
  validator executes those cases and retains finalized evidence. The contract
  must not expose `posted` or `paid` before finalized reconciliation.
- [x] ADR-0164 enriches the canonical finalized receipt projection with the
  receipt PDA, payer, merchant, mint, nonce, and policy digest from raw
  settlement events. Migration `0034` is additive and readers fail closed on
  incomplete legacy identity; backfill validation and Linux validator evidence
  remain release gates.
- [x] ADR-0165 adds the server-owned finalized settlement-intent receipt
  binding port, requiring tenant, finalized lifecycle, signature, slot, and
  asset/mint agreement. Migration `0035` adds the bounded lookup indexes;
  compiler integration and conflict-contract evidence remain open.
- [x] ADR-0166 adds one-time Ed25519 wallet-proof challenges, immutable active
  proposer/approver bindings, source-calculation evidence checks, and a
  durable server-generated adjustment-evidence intent with retry-stable nonce.
  Migration `0036` is additive. ADR-0167 now implements the bounded finalized
  registry verifier and immutable registry snapshot; validator execution,
  wallet submission, and finalized reconciliation remain open.
- [x] The ADR-0166 PostgreSQL authority and evidence-intent contract drill is
  executable with `make adjustment-authority-drill`; it verifies active-key
  uniqueness, explicit rotation, immutable proof evidence, and one-time
  instruction-digest binding.
- [x] ADR-0167 implements finalized adjustment-registry verification: the
  derived PDA, owner, discriminator, exact layout, active state, authority
  keys, finalized commitment, provider agreement, and immutable compiler-intent
  evidence are enforced. The authenticated Docker endpoint integration is
  proven by `make integration-check`; validator-backed conformance, wallet
  submission, and finalized reconciliation remain release gates.
- [ ] ADR-0170/0171 wallet-bound submission and finalized recording gate:
  canonical signed `record_adjustment` bytes, bounded retry lifecycle, typed
  finalized event projection, exact intent/event matching, indexer-lag
  handling, wallet-workspace accessibility, and operator-visible
  unknown/conflict states must pass before production enablement.
- [x] ADR-0184 closes the status-only finalized-response gap for adjustments:
  migration `0048` persists the matched event index/evidence digest, the API
  emits a server-owned typed proof envelope, and the browser rejects finalized
  responses without that proof. This is local contract and integration
  evidence; live wallet/provider, validator, production SLO, and full
  WCAG/assistive-technology evidence remain open.
- [ ] ADR-0154 governed refund movement is implemented and proven against the
  pinned Linux validator for the published Refund scope; tax-vault and
  merchant-funder sources, payer-bound destination, claim cancellation,
  finalized reconciliation, and unknown-transaction handling must pass before
  any paid-state API is introduced.
- [ ] ADR-0173 server-owned refund-claim wallet orchestration is not enabled
  for production. The tenant-scoped funding configuration control plane,
  durable claim/execute intents, exact signed-byte verification, finalized
  claim-event reconciliation, automatic bounded claim worker, and
  transaction-bound execute balance proof are implemented. Linux validator
  conformance, live wallet evidence, concurrent/load/SLO evidence, and final
  operational approval remain required before refund movement is enabled.
- [x] ADR-0174 finalized refund-funding topology verification now validates the
  complete legacy-SPL account set at one finalized slot, requires provider
  agreement, binds immutable evidence to the reviewed configuration, and
  rejects Token-2022 until a separate compatibility decision. ADR-0181 adds
  UUIDv7 lease-fenced cross-replica ownership and stale reclaim. The Docker
  verifier drill proves one finalized read/activation under two replicas and
  one stale reclaim after claimant termination; retained production telemetry
  and Linux validator topology-rotation conformance remain release gates. The
  Ubuntu CI job retains the fixed-cardinality result artifact for regression
  review without treating synthetic evidence as production-provider proof.
- [x] ADR-0155 adds a typed, finalized-only `AdjustmentRefundExecuted`
  projection, raw-event binding, idempotent replay merge, and PostgreSQL
  migration `0032`. It is evidence only and does not close the ADR-0154
  validator, balance-reconciliation, or paid-state gates.
- [x] ADR-0156 adds the pure exact three-leg finalized balance proof with
  same-slot enforcement, checked-u64 underflow/overflow handling, and boundary
  tests, plus a bounded finalized post-state/topology reader. It is diagnostic
  only and is not a before/after proof source.
- [ ] ADR-0157 transaction-bound refund proof is implemented with finalized
  `getTransaction` pre/post token balances, exact account-index/event binding,
  immutable proof persistence, and bounded failover. Paid-state behavior
  remains gated on validator, replay, and operator-reconciliation evidence.
- [ ] ADR-0158 server-owned proof orchestration selects only unproved indexed
  executions, resolves claim topology inside the complete provider attempt,
  invokes checked domain arithmetic, and persists one bounded proof per cycle.
- [x] ADR-0159 exports a fixed-cardinality refund-proof worker operation and a
  durable configured-stream pending backlog gauge, with a versioned Prometheus
  alert and no identity-bearing metric labels. Scrape authorization,
  Alertmanager delivery, and retained production load/SLO evidence remain
  separate gates.
- [x] ADR-0180 binds claim finality to the exact persisted signature, proposer,
  immutable artifact, finalized `AdjustmentRefundClaimCreated` event, and
  recovery slot. Both the authenticated reconciliation endpoint and automatic
  indexer worker use the shared expectation builder and CAS/outbox transition;
  migration `0046` adds `SKIP LOCKED` stale-reclaim leases so replicas do not
  duplicate external recovery work; the worker exposes a fixed-cardinality
  pending backlog gauge and alert.
  Validator, live-wallet, and production-load evidence remain open.
- [ ] Loading, stale-policy, expired-quote, wallet rejection, RPC timeout, and reconciliation-required states have complete browser accessibility and live-provider evidence. The shared timeout boundary and unit coverage are implemented by ADR-0095.

## Public repository

- [x] README accurately states status, scope, limitations, architecture, and reproducible setup status; `make public-docs-check` verifies the public entry-point contract.
- [x] `AGENTS.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, and `CHANGELOG.md` are present, linked, and current for the implementation foundation; `make public-docs-check` verifies the public-file and status contract. This does not close the separate license or production-readiness gates.
- [ ] License choice is explicit before public source release.
- [x] ADR index, coverage matrix, API/IDL documentation, release notes, and the public-release contract are linked from the README.
- [ ] CI status, security scanning, dependency updates, and artifact provenance are visible without overstating readiness. ADR-0129 adds grouped Dependabot maintenance and a tag/manual release-candidate evidence workflow with checksums, SBOM, exact toolchain metadata, and validator/security evidence; a successful retained run, independent release review, and license decision remain open.
