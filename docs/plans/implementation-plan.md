# Implementation plan

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

Exit criteria: localnet property tests demonstrate amount conservation, recipient correctness, policy freshness, replay resistance, and atomic rollback.

## Phase 3 - policy issuer and transaction compiler

Deliverables:

- Rust policy-signing adapter using HSM/KMS-backed keys;
- policy registry publisher and revocation workflow;
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

Exit criteria: the same rule version and normalized facts produce byte-identical results across repeated runs; every tax total is explainable; unsupported or ambiguous rules fail closed; a published calculation can be independently replayed from its evidence bundle.

## Phase 4 - indexer and reconciliation

Deliverables:

- subscription consumer and slot-based backfill;
- append-only raw event store;
- receipt, vault, and merchant read models;
- checkpoint, gap detection, replay, and balance reconciliation;
- operator tooling for a signature, receipt, and vault audit trail.

Exit criteria: delete and rebuild projections from chain evidence with identical results; injected missed events are detected and repaired.

## Phase 5 - React merchant and checkout experience

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

Exit criteria: all production-readiness gates in [production-readiness.md](production-readiness.md) are green and legal sign-off is recorded.

## Phase 7 - controlled production and public release

Deliverables:

- controlled mainnet launch with allow-listed merchants, assets, and published jurisdictions;
- live SLO/error-budget dashboards and staffed incident response;
- completed restore, key-recovery, RPC-failover, indexer-replay, and rule-rollback drills;
- signed release artifacts, SBOM, provenance, migration notes, and public changelog;
- public coverage matrix, support policy, security disclosure workflow, and release notes;
- post-launch review against latency, correctness, tax coverage, UX, and operational evidence.

Exit criteria: controlled production meets the Tier-1 product standard for the published scope; general availability is a separate approval, not an automatic consequence of deployment.

## Version and dependency governance

As of 2026-09-25, the documentation sources report Rust stable 1.98, Agave/Solana CLI 4.3.0, React 19.3, Node.js 24.21.0 LTS, Solidity 0.8.37, Anchor CLI 1.2.0, and NGINX stable 1.30.5. Confirm availability and Solana compatibility during bootstrap, then pin exact versions in source and CI. Do not use a floating `stable` or `latest` channel in production CI.

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
