# Requirements traceability and evidence matrix

This matrix is the public review index for the mandates in
[`AGENTS.md`](../../AGENTS.md). It separates what the repository proves from
what requires Linux CI, staging, production infrastructure, legal ownership,
or an independent audit. A green local command never upgrades an item to a
production claim by itself.

## Evidence levels

| Level | Meaning | Permitted claim |
| --- | --- | --- |
| Repository | Source, unit/property tests, schemas, ADRs, and static contracts | The design and executable boundary exist in source control |
| Docker local | Pinned containers, synthetic fixtures, local PostgreSQL/NGINX/RPC, and bounded drills | The local workflow is reproducible and the tested failure semantics hold in the fixture topology |
| Linux CI | Retained `ubuntu-24.04`/amd64 validator, build, and release artifacts | The exact commit passed the named Linux workload in the retained run |
| Staging | Approved identity, wallets, providers, secrets, TLS, regional topology, and production-like data controls | The approved staging configuration passed its named operational gate |
| Production / external | Mainnet, HSM/KMS, legal/tax owner, independent audit, support, and live SLO evidence | The specific production or legal claim is approved and current |

## Mandate coverage

| AGENTS.md mandate | Authoritative implementation | Verification evidence | Current status |
| --- | --- | --- | --- |
| Design before implementation and alternatives before decisions | `docs/adr/`, `docs/architecture/alternative-analysis.md`, `docs/plans/implementation-plan.md` | `make public-docs-check`; ADR index and PR template | Repository and documentation gate verified |
| Deterministic Tier-1 tax engine | `crates/tax-engine`, `crates/jurisdiction`, `test-vectors/`, `docs/product/tax-coverage-matrix.md` | `cargo test --locked -p tax-engine`; golden-vector and property tests; Criterion benchmarks | Local deterministic evidence verified; legal coverage and approved rule ownership remain open |
| Solana settlement authority | `programs/settlement`, `crates/settlement-client`, `crates/settlement-coordinator` | Account-meta, conservation, replay, Token-2022, and signed-message tests; `make solana-conformance` | Source and local contracts verified; retained Linux/amd64 validator evidence remains open |
| Stablecoin asset admission | `programs/settlement` `AssetConfig` PDA, `crates/settlement-client`, browser envelope, route registry/verifier, ADR-0320, ADR-0321 | Focused Rust/client/API/coordinator/browser PDA and account-order tests; five-account finalized route-verifier tests; real-validator transaction builders; explicit cross-tenant aggregate-compile integration negative; planned Linux validator cases | On-chain/client/browser boundary and finalized asset-account proof are verified; tenant-scoped compiler rejection is verified in repository/Docker tests; existing-route backfill, audit, governance, and Linux/amd64 evidence remain open; missing asset evidence must block signing |
| Revenue-stream movement boundary | `programs/settlement`, `apps/web/src/solana.localnet.test.ts`, ADR-0301 | Real validator case for finalized accruals, single-use execution, duplicate rejection, and cancellation; conformance evidence contract | Repository case and local Docker path implemented; Linux/amd64 execution remains open before enablement |
| Attestation and evidence replay | `crates/policy`, `crates/fx-evidence`, `crates/storage`, raw evidence projections | Cross-language vectors, tamper/replay tests, migration and rebuild drills | Local evidence verified; production key custody and retention remain open |
| RBAC/PBAC/ABAC and tenant boundaries | `crates/api`, `crates/storage`, `apps/web/src/authorization.tsx`, session/OIDC boundary | API authorization tests, capability benchmark, browser tenant-reset journey | Local evidence verified; staging IdP lifecycle and revocation evidence remain open |
| PostgreSQL 18 and UUIDv7 | `db/migrations`, `db/verification/uuidv7_contract.sql`, storage domain types | `make migration-bootstrap-contract`; UUIDv7 and migration tests; PostgreSQL 18 Compose image | Local schema evidence verified; production backup/PITR and restore evidence remain open |
| Bounded low-latency concurrency | Rust/Tokio ports, bounded queues, deadlines, retry budgets, Criterion and load scripts | `make bench-tax-engine`, `make benchmark-tax-engine-service`, quote/document/authorization workloads | Local measurements exist; regional Linux resource and p99 SLO evidence remains open |
| React production UI with NGINX | `apps/web`, `ops/nginx`, `apps/web/e2e` | `make web-check`, `make web-browser-check`, CSP/color/language contracts, bounded API error disclosure (ADR-0319), browser axe checks, ADR-0312 outcome-first document-payment journey, ADR-0313 compact protection context, ADR-0315 tax-responsibility language boundary, ADR-0318 workflow-handoff URL integrity | Local browser evidence verified; live wallet, staging identity, manual assistive-technology, counsel, and merchant-term review remain open |
| Material 3 Expressive visual roles | `apps/web/src/styles.css`, ADR-0298, ADR-0300, ADR-0304, ADR-0310, ADR-0311, ADR-0317, semantic token contract | `make web-color-contract`, `make material-palette-contract`; exact Solana Aurora matrix, surface hierarchy, and gradient regression; theme/component tests; browser responsive/RTL/reduced-motion checks | Repository and local browser evidence verified; full operator usability review remains open |
| Outcome-first advanced evidence disclosure | Aggregate/document quote workspaces, ADR-0306 | Component closed-state assertion; real-browser disclosure interaction; `make web-browser-check` | Repository and local browser evidence verified; operator usability review remains open |
| Project-scoped local reset | `scripts/reset_local.sh`, ADR-0307 | `make reset-local-contract`; clean PostgreSQL 18/API/NGINX bootstrap | Repository contract and clean local bootstrap verified; command is intentionally destructive only to explicitly selected synthetic local state |
| Versioned presentation preferences | `apps/web/src/displayPreferences.ts`, ADR-0302 | Preference unit tests; 25-test Docker browser suite verifies reload restoration and blank credential; public-docs contract | Local browser evidence verified; server profile synchronization is intentionally deferred |
| Docker reproducibility and pinned supply chain | `compose.yaml`, `ops/docker/`, `scripts/verify_*contract.sh`, `make tier1-local-check` | Compose config, image/action pinning, security scan, bootstrap and runtime drills; ordered Tier-1 gate contract | Local contracts verified; complete Linux/amd64 execution and release artifact retention remain open |
| Release evidence provenance | `scripts/create_release_bundle.sh`, `scripts/verify_solana_conformance_evidence.sh`, ADR-0299, ADR-0303, ADR-0308 | `make release-bundle-integrity-contract`, workflow-order contract, exact-release browser and authenticated integration runs, release-candidate manifest | Validator, browser, and integration revision binding are verified locally; retained Linux release bundle and independent review remain open |
| Observability is non-authoritative | `crates/observability`, `ops/observability`, ADR-0223 | Metric registry contract, Prometheus/Alertmanager validation, OTLP and alert drills | Local evidence verified; production collector authentication, retention, delivery, and on-call response remain open |
| Key separation and fail-closed custody | Solana authority model, release claims, `docs/runbooks/`, ADR-0219 and related key-rotation ADRs | Negative authorization tests and synthetic rotation/mTLS contracts | Repository boundaries verified; HSM/KMS non-exportability, multisig ceremonies, and production rotation remain open |
| Non-custodial wallet boundary | `apps/web/src/wallet.ts`, authenticated API request DTOs, ADR-0059, ADR-0200, ADR-0314 | `make non-custodial-wallet-contract`, Wallet Standard tests, signed-message and artifact-binding tests | Repository source boundary verified; live wallet-provider matrix, Linux validator, custody ceremony, and independent security review remain open |
| Tax responsibility language boundary | `apps/web/src/ProductShell.tsx`, tax/quote/payment workspaces, ADR-0016, ADR-0271, ADR-0315 | `make ui-language-contract`, component language tests, browser journey and accessibility checks | Repository UI distinctions verified; counsel, merchant terms, jurisdiction owner, localization, and assistive-technology approval remain open |
| Published rule release provenance | `crates/storage`, `crates/api`, `db/migrations/0065_rule_release_provenance_readiness.sql`, `apps/web/src/GovernancePanel.tsx`, ADR-0316 | `cargo test --locked -p storage rule_release_provenance --lib`; `make rule-release-provenance-contract`; `make integration-check` publication-boundary and direct-SQL trigger cases | Rust/DB/UI shape and fail-closed local evidence verified; source verification, legal/tax owner approval, production migration evidence, and jurisdiction sign-off remain open |
| Public repository quality | `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CHANGELOG.md`, `docs/`, `.gitignore` | `make public-docs-check`; CI workflow and release-bundle contracts | Repository documentation gate verified; license decision and independent release review remain open |

## Required release evidence still external to this checkout

These items are intentionally not marked complete by local fixtures or
documentation:

- retained Linux/amd64 Agave validator conformance for every release-enabled
  movement path;
- staging OIDC, wallet, RPC, provider, TLS, secret-manager, and multi-tenant
  evidence;
- HSM/KMS-backed non-exportable issuer and release-key ceremonies with
  multisig/timelock governance;
- regional p50/p95/p99 quote, compile, submit, confirmation, finalization,
  resource, failover, and error-budget evidence;
- independent Solana/security review and closure of critical/high findings;
- legal/tax owner approval of each supported jurisdiction, rule provenance,
  merchant responsibility, filing boundary, privacy retention, and customer
  language;
- production backup/PITR, key recovery, provider exit, replay, incident, and
  on-call drills;
- explicit repository license selection before public source release.

## Review commands

Use the smallest command that matches the claim, then retain the output with
the release or review artifact:

```text
make check
make integration-check
make web-browser-check
make solana-conformance
make tier1-local-check
make public-docs-check
```

`make tier1-local-check` is the ordered end-to-end local evidence gate;
individual commands remain useful for focused diagnosis. `make check` is
repository and Docker-local evidence. `make solana-conformance` only becomes
Linux validator evidence when executed by the pinned Linux CI
workflow and its postcondition artifact is retained. None of these commands
establishes legal compliance, mainnet readiness, production custody, or a
regional SLO without the corresponding external evidence.
