# Local architecture-review evidence — 2026-10-05

This record captures reproducible local evidence for the current worktree. It
is a review artifact, not a production-readiness declaration. Measurements were
performed on Docker Desktop on an arm64 developer host against synthetic,
local-only data. They must not be presented as regional, mainnet, legal,
security-audit, or production SLO evidence.

## Environment

| Field | Value |
| --- | --- |
| Date | 2026-10-05 |
| Scope | `local_synthetic` |
| Host architecture | `arm64` |
| Container runtime | Docker Compose with pinned repository images and build inputs |
| Tax Engine image digest | `sha256:0e84052ae80ca9538626024cff8d7da56ea08a045b91f6fac9f4b2e9d6d1b948` |
| Authority | Local fixtures only; no staging or mainnet authority |

## Latency evidence

All measurements used bounded requests, explicit concurrency, and recorded
percentiles. A passing result means only that the named local workload met its
local budget; it does not establish a production SLO.

| Workload | Requests / concurrency | p50 | p95 | p99 | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| Warm Tax Engine service | 256 / 32 | 1.477 ms | 4.281 ms | 7.966 ms | Pass; 256/256 HTTP 200 |
| Aggregate quote service | 200 / 16 | 6.297 ms | 22.969 ms | 29.731 ms | Pass; exact idempotency/calculation/outbox cardinality |
| Document quote service | 200 / documented workload | 7.165 ms | 43.062 ms | 61.701 ms | Pass; exact persistence cardinality |
| Persisted rule simulation | 200 plus 64-way contention | 1.737 ms | 2.993 ms | 7.238 ms | Pass; 64/64 contention responses |
| Authorization capabilities | 256 / 32 | 1.141 ms | 3.276 ms | 4.943 ms | Pass; 19-policy catalog and decision semantics |

The exact command for the Tax Engine service result is:

```text
make benchmark-tax-engine-service
```

The command emitted `measured_at_utc=2026-10-05T17:33:59Z`, bounded the request
timeout at 10 seconds, and used a cache capacity and maximum concurrency of 64
and 32 respectively.

## Correctness and operational gates

The following local gates passed in the current worktree:

- `make check`
  - Rust formatting, Clippy, unit/integration tests, and panic-safety contract;
  - frontend tests, typecheck, production build, palette and accessibility
    contracts;
  - Docker Compose, observability, migration, deployment, and release-bundle
    contracts;
  - dependency and container scans with zero reported Cargo/pnpm
    vulnerabilities and zero reported Dockerfile high/critical findings;
  - public-repository documentation contract.
- `make integration-check`
  - concurrent idempotency fencing, ambiguous-provider reconciliation,
    process-crash recovery, immutable rule rollback, and finalized adjustment
    evidence.
- `make backup-restore-drill`
- `make migration-integrity-drill`
- `make migration-rollback-rehearsal`
- `make authorization-benchmark-contract`
- `make web-browser-check` — 48/48 browser journeys passed, including
  unavailable, expired, stale-data, authorization, and wallet-bound flows.
- A fresh exact-worktree `make check` also passed after the current browser
  rebuild: 76 frontend test files, 329 passing tests, 10 intentional skips;
  zero Cargo/pnpm vulnerabilities; zero Dockerfile high/critical findings;
  and all repository contracts green. The only emitted platform warnings were
  the expected arm64 Alertmanager image mismatch and the fixed amd64 EVM
  adapter build boundary.
- After ADR-0402's result-workspace cleanup, the current frontend suite passes
  77 test files with 330 passing tests and 10 intentional skips;
  `pnpm run build` and `pnpm exec tsc --noEmit` also pass. The regression
  proves one rendered tax component trace per validated component.
- The rebuilt Docker/Nginx browser gate then passed all 48 real-browser
  journeys, including progressive disclosures, responsive and RTL behavior,
  forced colors, localization, wallet-bound, and fail-closed states.
- The subsequent exact-current `make check` passed the complete Rust and
  frontend gates: 77 frontend test files, 330 passing tests, 10 intentional
  skips, clean typecheck/build, zero Cargo/pnpm vulnerabilities, zero
  Dockerfile high/critical findings, and all repository contracts green. The
  expected arm64 Alertmanager and fixed amd64 EVM build warnings remain
  platform disclosures, not failed checks.
- After ADR-0403, the focused shell suite passes 20 tests and proves that the
  verified account reference is rendered once in the shell while the
  persistent sidebar retains access state. This is a presentation-only
  reduction; server-owned capability decisions and page-level protection
  context are unchanged.
- The follow-up stylesheet cleanup removes the retired duplicate-account
  selector and adds a failing-first Material shell contract for its absence.
  The frontend suite now passes 331 tests with 10 intentional skips, and the
  Docker/Nginx browser suite passes all 48 real-browser journeys after the
  cleanup.
- The exact current `make check` after ADR-0403 passed the full Rust workspace,
  77 frontend test files (331 passing tests and 10 intentional skips),
  typecheck/build, Material palette and forced-colors contracts, security
  scans, runtime contracts, EVM checks, and public documentation checks. The
  expected arm64 Alertmanager and fixed linux/amd64 EVM build warnings remain
  platform disclosures; they are not failed checks.

## Follow-up integration evidence — 2026-10-06

The exact current worktree also passed `make integration-check` from a fresh
Docker integration project. The gate built the pinned API, worker, migrator,
edge, RPC-fixture, and attestation-fixture images, started PostgreSQL 18 and
NATS, applied migrations, and exercised the authenticated API through the
NGINX edge. The result is local synthetic evidence only.

The exercised scenarios and observed outcomes were:

- concurrent duplicate settlement intent: one `200`, one `409`, and exactly
  one RPC submission;
- ambiguous provider outcome: durable `unknown` state reconciled to
  `confirmed` with bounded retry behavior;
- API process crash: `submitting` intent recovered and reconciled to
  `confirmed` after restart;
- immutable tax-rule rollback: rollback contract passed;
- settlement adjustment: duplicate requests fenced with `200/409` and
  finalized event evidence recorded;
- all integration assertions completed with `local stack integration checks
  passed` and exit code `0`.

This run proves the bounded local integration path and recovery contracts. It
does not prove Linux/amd64 validator conformance, live-provider behavior,
regional latency, custody controls, legal scope, or production readiness.

## Full local gate re-run — 2026-10-06

The exact current worktree then passed `make check` with exit code `0`. This
re-run covered the complete Rust workspace, 77 frontend test files with 331
passing tests and 10 intentional skips, typecheck, production build, Material
3 semantic palette and 4.5:1 contrast checks, forced-colors and CSP contracts,
authorization/wallet/provenance/release-claim contracts, Docker topology and
observability validation, and EVM adapter tests. Trivy reported zero Cargo and
pnpm vulnerabilities and zero Dockerfile misconfigurations. Docker emitted the
known arm64 host warning for the amd64-only Alertmanager image; that warning is
an environment disclosure and did not change the exit status.

The ADR-0404 authorization affordance change was included in this rerun. Its
focused browser contract passed 40 tests, including all four supported locale
catalogs, typed reason-code coverage, and the assertion that the raw English
server reason is absent from the German navigation title. The full frontend
suite and typecheck then passed as part of `make check`.

The subsequent Docker/NGINX Playwright run passed all 48 real-browser
journeys. This included grouped and compact navigation, expired capability
states, German and Arabic RTL localization, forced colors, WCAG assertions,
theme persistence, wallet-bound flows, direct workspace links, and fail-closed
API/reconciliation states.

## Material browser chrome evidence — 2026-10-06

ADR-0405 moved browser `theme-color` values to the approved Material 3
background metadata boundary. The focused display-preference and Material
source-contract suite passed 12 tests: explicit light/dark modes select only
the corresponding metadata entry, while `system` preserves the two native
`prefers-color-scheme` media conditions. The production-source color contract
also passed without exceptions; TypeScript contains no palette literals.

The exact current `make check` then passed with 77 frontend test files, 335
passing tests, 10 intentional skips, clean typecheck/build, zero Cargo/pnpm
vulnerabilities, zero Dockerfile high/critical findings, and all repository
contracts green. The Docker/NGINX Playwright run passed all 48 real-browser
journeys after rebuilding the served SPA, including the existing theme,
localization, accessibility, wallet-bound, and fail-closed journeys.
The theme journey additionally asserted the initial system media queries and
the explicit light/dark `media` activation states for both browser metadata
entries.

The currently attached Docker Desktop PostgreSQL volume is not a clean
checkout: its ledger contains a historical checksum for
`0060_revenue_streaming_bounded_context` that differs from the repository
file. The migration runner rejected startup with checksum drift and now emits
forward-only recovery guidance, as required;
the applied migration must not be rewritten to make that volume pass. A clean
local bootstrap requires the documented synthetic-only reset with explicit
operator confirmation, or restoration of the matching project volume. This
environmental failure is not counted as a passing bootstrap result.

The local Solana conformance command remains intentionally fail-closed on this
host because the pinned Agave validator requires Linux/amd64-compatible
validator evidence. The current command result is:

```text
make solana-conformance
Solana conformance host preflight failed: host_os_must_be_linux
Authoritative evidence requires a native Linux/amd64 host and Linux/amd64 Docker engine; ARM Docker emulation is diagnostic only.
```

This is an expected release blocker on the current macOS host. A build-only
result or a mocked RPC result is not recorded as conformance.

## Remaining release evidence

These items remain open and must stay open until their named evidence is
retained:

- Linux/amd64 pinned-validator transactions for legacy SPL, extension-free
  Token-2022, duplicate nonce, unsupported extension, route activation, and
  settlement/refund invariants.
- Staging identity-provider configuration, tenant isolation, key rotation,
  revocation, and incident-rehearsal evidence.
- HSM/KMS or equivalent signer controls, multisig/timelock governance, and
  independent security/program audit.
- Live RPC/provider failover, regional latency/resource profiles, load/soak/
  chaos evidence, and production SLO approval.
- Legal/tax ownership, jurisdiction scope approval, rule provenance review,
  filing/reporting boundaries, and policy sign-off.
- Manual WCAG 2.2 AA review with keyboard-only, screen-reader, high-contrast,
  reduced-motion, RTL, and localization review.
- An approved public-repository license and release-owner sign-off.

The authoritative checklist remains
[production-readiness.md](../plans/production-readiness.md). Reviewers should
attach command output and immutable artifact digests to the relevant change or
release record rather than converting this local record into a readiness claim.

## Mobile shell action-row evidence — 2026-10-06

ADR-0406 added a failing-first Playwright contract at 568 CSS pixels. The
focused Docker run first failed against the inherited flex-column rule, then
passed after the presentation-only Material 3 action-row change. The complete
`make web-browser-check` subsequently passed 49/49 journeys through the
pinned NGINX edge. The contract verifies a two-row grid top bar, wrapped
horizontal actions, preserved breadcrumb visibility, and a bounded compact
header; it does not claim manual screen-reader, operator-usability, or
production evidence.

## Docker integration evidence — 2026-10-06

The current `make integration-check` completed with exit code `0` on a fresh
Docker PostgreSQL 18/NATS stack. It rebuilt and started the API, worker,
migrator, RPC fixture, and NGINX edge, then retained evidence for:

- concurrent settlement submission returning `[200, 409]` with one fixture
  `sendTransaction` call;
- ambiguous submission followed by durable finalized reconciliation;
- API process-crash recovery and subsequent finalized reconciliation;
- immutable governed rule rollback;
- wallet-bound adjustment event proof with concurrent duplicate rejection; and
- local stack teardown and restart behavior.

This is synthetic Docker-local evidence. It does not claim live RPC/provider,
Linux/amd64 validator, HSM/KMS, legal, regional SLO, or production identity
readiness.

## Refreshed latency evidence — 2026-10-06

The current bounded benchmark runs produced these additional local samples:

| Workload | Requests / concurrency | p50 | p95 | p99 | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| Aggregate quote service | 200 / 16 | 5.545 ms | 26.046 ms | 47.393 ms | Pass; 32/32 same-key contention and exact durable convergence |
| Authorization capabilities | 256 / 32 | 1.153 ms | 2.253 ms | 5.400 ms | Pass; 256/256 HTTP 200 and 25 ms p95 local budget met |

Both workloads used isolated Docker Compose projects, pinned repository images,
synthetic tenant credentials, and bounded timeouts. These measurements are
arm64 local evidence only and do not establish regional or production SLOs.

## Native tax-engine benchmark — 2026-10-06

`make bench-tax-engine` passed with Rust 1.98.1, three seconds of warm-up, and
100 Criterion samples per benchmark:

| Benchmark | Median |
| --- | ---: |
| Rule compilation | 755.47 ns |
| Uncached calculation | 1.4429 µs |
| Compiled calculation | 663.01 ns |
| 64-line / 16-component document | 256.80 µs |

Criterion reported improvement or noise-range changes for all four benchmarks.
This is native arm64 microbenchmark evidence only; service, Linux/amd64,
regional, RPC, and production SLO evidence remain separate gates.

## Tax applicability kernel — 2026-10-06

ADR-0407's first implementation slice added the `tax-applicability` workspace
crate. `cargo test --locked -p tax-applicability --lib` passed 7/7 tests, including
the checked-in `tax-applicability-v1.json` golden vector, explicit treatment
outcomes, precedence selection, unresolved behavior, ambiguity rejection, and
stale-evidence rejection plus bounded decision validation. `cargo test --locked
-p tax-engine --lib` passed 74/74 tests, including facts-digest binding,
document-scope binding, replay, legacy wire-shape preservation, and typed
fail-closed treatment gates. `cargo test --locked -p policy --lib` passed
14/14 tests, including signed applicability binding, source-evidence lifetime,
distinct canonical domain, and strict decimal-string sequence decoding.
`cargo clippy --locked --workspace --all-targets
-- -D warnings` also passed with the new crate and binding included.

This is a pure-domain and verification-kernel slice only. The policy crate now
signs and verifies a distinct `ApplicabilityAttestation` domain that binds the
facts and decision digests, jurisdiction-resolution digest, source evidence
window, rule release, issuer identity, and sequence. It does not claim API or
storage integration, settlement consumption, legal/tax approval, or production
jurisdiction coverage.

The exact `cargo test --locked --workspace --all-targets` run also passed with
the new crate and binding included. Existing Rust tests, Tax Engine Criterion
targets, the seven applicability tests, the initial 13 policy tests, and the
74 tax-engine tests were green. The subsequent focused policy run passed all
14 tests after strict sequence decoding and exact decision-digest binding were
added.
This is source-level regression evidence only; the new context is not yet wired
into quote, attestation, or settlement authority.

## Server-owned applicability context persistence — 2026-10-06

ADR-0409 added PostgreSQL migration `0070_tax_applicability_contexts.sql` with
UUIDv7 context/event IDs, immutable published/retired projections, four-eyes
subject constraints, append-only governance events, optional calculation
foreign keys, and bounded verification SQL. `make migration-integrity-drill`
passed with 70 migrations applied concurrently and idempotently; the new
context lifecycle, UUIDv7 identity, published immutability, and append-only
event contract passed.

The storage adapter now has typed create, transition, and published-valid load
boundaries. It re-verifies the signed applicability attestation, tenant
identity, facts/decision digests, projected JSON, source-evidence expiry, and
effective window. Storage tests passed 49/49. The API rejects caller-owned
applicability in both top-level quote payloads and nested Tax Engine facts;
API tests passed 86/86. Quote reference persistence and Solana settlement
binding remain intentionally open for the next slice.
