# Solana Tax Protocol

This directory is the architecture and delivery baseline for a production-grade, low-latency Solana payment-routing protocol. The protocol settles a payment, withholds an attested tax amount into a program-controlled vault, and distributes the net amount to a pre-authorized split plan in one atomic transaction.

## Documents

- [Source analysis and requirement boundary](source-analysis.md)
- [System architecture](architecture/system-architecture.md)
- [Domain model and bounded contexts](architecture/domain-model.md)
- [Refund Movement bounded context](adr/0175-durable-refund-claim-execute-intents.md)
- [Tier-1 tax engine](architecture/tax-engine.md)
- [Server-owned FX evidence bounded context](architecture/fx-evidence.md)
- [FX provider policy and issuance boundary](adr/0246-fx-provider-policy-and-issuance.md)
- [FX provider policy snapshot lifecycle](adr/0247-fx-provider-policy-snapshot-reload.md)
- [Registry-bound FX evidence issuance](adr/0248-registry-bound-fx-issuance.md)
- [FX provider identity allowlist](adr/0249-fx-provider-identity-allowlist.md)
- [Deterministic FX provider agreement](adr/0250-deterministic-fx-provider-agreement.md)
- [Accelerated Tier-1 delivery and decision freeze](adr/0251-accelerated-tier1-delivery-and-decision-freeze.md)
- [Bounded concurrent FX provider acquisition](adr/0252-bounded-concurrent-fx-provider-acquisition.md)
- [Bounded async FX signing boundary](adr/0253-bounded-async-fx-signing-boundary.md)
- [Deterministic FX acquisition outcome order](adr/0254-deterministic-fx-acquisition-outcome-order.md)
- [Bounded HTTP FX provider adapter](adr/0255-bounded-http-fx-provider-adapter.md)
- [Document quote, attestation, payment binding, and checkout contracts](adr/0204-authoritative-document-quote-boundary.md)
- [Jurisdiction-resolution bounded context](architecture/jurisdiction-resolution.md)
- [Gold-standard UI/UX](architecture/ui-ux.md)
- [Tier-1 decision register](operations/decision-register.md)
- [Architecture alternative analysis](architecture/alternative-analysis.md)
- [Docker full-topology bootstrap](adr/0210-docker-full-topology-bootstrap.md)
- [Docker host-port override policy](adr/0211-compose-host-port-overrides.md)
- [Production Rust panic-safety contract](adr/0212-production-rust-panic-safety-contract.md)
- [Typed finalized refund-claim evidence](adr/0213-typed-finalized-refund-claim-evidence.md)
- [Extension-free Token-2022 Split Plan conformance](adr/0214-token-2022-split-conformance.md)
- [Bounded load-test resource-profile evidence](adr/0215-bounded-load-resource-profile.md)
- [Server-owned mint precision at the settlement-route boundary](adr/0216-server-owned-mint-precision-at-the-route-boundary.md)
- [Retained Docker integration evidence](adr/0217-retained-docker-integration-evidence.md)
- [Explicit release integrity hashes](adr/0218-explicit-release-integrity-hashes.md)
- [JWKS JSON deployment-preflight parity](adr/0219-jwks-json-preflight-parity.md)
- [Jurisdiction-provider JSON deployment-preflight parity](adr/0220-jurisdiction-provider-json-preflight-parity.md)
- [Structured migration-connection lifecycle](adr/0221-structured-migration-connection-lifecycle.md)
- [Fail-closed shutdown-signal handling](adr/0222-fail-closed-shutdown-signal-handling.md)
- [Observable non-authoritative metrics failures](adr/0223-observable-non-authoritative-metrics-failures.md)
- [Cross-language calculation-attestation vector](adr/0224-cross-language-calculation-attestation-vector.md)
- [Dedicated warm Tax Engine service adapter](adr/0225-dedicated-warm-tax-engine-service.md)
- [Tax Engine deployment preflight boundary](adr/0226-tax-engine-deployment-preflight.md)
- [Rustls mTLS transport for the Tax Engine service](adr/0227-tax-engine-rustls-mtls-transport.md)
- [Atomic Tax Engine mTLS certificate reload](adr/0228-tax-engine-mtls-certificate-reload.md)
- [Canonical Tax Engine observability identity](adr/0229-tax-engine-observability-identity.md)
- [Executable Tax Engine container runtime contract](adr/0230-tax-engine-runtime-contract.md)
- [Explicit Tax Engine request-rejection semantics](adr/0231-tax-engine-request-rejection-semantics.md)
- [Warm Tax Engine service concurrency evidence](adr/0232-tax-engine-service-concurrency-evidence.md)
- [Real-container Tax Engine mTLS contract](adr/0233-tax-engine-container-mtls-contract.md)
- [Atomic API JWKS reload for bounded identity rotation](adr/0234-api-jwks-atomic-reload.md)
- [Atomic jurisdiction-provider key registry reload](adr/0235-jurisdiction-provider-key-atomic-reload.md)
- [Tax Engine mTLS client identity binding](adr/0236-tax-engine-mtls-client-identity.md)
- [Atomic Tax Engine service-token rotation](adr/0237-tax-engine-service-token-rotation.md)
- [Governed refund-claim cancellation workflow](adr/0238-governed-refund-claim-cancellation-workflow.md)
- [Catalog-driven UUIDv7 invariant closure](adr/0239-catalog-driven-uuidv7-invariant-closure.md)
- [Lease-safe refund-claim cancellation transition trigger](adr/0240-refund-claim-cancellation-lease-trigger.md)
- [Settlement transaction and state machine](architecture/settlement-flow.md)
- [Bounded immutable Split Plan decision](adr/0091-bounded-immutable-split-plan-context.md)
- [Finalized split-settlement evidence projection](adr/0093-finalized-split-settlement-evidence-projection.md)
- [Governed Split Plan checkout review](adr/0094-governed-split-plan-checkout-review.md)
- [Validator-backed Split Plan conformance](adr/0140-validator-backed-split-settlement-conformance.md)
- [Validator-backed policy revocation conformance](adr/0141-validator-backed-policy-revocation-conformance.md)
- [SPA workspace focus-management contract](adr/0142-spa-workspace-focus-management.md)
- [Structured Tier-1 rule workbench](adr/0143-structured-tier-1-rule-workbench.md)
- [Server-owned persisted rule simulation](adr/0144-server-owned-persisted-rule-simulation.md)
- [Persisted-rule simulation latency evidence](adr/0145-reproducible-rule-simulation-latency-evidence.md)
- [Locale, time-zone, and RTL display contract](adr/0146-centralized-locale-timezone-and-rtl-display-contract.md)
- [Lazy workspace error containment and recovery](adr/0147-lazy-workspace-error-containment.md)
- [Reproducible real-browser UI journeys](adr/0148-real-browser-ui-journey-contract.md)
- [Secure client idempotency identity](adr/0149-secure-client-idempotency-identity.md)
- [Bounded browser API deadlines and cancellation](adr/0095-bounded-browser-api-deadlines.md)
- [Authenticated, idempotent rule-draft authoring](adr/0096-authenticated-rule-draft-authoring.md)
- [Fail-closed browser durable-intent recovery](adr/0098-browser-durable-intent-recovery.md)
- [Runtime validation of settlement artifacts](adr/0099-runtime-validation-of-settlement-artifacts.md)
- [Server-owned calculation-attestation ingestion](adr/0100-server-owned-calculation-attestation-ingestion.md)
- [Deterministic jurisdiction-resolution bounded context](adr/0101-deterministic-jurisdiction-resolution.md)
- [Jurisdiction evidence binding](adr/0102-bind-jurisdiction-evidence-to-calculation.md)
- [Server-verified jurisdiction quote boundary](adr/0103-server-verified-jurisdiction-quote-boundary.md)
- [Governed vault-release claims](adr/0105-governed-vault-release-claims.md)
- [Settlement Route Registry bounded context](architecture/settlement-route-registry.md)
- [Settlement route verification architecture](architecture/settlement-route-verification.md)
- [Low-latency and reliability architecture](architecture/low-latency.md)
- [Toolchain compatibility matrix](architecture/toolchain-compatibility.md)
- [Security and threat model](architecture/security-threat-model.md)
- [Operations, observability, and SRE](architecture/operations.md)
- [SLO and error-budget policy](operations/slo-error-budget-policy.md)
- [Dependency register and exit plan](operations/dependency-register.md)
- [API contract baseline](api-contract.md)
- [Implementation plan](plans/implementation-plan.md)
- [Tier-1 product standard](plans/tier-1-product-standard.md)
- [Tax coverage matrix](product/tax-coverage-matrix.md)
- [Production-readiness plan](plans/production-readiness.md)
- [Public repository release contract](public-repository-release.md)
- [Architecture decision records](adr/README.md)
- [ADR-0150: Immutable settlement adjustments](adr/0150-immutable-settlement-adjustments.md)
- [ADR-0151: Durable settlement-adjustment workflow](adr/0151-durable-settlement-adjustment-workflow.md)
- [ADR-0152: Settlement-adjustment operator workspace](adr/0152-settlement-adjustment-operator-workspace.md)
- [ADR-0153: On-chain adjustment evidence boundary](adr/0153-on-chain-adjustment-evidence-boundary.md)
- [ADR-0154: Governed adjustment refund movement](adr/0154-governed-adjustment-refund-movement.md)
- [ADR-0155: Finalized refund execution evidence projection](adr/0155-finalized-refund-execution-evidence.md)
- [ADR-0156: Finalized refund balance proof](adr/0156-finalized-refund-balance-proof.md)
- [ADR-0157: Transaction-bound refund balance proof](adr/0157-transaction-bound-refund-balance-proof.md)
- [ADR-0158: Server-owned refund-proof orchestration](adr/0158-server-owned-refund-proof-orchestration.md)
- [ADR-0159: Fixed-cardinality refund-proof operational observability](adr/0159-refund-proof-operational-observability.md)
- [ADR-0164: Canonical settlement-receipt identity projection](adr/0164-canonical-settlement-receipt-identity-projection.md)
- [ADR-0165: Server-owned settlement-intent receipt binding](adr/0165-server-owned-settlement-receipt-binding.md)
- [ADR-0166: Wallet-bound adjustment authorities and durable compiler intents](adr/0166-wallet-bound-adjustment-authorities.md)
- [ADR-0167: Finalized server-owned adjustment-registry verification](adr/0167-finalized-adjustment-registry-verification.md)
- [ADR-0168: Dockerized Solana conformance runner](adr/0168-dockerized-solana-conformance-runner.md)
- [ADR-0169: SBF stack-frame safety boundary](adr/0169-sbf-stack-frame-safety.md)
- [ADR-0170: Wallet-bound adjustment submission and finalized recording evidence](adr/0170-adjustment-submission-and-finalized-recording.md)
- [ADR-0171: Wallet-bound adjustment browser workflow](adr/0171-adjustment-wallet-workspace.md)
- [ADR-0172: Exact decimal wire encoding for Solana `u64` values](adr/0172-exact-chain-u64-wire-encoding.md)
- [ADR-0173: Server-owned refund-claim wallet orchestration](adr/0173-refund-claim-wallet-orchestration.md)
- [ADR-0174: Finalized refund-funding topology verification](adr/0174-finalized-refund-funding-topology-verification.md)
- [ADR-0175: Durable refund claim and execute intents](adr/0175-durable-refund-claim-execute-intents.md)
- [ADR-0179: Durable, proof-gated refund-execute intent](adr/0179-durable-refund-execute-intent.md)
- [ADR-0180: Proof-driven refund-claim finality reconciliation](adr/0180-proof-driven-refund-claim-finality.md)
- [ADR-0181: Lease-fenced refund-funding topology verification](adr/0181-lease-fenced-refund-funding-verification.md)
- [ADR-0182: Encrypted backup artifact contract](adr/0182-encrypted-backup-artifact-contract.md)
- [ADR-0183: Expand/contract migration rollback rehearsal](adr/0183-migration-rollback-rehearsal.md)
- [ADR-0184: Typed finalized adjustment proof envelope](adr/0184-typed-finalized-adjustment-proof-envelope.md)
- [ADR-0185: Lease-fenced refund-proof candidate ownership](adr/0185-lease-fenced-refund-proof-candidate-ownership.md)
- [ADR-0186: Bounded refund-intent serialization retries](adr/0186-bounded-refund-intent-serialization-retries.md)
- [ADR-0187: Refund-execute idempotent replay authorization](adr/0187-refund-execute-idempotent-replay-authorization.md)
- [ADR-0188: Refund-execute contention latency evidence](adr/0188-refund-execute-contention-latency-evidence.md)
- [ADR-0189: Automated indexer replay recovery contract](adr/0189-automated-indexer-replay-recovery-contract.md)
- [ADR-0190: Indexer replay retry-exhaustion contract](adr/0190-indexer-replay-retry-exhaustion-contract.md)
- [ADR-0191: Alertmanager delivery contract](adr/0191-alertmanager-delivery-contract.md)
- [ADR-0192: Real-browser vault-reconciliation evidence](adr/0192-real-browser-vault-reconciliation-evidence.md)
- [ADR-0193: Server-owned reconciliation freshness](adr/0193-server-owned-reconciliation-freshness.md)
- [ADR-0194: Wallet-bound adjustment finality drill](adr/0194-adjustment-wallet-finality-drill.md)
- [ADR-0195: Transaction-bound refund proof drill](adr/0195-transaction-bound-refund-proof-drill.md)
- [ADR-0196: Client quote-expiry boundary](adr/0196-client-quote-expiry-boundary.md)
- [ADR-0197: Strict browser tax-evidence response parsing](adr/0197-strict-browser-tax-evidence-responses.md)
- [ADR-0198: Strict vault-release lifecycle response contract](adr/0198-strict-vault-release-response-contract.md)
- [ADR-0199: Authoritative Linux validator evidence contract](adr/0199-linux-validator-evidence-contract.md)
- [ADR-0200: Solana Wallet Standard signing boundary](adr/0200-solana-wallet-standard-boundary.md)
- [ADR-0201: Bounded OTLP trace-export boundary](adr/0201-otlp-trace-export-boundary.md)
- [ADR-0202: Explicit tax rounding scope and deterministic residual allocation](adr/0202-explicit-tax-rounding-scope.md)
- [ADR-0203: Versioned line and invoice tax facts](adr/0203-versioned-line-invoice-tax-facts.md)
- [ADR-0204: Authoritative document quote boundary](adr/0204-authoritative-document-quote-boundary.md)
- [ADR-0205: Dedicated document-calculation attestation boundary](adr/0205-document-calculation-attestation-boundary.md)
- [ADR-0206: Document-to-payment binding](adr/0206-document-to-payment-binding.md)
- [ADR-0207: Document-bound checkout workspace](adr/0207-document-checkout-workspace.md)
- [ADR-0208: Authenticated document-quote workspace](adr/0208-authenticated-document-quote-workspace.md)
- [ADR-0209: Bounded document-quote latency evidence](adr/0209-document-quote-latency-evidence.md)
- [ADR-0210: Docker full-topology bootstrap](adr/0210-docker-full-topology-bootstrap.md)
- [Operations runbooks](runbooks/README.md)

## Status and scope

This is the architecture and implementation baseline for a system still in its foundation phase. It does not claim that tax treatment is legally correct for every jurisdiction, that an oracle is a tax authority, or that escrow funds can be withdrawn without the configured governance and release controls. Legal, tax, custody, and licensing review are release gates. The current executable foundation is deliberately limited to the deterministic tax engine, authenticated persisted published-rule quote path with an atomic outbox event, server-owned persisted-rule simulation with tenant, jurisdiction, digest, and retirement checks, server-owned quote-bound calculation-attestation ingestion, authenticated tax-rule author/reviewer/approver workflow with append-only governance evidence, repeatable-read governance projection and operator workspace, authenticated replay-gap operations and their typed operator workspace, bounded NATS JetStream dispatcher, finalized-indexer/reconciliation domain and coordinator plus PostgreSQL persistence slice, isolated preview API, authenticated quote UI, governed vault-release API/UI and append-only evidence projection, PostgreSQL migrations, product shell, and a governed token-interface settlement contract with cached policy-attestation binding. It is not yet a production settlement deployment.

The current foundation additionally includes wallet-bound adjustment authority
proofs, the finalized server-owned adjustment evidence compiler, the durable
chain-intent submission/reconciliation API, and the accessible browser wallet
workflow. The authenticated Docker path is covered by
`make integration-check`; ADR-0194 extends that path through real wallet-byte
serialization, concurrent idempotent submission, explicit finalized fixture
observation, typed event projection, and proof-envelope replay. Validator
registry lifecycle, live wallet/provider evidence, and production finalized
reconciliation remain release gates. ADR-0184 also
requires a persisted typed finalized-event proof envelope before the browser
can render an adjustment as finalized; migration `0048` and the clean-stack
integration contract are implemented, while live-wallet/provider and
production accessibility evidence remain open.

ADR-0200 adds the production wallet boundary used by checkout, adjustment, and
refund workspaces: Wallet Standard discovery is capability-filtered, multiple
providers and multi-account sessions require explicit selection, selected
accounts and returned signed bytes are verified, and the legacy injected
adapter is used only when no compatible standard provider is registered. The
local unit/component and
real-browser suites pass; live provider/account coverage, Linux validator
evidence, regional RPC performance, and production accessibility review remain
release gates.

ADR-0201 adds one bounded tracing bootstrap for the API, worker, indexer, and
route-verifier. OTLP gRPC export is opt-in, TLS-gated outside local Docker,
redaction-aware, and flushed during bounded graceful shutdown. Run
`make otel-trace-drill` to prove local request-span receipt and correlation;
collector authentication, retention, dashboards, alert routing, and on-call
evidence remain production gates.

ADR-0202 makes tax rounding scope explicit. Component scope remains the
backward-compatible default for tax-on-tax graphs; independent components may
declare settlement-scope rounding with deterministic largest-remainder
allocation. Positive atomic quanta make decimal-scale/truncation semantics
explicit, and incompatible floor/cap boundaries fail closed. Ambiguous
mixed/dependent settlement rules fail closed, and line/invoice aggregation
remains a separately versioned facts contract.

ADR-0203 defines that document contract: bounded, versioned line facts with
explicit line/invoice rounding, canonical line identity, deterministic
residual allocation, and replay separation from aggregate `TaxFacts`.

The current foundation also includes ADR-0145's facts-only, two-replica
persisted-rule simulation latency harness. The 2026-09-27 Docker run measured
1.958/3.001/6.125 ms at p50/p95/p99 with zero errors and exact digest binding;
it is local service evidence only, and regional and production SLO acceptance
remain open.

ADR-0146 adds centralized exact atomic/timestamp formatting, bounded locale and
IANA time-zone settings, and RTL direction handling. Full translation catalogs,
real-browser responsive review, and assistive-technology evidence remain open.

ADR-0195 adds a Docker proof-worker drill that resolves the finalized refund
claim topology, reads one transaction-bound event and exact pre/post token
balances, persists one immutable balance proof, and advances the execute intent
only through that proof. The provider is deliberately synthetic and finalized;
live RPC-provider, validator, production load/SLO, and paid-state enablement
remain release gates.

ADR-0147 contains lazy workspace import/render failures behind a redacted,
accessible recovery surface while preserving the global shell. It does not
convert any API, wallet, settlement, or chain outcome into a generic success.

The local Agave validator is available through the resource-heavy `solana`
Compose profile: `make solana-up`. It uses a pinned Agave 4.3.0 release
installer and synthetic local state; it is not a testnet or mainnet node. The
profile requires a Linux runtime exposing the `io_uring` capabilities required
by Agave 4.3.0; image build success alone does not prove validator start-up.

The executable foundation also includes the deterministic raw-evidence
projection rebuild primitive defined by ADR-0088. Its clean-database digest
comparison runs through the Docker drill and CI; broader merchant, vault, and
reporting projection rebuilds remain separately scoped.

Run `make postgres-contention-drill` for the isolated PostgreSQL 18 lock,
statement-timeout, migration, evidence-index, concurrent refund-nonce,
same-key refund-claim, lifecycle-event, and outbox-cardinality contract. It
is a bounded database safety drill, not a production saturation or regional
SLO claim.
Run `make refund-execute-api-contention-drill` for the authenticated API
contention contract. It drives real claim and finalized execute preparation
with 16 concurrent same-key callers and verifies one execute intent, one
lifecycle event, one outbox row, and stable response digests, then signs and
submits both immutable movement artifacts with synthetic test-only wallets. It
verifies one fixture RPC send per movement and durable `submitted` state. It is
synthetic Docker evidence and does not replace live wallet/provider, validator,
finality, or production SLO evidence. It also reports bounded local p50/p95/p99
latency samples with workload metadata.

Run `make migration-integrity-drill` for the Rust migration runner's ordered
PostgreSQL ledger, advisory-lock, idempotent re-run, and historical checksum
drift contract. Run `make migration-rollback-rehearsal` for the isolated
expand/contract pre-deploy restore, old-schema read, and forward-recovery
contract. Staging/production migration-service cutover, KMS/HSM recovery, and
live rollback authorization remain release gates.

Run `make deployment-config-contract` to exercise the fail-closed staging and
production configuration preflight. It checks structure and forbidden local
defaults without printing secret values; it does not prove credential validity
or deployment approval.

Run `make service-lifecycle-contract` to verify that API, worker, indexer, and
route-verifier shutdown futures are owned by their service lifecycles and that
signal-listener failures are not silently ignored.

Run `make solana-conformance` for the validator-backed settlement gate. It
builds the pinned SBF artifact, loads the fixed settlement program ID, and
executes real legacy SPL, extension-free Token-2022, Split Plan fan-out,
policy revocation, duplicate-nonce, wrong-recipient-order, and
unsupported-extension cases.
The TypeScript client runs inside a pinned non-root Docker image, so the gate
does not require host Node.js or pnpm. Missing build, validator, or RPC
evidence fails the command; it is not downgraded to a mock or skipped release
result.

The authoritative Ubuntu CI/release gate additionally runs
`scripts/verify_solana_conformance_evidence.sh`, which proves the Linux/amd64
runtime, pinned Agave startup, exact SBF hash, container identity, and complete
retained test matrix. A local Docker Desktop `io_uring` failure remains a
non-zero diagnostic and is not a conformance pass.

Run `make integration-check` for the authenticated Docker quote and delivery
path. The harness builds an isolated, non-root synthetic external issuer
fixture, generates an ephemeral Ed25519 signing key, and removes its Compose
volume on exit; no development private key is stored in the repository or API
image. It proves quote-bound attestation ingestion, exact replay, conflict and
tenant rejection, non-simulation single/Split Plan compilation, the governed
vault-release lifecycle and database immutability guards, and that the outbox
worker publishes transactional events only after PostgreSQL commit.
It also runs the rollback-only PostgreSQL 18 refund-proof backlog contract.

Run `make refund-funding-verifier-drill` for the isolated lease-fenced
two-replica verifier contention/crash drill described by
[ADR-0181](adr/0181-lease-fenced-refund-funding-verification.md). Its RPC
fixture and addresses are synthetic and local-only. CI retains its
public-safe fixed-cardinality `result.env` artifact; this is regression
evidence, not production-provider or validator evidence.

Run `make encrypted-backup-restore-drill` for the ADR-0182 synthetic
encrypted-backup artifact contract. It rejects tampered ciphertext before
restore; production KMS/HSM and off-site recovery evidence remain separate.

The adjustment evidence compiler requires a configured finalized Solana RPC
coordinator. It derives and verifies the active adjustment registry, snapshots
that evidence in the immutable compiler intent, and never treats simulation
configuration as chain evidence. Validator-backed registry initialization,
rotation, wallet submission, and finalized reconciliation remain release gates.

The settlement submission boundary also contains the governed v0+ALT path
from [ADR-0161](adr/0161-governed-v0-alt-settlement-path.md). Rust tests prove
server-owned finalized lookup resolution, canonical ALT account parsing,
bounded table/address counts, exact expanded-account matching, and transport
failover. The validator-backed TypeScript conformance suite now creates and
extends a real ALT and submits a v0 settlement with finalized conservation
assertions; Linux CI execution, target-cluster compute/priority-fee evidence,
and API-level fixture coverage remain explicit production gates.
Every compiled settlement artifact also carries the bounded server-owned
compute-budget policy from [ADR-0162](adr/0162-server-owned-compute-budget-policy.md);
wallet transactions serialize its exact two-instruction prefix and the API
verifies it before submission.

Run `make vault-reconciliation-drill` for the real scheduled indexer cycle
against a bounded synthetic finalized-account RPC boundary. It proves a
matched run, an append-only mismatch run, and fixed-cardinality mismatch/error
metrics; it is not live-chain or provider-failover evidence. Run
`VAULT_RECONCILIATION_PROVIDER_MODE=failover make vault-reconciliation-drill`
for the opt-in two-provider fixture mode, which also proves that both
providers were attempted.

Run `make observability-up` to start the profiled Redis, Prometheus, Grafana,
and OpenTelemetry Collector topology. Host ports are Redis `46379`,
Prometheus `49090`, Grafana `43000`, and OTLP `44317`/`44318`; override them
with environment variables documented in `compose.yaml`.

Run `make indexer-replay-drill` for the two-replica automated replay contract.
It proves durable gap claiming, bounded projection recovery, forward-checkpoint
isolation, and fixed-cardinality replay metrics in Docker; it does not replace
live provider, validator, or production recovery-SLO evidence.
Run `make indexer-replay-dead-letter-drill` for the bounded provider-failure
contract. It proves two replicas exhaust exactly eight attempts, release the
lease, persist dead-letter state, and refresh the dead-letter metric without
advancing the forward checkpoint; production alert delivery remains open.

Run `make refund-cancellation-reconciliation-drill` for the two-replica
governed refund-claim cancellation contract. It recovers a synthetic finalized
governance event through the real targeted RPC path, proves UUIDv7 lease
ownership and stale reclaim, persists immutable raw evidence, atomically
converges the cancellation and original claim, and checks idempotent outbox
cardinality. The drill is local synthetic evidence and requires Docker; Linux
validator, live-provider, governance-key, and production evidence remain
separate gates.

Run `make observability-config` to validate the Prometheus configuration and
versioned alert rules with pinned Prometheus and Alertmanager images. Run
`make alert-delivery-drill` to induce an isolated worker scrape outage and
prove the real Prometheus rule reaches Alertmanager and the test-only Rust
receiver. Production receiver authentication, escalation, and on-call
response remain deployment gates.

Run `make otel-trace-drill` for the bounded local OTLP receipt check. It starts
the real API and collector containers, sends a health request, verifies the
API-owned UUIDv7 request correlation span, and rejects prohibited sensitive
payload fields in the captured evidence. It is local transport evidence, not
production telemetry or SLO evidence.

The real-browser journey also exercises the built vault-reconciliation SPA
workspace with synthetic server-shaped evidence, mismatch stop behavior, run
history loading, and unauthorized fail-closed behavior under
[ADR-0192](adr/0192-real-browser-vault-reconciliation-evidence.md). This is
browser-contract evidence, not live-provider or validator evidence.

The first production scope is stablecoin checkout on Solana. Non-stable assets, swaps, fiat off-ramp execution, and jurisdiction-specific legal determinations are separate adapters and are not implicit parts of the settlement hot path.

## Toolchain baseline

The baseline is recorded as of 2026-09-25 and must be reproduced from lockfiles and CI images:

| Area | Baseline | Policy |
| --- | --- | --- |
| Rust | Stable 1.98 | Pin in `rust-toolchain.toml`; upgrade only through a compatibility PR. |
| Solana/Agave CLI | 4.3.0 stable | Pin the exact release; do not use `stable`, `edge`, or alpha channels in CI. |
| Solana programs | Rust + Anchor v1 templates | Pin the Agave/Solana CLI and Anchor with AVM; do not build programs in Solidity. |
| Anchor CLI | 1.2.0 documentation baseline | Verify the exact compatible release during bootstrap and commit the chosen version. |
| React | 19.3 | Use a Vite SPA; no Next.js and no server components. |
| Node.js | 24.21.0 LTS | Build tooling only; pin the exact patch release and package-manager lockfile. |
| pnpm | 12.3.4 | Declare the exact version in `apps/web/package.json` and use frozen-lockfile installs. |
| PostgreSQL | 18.x | Latest security patch for major 18; native `uuid` storage and UUIDv7 for sortable internal IDs. |
| Solidity | 0.8.37 | Optional EVM adapter only; separate package, chain, threat model, and audit. |
| NGINX | 1.30.5 stable | Serve the compiled SPA and reverse-proxy non-blocking APIs; keep the transaction path out of NGINX. |

The official version references are listed in [implementation plan](plans/implementation-plan.md#version-and-dependency-governance).
