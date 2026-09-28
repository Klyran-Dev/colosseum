# Solana Tax Protocol

Production-grade architecture for a low-latency Solana payment-routing and Tier-1 tax platform. The system calculates complex tax rules off-chain, produces replayable signed attestations, atomically withholds approved tax amounts into governed stablecoin vaults, and distributes net revenue according to versioned split plans.

## Project status

The repository contains the executable foundation: a deterministic Rust tax-engine crate with immutable compiled rule graphs, authenticated idempotent tenant-scoped rule-draft authoring and an author/reviewer/approver governance workflow with append-only evidence, an immutable settlement-adjustments domain and durable proposal/approval/rejection API plus accessible operator workspace for bounded refund/reversal/correction/chargeback deltas, a persisted published-rule quote boundary with transactional idempotency and scoped EdDSA authentication, a tenant-scoped settlement route registry with deterministic payer ATA derivation, a finalized multi-provider route-verifier service with append-only activation evidence, a deterministic non-custodial settlement instruction compiler, a bounded wallet-signed transaction coordinator, UUIDv7 settlement-intent persistence, an atomic PostgreSQL outbox, a bounded Rust worker with NATS JetStream delivery, a finalized-indexer persistence slice with raw transaction evidence, typed split-settlement evidence, and lease-fenced replay, an isolated preview API, an authenticated React/Vite quote UI, a PostgreSQL 18 migration baseline, bounded Prometheus metrics and validated alert rules, and pinned Docker/NGINX local orchestration. It does not claim production deployment, legal/tax coverage, audit completion, or mainnet readiness. Production submission enablement, adjustment on-chain posting, live identity-provider key rotation, production HSM/KMS attestation issuance, route governance ceremonies, identity-provider governance, RPC-backed indexing, production alert receiver delivery, and local validator slices remain gated by the design, TDD, security, performance, and operational requirements in [AGENTS.md](AGENTS.md). The Docker integration gate does include an isolated synthetic external issuer fixture; its key is generated only for the test run and is never placed in the API image.

The validator conformance command now includes the real bounded Split Plan
fan-out path, finalized conservation checks, duplicate receipt rejection, and
wrong-recipient-order failure evidence. It requires the pinned Linux validator
runtime and remains a release gate rather than a local mock.

## Architecture

```text
React + Vite SPA -> NGINX -> Rust API / Tax Engine / Indexer
                                      |
                              signed calculation attestation
                                      |
                              Solana settlement program
                              |       |        |
                         tax vault  splits  receipts/events
```

The Solana core uses Rust/Anchor-compatible interfaces. The Tier-1 Tax Engine is a deterministic Rust bounded context supporting progressive, cascading, tax-on-tax, threshold, exemption, min/max, truncation, and rounding rules. Jurisdiction resolution is a separate deterministic Rust bounded context that binds versioned precedence policies to external evidence and fails closed on ambiguity. Solidity is isolated to an optional EVM adapter. Next.js is intentionally not used.

## Documentation

- [Documentation index](docs/README.md)
- [Tier-1 product standard](docs/plans/tier-1-product-standard.md)
- [Implementation plan](docs/plans/implementation-plan.md)
- [Production-readiness gates](docs/plans/production-readiness.md)
- [Tax coverage matrix](docs/product/tax-coverage-matrix.md)
- [Architecture and alternatives](docs/architecture/alternative-analysis.md)
- [API contract baseline](docs/api-contract.md)
- [Architecture decision records](docs/adr/README.md)
- [Public repository release contract](docs/public-repository-release.md)
- [Source analysis and requirement boundary](docs/source-analysis.md)
- [Security policy](SECURITY.md)
- [Contributing](CONTRIBUTING.md)
- [Code of conduct](CODE_OF_CONDUCT.md)
- [Operations runbooks](docs/runbooks/README.md)

## Engineering contract

All implementation work follows [AGENTS.md](AGENTS.md): design before code, alternatives before decisions, TDD, production-quality vertical slices, bounded concurrency, measurable latency, Docker-based development, explicit security controls, and no demo-only paths. `make security-scan` runs the pinned Trivy vulnerability, secret, and Docker misconfiguration gate used by CI; critical/high findings fail the check.

## Public repository status

The executable foundation can be checked with `make check`; `make integration-check` additionally builds an isolated Docker stack and verifies authenticated persisted quotes, an isolated synthetic external issuer, quote-bound attestation ingestion and exact replay, conflict/tenant rejection, non-simulation single/Split Plan compilation, quote-bound settlement compilation from server-owned amounts, authenticated replay-gap operator actions, NGINX forwarding and bounded-burst `429` admission, governed vault-release proposal/approval/on-chain-created/executed transitions with append-only evidence and mutation rejection, idempotency conflict behavior, and asynchronous outbox publication through NATS JetStream. Run `make bench-tax-engine` for the explicit Criterion hot-path benchmark (record the workload and machine before comparing results). Run `make benchmark-quote-service` for the bounded HTTP workload, including same-key idempotency contention and exact persistence convergence. Run `make backup-restore-drill` for the isolated PostgreSQL logical backup/restore check, and `make migration-integrity-drill` for ordered, checksum-verified migration ledger evidence. Run `make solana-conformance` for the explicit validator-backed release gate: it builds the pinned SBF artifact and executes real legacy SPL, extension-free Token-2022, duplicate-nonce, and unsupported-extension transactions. Start the local stack with `docker compose up --build`, then open `http://localhost:8088`; the direct API port is `18080`, PostgreSQL is `55432`, and NATS is `44222`. On a new named volume, the one-shot Rust migration runner applies the versioned migrations from `db/migrations/`; `make migrate` is safe to rerun. The optional Solana profile requires a Linux runtime with the `io_uring` capabilities required by Agave 4.3.0. Do not infer production support from architecture documents or the simulation endpoint alone.

The quote-load drill runs two independent API containers, alternates requests
across replicas, and proves cross-replica same-key convergence at the
PostgreSQL fence; CI uploads its bounded output as a public-safe artifact.

Run `make benchmark-rule-simulation` for the separate facts-only persisted-rule
simulation workload. It verifies server-owned rule identity and digest binding,
two-replica read contention, and p50/p95/p99 latency without creating financial
rows. The SPA also exposes bounded locale, IANA time-zone, and RTL display
settings; these are display concerns and never alter tax or settlement inputs.

Lazy operational workspaces are also protected by a redacted error boundary:
chunk or render failures leave navigation available and expose an explicit
reload action without implying a financial or Solana outcome.

Run `make web-browser-check` for the opt-in real-browser UI contract. It uses
the pinned Playwright Docker profile through the NGINX edge and verifies
keyboard focus, lazy workspace loading, narrow RTL overflow, reduced motion,
and redacted chunk-failure recovery. Client-only idempotency keys use a
CSPRNG-only secure-context fallback; server UUIDv7 entity authority is
unchanged.

Run `make deployment-config-contract` to check the non-local deployment
preflight. It is intentionally synthetic and structural: it catches local
defaults, insecure flags, and fixture leakage without claiming that secrets,
providers, approvals, or production readiness are valid.

Run `make projection-rebuild-drill` for the Dockerized clean-projection
rebuild and checkpoint-invariance check. It is a synthetic local drill, not a
claim of production recovery readiness.
Run `make postgres-contention-drill` for the pinned PostgreSQL lock,
statement-timeout, and evidence-index contract; production saturation remains
separate.
Run `make vault-reconciliation-drill` to exercise the real scheduled indexer
against a bounded synthetic finalized token-account RPC response. The drill
proves matched and append-only mismatch runs plus fixed-cardinality alert
inputs; it does not replace live validator, provider-failover, or Alertmanager
evidence. Run `VAULT_RECONCILIATION_PROVIDER_MODE=failover make
vault-reconciliation-drill` for the opt-in two-provider fixture mode, which
also proves that both providers were attempted.

Start the local cache and observability profile with `make observability-up`.
It provides disposable Redis on `46379`, Prometheus on `49090`, Grafana on
`43000`, and OTLP gRPC/HTTP ingress on `44317`/`44318`; the Grafana dashboard
is provisioned from checked-in configuration. These services are local
operator tooling and never become tax or settlement authority. API responses
also expose an API-owned UUIDv7 `x-request-id` support reference; caller- and
edge-supplied values are overwritten.
