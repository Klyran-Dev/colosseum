# Solana Tax Protocol

Production-grade architecture for a low-latency Solana payment-routing and Tier-1 tax platform. The system calculates complex tax rules off-chain, produces replayable signed attestations, atomically withholds approved tax amounts into governed stablecoin vaults, and distributes net revenue according to versioned split plans.

## Project status

The repository contains the executable foundation: a deterministic Rust tax-engine crate with immutable compiled rule graphs and a separately deployable authenticated warm Tax Engine adapter, authenticated idempotent tenant-scoped rule-draft authoring and an author/reviewer/approver governance workflow with append-only evidence, bounded line/invoice document tax facts, dedicated document attestations and document-to-payment authorization, an immutable settlement-adjustments domain and durable proposal/approval/rejection API plus accessible operator workspace for bounded refund/reversal/correction/chargeback deltas, a persisted published-rule quote boundary with transactional idempotency and scoped EdDSA authentication, a tenant-scoped settlement route registry with deterministic payer ATA derivation, a finalized multi-provider route-verifier service with append-only activation evidence, deterministic aggregate, Split Plan, and document settlement instruction compilers, bounded wallet-signed transaction coordinators, UUIDv7 settlement-intent persistence, an atomic PostgreSQL outbox, a bounded Rust worker with NATS JetStream delivery, a finalized-indexer persistence slice with raw transaction evidence, typed split/document-settlement evidence, and lease-fenced replay, an isolated preview API, authenticated React/Vite quote and document-checkout workspaces, a PostgreSQL 18 migration baseline, bounded Prometheus metrics and validated alert rules, and pinned Docker/NGINX local orchestration. It does not claim production deployment, legal/tax coverage, audit completion, or mainnet readiness. Production submission enablement, adjustment on-chain posting, live identity-provider key rotation, production HSM/KMS attestation issuance, route governance ceremonies, identity-provider governance, RPC-backed indexing, production alert receiver delivery, and local validator slices remain gated by the design, TDD, security, performance, and operational requirements in [AGENTS.md](AGENTS.md). The Docker integration gate does include an isolated synthetic external issuer fixture; its key is generated only for the test run and is never placed in the API image.

Adjustment finality responses now carry a server-owned typed event-proof
envelope; a browser status string alone cannot render an adjustment as
finalized. Migration `0048` and the clean PostgreSQL 18 Docker integration
contract verify this boundary. Live wallet/provider, validator, production
SLO, and full browser accessibility evidence remain release gates.

The validator conformance command now includes the real bounded Split Plan
fan-out path, document-bound settlement with on-chain provenance, finalized
conservation checks, duplicate receipt rejection, and wrong-recipient-order
failure evidence. It requires the pinned Linux validator runtime and remains a
release gate rather than a local mock.

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
- [Document quote and checkout ADRs](docs/adr/0204-authoritative-document-quote-boundary.md)
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

The executable foundation can be checked with `make check`; `make integration-check` additionally builds an isolated Docker stack and verifies authenticated persisted quotes, an isolated synthetic external issuer, quote-bound attestation ingestion and exact replay, conflict/tenant rejection, non-simulation single/Split Plan compilation, quote-bound settlement compilation from server-owned amounts, authenticated replay-gap operator actions, NGINX forwarding and bounded-burst `429` admission, governed vault-release proposal/approval/on-chain-created/executed transitions with append-only evidence and mutation rejection, governed rule rollback through a corrected immutable release, idempotency conflict behavior, asynchronous outbox publication through NATS JetStream, and the ADR-0194 wallet-bound adjustment finality drill with real signed instruction serialization, concurrent [200,409] submission, one fixture RPC send, explicit finalized observation, typed event projection, and proof-envelope replay. Run `make bench-tax-engine` for the explicit Criterion hot-path benchmark (record the workload and machine before comparing results). Run `make benchmark-quote-service` for the bounded HTTP workload, including same-key idempotency contention and exact persistence convergence. Run `make backup-restore-drill` for the isolated PostgreSQL logical backup/restore check, `make migration-integrity-drill` for ordered, checksum-verified migration ledger evidence, and `make migration-rollback-rehearsal` for pre-deploy restore, old-schema compatibility, and forward-recovery evidence. Run `make solana-conformance` for the explicit validator-backed release gate: it builds the pinned SBF artifact and executes real legacy SPL, extension-free Token-2022, duplicate-nonce, and unsupported-extension transactions. Start the local stack with `docker compose up --build`, then open `http://localhost:8088`; the direct API port is `18080`, PostgreSQL is `55432`, and NATS is `44222`. On a new named volume, the one-shot Rust migration runner applies the versioned migrations from `db/migrations/`; `make migrate` is safe to rerun. The optional Solana profile requires a Linux runtime with the `io_uring` capabilities required by Agave 4.3.0. Do not infer production support from architecture documents or the simulation endpoint alone.

The conformance client is executed inside a pinned non-root Docker image, so
the validator gate does not require host Node.js or pnpm. The SBF build also
fails closed on known stack-frame safety diagnostics. Validator startup probes
are bounded and retain a typed failure reason when the pinned Agave process
exits before RPC readiness, so unsupported host capabilities fail fast instead
of hanging the gate.

The quote-load drill runs two independent API containers, alternates requests
across replicas, and proves cross-replica same-key convergence at the
PostgreSQL fence; CI uploads its bounded output as a public-safe artifact.
Run `make benchmark-document-quote-service` for the separate two-line
document-quote workload defined by ADR-0209; its output is local evidence and
does not claim a production SLO.

Run `make bootstrap` to start the full Docker topology: PostgreSQL migrations
and fixtures, API, warm Tax Engine service, worker, NATS, Redis, indexer, route verifier, observability,
the pinned local validator, and the real NGINX-served SPA. Compose waits for
declared health checks and fails closed when a required runtime capability is
unavailable. The one-shot conformance and browser gates remain explicit
commands (`make solana-conformance` and `make web-browser-check`).
If a host port is already in use, provide an explicit alternate port map (for
example `REDIS_PORT=46380 SOLANA_RPC_PORT=18899 SOLANA_FAUCET_PORT=18900`)
and keep the printed endpoints as the client configuration for that stack.

Run `make benchmark-rule-simulation` for the separate facts-only persisted-rule
simulation workload. It verifies server-owned rule identity and digest binding,
two-replica read contention, and p50/p95/p99 latency without creating financial
rows. The SPA also exposes bounded locale, IANA time-zone, and RTL display
settings; these are display concerns and never alter tax or settlement inputs.

Run `make tax-engine-runtime-contract` to build the pinned Tax Engine image
and exercise its real private HTTP listener for liveness, authenticated
readiness, exact calculation, malformed/oversized request rejection, and
fixed-cardinality metrics. The contract is also included in `make check` and
uses synthetic local-only credentials and facts.

Run `make benchmark-tax-engine-service` for the bounded warm-service workload.
It measures the real pinned image with 256 authenticated aggregate requests at
concurrency 32 by default, validates exact results for every response, and
prints local p50/p95/p99 metadata. It is diagnostic synthetic evidence, not a
regional production SLO or settlement-path benchmark.

Run `make tax-engine-mtls-runtime-contract` to exercise the real pinned Tax
Engine image in non-local staging mode with synthetic mutual TLS. It verifies
client certificate rejection/acceptance, bearer authorization, exact
calculation, ALPN, and bounded SIGHUP reload behavior. It does not claim
production certificate lifecycle or secret-manager evidence.

Run `make api-auth-jwks-reload-contract` to exercise the real API image with
an atomic file-backed EdDSA JWKS rotation. It proves the old key is rejected
after a valid `SIGHUP` replacement, the new key is accepted, and malformed
replacement material retains the last known-good snapshot. Production
identity-provider overlap, revocation, secret-manager delivery, and regional
reload evidence remain separate gates.

Run `make refund-funding-verifier-drill` for the Docker-only synthetic
two-replica refund-funding verification evidence. It proves lease-fenced single
activation, one finalized RPC read under contention, and stale-lease reclaim
after verifier termination; it does not claim live-provider or production
validator readiness.

Run `make refund-proof-lease-drill` for the Docker-only synthetic refund-proof
worker evidence. It proves one winner for concurrent claims of the same
execution, UUIDv7 stale-lease reclaim, stale-worker fencing, proof exclusion,
and three-attempt dead-letter handling; it does not claim live-provider or
production validator readiness.

Run `make refund-proof-integration-drill` for the real Docker indexer proof
boundary. It resolves a synthetic finalized refund claim PDA, parses the exact
transaction-bound event and pre/post token balances, persists one immutable
proof, finalizes the execute intent/outbox once, and rejects mutation/replay
duplication. It is local synthetic-provider evidence only and does not claim
live RPC-provider, validator, production SLO, or paid-state readiness.

Run `make encrypted-backup-restore-drill` for the isolated PostgreSQL 18
encrypted-artifact and tamper-rejection contract. It uses an ephemeral
synthetic key and does not claim production KMS/HSM, off-site retention, or
RPO/RTO readiness.

Lazy operational workspaces are also protected by a redacted error boundary:
chunk or render failures leave navigation available and expose an explicit
reload action without implying a financial or Solana outcome.

Run `make web-browser-check` for the opt-in real-browser UI contract. It uses
the pinned Playwright Docker profile through the NGINX edge and verifies
keyboard focus, lazy workspace loading, narrow RTL overflow, reduced motion,
redacted chunk-failure recovery, an API-unavailable fail-closed state, and
pinned axe-core WCAG A/AA audits across the quote and operational workspaces.
The runner type-checks the e2e project before launching the browser.
Client-only idempotency keys use a CSPRNG-only
secure-context fallback; server UUIDv7 entity authority is unchanged. This
automated gate does not replace manual assistive-technology, legal/tax,
live-RPC, or mainnet evidence.

Run `make deployment-config-contract` to check the non-local deployment
preflight. It is intentionally synthetic and structural: it catches local
defaults, insecure flags, and fixture leakage without claiming that secrets,
providers, approvals, or production readiness are valid.

Run `make projection-rebuild-drill` for the Dockerized clean-projection
rebuild and checkpoint-invariance check. It is a synthetic local drill, not a
claim of production recovery readiness.
Run `make indexer-replay-drill` for the two-replica automated replay contract:
the running indexer claims and completes a persisted gap through PostgreSQL 18
and the bounded RPC fixture without advancing the forward checkpoint. This is
CI/local recovery evidence, not live-provider or production recovery SLO
evidence.
Run `make indexer-replay-dead-letter-drill` for the failure contract: two
replicas exhaust exactly eight provider-failure attempts, persist bounded
dead-letter state, release the lease, and leave the forward checkpoint
unchanged. Production alert delivery and on-call response remain separate
gates.
Run `make alert-delivery-drill` for the isolated operational path: it induces
a bounded worker scrape outage and proves a real Prometheus rule reaches the
pinned Alertmanager and test-only Rust receiver. This does not claim
production receiver authentication, escalation, or on-call response.
Run `make postgres-contention-drill` for the pinned PostgreSQL lock,
statement-timeout, evidence-index, concurrent refund-nonce, same-key
refund-claim, lifecycle-event, and outbox-cardinality contract; production
saturation and regional SLO evidence remain separate.
Run `make refund-execute-api-contention-drill` for the authenticated Docker API
contract: real claim and finalized execute preparation with 16 concurrent
same-key callers converging to one intent, lifecycle event, outbox row, and
response digest, followed by synthetic wallet-signed claim and execute
submissions proving one fixture RPC send per movement. Synthetic evidence does
not claim live wallet/provider finality or production SLOs. The drill also
reports bounded local p50/p95/p99 latency samples with workload metadata.
Run `make vault-reconciliation-drill` to exercise the real scheduled indexer
against a bounded synthetic finalized token-account RPC response. The drill
proves matched and append-only mismatch runs plus fixed-cardinality alert
inputs; it does not replace live validator, provider-failover, or Alertmanager
evidence. Run `VAULT_RECONCILIATION_PROVIDER_MODE=failover make
vault-reconciliation-drill` for the opt-in two-provider fixture mode, which
also proves that both providers were attempted.

Start the local cache and observability profile with `make observability-up`.
It provides disposable Redis on `46379`, Prometheus on `49090`, Grafana on
`43000`, Alertmanager on `49093`, and OTLP gRPC/HTTP ingress on `44317`/`44318`;
the Grafana dashboard and Alertmanager route are provisioned from checked-in
configuration. `make alert-delivery-drill` retains local delivery evidence.
These services are local operator tooling and never become tax or settlement authority. API responses
also expose an API-owned UUIDv7 `x-request-id` support reference; caller- and
edge-supplied values are overwritten.
