# Principal architecture review — 2026-10-06

## Review posture

This repository is ready for a technical code review of its implemented local
vertical slices. It is not a production-readiness approval, mainnet approval,
tax-compliance certification, or independent security audit. The distinction
is intentional: source code, Docker-local evidence, Linux validator evidence,
staging evidence, and external legal/security approval are separate claims.

The review scope is the complete current worktree and the mandates in
[`AGENTS.md`](../../AGENTS.md). Reviewers should preserve unrelated local
changes and use the current commit/worktree as the evidence subject.

## Recommended review order

1. Read the bounded-context and trust-boundary model in
   [`system-architecture.md`](../architecture/system-architecture.md),
   [`domain-model.md`](../architecture/domain-model.md), and
   [`security-threat-model.md`](../architecture/security-threat-model.md).
2. Review the alternative analysis before evaluating implementation choices:
   [`alternative-analysis.md`](../architecture/alternative-analysis.md).
3. Review the authority paths in this order:
   `programs/settlement` → `crates/settlement-client` →
   `crates/settlement-coordinator` → `crates/storage` → `crates/api` →
   `apps/web/src`.
4. Review the Tier-1 tax semantics and replay boundary in
   [`tax-engine.md`](../architecture/tax-engine.md), the tax coverage matrix,
   and the versioned golden vectors.
5. Review the delivery and failure evidence in
   [`production-readiness.md`](../plans/production-readiness.md) and the
   [`requirements-traceability.md`](../operations/requirements-traceability.md)
   matrix.

## Current local evidence

The exact current `make check` passed on 2026-10-06. It included:

- the Rust workspace clippy, unit, property, and Criterion targets;
- 79 frontend test files with 346 passing tests and 10 intentional skips;
- TypeScript type checking and the production Vite build;
- Material 3 semantic color, 4.5:1 palette contrast, forced-colors, CSP,
  release-claim, authorization, and UI-language contracts;
- PostgreSQL 18 migration, UUIDv7, Docker, observability, EVM-adapter,
  runtime, reload, and configuration contracts;
- zero Cargo/pnpm vulnerability findings and zero Dockerfile high/critical
  findings from the pinned Trivy scan; and
- public-repository, documentation, and release-bundle-integrity contracts.

The repository `make check` target now invokes
`make release-bundle-integrity-contract`, so the required conformance preflight
schema, exact source-revision rejection, artifact/configuration hashes, and
release manifest checks are covered by the ordinary repository gate as well as
the release-candidate workflow.

ADR-0417 was then implemented and revalidated:

- calculation and document attestation v2 commit a domain-separated
  treatment-outcome digest while preserving v1 canonical bytes for historical
  evidence;
- the attestation fixture derives the digest from the actual aggregate or
  document result, and API/storage verification derives it again from the
  server-owned result before persistence or settlement compilation;
- 17 policy tests, 96 API tests, and the shared treatment golden vector passed;
  and
- `make migration-integrity-drill`, `make integration-check`, and
  `make public-docs-check` passed after the change.

The authenticated Docker integration also now covers ADR-0418 directly: a
signed applicability attestation is created through the API, replayed
idempotently, read under the authenticated tenant, rejected across tenants,
advanced through distinct author/reviewer/approver identities, and checked for
four append-only lifecycle events, four outbox records, and published-row
immutability. The corrected storage mapping binds the reviewer at approval and
the approver at publication; this is local PostgreSQL evidence only.

The Docker/Nginx real-browser gate passed all 49 journeys, including keyboard
navigation, responsive layout, RTL, localization, forced colors, WCAG axe
checks, wallet-bound flows, authorization expiry, failure recovery,
reconciliation-required states, and the ADR-0405 browser `theme-color` media
activation contract for system, light, and dark preferences. The latest run
also includes the ADR-0406 compact mobile shell action-row geometry contract.

These results prove repository and Docker-local behavior only. They do not
promote synthetic RPC, local credentials, Docker Desktop arm64 execution, or
test fixtures into production evidence.

The complete `make tier1-local-check` was then run on this worktree. Every
local layer passed, including the authenticated Docker integration and all 49
real-browser journeys. The gate correctly stopped at the validator boundary:
`make solana-conformance` failed closed with `host_os_must_be_linux` because
this host is macOS/ARM. Native Linux/amd64 validator evidence remains open and
is not substituted with emulation or a mock.

The pinned toolchain was revalidated against the official release channels on
2026-10-06. The selected stable/LTS baseline remains unchanged: Rust 1.98.1,
React 19.3.0, Node.js 24.21.0 LTS, PostgreSQL 18.6, and NGINX 1.30.5, with
Agave/Solana 4.3.0 and Anchor 1.2.0 retained at the Solana compatibility
boundary. Node 26 Current and NGINX 1.31 mainline are intentionally not
adopted without the compatibility-change process and evidence required by
ADR-0163; this is a production stability decision, not a claim that older
versions are universally newest.

The runtime panic-safety audit also passed on this worktree. A source scan of
the Rust crates and settlement program found no `unwrap`, `expect`, or
`panic!` use before the test-module boundary; the remaining matches are
test/fixture/benchmark assertions. The repository additionally enforces the
crate-root `not(test)` Clippy denies through
[`make rust-panic-safety-contract`](../../Makefile) and the workspace
`cargo clippy --locked --workspace --all-targets -- -D warnings` gate. This is
an implementation guard, not a substitute for fuzzing, fault injection, or
independent security review.

## Principal review invariants

The reviewer should require evidence for each of these invariants:

- Solana finalized chain evidence is authoritative for token movement; API,
  database, cache, event stream, and UI state are projections or preparation
  artifacts only.
- The tax engine is deterministic, exact, versioned, replayable, bounded, and
  independent of HTTP, storage, wall-clock, RPC, and wallet state.
- Attestations bind merchant, asset, facts, policy/rule digest, engine version,
  expiry, amount bounds, and (for v2) all treatment outcome channels; the
  settlement program verifies the resulting policy digest binding.
- Every tenant-scoped read/write derives scope from the authenticated
  principal, and browser capability decisions never authorize protected work.
- Money values use atomic integer units; financial JSON counters use canonical
  decimal strings; internal sortable identifiers use PostgreSQL native UUIDv7.
- Submission is idempotent and fail-closed across retries, ambiguous provider
  outcomes, stale blockhashes, duplicate nonces, account substitution, and
  indexer gaps.
- Browser wallets remain the user signing boundary; issuer, governance, and
  vault-release keys never enter browser or ordinary API custody.
- UI states distinguish preparation, authorization, approval, signing,
  submission, finalized evidence, unknown outcome, and reconciliation-required
  recovery without implying tax filing, legal compliance, or payment success.

## Release gates that remain open

The following cannot be closed by additional local mocks or documentation:

- retained Linux/amd64 validator conformance for every enabled Solana movement
  path, including legacy SPL, extension-free Token-2022, split, refund,
  adjustment, revenue-stream, pause, and revocation cases;
- staging OIDC, wallet, provider, RPC, TLS, secret-manager, and multi-tenant
  evidence;
- HSM/KMS non-exportability, multisig/timelock ceremonies, and key rotation;
- regional latency/resource/failover evidence and production error-budget
  ownership;
- independent Solana/security review and closure of critical/high findings;
- legal/tax owner approval of jurisdictions, provenance, merchant terms,
  privacy retention, and customer-facing language;
- production backup/PITR, key recovery, provider exit, replay, incident, and
  on-call drills; and
- explicit repository license selection before public source release.

The authoritative checklist and evidence levels are maintained in the
[production-readiness plan](../plans/production-readiness.md) and the
[requirements traceability matrix](../operations/requirements-traceability.md).

## Review commands

```text
make check
make integration-check
make web-browser-check
make public-docs-check
make solana-conformance
make tier1-local-check
```

`make solana-conformance` is only Linux/amd64 validator evidence when the
pinned CI workflow executes it and retains its postcondition artifact. A local
arm64 run must fail closed rather than downgrade to a mock or skipped result.
