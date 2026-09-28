# Solana Tax Protocol

This directory is the architecture and delivery baseline for a production-grade, low-latency Solana payment-routing protocol. The protocol settles a payment, withholds an attested tax amount into a program-controlled vault, and distributes the net amount to a pre-authorized split plan in one atomic transaction.

## Documents

- [Source analysis and requirement boundary](source-analysis.md)
- [System architecture](architecture/system-architecture.md)
- [Domain model and bounded contexts](architecture/domain-model.md)
- [Tier-1 tax engine](architecture/tax-engine.md)
- [Jurisdiction-resolution bounded context](architecture/jurisdiction-resolution.md)
- [Gold-standard UI/UX](architecture/ui-ux.md)
- [Architecture alternative analysis](architecture/alternative-analysis.md)
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
- [Operations runbooks](runbooks/README.md)

## Status and scope

This is the architecture and implementation baseline for a system still in its foundation phase. It does not claim that tax treatment is legally correct for every jurisdiction, that an oracle is a tax authority, or that escrow funds can be withdrawn without the configured governance and release controls. Legal, tax, custody, and licensing review are release gates. The current executable foundation is deliberately limited to the deterministic tax engine, authenticated persisted published-rule quote path with an atomic outbox event, server-owned persisted-rule simulation with tenant, jurisdiction, digest, and retirement checks, server-owned quote-bound calculation-attestation ingestion, authenticated tax-rule author/reviewer/approver workflow with append-only governance evidence, repeatable-read governance projection and operator workspace, authenticated replay-gap operations and their typed operator workspace, bounded NATS JetStream dispatcher, finalized-indexer/reconciliation domain and coordinator plus PostgreSQL persistence slice, isolated preview API, authenticated quote UI, governed vault-release API/UI and append-only evidence projection, PostgreSQL migrations, product shell, and a governed token-interface settlement contract with cached policy-attestation binding. It is not yet a production settlement deployment.

The current foundation additionally includes wallet-bound adjustment authority
proofs and the finalized server-owned adjustment evidence compiler. The
authenticated Docker path is covered by `make integration-check`; validator
registry lifecycle, wallet submission, and finalized reconciliation remain
release gates.

The current foundation also includes ADR-0145's facts-only, two-replica
persisted-rule simulation latency harness. The 2026-09-27 Docker run measured
1.958/3.001/6.125 ms at p50/p95/p99 with zero errors and exact digest binding;
it is local service evidence only, and regional and production SLO acceptance
remain open.

ADR-0146 adds centralized exact atomic/timestamp formatting, bounded locale and
IANA time-zone settings, and RTL direction handling. Full translation catalogs,
real-browser responsive review, and assistive-technology evidence remain open.

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
statement-timeout, migration, and evidence-index contract. It is a bounded
database safety drill, not a production saturation or regional SLO claim.

Run `make migration-integrity-drill` for the Rust migration runner's ordered
PostgreSQL ledger, advisory-lock, idempotent re-run, and historical checksum
drift contract. Staging/production migration-service cutover and rollback
rehearsal remain release gates.

Run `make deployment-config-contract` to exercise the fail-closed staging and
production configuration preflight. It checks structure and forbidden local
defaults without printing secret values; it does not prove credential validity
or deployment approval.

Run `make solana-conformance` for the validator-backed settlement gate. It
builds the pinned SBF artifact, loads the fixed settlement program ID, and
executes real legacy SPL, extension-free Token-2022, Split Plan fan-out,
policy revocation, duplicate-nonce, wrong-recipient-order, and
unsupported-extension cases.
The TypeScript client runs inside a pinned non-root Docker image, so the gate
does not require host Node.js or pnpm. Missing build, validator, or RPC
evidence fails the command; it is not downgraded to a mock or skipped release
result.

Run `make integration-check` for the authenticated Docker quote and delivery
path. The harness builds an isolated, non-root synthetic external issuer
fixture, generates an ephemeral Ed25519 signing key, and removes its Compose
volume on exit; no development private key is stored in the repository or API
image. It proves quote-bound attestation ingestion, exact replay, conflict and
tenant rejection, non-simulation single/Split Plan compilation, the governed
vault-release lifecycle and database immutability guards, and that the outbox
worker publishes transactional events only after PostgreSQL commit.
It also runs the rollback-only PostgreSQL 18 refund-proof backlog contract.

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

Run `make observability-config` to validate the Prometheus configuration and
versioned alert rules with the pinned Prometheus image. This validates rule
syntax and wiring only; production notification delivery still requires the
approved receiver configuration.

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
| PostgreSQL | 18.x | Latest security patch for major 18; native `uuid` storage and UUIDv7 for sortable internal IDs. |
| Solidity | 0.8.37 | Optional EVM adapter only; separate package, chain, threat model, and audit. |
| NGINX | 1.30.5 stable | Serve the compiled SPA and reverse-proxy non-blocking APIs; keep the transaction path out of NGINX. |

The official version references are listed in [implementation plan](plans/implementation-plan.md#version-and-dependency-governance).
