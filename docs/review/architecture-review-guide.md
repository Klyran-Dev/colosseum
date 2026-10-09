# Architecture review guide

This guide is the shortest reliable path for reviewing the repository as a
production candidate. It is a review aid, not a production-readiness claim.
The reviewer should verify the source, run the relevant gate, and record any
remaining evidence gap rather than inferring readiness from a green unit test.

## Review order

Review in this order because each boundary constrains the next one:

1. [System architecture](../architecture/system-architecture.md) and
   [bounded contexts](../architecture/domain-model.md).
2. [Alternative analysis](../architecture/alternative-analysis.md) and the
   [ADR index](../adr/README.md).
3. [Security and threat model](../architecture/security-threat-model.md).
4. [API contract](../api-contract.md), database migrations, and storage
   authorization predicates.
5. Rust domain crates and their tests.
6. Solana program account constraints and settlement-client wire vectors.
7. React shell, authorization presentation, wallet boundary, and browser
   journeys.
8. Docker topology, observability, release workflow, and runbooks.

The reviewer should start with the relevant ADR, then inspect the production
code, then run the narrowest gate that proves the decision. The full gate is
the final local cross-boundary check.

## High-risk boundary map

| Boundary | Authoritative source | Required invariant | Local proof | Evidence still required |
| --- | --- | --- | --- | --- |
| Tax arithmetic | `crates/tax-engine` and [Tier-1 tax engine](../architecture/tax-engine.md) | Exact integer/rational semantics, bounded DAG, replayable trace | Unit, property, golden-vector, mutation, and benchmark tests | Legal ownership and jurisdiction approval |
| Tenant authorization | `crates/api`, `crates/storage`, `apps/web/src/authorization.ts` | Authenticated tenant scope and server re-authorization on every protected action | API tests, capability contract, browser deny/expiry journeys | Staging IdP, multi-tenant, rotation, and incident evidence |
| Route admission | `crates/route-verifier`, `crates/indexer`, PostgreSQL route projections | Only finalized v2 asset/account evidence can activate a route | Storage/direct-SQL contracts and Docker integration drill | Non-local backfill, Linux/amd64 validator evidence, independent review |
| Route-verification operations | `crates/api`, `crates/storage`, `apps/web/src/ReplayOpsPanel.tsx` | System operators may read bounded verifier evidence, but a run row never activates a route or proves finality | UUIDv7/counter parser tests, exact-role API test, explicit-disclosure UI test, Docker browser journey | Non-local backfill, Linux/amd64 validator evidence, audit, and production operator review |
| Settlement authority | `programs/settlement`, `crates/settlement-client`, signed-message verifier | Account order, mint/program/decimals, nonce, amount conservation, and policy binding | Rust vectors, browser envelope checks, compiler tests | Linux validator transactions, audit, live RPC/provider evidence |
| Wallet custody | `apps/web/src/wallet.ts` and wallet boundary ADRs | Browser requests a signature but never receives or stores merchant keys | Wallet rejection, account selection, mismatch, and reload journeys | Live Wallet Standard matrix and wallet-provider review |
| Evidence | PostgreSQL append-only projections and outbox | Every accepted decision is replayable and never silently upgraded to finality | Integration reconciliation, crash, ambiguous-provider, and rollback drills | Backup/PITR, retention, restore, and operational review |
| UI presentation | `apps/web/src/ProductShell.tsx`, `styles.css`, and [UI architecture](../architecture/ui-ux.md) | Semantic Material roles, plain-language outcomes, accessible fail-closed states | Web tests, palette/color contracts, axe, responsive/RTL/reduced-motion journeys | Manual screen-reader, keyboard-only, high-contrast, localization, and operator review |

## Commands

Run commands from a clean Docker-enabled checkout. The commands below do not
replace staging or production approval.

```text
make check
make integration-check
make web-browser-check
make solana-conformance
make public-docs-check
git diff --check
```

`make solana-conformance` must fail closed when the host cannot run the pinned
Agave validator. A build-only result, mock RPC, skipped test, or local
`io_uring` failure is not validator evidence. The authoritative release path
is the Linux/amd64 CI workflow and its retained evidence bundle.

## Review questions

For every change, the reviewer should be able to answer:

- What is the source of truth, and which values are only projections or cache?
- What happens on duplicate submission, timeout, provider disagreement,
  process crash, stale authorization, stale rule, or uncertain finality?
- Which account, tenant, signer, route, rule, and evidence fields are bound
  cryptographically or transactionally?
- What is the p50/p95/p99 budget and what workload produced the measurement?
- Can the operation be retried without duplicate movement or duplicate
  evidence?
- Which data is intentionally kept off-chain, redacted, encrypted, or
  retained only for a bounded period?
- Does the UI tell the operator the next safe action without implying that a
  calculation is a filing, a submitted transaction is finalized, or a
  database row authorizes token movement?
- What rollback, migration, alert, runbook, and ownership evidence accompanies
  the change?

## Local evidence versus release evidence

Local Docker fixtures prove wiring, deterministic behavior, failure handling,
and contract shape. They do not prove legal correctness, independent audit,
mainnet behavior, HSM/KMS custody, production identity configuration, regional
latency, or a live validator run.

The current open gates are tracked in the
[production-readiness plan](../plans/production-readiness.md) and the
[implementation plan](../plans/implementation-plan.md). Reviewers must leave
those items unchecked until the named external evidence exists. The public
repository must not use “production-ready”, “supported jurisdiction”, audit,
or benchmark claims without the corresponding retained evidence.

## Review record

The latest local evidence is recorded in
[2026-10-05-local-evidence.md](2026-10-05-local-evidence.md). It is scoped to
Docker synthetic workloads on an arm64 developer host and deliberately leaves
external release gates open.

Use the pull request, release candidate, or change record to capture:

- commit or artifact digest reviewed;
- commands and environment used;
- passed gates and retained artifact locations;
- unresolved findings with owner and due date;
- explicit decision about whether the change is local-only, staging-ready,
  or eligible for a controlled-production review.
