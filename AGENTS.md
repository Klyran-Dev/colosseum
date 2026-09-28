# AGENTS.md — Principal Low-Latency Blockchain Architect Mandates

This document is the repository-wide engineering contract. It governs every agent, developer, reviewer, and automation working in this repository. More specific `AGENTS.md` files may add constraints for a subtree, but may not weaken these mandates.

## Mission

Build and operate a production-grade Solana payment, tax-calculation, withholding, escrow, and revenue-split platform. The system must be low-latency, highly concurrent, deterministic, auditable, secure, accessible, and ready for real users and real money. It is not a hackathon demo, prototype, toy implementation, or “MVP UI.”

The source of truth for money movement is Solana. The Tier-1 Tax Engine is an off-chain deterministic calculation bounded context. The React application is a real product surface, not a mockup. The API, indexer, tax engine, and UI must be designed for production operation from their first implementation.

## Non-negotiable principles

1. **Design before implementation.** No production code starts until the design, alternatives, invariants, threat model, latency budget, data contracts, and ADR are written or updated.
2. **Compare before deciding.** Every material architectural decision must compare viable alternatives against explicit criteria. Record the decision and rejected alternatives in `docs/architecture/alternative-analysis.md` and an ADR.
3. **No demo paths.** Do not create fake checkout success, hard-coded tax totals, placeholder security, simulated transaction completion, static dashboards pretending to be live, or a UI that only works on the happy path. Test doubles belong in tests and are visibly isolated from production paths.
4. **Production-quality vertical slices.** A feature may be incomplete in breadth, but every implemented slice must be complete in correctness, security, error handling, accessibility, observability, persistence, and operational behavior.
5. **TDD is mandatory.** Write a failing test or executable specification first, implement the smallest correct behavior, then refactor. Money-moving, tax, authorization, replay, and concurrency logic require tests before implementation.
6. **Correctness outranks latency.** Never remove validation, finality checks, authorization, replay protection, amount conservation, or audit evidence to improve a benchmark.
7. **Measure, do not guess.** Optimize from profiles and representative load tests. Every low-latency claim must name the workload, percentile, region, RPC provider, transaction format, commitment level, and resource budget.
8. **Fail closed.** Ambiguous tax rules, stale policies, missing evidence, unknown transaction status, mismatched accounts, unsupported token extensions, and provider disagreement must not silently settle funds.
9. **No key custody in web/API code.** User private keys remain in wallets. Issuer, governance, and release keys use HSM/KMS/multisig controls. Never log secrets, seed phrases, raw sensitive evidence, or credentials.
10. **Evidence is a product feature.** Every calculation and settlement must be replayable from versioned inputs, rule digest, engine version, policy/attestation, transaction signature, receipt, and chain evidence.

## Architecture mandates

### On-chain

- Use Rust with Anchor-compatible Solana program interfaces for the Solana settlement core. Solidity is only an optional, separately audited EVM adapter; it must never become a second authority for Solana funds.
- Keep the on-chain program small, explicit, bounded, and deterministic. Do not execute a global tax-rule graph, arbitrary network call, floating-point calculation, or unbounded loop on-chain.
- Use PDAs and explicit account constraints. Validate account owner, mint, token program, decimals, extension policy, authority, writable/signer privileges, and expected addresses.
- Use integer atomic units and checked arithmetic. Reject overflow, invalid precision, negative/underflow outcomes, and conservation mismatches.
- Make payment, tax withholding, net splits, receipt creation, and events atomic within one bounded settlement instruction.
- Protect against replay with merchant-scoped client nonces and deterministic receipt accounts.
- Treat Token-2022 extensions as opt-in compatibility contracts. Reject unsupported transfer hooks, fees, delegates, confidential features, or authorities rather than guessing behavior.
- Bound recipient fan-out, rule/attestation size, compute units, transaction size, and all user-controlled collection sizes.
- Separate merchant configuration, policy issuer, vault release, and program upgrade authorities. Govern privileged actions with multisig/timelock where compatible with incident response.
- Vault releases must use a merchant/mint-scoped registry with separate governance, proposal, and approval authorities. Immutable release-claim PDAs must bind nonce, amount, destination, reference digest, and a bounded execution window; only the vault-authority PDA may sign the checked outbound CPI. API services and browsers never custody or sign vault-release keys. Claim creation, execution, cancellation, and finalized balance evidence are separate auditable lifecycle events.

### Tier-1 Tax Engine

- Implement the tax engine as a pure Rust domain library with a warm service adapter. Domain code must not depend on HTTP, database, system clock, RPC, filesystem, or wallet state.
- Use a canonical versioned tax-rule DSL compiled into an immutable directed acyclic rule graph.
- Support, with explicit semantics and golden vectors: simple rates, progressive/marginal brackets, tax-on-tax, cascading bases, inclusive/exclusive pricing, thresholds, exemptions, credits, min/max, caps/floors, truncation, rounding, residual allocation, and cumulative-period snapshots.
- Use exact fixed-point integer or rational intermediates. Never use binary floating point for tax, money, rates, thresholds, or percentages.
- Declare and test bracket boundaries, component dependency order, tax-base selection, clamp order, rounding mode, decimal scale, rounding scope, and residual allocation. Do not rely on language defaults.
- Reject cycles, gaps, ambiguous precedence, overlapping brackets, unbounded rule graphs, missing evidence, invalid effective windows, and incompatible rule versions.
- Published rule sets are immutable. Corrections are new versions with effective windows, supersession references, provenance, review status, and legal/tax ownership.
- Emit a complete calculation trace and signed `CalculationAttestation` bound to merchant, asset, facts digest, jurisdiction result, rule digest, engine version, expiry, and amount bounds.
- The Solana program verifies the attestation and settlement invariants; it does not blindly trust an API-calculated tax amount.

### Off-chain services

- Use Rust/Tokio for API, transaction compiler, tax engine service, indexer, reconciliation, and operational workers unless an ADR proves another technology materially better.
- Use structured concurrency: bounded task lifetimes, cancellation propagation, deadlines, backpressure, bounded queues, and explicit retry budgets.
- Require an explicit process deployment environment (`local`, `staging`, or
  `production`) at service startup. Simulation mode and insecure RPC transport
  are local-only combinations; staging and production must fail closed before
  binding listeners when either is enabled or the environment is missing or
  unknown. Environment labels do not replace deployment approval, secret
  provisioning, HSM/KMS, multisig, or mainnet allow-list controls.
- Never spawn unbounded tasks, use unbounded channels for user-controlled traffic, block an async executor, hold a mutex across `.await`, or retry indefinitely.
- Keep RPC, database, cache, signer, clock, policy, and message-bus integrations behind ports/traits. Domain and application code must be testable without network access.
- The API prepares and submits transactions; it does not custody user funds or become a settlement authority.
- Every persisted quote, policy mutation, settlement intent, and evidence query must derive tenant scope from an authenticated principal. Never accept a caller-supplied merchant identifier as an authority boundary. OIDC-compatible EdDSA JWT verification, issuer/audience checks, key rotation, expiry, revocation policy, and role/scope checks must be explicit and fail closed.
- The indexer treats subscriptions as acceleration signals, not final truth. Persist checkpoints, detect gaps, backfill by slot/signature, and reconcile against finalized chain state.
- Do not place a message broker, tax API, database round-trip, or slow external dependency in the wallet-signing/settlement critical path unless a measured design explicitly requires it.
- Use PostgreSQL for durable projections and append-only evidence metadata. Redis is disposable cache/coordination only and is never authoritative. Use an event stream only for asynchronous work, never as the source of settlement truth.
- Use PostgreSQL 18.x as the durable relational store, always on the latest security patch for the selected major version. Pin the exact container image tag and digest; never use a floating `latest` tag.
- Use PostgreSQL's native `uuid` type and UUIDv7 (RFC 9562) for application-generated sortable identifiers: entity IDs, calculation IDs, settlement intent IDs, idempotency records, outbox events, audit events, and correlation IDs.
- Generate UUIDv7 with a cryptographically secure implementation or PostgreSQL `uuidv7()`. UUIDv7 is time-ordered, not a strict total-order sequence; use explicit `created_at`, chain slot, event sequence, or a database sequence when strict ordering is required.
- Do not use UUIDv7 as a secret, bearer token, password-reset token, signature, policy/rule digest, Solana public key, transaction signature, or content-addressed evidence identifier. Use CSPRNG tokens, cryptographic hashes, or native Solana identifiers for those purposes.
- Use UTC `timestamptz` for business timestamps, atomic integer units for money, exact numeric/rational representations for tax rates, and never floating-point database columns for financial values.
- Serialize browser-facing Solana `u64` nonces, slots, sequence values, and other protocol counters as canonical decimal JSON strings. Keep `u64` in Rust and reject JavaScript-number or non-canonical-string forms at browser boundaries; never allow a safe-integer conversion to become an authority or digest input.
- Apply expand/contract migrations, bounded lock/statement timeouts, idempotent unique constraints, safe indexes, connection-pool limits, outbox semantics, backup/restore verification, and partitioning/retention plans for append-only evidence at scale.
- Commit domain evidence and its first asynchronous publication record in one transaction. Replays must not emit duplicate creation events; outbox dispatch must be bounded, at-least-once, idempotent, observable, and never the authority for Solana funds.
- Persist vault-release commands idempotently before returning a release artifact, fence proposer/approver transitions by expected state, and fail closed on unknown provider outcomes. Database release rows and UI state never authorize token movement; finalized Solana claim/event evidence remains authoritative.

### Frontend and UI/UX

- Use React 19.3, TypeScript, Vite, and a static SPA served by NGINX. Do not use Next.js, server components, or a server-rendering runtime.
- Build a real product UI: checkout, tax breakdown, “Why this tax?”, rule authoring, simulator, approval queue, published versions, settlements, vaults, audit/evidence explorer, settings, and supportable failure states.
- No placeholder charts, fake success states, hard-coded amounts, disabled buttons without explanation, or “coming soon” controls in a claimed production flow.
- The checkout must show gross amount, tax components, net amount, protocol/network fee, asset mint, quote expiry, policy/rule version, attestation status, and exact wallet authorization before signing.
- Tax explanations use progressive disclosure: summary, inputs, rule/component trace, evidence, limitations, and correlation identifiers.
- Meet WCAG 2.2 AA: keyboard navigation, focus management, semantic labels, screen-reader support, color-independent status, reduced motion, responsive layouts, RTL readiness, locale-aware money/date/time formatting, and accessible errors.
- Every async operation has loading, success, failure, timeout, retry, cancellation, stale-data, and reconciliation-required states.
- Never state or imply that automated withholding equals legal compliance, filing, reporting, or tax advice.

## Required design-before-code workflow

For every feature or material change:

1. Write the problem statement, users, scope, non-goals, assumptions, and acceptance criteria.
2. Identify affected bounded contexts and invariants.
3. Write the end-to-end sequence, state machine, data/account model, failure modes, and threat model.
4. Define latency, throughput, memory, compute-unit, transaction-size, and availability budgets.
5. Compare alternatives in `docs/architecture/alternative-analysis.md`; include criteria, trade-offs, rejected options, and why the selected option wins.
6. Create or update an ADR before implementation.
7. Define versioned API/IDL/schema contracts and compatibility rules.
8. Write tests, golden vectors, fixtures, and load scenarios before production code.
9. Implement the smallest production-quality slice.
10. Run unit, property, fuzz, integration, contract, accessibility, security, performance, and failure-path tests appropriate to the change.
11. Update runbooks, dashboards, threat model, production-readiness checklist, and user-facing documentation.
12. Review the diff for accidental scope expansion, secret leakage, unbounded work, missing authorization, and misleading UX.

If design reveals unresolved authority, legal, security, or data-contract ambiguity, stop and record the decision needed. Do not encode an assumption in money-moving code.

## Principal-level architecture review standard

Before approval, the author and reviewer must be able to answer:

- What is the authoritative source of truth, and which data is only a projection or cache?
- What are the invariants, trust boundaries, failure modes, and unsafe states?
- What is the p50/p95/p99 latency, throughput, contention, memory, compute, storage, and cost budget?
- How does the design behave under overload, partial failure, retry storms, stale data, clock skew, provider disagreement, and operator error?
- How are schema, rule, program, database, and API changes migrated, rolled back, and replayed?
- Which alternatives were rejected, and what evidence would cause us to revisit the decision?
- What are the privacy, legal, security, supply-chain, accessibility, and operational consequences?
- How are ownership, on-call, dashboards, alerts, runbooks, support, and incident communication assigned?
- What evidence proves the design rather than merely describing it?

“Looks scalable,” “uses the latest technology,” and “passes the happy path” are not architecture evidence.

## Alternative analysis baseline

The current choices are recorded in `docs/architecture/alternative-analysis.md`. The following are the default decisions:

| Area | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Solana program | Rust + Anchor-compatible interfaces | Native Rust without Anchor, Solidity/SVM transpilers, other chains | Native Solana execution, account constraints, IDL, ecosystem, auditability |
| Tax calculation | Pure Rust domain engine + attestation | On-chain rule engine, external SaaS-only engine, JavaScript-only engine | Determinism, reuse, latency, replay, typed money semantics, on-chain bounded verification |
| API/indexer | Rust + Tokio/Axum-style services | Node.js, Go, Java/Kotlin | Shared domain types, predictable concurrency, low allocation/latency control, Solana/Rust alignment |
| Web app | React + TypeScript + Vite SPA | Next.js, Remix, server-rendered app, micro-frontends | User constraint, wallet-centric client behavior, simple static delivery, independent scaling |
| Edge/static delivery | NGINX | Envoy, Caddy, cloud-only CDN | Explicit user requirement, mature static/reverse-proxy behavior, deterministic deployment |
| Durable store | PostgreSQL 18.x with native `uuid`/`uuidv7()` | PostgreSQL 17 or older, document database, SQLite, managed-only database | Current supported major, time-ordered UUID generation, transactions, evidence queries, projections, operational maturity |
| Cache | Redis, non-authoritative | In-process only, Memcached | Shared cache/leases with explicit disposable semantics |
| Async stream | PostgreSQL outbox plus NATS JetStream when needed | Kafka, Redis Streams, direct HTTP fan-out | Low-latency bounded async work without putting a broker in settlement hot path |
| Local orchestration | Docker Compose + pinned images/digests | Kubernetes locally, bare-metal processes, dev containers only | Reproducible multi-service development with production-like boundaries |

Any change to these defaults requires a new comparison and ADR.

## TDD and test mandates

### Test order

Write tests in this order whenever applicable:

1. Domain example and invariant tests.
2. Boundary and failure-path tests.
3. Property and fuzz tests.
4. Contract/IDL serialization tests.
5. Integration tests against local Solana and Docker services.
6. API/UI tests and accessibility assertions.
7. Load, soak, contention, chaos, and failover tests.

### Money and tax gates

- Every tax scenario family has golden vectors, including exact boundary values.
- Every calculation result can be replayed from facts, rule digest, engine version, and evidence.
- Money-moving logic has no untested branches in the approved rule matrix.
- Mutation testing is required for tax arithmetic, authority checks, replay protection, and amount conservation.
- Cross-language representations have canonical serialization test vectors.
- Tests cover account substitution, Token-2022 extensions, stale policy, expired quote, duplicate nonce, RPC ambiguity, indexer gaps, and partial-failure rollback.

### Concurrency and performance gates

- Use Criterion or equivalent microbenchmarks for hot functions and representative end-to-end load tests for services.
- Test cancellation, deadlines, backpressure, retry storms, queue saturation, connection-pool exhaustion, and graceful shutdown.
- Use concurrency model testing such as Loom where shared state or lock ordering is security/correctness sensitive.
- Never accept a benchmark without workload definition, warm-up, sample size, percentile distribution, and resource profile.
- Performance regressions above the documented budget block merge or require an explicit ADR exception.

## Low-latency mandates

- Keep the synchronous quote/compile path cache-first and bounded.
- Precompute PDAs, account metadata, rule graphs, and stable configuration.
- Use versioned transactions, explicit compute budgets, minimum required account metas, and measured priority fees.
- Avoid ATA creation, swaps, large account fan-out, or external tax-provider calls in the settlement hot path unless the design proves they fit the budget.
- Use fresh blockhashes, bounded retries, multiple RPC providers, commitment-aware status, and immutable signed transaction bytes.
- Before enabling submission in any non-test environment, parse the signed
  Solana message and prove it contains the compiled settlement instruction,
  fixed program ID, canonical account order, expected intent/policy digest,
  payer signature, and approved compute/blockhash policy. A digest supplied by
  the client alone is not proof of message binding.
- The browser wallet boundary must validate the prepared settlement envelope
  before blockhash lookup or signing: fixed program ID, exact account count and
  order, signer/writable flags, PDA-field consistency, supported token program,
  distinct token accounts, canonical instruction bytes, and settle
  discriminator. This is defense in depth; the API must repeat verification on
  the actual signed message.
- Quote-bound settlement compilation must resolve merchant, mint, tax-vault,
  destination, and token-program accounts from an authenticated,
  tenant-scoped, versioned settlement-route registry. The browser may provide
  only a route ID and payer wallet key; it must never provide merchant-side
  account addresses or a program ID. Derive the payer source associated-token
  account locally and do not add ATA creation or an RPC lookup to the
  settlement hot path. Route activation requires separate chain-verification
  evidence; database configuration is not proof of on-chain account state.
- Route activation must run through the dedicated bounded route-verifier
  workflow: finalized `getMultipleAccounts`, fixed account order, exact
  System/SPL/Token-2022 owner and layout checks, mint binding, deterministic
  vault-authority verification, append-only evidence, and atomic activation.
  All-provider failure, provider disagreement, weak commitment, null account,
  malformed data, or uncertain state fails closed. The verifier is never called from
  quote, compile, wallet-signing, or settlement-submit hot paths. A
  `local_synthetic` evidence marker is permitted only in Docker fixtures and
  must never activate production routes.
- Shard writable accounts to reduce contention. Do not use global counters or shared hot PDAs for all merchants.
- Prefer immutable versioned plans and policies over frequently mutated shared state.
- Measure p50/p95/p99 quote, compile, submit, processed, confirmed, and finalized latency by region and provider.

## Docker and local-development mandates

Docker is the default development environment. Local development must work from a clean checkout with Docker Engine/Desktop and Compose, without requiring a host-installed Rust, Node, Solana CLI, PostgreSQL, Redis, NATS, or NGINX toolchain.

### Required local services

The implementation must provide a Compose profile for:

- pinned Agave/Solana local validator built from the selected stable release;
- Rust API and Tax Engine service;
- indexer/reconciliation worker;
- bounded finalized settlement-route verifier;
- PostgreSQL;
- Redis cache/lease service;
- NATS JetStream only if the outbox design uses it;
- React/Vite development container;
- NGINX edge container;
- OpenTelemetry Collector, Prometheus, and Grafana for local observability.

### Image and build policy

- Use the newest stable, security-supported image versions compatible with the selected toolchain at the time of bootstrap.
- Record exact tags and immutable image digests in the repository. “Latest” is a discovery policy, not a production tag.
- Re-resolve images on a scheduled maintenance change, scan them, regenerate the SBOM, run the full test suite, and review the resulting ADR/change.
- Use multi-stage BuildKit builds, minimal runtime images, non-root users, read-only filesystems where possible, dropped Linux capabilities, explicit resource limits, and health checks.
- Run Trivy/Grype-equivalent image and dependency scans, verify signatures where available, and fail CI on critical/high findings unless an approved time-bounded exception exists.
- Do not mount the host Docker socket into application containers.
- Use named volumes for databases, deterministic seed fixtures, isolated networks, and Compose profiles for optional observability.
- Localnet/testnet keys and sample tax evidence are synthetic and must never be reused in staging or mainnet.

### Compose quality gates

- `docker compose config` must succeed from a clean checkout.
- All services have health checks, startup ordering based on health, bounded restart policy, logs, and documented ports.
- A one-command bootstrap creates schemas, seeds safe fixtures, starts localnet, runs migrations, and exposes the real UI.
- A one-command test flow runs Rust, frontend, contract, localnet integration, accessibility, and API tests.
- `make solana-conformance` is the explicit localnet release gate: it builds
  the pinned SBF artifact, starts the pinned Agave validator with that artifact
  under the fixed program ID, and runs real legacy-SPL, extension-free
  Token-2022, duplicate-nonce, and unsupported-extension transactions. If the
  artifact, validator, or RPC evidence is unavailable, the command fails; it
  must not downgrade to a skipped or mocked result.
- A one-command teardown removes only this project’s named volumes and networks; never use broad host deletion commands.

## Toolchain and dependency policy

- Current baseline (recorded 2026-09-25; re-validate compatibility at bootstrap): Rust stable 1.98, Agave/Solana CLI 4.3.0, Anchor CLI 1.2.0, Node.js 24.21.0 LTS, React 19.3, Solidity 0.8.37 for the optional EVM adapter, and NGINX 1.30.5 stable.
- Pin Rust, Agave/Solana CLI, Anchor/AVM, Node.js, package manager, React, Solidity compiler, NGINX, Docker base images, and all production dependencies.
- Commit lockfiles and CI image digests. Use a compatibility matrix for Rust/Anchor/Agave/SDK versions.
- Prefer stable releases; do not use nightly, alpha, beta, or floating channels in production without a documented exception.
- Run formatting, linting, type checking, tests, audit, SBOM, and reproducible-build checks in CI.
- `unsafe` Rust, shelling out, dynamic code generation, and dependency additions require justification and review.
- Never use `unwrap`, `expect`, panic, ignored errors, unbounded retries, or silent fallback in request, settlement, tax, or reconciliation paths without a documented proof that the path is unreachable and a test enforcing it.

## Security and privacy mandates

- Validate all untrusted input at the boundary and again at authority-sensitive boundaries.
- Use least privilege for services, database roles, cloud identities, container capabilities, and program authorities.
- Keep PII and sensitive jurisdiction evidence off-chain where possible; store commitments and references rather than unnecessary raw data.
- Apply retention, encryption, access auditing, redaction, and deletion policies to off-chain evidence.
- Use CSP, secure cookies where applicable, strict transport security, CSRF protection for cookie-authenticated APIs, CORS allow-lists, rate limits, request-size limits, and anti-automation controls.
- Security review is required for every authority, signer, upgrade, release, schema, serialization, and external integration change.

## Public repository mandates

- Keep the root README accurate about implementation status, supported scope, limitations, architecture, setup, and release readiness.
- Maintain `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `CHANGELOG.md`, ADR index, production-readiness gates, and public coverage matrix as first-class product documentation.
- Do not add badges, claims, benchmarks, audit statements, supported-jurisdiction claims, or “production-ready” language without current evidence.
- Decide and publish the repository license before public source release; do not invent a license or copy an unapproved one.
- Treat GitHub issues, pull requests, release notes, and security advisories as part of the supported product interface.
- Require public-safe examples, synthetic fixtures, redacted logs, reproducible commands, and no credentials or private tax evidence.

## Definition of done

A change is not complete until:

- design and ADR/alternative analysis are current;
- tests were written first and pass;
- error, timeout, retry, cancellation, and degraded states are implemented;
- latency and resource budgets are measured;
- logs, metrics, traces, and alerts exist for production behavior;
- UI is accessible, localized-ready, responsive, and does not fake state;
- security, privacy, dependency, and supply-chain checks pass;
- Docker local development reproduces the relevant behavior;
- runbooks, API/IDL schemas, migration notes, and user-facing docs are updated;
- reviewers can reproduce the result from a clean checkout.
