# Production-readiness plan

Current evidence is intentionally partial: the repository now has authenticated
merchant-scoped quote persistence, database-enforced published-rule
immutability, a transactional quote outbox, bounded NATS JetStream delivery,
server-owned quote-bound calculation-attestation ingestion, and a cached
on-chain policy attestation binding. These are foundation gates, not completed
production readiness. Unchecked items remain release blockers.

ADR-0217 now retains the authoritative authenticated Docker integration stream
for successful and failed CI runs with bounded public-safe artifacts. This
improves reviewability of the synthetic service boundary but does not replace
Linux validator, live-provider, production-SLO, identity, or legal evidence.

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

On 2026-09-30, the conformance suite was extended with a real
`settle_document` case. The Dockerized builder produced artifact SHA-256
`0c13536b936cc2d15717ed142b0d55f3c6551c83c0b8f98691fa669089f511ae`; the
authoritative run then stopped at validator startup because this macOS Docker
runtime does not provide Agave 4.3.0's required `io_uring` capability. The
document case is therefore implemented and wired into the evidence verifier,
but its finalized transaction evidence remains unchecked until Linux/amd64 CI
executes the full matrix.

On 2026-09-30, `make integration-check` passed the credentialed document
quote failure matrix: missing and expired JWTs returned 401, two API replicas
converged on one UUIDv7 calculation, exact replay returned the persisted
evidence, changed input under the same idempotency key returned 409, and
cross-tenant replay/attestation/compilation attempts returned 404 with the
expected denial audit evidence. This closes the local Docker evidence gate;
staging identity-provider and production authorization evidence remain open.

ADR-0210 adds `make bootstrap`, the explicit Docker full-topology entry point.
Its dry-run contract is covered by `make check`; a successful runtime bootstrap
still requires Linux/amd64 validator capability evidence and is intentionally
not claimed from this macOS host.
ADR-0211 makes all host-facing Compose bindings, including validator RPC and
faucet ports, explicitly overridable so a second isolated topology can be
started without stopping an unrelated stack. A local attempt on this host
built all images and started every non-validator service healthy on a complete
alternate port map. Agave 4.3.0 then failed closed with
`io_uring_supported()` on the Docker Desktop LinuxKit runtime, so no validator
or full-topology runtime success is claimed from this macOS host.

ADR-0168 closes the local runner reproducibility gap: `make
solana-conformance` now executes the Vitest RPC client inside a pinned,
non-root Docker image and no longer depends on host Node.js or pnpm. This
improves bootstrap fidelity but does not close the independent Linux Agave
validator gate. Its health probes are now bounded and retain a typed failure
reason when Agave exits before RPC readiness; the canonical Docker Desktop
`io_uring NOT supported` diagnostic is classified as
`validator_io_uring_unsupported`, preventing a crashed validator from hanging
a developer or CI job.

ADR-0199 adds the authoritative postcondition gate. Ubuntu CI and the release
candidate workflow assert a Linux/amd64 Docker runtime, run the same
Dockerized conformance command, and verify the versioned result contract,
pinned Agave log, exact SBF hash, container identity, and complete real-RPC
case matrix before release evidence can pass. The local macOS `io_uring`
failure remains a typed non-zero result and cannot satisfy this gate; the
successful Linux run is still an open release item until retained evidence
exists.

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
- [ ] ADR-0214 extends the validator matrix with an extension-free Token-2022
  two-recipient Split Plan and exact finalized 0/25/45/30 balance assertions;
  the case is implemented, but successful Linux/amd64 execution remains
  required before Token-2022 Split Plan compatibility is evidenced.
- [ ] Validator-backed policy revocation proves a revoked attestation cannot
  authorize token movement under ADR-0141; the conformance implementation is
  present but requires successful Linux validator/CI execution.
- [ ] Live settlement CPI execution proves token-balance conservation and
  atomic rollback on the validator-backed localnet.
- [x] `u64` overflow, zero amount, cap, rate, and rounding boundaries are
  covered by tax-engine unit/property/golden tests and on-chain settlement
  validation tests. Validator-backed execution remains a separate release
  gate.
- [x] ADR-0202 and ADR-0203 make component, settlement, line, and invoice
  rounding explicit, preserve legacy component digests, reject ambiguous
  graphs, and prove conserving largest-remainder allocation, positive
  atomic-quantum semantics, canonical document identity, replay, and bounded
  document calculations through Rust/API tests and versioned golden vectors.
  Approved jurisdiction coverage, production document-quote authorization,
  persistence, and legal/tax owner approval remain separate coverage gates.
- [x] ADR-0204 document quotes have a dedicated authenticated persistence,
  encrypted replay, idempotency, and outbox boundary. The production
  `/v1/tax/document-quote` and replay routes are covered by the Docker
  two-replica, tenant-isolation, tamper/replay, and migration gates. The
  simulation route remains intentionally separate. Document-to-payment
  mapping remains open.
- [x] ADR-0205 document attestations use a dedicated signed contract, storage
  table, route, replay/conflict semantics, and outbox event. Docker/PostgreSQL
  evidence proves contention convergence, tenant isolation, publication, and
  aggregate-path document-ID rejection.
- [x] ADR-0206 document-to-payment binding has a separate domain, typed
  authorization, on-chain provenance receipt, wallet-submit verification, and
  Docker/PostgreSQL replay, conflict, isolation, and scope evidence. Full-
  document and whole-line subset payments are the bounded initial scope;
  partial-line re-rating remains a separate future contract.
- [x] ADR-0206's Docker conformance matrix now includes document-bound
  settlement, document receipt provenance bytes, amount conservation, and
  duplicate-receipt rejection.
- [ ] ADR-0206 finalized live-chain evidence and validator-backed
  `settle_document` conformance are still required before production release.
- [x] ADR-0207 adds a separate lazy document checkout workspace with server-
  verified attestation, explicit full/whole-line scope, typed compilation,
  exact wallet handoff, durable status recovery, and reconciliation controls.
  Component, accessibility, production build, and real-browser lazy-loading
  checks pass.
- [x] Document quote and checkout degraded-state journeys now have pinned
  real-browser evidence: unavailable document quote API and unavailable active
  settlement routes fail closed, preserve empty/evidence states, keep compile
  and signing unavailable, and pass WCAG 2.2 A/AA axe checks. Real injected-
  wallet attestation, signed submission, and finalized-chain browser evidence
  remain separate production gates.
- [x] The pinned browser suite also proves the successful document evidence
  handoff: authenticated quote evidence is rendered, issuer attestation is
  verified before compilation, server-owned authorization amounts are shown,
  and signing becomes available only after a durable reviewed artifact is
  returned. The journey uses synthetic API/provider fixtures and never claims
  submission or finalized settlement.
- [x] ADR-0208 adds the authenticated document-quote workspace. It validates
  bounded facts and evidence JSON locally, calls the production quote boundary,
  renders server-owned UUIDv7/result/expiry evidence, and hands the persisted
  calculation ID to document checkout. Unit, accessibility, production-build,
  and real-browser lazy-workspace checks pass.
- [x] Credentialed document-quote API journeys now prove tenant-isolated
  provider-signed jurisdiction evidence, two-replica idempotency replay and
  conflict behavior, cross-tenant denial/audit evidence, and missing/expired
  authentication responses in the Docker local-stack gate. Staging and live
  identity-provider evidence remain separate release gates.
- [x] A fresh local Docker document-quote load run passed 200/200 requests
  across two API replicas, 32/32 same-key contention responses, exact
  200/200/200 idempotency/calculation/outbox cardinality, and p50/p95/p99 of
  5.467/34.901/45.221 ms. This remains local synthetic evidence; retained CI,
  regional/resource-profile, and production SLO acceptance are still open.
- [x] ADR-0215 adds bounded Docker JSON resource-profile retention beside the
  aggregate and document benchmark summaries. The aggregate profile-enabled
  run captured four named containers in one timestamped sampling epoch, and
  the follow-up document run did the same while preserving the exact
  persistence and latency invariants; CI retention, regional profiles, and
  production SLO acceptance remain open.
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
- [x] ADR-0216 persists finalized verifier-derived mint precision on immutable
  settlement-route versions, exposes it through the route API, rejects missing
  or out-of-range browser values, and formats checkout atomic amounts with
  explicit `BigInt` decimal scaling. Migration and Linux validator-backed route
  activation evidence remain release gates.
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
  separately governed system-wide finance view remain release gates. ADR-0192
  now adds real-browser evidence for server-shaped mismatch, history, and
  unauthorized states. ADR-0193 adds a server-owned database-clock freshness
  bit and stale stop state; live-provider, validator, timeout, scheduler SLO,
  and production identity evidence remain open.
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
- [x] ADR-0224 adds a shared calculation-attestation golden vector consumed by
  Rust and TypeScript, proving canonical payload and digest parity without
  moving issuer or settlement authority into the browser. Production key
  rotation, HSM/KMS ceremony, live issuer evidence, and finalized chain
  evidence remain separate release gates.
- [x] ADR-0225 adds a separately deployable warm Rust Tax Engine service with
  authenticated bounded calculation/document endpoints, immutable rule-digest
  cache keys, bounded blocking concurrency, Docker health checks, metrics, and
  lifecycle coverage. The API keeps the settlement hot path in-process;
  staging/production service identity, TLS/mTLS, rotation, regional SLO, and
  load evidence remain release gates.
- [x] ADR-0226 extends the shared non-local deployment preflight to require
  Tax Engine environment parity, complete mTLS paths, and rejection of the
  local service token. Secret validity, service identity, certificate rotation,
  network policy, and production SLO evidence remain separate release gates.
- [x] ADR-0227 adds direct Rustls mTLS serving to the Tax Engine adapter;
  non-local startup requires a server certificate/key and client CA, and
  malformed or incomplete material fails closed without plaintext fallback.
  Explicit provider selection and trusted/untrusted client-handshake tests
  cover the transport boundary; production certificate lifecycle, CA rotation,
  compromise response, and regional mTLS/SLO evidence remain open.
- [x] ADR-0228 adds a Unix `SIGHUP` reload path that validates replacement
  certificate, key, client CA, verifier, and ALPN in a blocking task before an
  atomic Rustls snapshot swap. Valid and malformed replacement tests prove the
  new identity is usable and the last known-good identity remains live after a
  failed reload.
- [ ] Prove production secret-manager atomic file delivery, permissions,
  certificate/key rotation, CA overlap and compromise response, regional
  reload latency, connection-drain behavior, and rejected-reload alert/on-call
  evidence.
- [x] ADR-0230 adds an isolated real-container Tax Engine contract for image
  startup, liveness/readiness authentication, exact calculation, body limits,
  and canonical metrics. Linux/amd64 retention, production mTLS, resource
  profiles, regional SLOs, and production identity issuance remain open.
- [x] ADR-0231 makes Tax Engine request rejection explicit: malformed JSON is
  `400` while the 128 KiB admission limit remains `413`, with focused and
  pinned-container evidence. Production gateway preservation and retry
  telemetry remain open.
- [x] ADR-0232 adds a real-image warm Tax Engine concurrency contract: 256
  authenticated aggregate requests at concurrency 32, exact response checks,
  bounded timeouts, and p50/p95/p99 metadata. The 2026-10-01 local arm64 run
  returned 256/256 HTTP 200 responses with p50/p95/p99 of 1.344/3.650/10.302
  ms. This is local synthetic service evidence; Linux/amd64 retention,
  resource profiles, regional SLOs, mTLS, and production identity issuance
  remain open.
- [x] ADR-0233 adds a real pinned-image staging-mode mTLS contract covering
  trusted/untrusted client certificates, no-client rejection, bearer auth,
  exact calculation, h2 ALPN, invalid CA reload retention, and valid reload.
  Production secret-manager delivery, certificate lifecycle/revocation, CA
  overlap, regional reload SLO, identity issuance/rotation, and on-call evidence remain
  open.
- [x] ADR-0234 adds a real API container contract for file-backed EdDSA JWKS
  rotation. Valid `SIGHUP` replacement rejects the removed key and accepts the
  new key; malformed replacement retains the last known-good snapshot. This
  closes the local API rotation contract only; provider-managed overlap and
  revocation, secret-manager permissions, regional reload timing, and
  production identity/on-call evidence remain open.
- [x] ADR-0235 adds a real API container contract for file-backed
  jurisdiction-provider key rotation. Valid evidence signed by the old key is
  rejected after replacement, the new key is accepted, and malformed
  replacement retains the last known-good registry. Live provider overlap and
  revocation, HSM/KMS custody, secret-manager permissions, regional reload
  timing, legal ownership, and production on-call evidence remain open.
- [x] ADR-0236 binds every non-local Tax Engine mTLS connection to an exact
  `spiffe://` URI SAN from a bounded immutable allow-list. Chain validation
  alone, subject/CN matching, and request-path introspection are rejected;
  wrong-identity certificates fail before HTTP authentication. Production
  identity issuance/rotation, CA overlap/revocation, regional handshake SLO,
  and alert/on-call evidence remain open.
- [x] ADR-0237 adds mutually exclusive inline/file-backed Tax Engine bearer
  credentials, bounded file validation, lock-free token snapshots, and
  atomic Unix `SIGHUP` rotation, plus fixed-cardinality reload telemetry and a
  critical rejection alert. Production secret-manager delivery,
  permissions, overlap/revocation, compromise response, regional reload SLO,
  and alert/on-call evidence remain open.
- [ ] ADR-0238 governed refund-claim cancellation has a server-owned compiler,
  exact signed-byte verification, a separate UUIDv7 cancellation intent, and
  finalized cancellation-event reconciliation. The compiler, verifier, and
  durable intent migration are implemented; API/UI lifecycle, Linux
  validator/provider,
  governance-key, production-load, UI accessibility, and operational evidence
  remain required before cancellation is enabled in production.
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
- [x] The same Docker contention drill exercises 16 concurrent refund nonce
  callers and 16 same-key refund-claim callers: nonce allocations are unique
  with aggregate 1..16, and the idempotent claim converges to exactly one
  intent, one initial lifecycle event, and one outbox row. This is database
  contract evidence.
- [x] The authenticated Docker API contention drill exercises the real claim
  preparation and finalized execute-preparation routes with 16 concurrent
  same-key callers. It proves HTTP 200 convergence to one execute intent, one
  lifecycle event, one outbox row, and one request/artifact digest, and covers
  the replay-authority boundary fixed by ADR-0187. This uses
  synthetic server-owned finalized evidence; retry telemetry, pool saturation,
  live wallet/provider submission, Linux validator conformance, and
  production-scale SLO evidence remain open.
- [x] The same drill now signs the server-owned claim and execute artifacts
  with deterministic synthetic test-only keypairs and submits each exact
  transaction through the authenticated API to the RPC fixture. Concurrent
  duplicate submissions produce one additional RPC send per intent and leave
  both durable intents in `submitted`; this proves transport fencing only and
  does not establish finalized chain movement or paid state.
- [x] ADR-0188 extends that drill with 16 client-observed latency samples and
  reports p50/p95/p99 in milliseconds with workload metadata. This is bounded
  Docker-local diagnostic evidence; regional, live-RPC, production-volume,
  and finality SLO acceptance remain open.
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
- [ ] HSM/KMS and multisig ceremonies are documented and rehearsed. ADR-0219
  now keeps the deployment preflight aligned with both supported JWKS input
  forms, but it does not prove provider-managed rotation or key-ceremony
  completion.
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
- [x] ADR-0209 adds a separate two-line document-quote workload with
  document-specific persistence/outbox invariants. The 2026-09-30 local
  Docker run completed 200 requests at concurrency 16 across two API replicas
  with 0% errors, 32/32 same-key contention success, and exact durable counts;
  p50/p95/p99 was 5.308/24.216/34.742 ms. A successful retained CI artifact
  and production-like regional/resource-profile measurements remain open.
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
  adds versioned alert rules and pinned-image `promtool` validation. ADR-0191
  and `make alert-delivery-drill` now prove the local Prometheus → Alertmanager
  → receiver route with a real isolated scrape outage, but this gate remains
  open until scrape access policy, production exporters, authenticated
  receiver delivery, dashboards, alerts, and load-derived SLO evidence are
  verified.
- [x] ADR-0201 implements the shared opt-in OTLP gRPC trace bootstrap across
  API, worker, indexer, and route-verifier. Configuration is bounded and
  insecure HTTP is local-only; `make otel-trace-drill` proves a real API
  request reaches the Docker collector with UUIDv7 correlation and no
  prohibited financial payload fields. Authenticated production exporters,
  retention, dashboards, alert routing, and on-call evidence remain open. CI
  and release-candidate workflows now retain the drill artifact for regression
  review without treating it as production telemetry evidence.
- [x] ADR-0123 exposes fixed-cardinality queued, leased, dead-lettered, and
  leased-slot-span replay gauges from the configured indexer stream, with
  Prometheus rules for backlog and dead-letter alerts. ADR-0191 proves local
  receiver delivery; scrape authorization, lease-expiry/claim-loss counters,
  and production alert response remain open.
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
- [x] ADR-0221 gives the migration runner the same structured-lifecycle
  guarantee: PostgreSQL driver failures cancel migration work and no detached
  connection task survives runner completion. Staging migration-service and
  production rollback evidence remain open.
- [x] ADR-0222 makes shutdown-signal listener failures terminate API, worker,
  indexer, and route-verifier processes instead of silently continuing. Real
  supervisor restart, signal delivery, and graceful-termination evidence
  remain open.
- [x] ADR-0223 makes unexpected fixed-operation metric-registration failures
  visible through bounded structured logs while keeping metrics non-authoritative
  for financial and reconciliation paths. Production scrape authorization,
  alert delivery, and on-call response evidence remain open.
- [x] Adaptive settlement provider health is exposed through bounded API gauges
  for score, consecutive failures, and cooldown milliseconds using only a
  fixed provider-index label (ADR-0117). Endpoint, merchant, request, and
  signature identity are excluded; production scrape and alert delivery remain
  open.
- [x] Indexer backfill and replay are automated. ADR-0189 and
  `make indexer-replay-drill` run two real indexer replicas against PostgreSQL
  18 and a bounded finalized-RPC fixture, proving automatic lease-fenced gap
  completion and forward-checkpoint isolation; live provider and production
  recovery SLO evidence remain open.
- [ ] Replay gaps have durable leases, dead-letter state, and retry exhaustion
  alerts. ADR-0086 now provides authenticated list/acknowledge/requeue
  operations plus immutable operator-action evidence; operator identity
  lifecycle, and ADR-0190 proves the automatic two-replica retry-exhaustion
  and dead-letter path. Production alert delivery, on-call response, and live
  recovery SLO evidence remain open.
- [x] Retained raw event evidence and normalized `indexed_receipts` rebuild from
  a clean projection with byte-identical results. ADR-0088 defines the
  canonical decoder and bounded keyset-paginated primitive; `make
  projection-rebuild-drill` proves digest equality and checkpoint invariance in
  Docker and CI. Merchant, vault, and reporting projections require separate
  raw-evidence contracts and remain open.
- [x] Vault balance reconciliation is scheduled and its matched/mismatch fixed-cardinality alert inputs are exercised through the real Docker indexer and synthetic finalized-account RPC boundary by ADR-0136; live provider, opening-balance ceremony, production Alertmanager delivery, and Token-2022 extension evidence remain open.
- [x] Public-safe runbooks cover policy compromise, upgrade rollback, vault
  release incident, RPC outage, indexer gap, and backup/restore. Production
  role binding, on-call ownership, and measured drills remain deployment
  gates.
- [ ] Mainnet configuration is separate from staging and requires approval. ADR-0130 now enforces explicit `local`/`staging`/`production` startup environments and rejects simulation or insecure RPC outside local; separate secret provisioning, deployment approval, allow-listing, and mainnet evidence remain open.
- [x] Repository-side staging/production preflight rejects local databases, RPC/NATS/identity defaults, insecure flags, and synthetic seed fixtures under ADR-0139; actual secret validity, provider health, deployment approval, and live staging/production evidence remain open.
- [x] ADR-0220 keeps the deployment preflight aligned with the API's structured
  or compatibility jurisdiction-provider key forms and rejects mixed or local
  provider material; live provider rotation, signature, custody, and staging
  evidence remain open.
- [ ] Emergency pause/unpause is exercised on localnet and staging with
  multisig/timelock authority, event alerting, and a documented break-glass
  runbook. ADR-0131 now provides finalized event projection, fixed-cardinality
  active/unknown alerts, and localnet pause-blocking assertions; staging
  governance, delivery, and incident evidence remain required.
- [x] ADR-0218 makes release bundles record named SHA-256 hashes for the final
  Solana program binary and resolved Compose configuration, and verifies both
  through a synthetic contract test. A successful Linux release-candidate run,
  independent review, and production signing/deployment ceremony remain open.
- [x] ADR-0217 makes the Docker integration evidence upload fail closed when
  the expected capture directory is missing; retained Linux evidence for the
  exact release commit remains required.
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
- [x] ADR-0196 adds an exact-second client expiry guard that prevents stale
  quote compilation and wallet signing while preserving server-owned expiry
  validation. The 10/10 pinned browser run proves the synthetic expired-quote,
  wallet-rejection rendering, and WCAG audit; live-provider and full recovery evidence remain
  open.
- [x] ADR-0197 adds strict browser parsing for calculation and replay evidence,
  preserving signed i128 atomic values and rejecting malformed identities,
  digests, statuses, component traces, and expiry before rendering. API,
  browser, live-provider, and production identity evidence remain separate
  gates.
- [x] ADR-0198 adds strict vault-release response parsing and canonical Rust
  `u64` wire strings, plus an authoritative claim reload after transitions;
  malformed lifecycle evidence and stale local projections fail closed. Live
  provider, validator, production identity, and full assistive-technology
  evidence remain separate gates.
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
- [ ] UI meets the complete WCAG 2.2 AA product gate, including manual screen-
  reader/assistive-technology review, responsive behavior, complete
  localization catalogs, currencies, time zones, and RTL. The deterministic
  automated portion is now evidenced separately by the pinned
  `make web-browser-check` contract.
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
  The 2026-10-01 pinned NGINX/Playwright run also passed 15/15 real-browser
  journeys, including keyboard/focus, narrow RTL, reduced-motion, lazy-load
  recovery, API failure states, quote expiry, wallet rejection, document
  evidence handoff, and reconciliation mismatch/authorization states. This is
  strong automated evidence, but manual screen-reader/assistive-technology,
  complete localization, live-wallet/provider, and production identity review
  remain open.
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
  the quote, API-unavailable fail-closed state, and every lazy workspace in the
  real browser; the 15/15 Docker run passed. Manual assistive-technology review
  and live-provider journeys
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
  unknown/conflict states must pass before production enablement. ADR-0194
  closes the local Docker evidence slice: real wallet-byte serialization,
  concurrent [200,409] submission, exactly one fixture RPC send, explicit
  finalized observation, typed event/raw evidence, and proof-envelope replay.
  Live wallet/provider, Linux validator, production load/SLO, and full
  assistive-technology evidence remain open.
- [x] ADR-0184 closes the status-only finalized-response gap for adjustments:
  migration `0048` persists the matched event index/evidence digest, the API
  emits a server-owned typed proof envelope, and the browser rejects finalized
  responses without that proof. This is local contract and integration
  evidence; live wallet/provider, validator, production SLO, and full
  WCAG/assistive-technology evidence remain open.
- [x] ADR-0200 implements the Wallet Standard compatibility boundary for
  checkout, adjustment, and refund workspaces: capability-filtered discovery,
  explicit multi-provider selection, selected-account identity checks,
  an accessible multi-account selector, returned-byte verification, and legacy
  fallback only when no compatible standard provider is registered.
  Unit/component and pinned real-browser evidence pass; live provider/account
  matrix, Linux validator/provider, production load/SLO, and
  assistive-technology evidence remain open.
- [ ] ADR-0154 governed refund movement is implemented and proven against the
  pinned Linux validator for the published Refund scope; tax-vault and
  merchant-funder sources, payer-bound destination, claim cancellation,
  finalized reconciliation, and unknown-transaction handling must pass before
  any paid-state API is introduced. The pinned client now includes a
  destination-substitution rejection and governance cancellation with
  unchanged tax-vault, funder, and payer balances; retained Linux execution
  evidence remains required.
- [ ] ADR-0173 server-owned refund-claim wallet orchestration is not enabled
  for production. The tenant-scoped funding configuration control plane,
  durable claim/execute intents, exact signed-byte verification, finalized
  claim-event reconciliation, automatic bounded claim worker, and
  transaction-bound execute balance proof are implemented. Linux validator
  conformance, live wallet evidence, load/SLO evidence, and final operational
  approval remain required before refund movement is enabled. Bounded
  concurrent idempotency/submission evidence is now retained by the API and
  browser wallet-workspace drills, but it is not production-scale evidence.
- [x] ADR-0186 adds bounded storage-local retries for refund nonce allocation
  and claim/execute intent creation, retrying only PostgreSQL `40001` and
  `40P01` conflicts while preserving immutable request and idempotency bindings.
  PostgreSQL contention evidence, load/SLO measurement, and production approval
  remain open.
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
- [x] ADR-0195 adds a real Docker proof-worker drill for ADR-0157: the indexer
  resolves the finalized claim PDA, reads the exact transaction-bound event and
  three-leg pre/post token balances, persists one immutable proof, finalizes the
  execute intent/outbox exactly once, and rejects mutation/replay duplication.
  The RPC is a synthetic finalized provider contract; live provider/validator,
  production load/SLO, and paid-state gates remain open.
- [ ] ADR-0157 transaction-bound refund proof is implemented with finalized
  `getTransaction` pre/post token balances, exact account-index/event binding,
  immutable proof persistence, and bounded failover. Paid-state behavior
  remains gated on validator, replay, and operator-reconciliation evidence.
- [x] ADR-0158 server-owned proof orchestration now has a bounded indexer
  worker with narrow candidate/provider/sink ports. Focused tests prove one
  candidate per cycle, exact checked arithmetic, empty-cycle behavior, and
  provider-topology substitution rejection. ADR-0185 completes the
  lease-fenced PostgreSQL candidate ownership, proof-exclusion selection,
  concurrent token fencing, transactional raw-event binding, and canonical
  production worker wiring. Linux validator/provider evidence, duplicate
  persistence against live database fixtures, and retained production SLO
  evidence remain separate release gates.
- [x] ADR-0185 lease-fenced refund-proof candidate ownership is implemented
  and verified with Docker-backed concurrent claimers, stale reclaim,
  exact-token fencing, proof exclusion, and service wiring. Linux validator
  conformance, finalized-provider evidence, duplicate-persistence fixtures,
  and retained production SLO evidence remain separate release gates.
- [x] CI and release-candidate workflows retain ADR-0185 lease-fencing
  evidence, including concurrent claim ownership, stale-token fencing,
  proof exclusion, and bounded dead-letter handling.
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
- [x] ADR-0213 exposes the server-derived typed finalized claim-event proof to
  the browser, rejects lifecycle/evidence mismatches, and renders event,
  slot, index, conserved amounts, and digest evidence without implying legal
  or tax compliance. Live injected-wallet and production-provider evidence
  remain open.
- [x] The pinned NGINX/Playwright suite covers finalized claim proof
  disclosure, axe validation, reload, and durable intent recovery. This is
  synthetic API evidence only; live injected-wallet, RPC-provider, and
  finalized-chain evidence remain open.
- [ ] Loading, stale-policy, expired-quote, wallet rejection, RPC timeout, and reconciliation-required states have complete browser accessibility and live-provider evidence. The shared timeout boundary and unit coverage are implemented by ADR-0095.

## Public repository

- [x] README accurately states status, scope, limitations, architecture, and reproducible setup status; `make public-docs-check` verifies the public entry-point contract.
- [x] `AGENTS.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, and `CHANGELOG.md` are present, linked, and current for the implementation foundation; `make public-docs-check` verifies the public-file and status contract. This does not close the separate license or production-readiness gates.
- [ ] License choice is explicit before public source release.
- [x] ADR index, coverage matrix, API/IDL documentation, release notes, and the public-release contract are linked from the README.
- [ ] CI status, security scanning, dependency updates, and artifact provenance are visible without overstating readiness. ADR-0129 adds grouped Dependabot maintenance and a tag/manual release-candidate evidence workflow with checksums, SBOM, exact toolchain metadata, and validator/security evidence; a successful retained run, independent release review, and license decision remain open.
