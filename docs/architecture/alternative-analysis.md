# Architecture alternative analysis

This document records the comparison baseline required before implementation. It is intentionally explicit: “latest” or “fast” is not an evaluation criterion without a workload, support horizon, security model, and operational cost.

## Evaluation criteria

- deterministic financial correctness;
- Solana and wallet compatibility;
- p50/p95/p99 latency and tail behavior;
- throughput and contention under concurrent merchants;
- auditability and replayability;
- security and key isolation;
- developer correctness and testability;
- operational complexity and failure recovery;
- accessibility and product-quality UX;
- support lifecycle and supply-chain reproducibility.

## Decisions

### Production Rust panic safety

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Review checklist only | No build changes | Regressions can be introduced between reviews | Rejected |
| Text grep over all Rust sources | Easy to run | False positives in tests/benches and no Rust configuration awareness | Rejected |
| Workspace-wide deny lint with no test distinction | Strong default | Rejects legitimate fixture assertions and obscures the production boundary | Rejected |
| **Per-crate `cfg_attr(not(test), deny(clippy::expect_used, clippy::panic, clippy::unwrap_used))` plus workspace Clippy** | Enforced at compile time, no runtime cost, test fixtures remain expressive | New crate roots must copy the contract | **Selected in ADR-0212** |

### Signed-message adversarial verification

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Example-only account-meta tests | Simple and fast | Misses positional and mutation combinations | Rejected |
| Coverage-guided `cargo-fuzz` as the mandatory gate | Broad exploration | Requires a separate nightly/toolchain and reproducibility contract | Deferred |
| **Bounded stable-Rust `proptest` over real signed transactions** | Reproducible, shrinkable, CI-friendly, and tests the actual trust boundary | Not coverage-guided; scheduled fuzzing remains useful | **Selected in ADR-0110** |

### React accessibility verification

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| JSX/source convention checks | Fast and simple | Cannot inspect the rendered accessibility tree | Rejected |
| Browser Playwright audit as the only per-PR gate | Real browser and interaction coverage | Heavier startup and browser lifecycle | Deferred as a complementary journey gate |
| **Testing Library + happy-dom + axe-core rendered baseline** | Deterministic, fast, visible in Vitest CI, and validates actual DOM semantics | Does not replace real-browser/assistive-technology review | **Selected in ADR-0111** |

### Locale, time-zone, and RTL display semantics

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Component-local browser-default `Intl` formatting | Minimal code | Inconsistent financial/timestamp output and hidden time-zone assumptions | Rejected |
| Large translation framework as the first localization change | Mature catalog workflows | Adds runtime/dependency surface before exact money and evidence display semantics are established | Deferred |
| JavaScript `Number` currency conversion | Convenient currency APIs | Loses precision for large atomic values | Rejected |
| **Centralized `BigInt`/`Intl` formatter module with explicit display context** | Exact, dependency-free, testable locale/time-zone/RTL contract across lazy workspaces | Translation catalogs and real-browser review remain follow-up gates | **Selected; ADR-0146** |

### Lazy workspace failure containment

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Rely on `Suspense` fallback alone | Minimal code | Does not catch rejected imports or render-time exceptions | Rejected |
| Root-level boundary for the entire SPA | Catches everything | Hides navigation and turns one workspace defect into a full-shell outage | Rejected |
| Render raw exception details | Useful for debugging | Can disclose implementation or sensitive evidence details | Rejected |
| **One bounded redacted boundary around the selected workspace** | Preserves global shell, exposes a deterministic recovery action, and avoids retry storms | Real-browser chunk/cache evidence remains separate | **Selected; ADR-0147** |

### Real-browser UI journey evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| `happy-dom` plus axe-core only | Fast and deterministic | Cannot prove browser focus, layout, responsive overflow, or code-split failure behavior | Retained as the fast baseline, rejected as the complete contract |
| Real-browser axe audit through the pinned Playwright runner | Proves rendered browser accessibility semantics at the NGINX edge | Adds browser runtime cost and still cannot certify assistive-technology quality | **Selected as a complementary automated gate in ADR-0148** |
| Host-installed browser in CI/developer machines | Low repository setup cost | Browser and OS drift weaken reproducibility and clean-checkout parity | Rejected |
| Browser binaries inside the NGINX image | One image to operate | Expands production attack surface and couples test tooling to the serving edge | Rejected |
| Unpinned browser image/download | Simple updates | Mutable binaries undermine evidence, debugging, and supply-chain review | Rejected |
| **Pinned official Playwright runner in a Docker Compose browser profile** | Real browser behavior, isolated assets, reproducible edge-network testing, and failure artifacts | Adds a heavier opt-in CI/local job; does not replace manual assistive-technology review | **Selected; ADR-0148** |

### Database pool saturation telemetry

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Query PostgreSQL for pool state | Familiar SQL observability | Consumes a scarce connection and fails during pool exhaustion | Rejected |
| Dynamic per-tenant/request labels | Detailed attribution | Unbounded cardinality and identifier leakage | Rejected |
| **Read-only deadpool status snapshot with fixed Prometheus gauges** | No DB round trip, bounded, cheap during overload | Eventually consistent snapshot | **Selected in ADR-0112** |

### Browser idempotency identity

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Require HTTPS for every local browser execution | Matches production secure-context behavior | Hides a clean-checkout HTTP failure and adds local certificate/bootstrap complexity | Rejected as the sole fix |
| Timestamp or `Math.random()` fallback | Minimal implementation | Predictable collisions and unsafe concurrent retry identity | Rejected |
| Client-generated UUIDv7 | Time ordered | Confuses client request keys with server-owned entity IDs and does not establish ordering authority | Rejected |
| **Native `randomUUID()` with CSPRNG `getRandomValues()` v4 fallback** | Secure in secure and local contexts, dependency-free, and authority-preserving | Request keys are intentionally not sortable entity IDs | **Selected; ADR-0149** |

### Evidence-volume query plans and retention

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Catalog index existence checks | Fast and simple | Does not prove planner selection at volume | Rejected |
| Force `enable_seqscan=off` | Makes index use deterministic | Manufactures evidence and hides selectivity defects | Rejected |
| Delete old rows in local drill | Exercises cleanup | Invents retention authority and can destroy replay evidence | Rejected |
| **Normal-planner EXPLAIN over bounded synthetic evidence volume plus explicit retention policy** | Auditable query evidence without unsafe deletion; clear production follow-on | Local volume is not production capacity; partition/archive work remains | **Selected in ADR-0113** |

### PostgreSQL point-in-time recovery evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Logical dump/restore only | Simple, portable, already reproducible | No arbitrary WAL target; changes after dump can be lost | Rejected as the sole recovery gate |
| Streaming replica only | Low recovery point gap and operationally familiar | Can replicate corruption/operator error; not an independent historical restore | Rejected as the sole recovery gate |
| Managed-provider PITR only | Strong production integrations and off-site durability | Provider-specific, not reproducible in a clean checkout | Required production evaluation, not the repository-only gate |
| **Physical base backup plus archived WAL with exact-LSN replay** | Proves PostgreSQL-native PITR boundary in Docker; deterministic and auditable | Local files are not encrypted, durable, or production-scale | **Selected in ADR-0114** |

### Settlement RPC provider routing

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Fixed provider order | Deterministic and simple | Repeatedly hits degraded endpoints; poor tail latency | Rejected |
| Per-request random provider | Spreads traffic without shared state | Unpredictable evidence and no failure quarantine | Rejected |
| Durable PostgreSQL/Redis health state | Shared across replicas | Adds hot-path network dependency and stale coordination | Rejected |
| Separate indexer and settlement health algorithms | Local ownership | Divergent cooldowns and operational semantics | Rejected |
| **Shared bounded process-local adaptive health** | Deterministic cooldown, bounded failover, low latency, disposable state | Requires per-process metrics and regional SLOs | **Selected in ADR-0115** |

### Settlement transaction budget

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Absolute 1,232-byte packet check only | Simple protocol boundary | No growth margin; compiler can approach the hard limit | Rejected |
| JSON/base64 envelope sizing | Easy API measurement | Does not represent validator wire serialization | Rejected |
| Live RPC sizing during compilation | Cluster-aware in theory | Adds hot-path latency and provider dependency | Rejected |
| Arbitrary extra static keys | Wallet flexibility | Wastes packet bytes and expands review surface | Rejected |
| **Deterministic legacy wire estimate plus exact static-key admission** | Pure, bounded, conservative, network-free | Compute/v0/ALT evidence remains separate | **Selected in ADR-0116** |

### Compute-unit and priority-fee policy

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| No compute-budget instructions | Smallest transaction shape | Leaves compute exhaustion and fee bidding to wallet/RPC defaults; not reproducible | Rejected |
| Client-selected compute limit and priority fee | Wallet flexibility | Client can alter execution budget and fee behavior after quote/compile; weakens replay and evidence binding | Rejected |
| RPC simulation on every signing request | Cluster-aware estimate | Adds a network round trip and provider variance to the hot path; simulation is not a settlement authority | Rejected |
| **Server-owned versioned policy serialized as exact compute-budget instructions** | Deterministic admission, explicit fee ceilings, replayable evidence, and bounded wallet behavior | Requires validator-backed calibration and policy rollout | **Selected in ADR-0162** |

### Settlement provider-health observability

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Logs only | Rich forensic context | Poor alertability; failure storms and sensitive cardinality | Rejected as sole signal |
| Endpoint URL metric labels | Easy provider identification | Sensitive topology and unstable cardinality | Rejected |
| Durable health state | Shared across replicas | Hot-path coordination and stale state | Rejected |
| **Fixed provider-index gauges from process-local snapshots** | Bounded, redacted, low overhead, alertable | Requires deployment-level provider-index mapping | **Selected in ADR-0117** |

### Settlement submission concurrency fence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Database/advisory lock held across RPC | Serializes requests | Holds database resources across external I/O and is difficult to recover | Rejected |
| In-process mutex/single-flight | Low local overhead | Does not fence replicas or survive restart | Rejected |
| Duplicate `sendTransaction` with Solana deduplication | Simple caller path | Wastes provider capacity and does not bind the response to the verified message | Rejected |
| **Durable `signed -> submitting` compare-and-set with signature binding** | Cross-replica ownership, crash visibility, no database lock across RPC, safe reconciliation | Requires an explicit lifecycle state and operational reconciliation | **Selected in ADR-0118** |
| **Explicit accepted-but-ambiguous local RPC fixture mode** | Deterministic unknown-state and signature-reconciliation evidence without production fault-injection surface | Synthetic provider behavior; does not replace live crash/failover evidence | **Selected in ADR-0119** |
| **Provider hold + project-scoped API SIGKILL + signature reconciliation** | Exercises the real durable claim/restart boundary without timing races or direct row mutation | Local single-replica evidence; requires a controlled fixture | **Selected in ADR-0120** |
| **Pinned health future selected with the bounded service loop** | Failure propagation, cancellation, one bounded task, no detached lifecycle | Couples service loop and health-server lifecycle explicitly | **Selected in ADR-0121** |
| Bare `RuleSet` JSON | Small payload, no envelope | Cannot identify document compatibility or fail closed on future representations | Rejected |
| Schema field inside `RuleSet` | Self-describing domain object | Changes rule identity/digest and couples transport schema to financial semantics | Rejected |
| **Versioned `RuleSetDocument` envelope with bounded legacy replay adapter** | Explicit new contract, stable rule digest, expand/contract migration | Temporary compatibility branch for historical rows | **Selected in ADR-0122** |


### Solana program implementation

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Rust + Anchor-compatible interfaces | Account validation, IDL, ecosystem, test tooling, review familiarity | Macro abstraction and version compatibility | **Selected**; use explicit account constraints and profile hot paths |
| Native Rust without Anchor | Maximum control and smaller abstraction surface | More boilerplate, easier account-validation omissions, client contract burden | Use selectively for specialized low-level pieces only after profiling/ADR |
| Solidity transpiler/SVM compatibility | EVM skill reuse | Non-native semantics, weaker Solana account ergonomics, audit complexity | Rejected for Solana core; Solidity remains optional EVM adapter |
| Separate EVM chain | Existing Solidity ecosystem | Cross-chain atomicity/custody/bridge risk, violates Solana-first scope | Rejected for core settlement |

### Tax calculation architecture

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Full rule graph on-chain | Maximum on-chain determinism | Compute/account/upgrade limits, expensive rule evolution, poor global-tax fit | Rejected for Tier-1 engine |
| External tax SaaS only | Broad possible coverage | Vendor lock-in, opaque calculation, network dependency, weak replay | Rejected as authority; may be an evidence adapter |
| JavaScript-only engine | Fast UI integration | Numeric/runtime drift, harder shared authoritative service semantics | Rejected as authority |
| **Pure Rust engine + signed attestation** | Deterministic, typed, replayable, low latency, shared native/WASM core | Requires rule governance and signer security | **Selected** |
| **Pure Rust domain engine plus a warm Rust service adapter** | Independent batch/simulation scaling while retaining in-process hot-path execution and one semantic authority | Requires bounded cache, service identity, and deployment topology | **Selected; see ADR-0225** |

### Service runtime

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Rust + Tokio/Axum-style services** | Low overhead, explicit concurrency, shared domain language, strong numeric safety | Higher hiring/learning curve | **Selected** |
| Node.js | Strong web ecosystem and fast iteration | Event-loop blocking risks, duplicated Rust domain semantics, numeric pitfalls | Use only for tooling if justified |
| Go | Strong concurrency and operations | Duplicated domain model and numeric semantics | Rejected for core services |
| Java/Kotlin | Mature enterprise tooling | Higher memory/latency floor and duplicated domain model | Rejected for latency-critical core |

### Frontend delivery

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **React + TypeScript + Vite SPA** | Wallet-centric client, static delivery, independent scaling, explicit user constraint | SEO/server rendering is separate work | **Selected** |
| Next.js | Full-stack and SSR | Explicitly prohibited; unnecessary server runtime | Rejected |
| Remix/server-rendered app | Good data loading model | Adds server runtime and deployment coupling | Rejected for core UI |
| Micro-frontends | Team autonomy | Runtime/version complexity and inconsistent UX | Rejected until organizational scale proves need |

### Persistence and async work

| Area | Selected | Alternatives rejected | Reason |
| --- | --- | --- | --- |
| Durable projections/evidence | PostgreSQL 18.x with native `uuid`/`uuidv7()` | PostgreSQL 17 or older, SQLite, document-only stores | Current supported major, native time-ordered UUID generation, transactions, replay queries, relational audit model |
| PostgreSQL recovery evidence | Docker-isolated `pg_dump`/`pg_restore` drill plus production PITR | Filesystem snapshot only, application row copier | Portable logical restore validates the schema/evidence contract; production still requires encrypted off-site backups and PITR |
| Tenant settlement account configuration | PostgreSQL tenant-scoped immutable route registry plus deterministic payer ATA derivation | Caller-supplied account tuples, RPC discovery in compile path, environment-only route | Removes account substitution while keeping the compile path bounded and avoiding RPC/ATA creation in the settlement hot path |
| Route activation verification | Dedicated bounded verifier with finalized `getMultipleAccounts`, provider failover, and append-only evidence | Verify on checkout, trust database rows, single-provider confirmed reads, on-chain registry reads | Keeps RPC and account parsing off the hot path while proving account topology at finalized commitment before activation |
| Disposable cache/leases | Redis | Memcached, cache-only in-process | Shared low-latency cache with explicit non-authority |
| Async delivery | Outbox + NATS JetStream when required | Kafka by default, direct fan-out | Keep broker off hot path; reduce operational weight while preserving replayable async work |
| Chain truth | Solana finalized state + receipt evidence | API database, WebSocket notification alone | Correctness and auditability |

### Merchant authentication and authorization

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **OIDC-compatible EdDSA JWT validated against a configured JWKS key set** | Standard browser/service integration, offline verification on the quote hot path, key rotation by `kid`, no introspection round-trip | Requires issuer/audience governance and a safe key-distribution/reload process | **Selected for the first protected API slice**; fail closed when issuer, audience, keys, signature, or merchant scope are invalid |
| Opaque bearer tokens with introspection | Immediate revocation and central policy | Network dependency and tail latency in the money path; availability coupling | Rejected for synchronous quote authorization; may be used at an edge gateway with bounded caching |
| Mutual TLS only | Strong workload identity and no bearer replay | Poor browser UX; does not express end-user merchant scope | Rejected as the sole product authentication mechanism; retain for service-to-service deployment profiles |
| Caller-supplied merchant identifiers | Simple and low latency | No identity proof; cross-merchant data and idempotency risk | Rejected for any persisted quote or settlement operation |

### Tax-rule ownership and governance isolation

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Global rule catalog with API-only tenant predicates | Shared releases and simple catalog administration | One missed predicate permits cross-tenant policy disclosure or substitution; the database cannot enforce the boundary | Rejected |
| Nullable owner with a global fallback | Low migration friction | Ambiguous authority and accidental global access; fail-closed behavior is not enforceable | Rejected |
| Separate database/schema per tenant | Strong physical isolation | Operationally expensive and poor fit for bounded shared services | Rejected for the first boundary |
| **Non-null merchant-owned rule releases with composite indexed predicates and constraints** | Explicit authority, bounded lookup, database-enforced calculation association, duplicate release keys per tenant | Requires reviewed ownership backfill and tenant-aware provisioning | **Selected in ADR-0079** |

### SBF platform-tools acquisition

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Default cargo-build-sbf downloader only | Minimal upstream-aligned path | API/asset restrictions can block a pinned build even when the release asset is reachable | Rejected as the only path |
| Floating latest compiler tools | Convenient | Non-reproducible bytes and unreviewed compiler changes | Rejected |
| Check the compiler archive into Git | Offline operation | Excessive repository size and maintenance burden | Rejected |
| **Pinned browser asset, SHA-256 verification, ignored reusable cache** | Reproducible and auditable; supports local archive/mirror recovery without weakening trust | Initial large download and scheduled digest review | **Selected in ADR-0080** |

### On-chain policy-attestation verification

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Governed, cached PolicyAttestation PDA checked in O(1) account reads** | Small settlement instruction, bounded compute, exact amount binding, easy revocation and expiry checks | Registration is a separate governed transaction; freshness depends on bounded expiry | **Selected for the first production settlement boundary** |
| Fresh Ed25519 verification instruction on every settlement | Strongest per-transaction freshness and no cached policy account | More instruction/account bytes, compute and transaction-size pressure, signer-key registry complexity | Retain as a later high-assurance path when measured budgets and issuer registry are complete |
| API-only signature verification | Lowest on-chain cost | Solana cannot independently reject API substitution or stale/replayed policy data | Rejected as settlement authority |
| Recompute the global tax graph on-chain | Maximum local computation | Unbounded rule evolution, compute/account limits, poor global-tax fit | Rejected; the program verifies bounded attested outputs |

### On-chain issuer governance

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Merchant/mint-scoped issuer-registry PDA with separate governance authority** | O(1) registration authorization, explicit issuer rotation, bounded settlement setup, auditable state | Governance rotation is a separate transaction and must be protected by multisig/timelock outside the program | **Selected for the first governed issuer slice** |
| Arbitrary signer supplied with each attestation | Minimal account surface and implementation effort | Any signer could authorize a policy; no durable authority boundary or rotation audit | Rejected; violates issuer and governance separation |
| Global issuer allow-list PDA | Centralized lookup and simpler administration | Cross-merchant contention, broad blast radius, unclear jurisdiction/asset scope | Rejected for the hot path; may be an upper governance layer |
| Fresh signature-only verification with no registry | Strong cryptographic proof per transaction | No durable issuer activation/revocation or scoped key lifecycle; larger instruction path | Rejected as the sole authority; retain as a later complementary verification mode |

### Token program compatibility

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Anchor token interface with a strict extension-free account contract** | One audited transfer path for legacy SPL and Token-2022, bounded account model, low client branching | Token-2022 extensions require later compatibility contracts; unsupported extensions are unavailable | **Selected for the first compatibility slice** |
| Separate legacy-SPL and Token-2022 instructions | Explicit per-program behavior | Duplicated authority/conservation logic and larger audit surface; clients can select the wrong path | Rejected for the first slice |
| Accept all Token-2022 extensions through generic CPI | Broad nominal asset coverage | Transfer fees/hooks/delegates/confidential semantics can invalidate amount conservation and authority assumptions | Rejected; fail closed |
| Legacy SPL only | Smallest current implementation | Cannot make a truthful Token-2022 compatibility claim | Rejected as the target architecture; retained only as the initial historical slice |

### Document quote latency evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Reuse aggregate quote measurements | No new harness work | Hides document payload, rule-scope, storage, and outbox differences | Rejected |
| Copy the aggregate benchmark into a second script | Isolated document workload | Duplicates timeout, credential, percentile, cleanup, and invariant logic | Rejected |
| Mixed aggregate/document workload | Capacity mix is closer to a fleet | Per-context regressions and durable attribution become ambiguous | Rejected as the primary gate |
| **Explicit document mode in the existing bounded Docker harness** | DRY, reproducible, and context-specific with shared safety gates | Adds one bounded CI workload | **Selected in ADR-0209** |
| External load tool or host-installed binary | Familiar ecosystem | Violates the Docker bootstrap contract and introduces toolchain drift | Rejected |

### Docker full-topology bootstrap

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Default `docker compose up` | Short command | Omits required profiles and has no explicit full-topology readiness contract | Rejected |
| Enable every Compose profile | Starts all declared services | Also launches one-shot conformance and browser-test jobs | Rejected |
| Host supervisor with custom polling | Flexible per-service control | Duplicates health semantics and depends on host tooling | Rejected |
| **Explicit long-running services with Compose `--wait`** | Pinned Docker boundary, health-derived readiness, no host toolchain | Starts more local containers and requires validator capabilities | **Selected in ADR-0210** |

### Docker host-port collision policy

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Fixed host ports | Simple copy/paste URLs | Collides with unrelated stacks even when Compose projects are isolated | Rejected |
| Random host-port allocation | Avoids most collisions | Breaks reproducible URLs and makes browser/wallet/test configuration unstable | Rejected |
| Host networking | Fewer published mappings | Removes isolation and broadens local security exposure | Rejected |
| **Explicit environment-backed host-port overrides** | Stable defaults, parallel topologies, unchanged internal contracts | Operators must choose and record a complete alternate map | **Selected in ADR-0211** |

### Settlement conformance evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Pure helper/unit tests | Fast and deterministic | Cannot prove CPI, runtime account ownership, deployed serialization, or token deltas | Rejected as the only gate |
| In-process `solana-program-test` only | Fast native transaction tests | Does not exercise the pinned Dockerized Agave validator and external RPC boundary | Rejected as release evidence; may complement the suite later |
| **Pinned external validator plus TypeScript transaction client** | Exercises the deployed SBF artifact, exact client envelope, legacy SPL, Token-2022, PDA initialization, and RPC confirmation | Slower and requires a pinned SBF build with external tool downloads | **Selected** for localnet release evidence |
| Mocked token programs or fabricated account data | Easy to run | Can produce false compatibility claims | Rejected |

The conformance client is run in the dedicated pinned Docker image defined by
ADR-0168. Host-installed Node.js and pnpm were rejected because they violate
the clean-checkout Docker contract and can diverge from the locked CI runtime.
The Playwright browser image was also rejected because chain conformance does
not need a browser and should keep its runtime surface minimal.

### Refund-proof operational observability

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Logs only | No code or storage query | Not restart-safe, difficult to alert, and poor for backlog detection | Rejected |
| Per-claim/signature metric labels | Easy drill-down | Unbounded cardinality, privacy leakage, and memory/alerting hazards | Rejected |
| Redis-backed worker counters | Shared across replicas | Adds a non-authoritative dependency and can reset or diverge from durable evidence | Rejected |
| **PostgreSQL pending snapshot plus fixed operation metrics** | Durable backlog truth, bounded labels, low overhead, compatible with existing scrape contract | One bounded aggregate query and process-local request counters | **Selected; see ADR-0159** |

### Chain indexing and reconciliation

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Finalized RPC subscription signal plus bounded slot-range backfill** | Works with standard Solana RPC, repairs missed notifications, explicit commitment semantics, straightforward local testing | Requires checkpoint persistence, provider failover, and bounded backfill queues | **Selected for the first indexer slice** |

Provider selection uses a bounded process-local health score layered over the
finalized-page port. Static order and round-robin were rejected for degraded
provider tail latency; a remote health registry was rejected for adding a hot
path dependency. The selected pool has at most three providers, per-attempt
deadlines, saturating atomic scores, and capped cooldowns. It only reorders
attempts and never weakens Solana commitment or coordinator validation.
| WebSocket notifications as the source of truth | Low notification latency and simple happy path | Notifications can be dropped, duplicated, reordered, or arrive before the required commitment | Rejected as authority; use only as an acceleration signal |
| Geyser/plugin-only ingestion | High throughput and rich slot data | Operational and provider coupling; heavier local/deployment footprint | Rejected for the first portable boundary; retain as a scale adapter |
| Database/API status as settlement truth | Convenient product reads | Cannot prove finalized token movement or repair a missed chain event | Rejected; finalized Solana state is authoritative |

The selected application boundary validates provider pages before persistence:
serializable storage receives only the exact planned range, a complete page,
and a finalized watermark. The current adapter uses bounded HTTPS JSON-RPC and
at most three deadline-bounded providers; it preserves finalized commitment
across failover. In-memory cursors lose progress on crash, broker-only offsets
do not atomically commit projections, and overwrite upserts would hide
conflicting provider evidence.

The selected persistence adapter uses serializable PostgreSQL transactions with
checkpoint fencing and exact receipt replay. In-memory cursors lose progress on
crash, broker-only offsets do not atomically commit projections, and overwrite
upserts would hide conflicting provider evidence.

### API authorization claims

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **JWT `scope`/`roles` claims enforced at each service boundary** | Offline low-latency authorization, explicit least privilege, auditable policy, no extra request hop | Claim issuance and revocation policy remain identity-provider responsibilities | **Selected** |
| Authentication-only merchant claim | Smallest implementation | Any authenticated principal could invoke every merchant operation | Rejected; identity is not authorization |
| Remote authorization introspection on every request | Immediate central policy changes | Adds network tail latency and availability coupling to the quote path | Rejected for the synchronous path; may be used for bounded control-plane workflows |
| API gateway-only authorization | Centralized edge policy | Internal routes can be called directly unless every service repeats policy | Rejected as sole enforcement; gateway policy may be defense in depth |

### Product UI data boundary

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Authenticated production quote workspace with isolated preview mode** | Real API contract is exercised, evidence states are honest, no client-side money authority, useful operator workflow | Wallet signing and settlement require a later audited adapter | **Selected** |
| Client-side tax calculation as checkout authority | Instant feedback and offline behavior | Numeric/rule drift, no published-rule evidence, easy to mistake preview for authority | Rejected as production checkout authority; retain only as visibly labeled preview |
| Static dashboard with fabricated live states | Easy demonstration | Misleading money/settlement status and no recoverable failure semantics | Rejected |
| Server-rendered/full-stack UI | Integrated data loading | Violates React/Vite SPA and NGINX-only delivery constraint | Rejected |

### Durable asynchronous publication

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Transactional PostgreSQL outbox** | Calculation/evidence and event commit atomically; replayable; no broker dependency in the quote path | Requires a bounded dispatcher and cleanup/retention policy | **Selected for the first durable event boundary** |
| Publish to a broker before database commit | Fast fan-out and broad integration options | Lost or duplicated events around partial commit; broker becomes part of the money path | Rejected |
| Direct webhook/event fan-out in the request | Simple consumer integration | Unbounded latency, retry storms, and external availability coupling | Rejected |

### Outbox delivery transport

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **NATS JetStream with PostgreSQL lease claims** | Low latency, durable acknowledgements, subject-based fan-out, message-id deduplication, bounded operational footprint | At-least-once delivery still requires idempotent consumers and broker operations | **Selected**; PostgreSQL remains the durable handoff and JetStream is downstream transport |
| Kafka | Mature partitioning and ecosystem | Higher local/operational cost than the first workload requires; more latency and resource overhead | Rejected for the first transport boundary; revisit at measured scale |
| Redis Streams | Familiar and fast | Cache/coordination semantics can be confused with evidence authority; weaker separation of concerns | Rejected as the durable event transport |
| Direct PostgreSQL polling by every consumer | Minimal dependencies | Consumer contention, duplicated query logic, and no shared delivery contract | Rejected |

### Local development and packaging

The workspace uses Rust 1.98.1 for off-chain services, while the selected
Agave 4.3.0 / platform-tools v1.57 SBF compiler requires the on-chain package
to declare a Rust 1.95 compatibility floor. This split is recorded in
[ADR-0067](../adr/0067-sbf-compiler-compatibility-boundary.md); it is not an
invitation to bypass Cargo's version checks.

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Docker Compose with pinned stable images/digests** | Reproducible local topology, low onboarding cost, production-like boundaries | Desktop resource use, Compose is not production orchestration | **Selected for local development** |
| Kubernetes locally | Production-like orchestration | High complexity and slow feedback | Rejected for default local workflow |
| Host-installed toolchains | Fast for experts | Drift, onboarding failure, inconsistent versions | Rejected as default |
| Floating `latest` image tags | Easy updates | Non-reproducible and unsafe | Rejected; resolve latest stable then pin digest |

### Local cache and observability topology

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Profiled Redis + Prometheus + Grafana + OpenTelemetry Collector** | Explicit failure domains, bounded metrics, reviewable dashboards, Docker-only reproducibility | Uses local CPU/memory and requires image maintenance | **Selected for local development** |
| Host-installed monitoring tools | Familiar to operators | Violates clean-checkout and toolchain reproducibility requirements | Rejected |
| One all-in-one monitoring image | Fewer containers | Blurred ownership, larger blast radius, opaque upgrades | Rejected |
| Remote telemetry in the request path | Centralized production data | Adds latency and availability coupling to money paths | Rejected |

### Edge admission control

| Alternative | Strengths | Risks/trade-offs | Decision |
|---|---|---|---|
| **NGINX bounded per-source-IP rate and connection limits** | Rejects overload before API/database work, deterministic local behavior, no hot-path dependency | NAT concentration and production tuning require measured limits | **Selected as defense in depth; ADR-0074** |
| API-only limits | Tenant-aware and easy to evolve | Still consumes edge sockets and request parsing capacity | Rejected as the sole control |
| Redis-backed distributed edge quotas | Shared limits across replicas | Adds network latency and a failure dependency to every request | Deferred to authenticated tenant quotas |
| Cloud WAF/CDN-only controls | Managed global perimeter | Vendor coupling and no protection for direct/self-hosted edge access | Rejected as the only control |

### Tax-rule governance workflow

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Authenticated API transition plus serializable PostgreSQL event transaction** | Explicit state machine, four-eyes separation, durable evidence, bounded synchronous control path | Requires identity-provider role lifecycle and operator ceremonies | **Selected; ADR-0076** |
| Database-only status mutation | Simple and durable | Cannot prove who authored, reviewed, or approved a release | Rejected |
| Workflow broker as authority | Rich orchestration and notifications | Adds asynchronous divergence and delivery dependency to publication correctness | Rejected as authority; may be used for notifications |
| Single administrator promotion role | Operationally simple | Violates separation of duties and increases blast radius | Rejected |

### Tax-rule governance read model and UI

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Authenticated repeatable-read PostgreSQL snapshot plus React operator workspace** | Current status and immutable event history are shown from one source, bounded control-plane latency, no projection lag | Requires explicit timeout/conflict/error UX and read-role provisioning | **Selected; ADR-0077** |

### Service-level latency evidence

| Option | Benefits | Costs / risks | Decision |
| --- | --- | --- | --- |
| Domain-only Criterion benchmark | Very fast feedback and stable hot-path signal | Omits HTTP, authentication, PostgreSQL contention, serialization, and outbox work | Rejected as the only latency evidence |
| Unpinned external load binary | Rich load-generation features | Toolchain drift and weaker clean-checkout reproducibility | Rejected |
| Unbounded request flood | Easy to write | Measures overload, creates retry storms, and cannot establish a declared budget | Rejected |
| **Pinned Docker stack with bounded `curl` workers** | Reuses required tooling, isolates state, records every response, and is easy to audit | Less feature-rich than a dedicated load platform; production regional evidence remains separate | **Selected; ADR-0078** |
| Reuse the quote workload for persisted-rule simulation | Avoids another script | Mixes read-only simulation with quote persistence, idempotency, outbox, and jurisdiction-provider behavior | Rejected; ADR-0145 |
| **Pinned two-replica simulation workload with facts-only response invariants** | Measures the governance read path, warm compiled-rule cache, pool pressure, and server-owned binding without financial writes | Local Docker latency is not regional production evidence | **Selected; ADR-0145** |
| Client-only lifecycle state | Fast local interaction | Stale or fabricated state can mislead operators and cannot prove evidence | Rejected |
| Separate event-store projection as immediate authority | Scales independent reads | Projection lag and failure can misrepresent publication state | Rejected as authority; may be added for analytics |
| Browser-decoded JWT role gating | Convenient affordances | Client claims are not an authorization boundary | Rejected |

### Emergency settlement control

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Singleton on-chain pause PDA checked before settlement CPI** | Directly protects wallet transactions, deterministic, low overhead, auditable | Requires governance signer custody and an explicit initialization transaction | **Selected** |
| API or NGINX kill switch | Fast to operate off-chain | Does not stop direct wallet transactions or already-built transactions | Rejected |
| Per-policy or per-merchant pause only | Fine-grained control | Cannot stop a shared program/token integration incident immediately | Rejected as the sole control |
| Program upgrade to disable settlement | Powerful | Slower, broader privilege, and harder to audit as an incident action | Rejected as the first response |

### Pause evidence and alerting

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Logs-only alerting | Minimal implementation | Not restart-safe, difficult to replay, and weak under provider failover | Rejected |
| API/NGINX kill switch | Fast off-chain admission control | Cannot stop direct wallet transactions and can misrepresent chain state | Rejected as authority |
| Database-owned pause flag | Simple operator read | Makes a projection a second settlement authority | Rejected |
| **Finalized `ProtocolPauseChanged` projection with fixed-cardinality metrics** | Replayable, restart-safe, auditable, bounded alert labels, subordinate to chain truth | Requires event decoding, migration, and independent staging verification | **Selected; ADR-0131** |

### Settlement instruction compilation

| Alternative | Benefit | Risk/cost | Decision |
|---|---|---|---|
| **Pure Rust compiler with derived PDAs and fixed account order** | Deterministic bytes, bounded validation, wallet review, no custody or RPC dependency | Transaction wrapping and blockhash policy remain separate | **Selected** |
| Browser-constructed arbitrary account metas | Fewer API fields | Account substitution and duplicated serialization logic | Rejected |
| API-held payer signing key | Operationally simple submission | Custody, key compromise, and loss of user authorization | Rejected |
| RPC-coupled compiler | Can attach a fresh blockhash immediately | Network latency and provider availability enter deterministic domain code | Rejected |

### Cross-language settlement serialization

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Shared versioned JSON golden vectors consumed by Rust and TypeScript** | One reviewable contract for program ID, bytes, PDAs, account order, and flags; catches serializer drift | Requires deliberate vector versioning and synthetic fixture maintenance | **Selected** |
| Rust-only compiler tests | Smallest test surface | Does not prove browser wallet construction preserves the artifact | Rejected |
| TypeScript-only snapshots | Convenient frontend feedback | Makes the browser an accidental protocol authority | Rejected |
| Shared Rust/WASM compiler in the browser | Reduces duplicate derivation logic | Larger bundle and runtime coupling; requires measured browser performance | Deferred pending evidence |

### Threshold and exemption semantics

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Explicit threshold basis/mode plus signed cumulative snapshot input** | Deterministic, replayable, supports excess-only and trigger semantics, keeps the hot path pure | Requires upstream evidence ownership and legal review of each rule | **Selected** |
| Read cumulative state from PostgreSQL during calculation | Convenient integration | Race-prone, nondeterministic, and adds a database dependency to the domain path | Rejected |
| Caller-provided exemption boolean | Simple request shape | Unverifiable and not suitable for audit or attestation | Rejected |
| Model thresholds as credits | Reuses an existing field | Changes tax output rather than the taxable base and cannot express activation semantics | Rejected |

### Tax rounding scope and residual allocation

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Implicit component rounding | Backward-compatible by omission | Cannot express aggregate settlement rounding and hides the legal calculation scope | Rejected |
| Mixed component and settlement scopes | Flexible authoring | Dependent components have ambiguous rounded-versus-exact inputs | Rejected |
| Total rounding with residual discard | Minimal arithmetic | Breaks conservation and creates unexplained atomic-unit loss | Rejected |
| **Explicit component/settlement scope with bounded largest-remainder allocation** | Deterministic, replayable, conserving, and compatible with tax-on-tax component graphs | Settlement scope is restricted to independent components | **Selected in ADR-0202** |
| Add line/invoice facts in the rounding change | Broader coverage | Expands the fact, API, storage, UI, and golden-vector contracts beyond one bounded decision | Deferred to a dedicated line-facts ADR |

### Tax decimal-scale representation

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Binary floating-point decimal places | Familiar | Unsafe financial boundaries and cross-runtime drift | Rejected |
| Adapter-specific decimal strings | Human-readable | Parser and currency-precision divergence | Rejected |
| **Positive `quantum_atomic` integer** | Exact, asset-precision aware, language-neutral, and bounded | Rule authors must declare the asset's atomic increment | **Selected in ADR-0202** |

Quantum compatibility is also explicit: floor and cap boundaries that are not
exact multiples of the declared quantum are rejected, preserving the meaning
of min/max semantics after final rounding.

### Line and invoice tax facts

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Reuse aggregate `TaxFacts` for invoices | Minimal schema | Loses line identity and makes rounding/replay ambiguous | Rejected |
| Browser-owned line aggregation | Fast preview | Caller-controlled evidence and cross-language numeric drift | Rejected |
| **Versioned bounded document facts with explicit line/invoice scope** | Canonical identity, bounded work, conserving allocation, replayable evidence | Adds a separate domain/result contract | **Selected in ADR-0203** |

### Production document quote persistence

| Option | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Reuse aggregate `tax_calculations` | Small migration | Shape confusion and unsafe settlement/attestation reuse | Rejected |
| Browser/object-storage persistence | Flexible and responsive | Caller-controlled authority and missing transactional evidence | Rejected |
| **Dedicated PostgreSQL document-calculation adapter** | Explicit type firewall, tenant scope, idempotency, encrypted replay, outbox | Additional schema and API surface | **Selected in ADR-0204** |
| Unbounded generic line rule graph | Broad expressiveness | Unbounded latency/memory and difficult audit/attestation limits | Rejected |

### Document-calculation attestation

| Option | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Reuse aggregate `CalculationAttestation` | No new signer contract | Cannot distinguish document and aggregate authority; unsafe settlement reuse | Rejected |
| Add document fields to aggregate schema v1 | One wire type | Breaks existing signed payloads and verifier compatibility | Rejected |
| Signed ad-hoc JSON envelope | Flexible | Ambiguous canonicalization and domain separation | Rejected |
| **Dedicated policy contract, persistence table, API route, and outbox event** | Explicit scope firewall and independent replay | Additional audited contract surface | **Selected in ADR-0205** |

### Calculation-attestation cross-language compatibility

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Rust-only canonical-payload tests | Minimal implementation | TypeScript serialization drift can change the authority-bound digest unnoticed | Rejected |
| JSON serialization comparison | Easy to inspect | JSON ordering, UTF-16 text lengths, and numeric coercion are not the signed byte contract | Rejected |
| Browser-generated or browser-signed attestations | Low apparent interaction latency | Makes an untrusted client an issuer authority and permits evidence substitution | Rejected |
| **Versioned binary golden vector consumed by Rust and TypeScript** | Exact byte/digest parity, deterministic review, no network dependency | Requires explicit fixture maintenance and mutation tests | **Selected; see ADR-0224** |

### Document-to-payment binding

| Option | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Reuse aggregate `settle` with document totals | Smallest code change | Loses document provenance on chain and permits aggregate/document coercion | Rejected |
| Put document identity only inside the policy digest | No program instruction change | Chain evidence cannot independently distinguish source and payment scope | Rejected |
| Prorate arbitrary partial payments from stored totals | Flexible checkout behavior | Incorrect for progressive, thresholded, cascading, capped, credited, or rounded rules | Rejected |
| Put document lines/rule graph on Solana | On-chain visibility | Unbounded compute/account/packet cost, privacy leakage, and poor rule evolution | Rejected |
| **Dedicated domain plus `settle_document` and document receipt** | Type firewall, deterministic whole-line semantics, on-chain provenance | Requires a separately audited instruction and event | **Selected in ADR-0206** |

## Re-evaluation triggers

Re-open these decisions when:

- measured p99 breaches the documented budget after profiling;
- the rule graph exceeds bounded compute/memory limits;
- transaction account/compute limits require a new batching model;
- legal/tax governance requires a new evidence or privacy model;
- service volume or team topology justifies a different event-stream architecture;
- a selected dependency loses security support or Solana compatibility.

## Identifier policy

UUIDv7 is selected for internal sortable IDs because it provides distributed uniqueness with time-ordered locality. It is not used as a security secret or as a replacement for domain ordering. Strict ordering uses an explicit sequence/slot/offset, and cryptographic evidence uses a content digest. Solana public keys and transaction signatures remain native strings/bytes with their native validation.

## Compiled tax-rule cache

The quote adapter uses a bounded process-local LRU keyed by the canonical
32-byte rule digest. The database still decides whether a rule is published
and effective; the cache only stores the immutable compiled graph after digest
verification. An unbounded map, a rule-ID-only key, and Redis on the synchronous
quote path were rejected for memory, integrity, and tail-latency reasons.

### Browser security policy

| Alternative | Benefit | Risk/cost | Decision |
| --- | --- | --- | --- |
| **Strict same-origin CSP and security headers at NGINX, with Docker integration assertions** | Protects static and API responses at the first boundary and keeps policy deterministic | New wallet/telemetry origins require explicit review | **Selected; see ADR-0081** |
| React document meta tags only | Simple frontend ownership | Does not cover error/API responses and can be removed by a future document change | Rejected |
| Per-service application middleware | Flexible | Header drift and incomplete coverage across static assets and services | Rejected |
| Wildcard CORS or permissive CSP | Broad integration compatibility | Increases data-exfiltration and injection blast radius | Rejected |

### Request correlation authority

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **API-owned UUIDv7 middleware** | Covers direct, edge, auth-failure, and handler-failure paths; sortable support IDs; one implementation boundary | Small per-request UUID allocation; downstream services must propagate explicitly | **Selected; see ADR-0082** |
| NGINX `$request_id` as the system ID | Available at the first edge | Non-UUIDv7, absent on direct API traffic, and not authoritative to Rust services | Rejected |
| Caller-supplied `x-request-id` | Useful for client correlation | Spoofable and can introduce unbounded or invalid log values | Rejected |
| UUIDv4 framework helper | Mature middleware integration | Violates the repository’s UUIDv7 correlation policy | Rejected |

### Rust trace export

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Shared `tracing` + bounded OTLP gRPC exporter** | Uses existing spans, preserves JSON logs, one redaction/configuration boundary, collector portability | Adds pinned OpenTelemetry dependencies and a lifecycle boundary | **Selected; see ADR-0201** |
| Prometheus-only metrics and JSON logs | Lowest dependency cost | No distributed span context or collector trace evidence | Rejected for Tier-1 operations |
| Synchronous exporter in request paths | Immediate delivery | Couples business latency and availability to telemetry | Rejected |
| Per-service custom exporters | Local control | Configuration and privacy drift across services | Rejected |

### UUIDv7 schema evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Catalog-level SQL contract executed in clean-stack and restore drills** | Verifies final PostgreSQL defaults, native types, and version checks; catches migration drift | Requires maintaining an explicit generated-ID inventory | **Selected; see ADR-0083** |
| Static migration-file inspection | Easy to run | Does not prove the final catalog or restored database | Rejected |
| Application-only UUID tests | Covers one generator path | Direct SQL and database defaults can drift independently | Rejected |
| UUIDv7 for every UUID-shaped value | Superficially uniform | Misclassifies secrets, hashes, external IDs, and Solana-native identifiers | Rejected |

### Alert evaluation and validation

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Dashboard-only visibility | Low setup cost | Human detection, no paging contract, poor failure detection | Rejected |
| Service-local alert delivery | Close to the failure | Duplicated policy, hot-path coupling, notification side effects in services | Rejected |
| Managed-provider-only rules | Managed operations | Not reproducible or portable from the public repository | Rejected |
| **Versioned Prometheus rules validated by pinned `promtool`** | Reviewable, portable, bounded labels, CI-verifiable, low overhead | Production receiver wiring remains environment-specific | **Selected; see ADR-0084** |

### Validator conformance evidence

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| CI console output only | No extra files | Weak forensic context and no stable artifact hash bundle | Rejected |
| Upload the entire workspace/ledger | Maximum context | Excessive size, generated state, and secret exposure risk | Rejected |
| Boolean pass/fail status only | Minimal | Cannot diagnose validator/runtime failures or audit exact inputs | Rejected |
| **Bounded public-safe evidence bundle retained on success and failure** | Exact metadata, program hash, logs, test output, and reproducible review | Requires retention policy and careful redaction | **Selected; see ADR-0085** |
| Trust the conformance process exit code | Minimal CI configuration | Does not prove Linux capability, pinned runtime, artifact identity, or complete case coverage | Rejected |
| Privileged Docker workaround for constrained kernels | May start Agave in more environments | Expands privilege and changes the evidence environment | Rejected |
| **Bounded postcondition verifier on Linux CI/release** | Preserves local fail-closed diagnostics and makes runtime/artifact/test evidence reviewable | Adds an explicit release gate | **Selected; see ADR-0199** |

### Replay-gap operator control

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Manual SQL | Immediate access | Unauthenticated mutation, no bounded audit trail, easy to confuse repair with acknowledgement | Rejected |
| Merchant-scoped API | Reuses existing tenant auth | Replay streams are system-level; merchant claims are not a safe operator boundary | Rejected |
| Acknowledge by marking completed | Simple state model | Falsely represents finalized projection repair | Rejected |
| **System operator role/scope plus append-only action ledger** | Explicit authority, atomic requeue, auditable and replayable | Requires operator identity governance and runbook | **Selected; see ADR-0086** |

### Replay-gap operator UI

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| SQL-only operations runbook | No frontend work | Poor evidence context and bypass risk | Rejected |
| Generic JSON/admin console | Fast to expose endpoints | Unbounded input and unsafe rendering of unvalidated state | Rejected |
| Cached always-on dashboard | Convenient monitoring | Stale state can imply repair or finality | Rejected |
| **Lazy-loaded typed React workspace using the authenticated API** | Bounded input, explicit failures, accessible review flow, no local authority | Requires a dedicated parser and action-state model | **Selected; see ADR-0087** |

### Indexer projection rebuild

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Rebuild from normalized projections or a database backup | Simple operational path | Cannot independently detect projection corruption | Rejected |
| Re-query live RPC during recovery | Fresh provider data | Provider availability, finality timing, and provider-specific encoding become recovery dependencies | Rejected |
| SQL-only decoding of Anchor bytes | Avoids a service binary | Duplicates protocol semantics and weakens typed validation | Rejected |
| One unbounded transaction/export | Easy implementation | Lock, timeout, and memory blast radius at scale | Rejected |
| **Rust canonical decoder with keyset-paginated bounded batches** | Reuses event invariants, supports clean rebuilds, and limits resource usage | Requires an operational drill and projection-specific contracts | **Selected; see ADR-0088** |

### Split-settlement evidence projection

| Alternative | Advantages | Risks | Decision |
| --- | --- | --- | --- |
| Treat `SplitSettlementExecuted` as a generic receipt | Minimal schema change | Loses split-plan version/digest and weakens replay and reconciliation evidence | Rejected |
| Infer split recipients from transaction account metas | No additional event decoder | Account order is not a durable executed-state contract; cannot prove recipient amounts | Rejected |
| Retain only raw split event bytes | Strong forensic preservation | Makes bounded operational queries and support workflows need repeated decoding | Rejected |
| **Typed split projection plus immutable raw evidence** | Preserves event identity, bounded queries, idempotent replay, and recovery | Requires a migration and a second typed merge path | **Selected; ADR-0093** |

### PostgreSQL contention evidence

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Unit-test timeout constants only | Fast and deterministic | Does not exercise PostgreSQL lock cancellation, SQLSTATEs, migrations, or indexes | Rejected |
| Full quote load only | Covers HTTP, idempotency, and connection use | Does not create a deterministic lock or statement-timeout scenario | Rejected |
| Unpinned `pgbench` workload | Broad throughput signal | Not a stable contract for exact schema and lock invariants | Rejected |
| **Pinned Docker PostgreSQL with targeted SQL contention and catalog assertions** | Reproducible, bounded, and auditable | Production saturation and regional evidence remain separate | **Selected; see ADR-0089** |

### Idempotency contention evidence

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Independent idempotency keys only | Simple and parallel | Misses the shared-row serialization/replay boundary | Rejected |
| Unit-test the idempotency helper | Fast | Does not exercise PostgreSQL locking, retries, or the outbox under HTTP concurrency | Rejected |
| Unbounded stress test | Broad pressure | Noisy CI and weak deterministic correctness signal | Rejected |
| **Bounded same-key Docker workload with database convergence assertions** | Real API/database path, finite, and auditable | Separate production-scale contention evidence is still needed | **Selected; see ADR-0090** |

### Revenue split execution

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Keep one destination and distribute off-chain | Smallest instruction | Loses atomic conservation and introduces custody/credit risk | Rejected |
| Arbitrary recipient array supplied by the caller | Flexible | Unbounded fan-out and recipient substitution risk | Rejected |
| Asynchronous escrow per recipient | Isolated recipient failure | More state, fees, latency, and recovery paths | Rejected |
| Replace the existing v1 instruction | One contract | Breaks signed-message and deployed-client compatibility | Rejected |
| **Immutable bounded Split Plan PDA with additive split instruction** | Atomic, deterministic, auditable, bounded, and backward compatible | Requires a new plan lifecycle and conformance evidence | **Selected; see ADR-0091** |

### Authenticated Split Plan resolution

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Caller-provided recipient list | Minimal API work | Destination substitution and no governed evidence | Rejected |
| Redis-only plan storage | Low-latency reads | Disposable and not authoritative or auditable | Rejected |
| On-chain-only plan lookup | Strong chain authority | RPC in quote/compile hot path and poor tenant query ergonomics | Rejected |
| Route table owns plan fields | One lookup | Couples account topology and plan versioning | Rejected |
| **Tenant-scoped PostgreSQL registry plus on-chain PDA verification** | Auditable, cacheable, quote-bound, and chain-authoritative | Requires immutable lifecycle and activation evidence | **Selected; see ADR-0092** |

### Governed Split Plan checkout review

| Alternative | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Accept browser recipient JSON | Flexible client UX | Destination substitution and no governed evidence | Rejected |
| Fetch a plan by opaque ID only | Small API surface | No discoverable review, encouraging guessed IDs | Rejected |
| Redis-only plan read model | Low-latency reads | Disposable cache cannot validate canonical evidence | Rejected |
| **Authenticated bounded PostgreSQL list plus ID-bound compiler** | Tenant-scoped, canonical, reviewable, and reuses chain-bound compiler | Adds a bounded read endpoint and explicit UI state machine | **Selected; ADR-0094** |

### Browser API deadline and cancellation policy

| Alternative | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| No browser deadline | No timeout configuration | Indefinite pending UI, stale authorization context, repeated user actions | Rejected |
| Ad-hoc `AbortSignal.timeout` at each call site | Small local changes | Duplicated policy and inconsistent cancellation/testing | Rejected |
| Browser retries for all failures | May mask transient reads | Can duplicate settlement side effects and hide provider ambiguity | Rejected |
| **Shared bounded fetch boundary with explicit read/submit budgets** | One tested cancellation contract with endpoint-specific risk budgets | Requires all call sites to use the boundary | **Selected; ADR-0095** |

### Rule-draft authoring authority

| Alternative | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Database fixtures/operator SQL | Fast for bootstrap | Bypasses tenant auth, idempotency, audit, and product controls | Rejected |
| Client-only draft state | No backend work | Cannot support review, replay, durable evidence, or multi-operator work | Rejected |
| Direct publication on create | Small workflow | Bypasses four-eyes review and effective-window governance | Rejected |
| External SaaS-owned rule document | Delegates rule authoring | Opaque semantics and dependency; cannot own protocol digest/evidence | Rejected |
| **Authenticated Rust API with transactional PostgreSQL/outbox creation** | Canonical validation, tenant isolation, idempotency, evidence, and real UI workflow | Requires migration and bounded JSON authoring form | **Selected; ADR-0096** |

### Settlement submission and lifecycle

The accepted design uses a bounded server-side coordinator after wallet
signing. It reuses the hardened JSON-RPC transport, preserves immutable signed
bytes across at most three provider attempts, and separates `submitted` from
chain commitment. PostgreSQL UUIDv7 intents and append-only events fence
idempotency and make a provider timeout `unknown` rather than silently creating
a duplicate. Custodial signing, browser-only submission as the sole path, and
unbounded retry/rebuild strategies are rejected; see ADR-0056 and ADR-0057.
ADR-0058 adds the mandatory cryptographic message-binding gate: the exact
compiled instruction, program, account domain, signer/writable policy, and
wire transaction signatures are independently verified before durable state or
RPC submission. Address-table lookups and extra instructions remain rejected
until separately governed.

### Checkout wallet handoff

The first real checkout handoff uses an explicit browser wallet port backed by
the latest stable `@solana/web3.js` 1.x maintenance release. It builds one
legacy transaction from the server compiler artifact, uses a confirmed recent
blockhash, requests the wallet signature, and submits unchanged base64 bytes to
the server coordinator. `@solana/kit` 8.x is the strategic SDK direction but is
deferred for the first compatibility slice because injected wallets still
commonly expose the web3.js `Transaction` boundary; wallet-standard support
must be proven against the supported provider matrix before switching. Wallet
adapter registries, server-side signing, and browser-only RPC submission were
rejected; see ADR-0059.

### Wallet Standard compatibility boundary

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Legacy `window.solana` only | Minimal bundle and implementation | Cannot enumerate providers or prove advertised capabilities | Rejected |
| **Wallet Standard discovery with explicit selection and legacy fallback** | Capability-aware multi-wallet support while preserving the existing web3.js and server signed-byte boundary | Adds bounded selection/lifecycle state and exact dependencies | **Selected; ADR-0200** |
| Implicitly use the first account returned by a provider | Lowest interaction cost | Can authorize the wrong signer and cannot be safely recovered after compilation | Rejected |
| **Accessible explicit account selector after `standard:connect`** | Preserves signer intent and makes multi-account ambiguity visible before signing | Adds one bounded selection step for multi-account wallets | **Selected; ADR-0200** |
| Full Solana Wallet Adapter registry | Broad adapter coverage | Larger bundle and lifecycle abstraction than this product boundary requires | Rejected for this slice |
| Direct browser `sendTransaction` | Fewer API calls | Bypasses durable intent/reconciliation authority | Rejected |

### Jurisdiction resolution

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Caller-provided jurisdiction string | Lowest latency and simplest API | Caller substitution, no evidence provenance, no replayable precedence | Rejected |
| Per-provider precedence logic | Flexible provider integrations | Duplicated semantics and inconsistent conflict handling | Rejected |
| External tax SaaS authority | Apparent global coverage | Opaque rule/evidence binding, outage and vendor lock-in risk | Rejected |
| On-chain jurisdiction resolution | Settlement-visible result | Privacy, changing law, unbounded data, and account/compute constraints | Rejected |
| **Pure Rust versioned policy resolver over authenticated evidence** | Deterministic, provider-agnostic, replayable, low-latency after compilation, fail-closed | Requires legal/tax-owned policies and evidence adapters | **Selected; see ADR-0101** |

### Binding jurisdiction evidence to calculation facts

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Keep only the jurisdiction string | No schema migration | No provenance or replay proof | Rejected |
| Store a caller-provided digest | Small payload | A digest alone does not prove resolver execution | Rejected |
| Store raw address/KYC evidence in every calculation | Maximum local context | PII, retention, payload-size, and privacy risks | Rejected |
| Re-resolve during replay | Fresh provider data | Historical results drift and replay gains network dependencies | Rejected |
| **Server-owned bounded resolution envelope in facts/results** | Digest-bound, replayable, privacy-minimized, backward compatible | Requires application resolver integration and explicit legacy handling | **Selected; see ADR-0102** |

### Production quote jurisdiction authority

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Caller-supplied candidates or evidence digests | Simple and fast | Forged evidence and no provider provenance | Rejected |
| Synchronous external tax/geocoding call in quote path | Broad apparent coverage | Latency, outage, replay drift, opaque precedence | Rejected |
| Redis/configuration-only policy | Fast reads | Non-authoritative and difficult to audit or rotate safely | Rejected |
| Global policy registry independent of the selected rule release | Reusable policies | Effective-window and tenant-selection skew | Rejected |
| **Signed provider evidence plus immutable tenant policy bound to the published rule release** | Cryptographic provenance, bounded hot path, replayable, tenant-safe | Requires key governance and policy lifecycle | **Selected; see ADR-0103** |

### Jurisdiction policy governance

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Direct SQL or fixtures | Fast bootstrap | Bypasses identity, four-eyes review, idempotency, and audit | Rejected |
| Couple policy changes to rule JSON only | One approval surface | Cannot correct precedence independently; weak policy provenance | Rejected |
| External workflow/SaaS authority | Rich workflow features | Adds availability dependency and replay drift | Rejected |
| **Authenticated Rust API plus PostgreSQL four-eyes lifecycle** | Tenant-safe, bounded, replayable, auditable publication | Requires role provisioning and governance UI | **Selected; see ADR-0104** |

### Vault release control

| Option | Advantages | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| API-held release signer | Simple request path | Key custody in the API, weak separation, and difficult incident containment | Rejected |
| Single on-chain release authority | Small account model | One compromised or mistaken signer can move the full scoped vault balance | Rejected |
| Off-chain claim ledger followed by an unconstrained transfer | Rich workflow metadata | Database is not authoritative for token movement; destination substitution and replay remain possible | Rejected |
| **Merchant/mint registry with separate proposer/approver keys and immutable release-claim PDAs** | Four-eyes control, exact amount/destination binding, PDA-signed checked CPI, atomic replay protection | Adds registry and claim lifecycle state plus key-ceremony operational work | **Selected; see ADR-0105** |

### Settlement intent tenant boundary

| Alternative | Advantages | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Global intent lookup followed by an API merchant check | Minimal method signature | Persistence boundary can leak cross-tenant existence and future callers may omit the check | Rejected |
| PostgreSQL session tenant variable/RLS only | Centralized policy | Pooled-connection context installation and reset are easy to misconfigure; harder offline verification | Rejected as the sole control |
| Opaque merchant-prefixed intent identifiers | Hides some identifiers | Changes the public contract and conflates identity with authorization | Rejected |
| **Explicit merchant predicate on every intent read and state transition** | Auditable, fail-closed, compatible with UUIDv7 IDs, and independently testable | Adds one required domain parameter and intentionally returns 404 for cross-tenant IDs | **Selected; ADR-0107** |

### Sensitive tax-evidence persistence

| Area | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Replay evidence at rest | Application AEAD over normalized facts with injected KMS/HSM key provider; raw jurisdiction envelopes discarded | Plaintext JSONB, PostgreSQL `pgcrypto`, disk encryption only, raw envelope retention | Minimizes retained evidence, binds ciphertext to tenant/digest, supports rotation and audited access without widening the settlement hot path |

### Evidence-key lifecycle

| Area | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Key resolution | Injected provider with a bounded warm current/previous-key ring | Per-request KMS/HSM calls, one immutable environment key, database-held key history | Keeps quote/replay latency bounded while supporting rotation, revocation, and an auditable KMS/HSM adapter; see ADR-0109 |

### Indexer replay telemetry

| Option | Advantages | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Per-stream metric labels | Easy dashboard filtering | Unbounded cardinality from configuration data | Rejected |
| Independent database exporter | Separates telemetry from the service | Adds lag and another operational boundary | Rejected |
| Log-only backlog reporting | No metric implementation | Cannot provide stable alert expressions | Rejected |
| **Fixed-cardinality gauges from the indexer** | Scoped, bounded, alertable, and operationally close to replay state | Requires one bounded database snapshot query | **Selected; see ADR-0123** |

### Targeted finalized signature recovery

| Alternative | Benefits | Risks / costs | Decision |
| --- | --- | --- | --- |
| Forward-only finalized `getBlock` scan | Strong checkpoint semantics and complete block evidence | Expensive for narrow investigations and ambiguous signatures | Retained as authoritative projection path |
| Provider-specific operator queries | Fast to try manually | Unbounded, inconsistent commitment/response semantics, not reproducible | Rejected |
| Unbounded `getSignaturesForAddress` loop | Finds all history | Unbounded provider work and cancellation/retry risk | Rejected |
| **Bounded typed finalized signature pages with separate evidence fetch** | Fixed work budget, canonical cursor, provider-independent contract, supports targeted recovery | Needs a separate authorized worker to fetch and verify full finalized evidence | **Selected in ADR-0124** |

### Targeted signature-to-evidence recovery

| Alternative | Benefits | Risks / costs | Decision |
| --- | --- | --- | --- |
| Rebuild from retained raw finalized evidence | No provider dependency; strongest replay determinism | Cannot recover evidence that was never retained | Retained as the preferred rebuild path |
| Poll `getTransaction` without address discovery | Fewer calls for a known signature | Does not prove address scope and encourages unbounded/manual provider use | Rejected |
| Persist signature-list entries as projection evidence | Very low latency | A signature list contains no message, event bytes, or amounts | Rejected |
| **Bounded address signature discovery followed by finalized `getTransaction` and projection-only persistence** | Binds the target to finalized evidence, preserves checkpoint authority, supports idempotent repair | Requires an authorized control-plane wrapper and live provider evidence | **Selected in ADR-0125** |

### Targeted-recovery audit reconciliation telemetry

| Alternative | Benefits | Risks / costs | Decision |
| --- | --- | --- | --- |
| Query the audit table on every API metrics scrape | Direct visibility from the writer | Adds a database round trip to the API metrics endpoint and couples telemetry to request serving | Rejected |
| Per-request/address/signature metric labels | Easy target lookup | Unbounded sensitive cardinality and memory/alerting hazards | Rejected |
| Log-only stale requested rows | Minimal implementation | No stable alert expression or bounded dashboard signal | Rejected |
| **Fixed-cardinality stale-request aggregate refreshed by the indexer service** | Bounded, alertable, no hot-path mutation, reuses operational refresh loop | Requires one indexed aggregate query and reconciliation runbook | **Selected; see ADR-0126** |

### Finalized vault-balance reconciliation

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Cumulative receipt tax only | Simple projection query | Ignores governed vault outflows | Rejected |
| PostgreSQL-only expected/observed comparison | Cheap and deterministic | PostgreSQL is not the Solana funds authority | Rejected |
| On-chain cumulative vault counter | Strong on-chain evidence | Adds writable hot state and contention to settlement | Rejected |
| `getTokenAccountBalance` without account proof | Small RPC response | Does not prove owner, mint, token program, or extension compatibility | Rejected |
| **Explicit opening balance plus finalized receipt/release delta, checked against a finalized verified token account** | Auditable initial condition, exact u64 semantics, preserves Solana authority, bounded off-hot-path work | Requires opening-balance ceremony and a separate Token-2022 extension ADR | **Selected in ADR-0127** |

### Vault-reconciliation operator read model

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Browser target-creation and opening-balance form | Convenient onboarding | Browser cannot prove finalized chain evidence; creates an unsafe authority boundary | Rejected |
| Global system-wide dashboard at the merchant API boundary | Centralized visibility | Cross-tenant disclosure and broader operator authority | Rejected |
| Logs only | Minimal implementation | No bounded authenticated evidence or usable recovery state | Rejected |
| **Tenant-scoped authenticated read model with server-owned immutable values** | Protects tenant boundary, supports explicit UI failure states, no settlement mutation, bounded indexed queries | System-wide finance view remains a separate governed surface | **Selected in ADR-0128** |

### Release artifact integrity

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Manual release notes and ad-hoc binaries | Low setup cost | Not reproducible; weak source-to-artifact binding | Rejected |
| Tag-triggered deployment or image publication | Fast path to operations | Adds unapproved deployment authority and can bypass license/readiness gates | Rejected |
| Floating actions, images, or dependency updates | Low maintenance effort | Non-reproducible and unsafe supply-chain boundary | Rejected |
| **Tag/manual evidence bundle with checksums, SBOM, and validator evidence** | Binds tested source to exact artifacts without deployment authority | Requires a successful retained workflow run and independent review | **Selected in ADR-0129** |

### Deployment environment safety

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Blank environment with permissive defaults | Convenient local startup | Ambiguous security boundary and unsafe mode combinations | Rejected |
| Separate binaries per environment | Strong visual separation | Duplicated startup logic and release drift | Rejected |
| Deployment-tool-only validation | Centralized policy | Direct process starts can bypass it | Rejected |
| **Explicit runtime environment contract with checked-in local defaults** | Fail-closed startup, deterministic tests, one immutable binary | Production provisioning and launch approval remain separate controls | **Selected in ADR-0130** |

### Worker deployment environment safety

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Check only whether insecure RPC is enabled | Small change | Secure-RPC workers can still start with an ambiguous environment identity | Rejected |
| Infer environment from RPC endpoint names | No additional setting | Endpoint names are not authority, and proxies/failover make inference unreliable | Rejected |
| Separate worker binaries per environment | Visible separation | Duplicated startup logic and release drift | Rejected |
| **Required `*_ENV` contract in each worker with explicit Compose local defaults** | Fail-closed identity, deterministic tests, one immutable binary | Deployment provisioning still needs independent approval and secrets | **Selected; see ADR-0132** |

### Multi-replica quote idempotency evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Single API process under high concurrency | Simple and fast to run | Cannot expose process-local state or replica routing errors | Rejected as the only evidence |
| In-process or Redis lock | May reduce duplicate work | Adds a non-authoritative lock boundary and can diverge from PostgreSQL | Rejected |
| Sticky load-balancer sessions | Easy routing model | Masks failover and retry-to-another-replica behavior | Rejected |
| **Two independent API containers fenced by PostgreSQL** | Reproducible, bounded, and validates the actual durable authority | Local evidence does not prove regional production capacity | **Selected; see ADR-0133** |

### Multi-replica settlement submission-fence evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Submit both requests to one API process | Fast and simple | Does not exercise replica routing or process-local state separation | Rejected as the only evidence |
| Sticky routing or process-local single-flight | May reduce duplicate work | Masks failover and cannot be the durable money-moving authority | Rejected |
| Rely on RPC/Solana duplicate-signature handling | No application change | Allows duplicate provider work and weakens operational guarantees | Rejected |
| **Two API processes with PostgreSQL compare-and-set fencing** | Directly validates the durable submission authority with bounded Docker evidence | Does not prove regional provider failover or production capacity | **Selected; see ADR-0134** |

### SLO, incident, and dependency controls

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Infrastructure availability metrics only | Easy to collect and compare | Hides quote correctness, finality, reconciliation, and financial-integrity failures | Rejected |
| Vendor status page as the operating contract | Fast to adopt | Provider lock-in, incomplete user-journey semantics, and weak public reproducibility | Rejected |
| Team-local undocumented targets | Flexible during early development | No consistent ownership, release gate, RTO/RPO, or incident severity | Rejected |
| **Versioned user-journey SLO policy plus dependency register and public incident runbook** | Binds latency, correctness, recovery, ownership, and exit paths while keeping provider adapters replaceable | Requires deployment-specific owner/provider binding and retained evidence | **Selected; see ADR-0135** |

### Scheduled vault reconciliation evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| PostgreSQL row seeded directly by the drill | Fast read-model and authorization assertion | Does not exercise the scheduler, RPC serialization, account owner/mint/layout validation, or expectation calculation | Rejected as the only evidence |
| Unit-only mock of the finalized balance port | Deterministic and fast | Cannot detect Compose wiring, process startup, JSON-RPC, or persistence regressions | Retained as a complement, not release evidence |
| Full validator for every reconciliation test | Strong ledger realism | Slow and environment-dependent; validator remains a separate Linux conformance gate | Rejected as the only routine drill |
| **Bounded synthetic `getMultipleAccounts` fixture plus the real scheduled indexer process** | Exercises the deployed service boundary deterministically and supports matched/mismatch evidence | Synthetic account semantics do not replace live validator or provider-failover tests | **Selected; see ADR-0136** |

### Multi-provider RPC failover evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Unit tests only | Fast and deterministic | Cannot prove endpoint wiring, Docker networking, or service transport behavior | Rejected as the only evidence |
| Unbounded retries against one provider | Simple implementation | Violates deadline, backpressure, and overload controls | Rejected |
| Random provider selection per request | Spreads traffic | Nondeterministic evidence and weak adaptive health semantics | Rejected |
| Public/testnet outage injection | More realistic network | Uncontrolled, non-repeatable, and unsuitable for CI traffic | Rejected |
| **Two isolated fixtures with deterministic first-provider method failure** | Process-level, bounded, repeatable failover evidence | Synthetic failure does not prove live provider SLOs or regional capacity | **Selected; see ADR-0137** |

### Database migration execution

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| PostgreSQL init-directory ordering only | Simple clean bootstrap | No durable applied ledger, lock, or historical checksum verification | Rejected for staging/production |
| Re-run all idempotent SQL files | Minimal tooling | Idempotence does not prove compatibility or detect source drift | Rejected |
| Provider-specific migration service | Mature operational integrations | Adds vendor dependency and weakens clean-checkout reproducibility | Rejected as repository authority |
| **Rust runner with advisory lock, ordered ledger, and SHA-256 checksums** | Deterministic, typed, fail-closed, low-latency deployment boundary | Requires staged Compose/deployment integration | **Selected; see ADR-0138** |

### Migration database-connection lifecycle

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Detached `tokio::spawn` connection driver | Simple and common in small binaries | Connection failures can arrive after a false-success migration result; task lifetime is detached | Rejected |
| `tokio::try_join!` migration and connection futures | Propagates errors | A healthy PostgreSQL driver normally does not finish before the client closes, so successful migration waits indefinitely | Rejected |
| Post-migration connection health check | Easy to add | Cannot prove that a failure did not occur during migration | Rejected |
| **Owned `tokio::select!` migration/connection supervisor** | Bounded lifetime, prompt failure propagation, cancellation, and no detached task | Requires explicit clean-termination semantics | **Selected; see ADR-0221** |

### Service shutdown-signal handling

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Convert `ctrl_c()` errors to `None` | Keeps the loop simple | Masks failed shutdown control and can leave a process running indefinitely | Rejected |
| Retry signal registration forever | May recover from transient setup errors | Unbounded retry and no deterministic lifecycle result | Rejected |
| Detached signal-listener task | Separates signal code | Loses ownership and complicates cancellation/error propagation | Rejected |
| **Handle `ctrl_c()` success/error in the existing structured service loop** | Fail-closed, bounded, minimal, and preserves orderly shutdown | Requires explicit error branch in each entrypoint | **Selected; see ADR-0222** |

### Non-authoritative metric-observation failures

Metric export is operational evidence, not settlement authority. Unexpected
rejection by the fixed operation registry must therefore be visible without
allowing telemetry to fail a quote, settlement, reconciliation, or worker
operation.

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Discard `observe` errors | Zero request-path overhead | Silently loses evidence and hides registry drift | Rejected |
| Propagate errors through financial paths | Makes failures visible to callers | Incorrectly makes non-authoritative telemetry a money-path dependency | Rejected |
| Queue metric errors asynchronously | Decouples the caller | Adds unbounded/lossy queue and lifecycle complexity for a bounded validation error | Rejected |
| **Synchronous bounded structured log via `observe_or_log`** | Visible, bounded, no retry or external dependency, preserves financial-path availability | Requires a fixed log contract and log-volume monitoring | **Selected; see ADR-0223** |

All PostgreSQL-backed operational drills use the same migration authority.
Each clean-stack drill waits for PostgreSQL health and runs the pinned
one-shot migration service before touching application tables; see
[ADR-0160](../adr/0160-clean-stack-drill-migration-gate.md).

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Duplicate schema setup in each drill | Direct and locally fast | Schema drift and false evidence when migrations break | Rejected |
| Reuse a pre-migrated shared volume | Fast repeat runs | Order-dependent and not clean-checkout reproducible | Rejected |
| **Run the authoritative migrator after PostgreSQL health** | Clean-stack reproducibility, checksum/ordering coverage, minimal topology | Adds bounded setup time | **Selected; ADR-0160** |

### v0 transaction and Address Lookup Table admission

The verifier must resolve dynamic accounts from chain state before accepting a
wallet-signed v0 settlement; see [ADR-0161](../adr/0161-governed-v0-alt-settlement-path.md).

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Reject v0+ALT indefinitely | Smallest attack surface | Leaves a required production transaction path unavailable | Rejected |
| Trust client-provided table contents or digest | No RPC read and low latency | Client-controlled authority boundary and account substitution risk | Rejected |
| Accept any provider-returned table without bounds | Broad compatibility | Unbounded reads, provider disagreement, and extra-account risk | Rejected |
| **Server-owned bounded finalized resolution with exact expanded-domain verification** | Preserves v0 capability while retaining account, privilege, size, and failover controls | Adds bounded RPC reads before submission | **Selected; ADR-0161** |

## Non-local deployment configuration

| Decision | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Staging/production configuration boundary | Shared repository preflight plus service-owned runtime validation (ADR-0139) | Convention-only environment variables, deployment-tool-only validation, separate binaries per environment | Catches cross-service local-default leakage before rollout while retaining independent fail-closed checks in every service and one immutable release artifact |

The preflight requires non-local TLS-verified databases, HTTPS RPC endpoints,
identity and evidence keys, jurisdiction-provider inputs, and no synthetic seed
directory. It does not replace secret validity checks, HSM/KMS ceremonies,
deployment approvals, provider health, or live staging evidence.

### Tax Engine deployment identity preflight

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Operator convention only | No additional manifest fields | Local token or omitted service label can reach non-local rollout | Rejected |
| Tax Engine-only checker | Narrow ownership | Duplicates shared deployment policy and can drift | Rejected |
| **Shared preflight plus Tax Engine runtime validation** | Cross-service parity, early local-default rejection, independent process safety | Secret validity, mTLS, and rotation remain deployment-owned | **Selected in ADR-0226** |

### Tax Engine service transport

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Plain HTTP plus private network policy | Lowest handshake overhead | No cryptographic workload identity or confidentiality at the service boundary | Rejected |
| NGINX/sidecar-only mTLS termination | Centralized certificate operations | Extra hop and bypassable proxy authority on the internal calculation path | Rejected as the default |
| Bearer token over externally terminated TLS | Simple application integration | Does not protect a bypassed/internal plaintext hop from token replay | Rejected as the sole control |
| **Direct Rustls mTLS in the Tax Engine process** | Explicit client CA verification, no non-local plaintext fallback, bounded path | Certificate provisioning and rotation remain deployment work | **Selected in ADR-0227** |

Rustls provider selection is explicit: automatic feature-based selection is
rejected because the workspace contains both AWS-LC and ring feature paths;
AWS-LC is installed process-wide before the Tax Engine constructs TLS configs.
Changing that provider requires a compatibility and compliance ADR.

### Tax Engine mTLS certificate rotation

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Restart the process for every rotation | Simple lifecycle | Connection churn and a larger emergency blast radius | Rejected as the only mechanism |
| Poll or watch certificate files | Automatic detection | Partial-write races, platform-specific semantics, hidden operator action | Rejected |
| Sidecar-only reload and proxy termination | Centralized certificate operations | Extra hop and a second internal transport authority | Rejected as the default |
| **Explicit SIGHUP with validated atomic Rustls snapshot replacement** | Bounded, auditable, no extra hop, retains last known-good config | Requires Unix signal and atomic secret-delivery ceremony | **Selected in ADR-0228** |

The reload path validates certificate, key, client CA, verifier, and ALPN in a
blocking task before swapping the live snapshot. Invalid replacements are
observable but cannot displace a valid configuration.

### Tax Engine observability identity

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Accept both `tax-engine` and `tax_engine` | Transitional compatibility | Splits fixed-cardinality dashboards, alerts, and evidence queries | Rejected |
| Accept arbitrary service names | Flexible reuse | Allows unreviewed telemetry identities and cardinality drift | Rejected |
| **Use one explicit `tax_engine` identity** | Matches existing metrics and preserves the allow-list boundary | Requires one canonical spelling in operations | **Selected in ADR-0229** |

### Tax Engine container runtime verification

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Unit tests only | Fast and deterministic | Misses image entrypoint, environment, and listener drift | Rejected as the sole gate |
| Full end-to-end stack only | Broad system coverage | Slow and poor failure localization for the Tax Engine boundary | Rejected as the only service contract |
| Host-process smoke test | Simple to run | Does not verify the Docker production boundary | Rejected |
| **Isolated pinned Docker runtime contract** | Exercises the real image with bounded focused assertions and cleanup | Requires Docker in CI/local verification | **Selected in ADR-0230** |

### Tax Engine request-rejection semantics

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Preserve Axum's default `422` response | No handler code | Diverges from the repository API contract | Rejected |
| Map every JSON rejection to `400` | Simple and predictable | Masks body-limit admission control | Rejected |
| Broad global rejection middleware | Centralized | Couples unrelated routes and obscures rejection provenance | Rejected |
| **Explicit typed-boundary mapping with body-limit `413`** | Preserves client and operational semantics with bounded error detail | Small explicit mapping function | **Selected in ADR-0231** |

### Warm Tax Engine service concurrency evidence

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Pure Criterion benchmark only | Fast and stable | Omits HTTP, container, authentication, scheduler, and serialization costs | Rejected as the sole service gate |
| Full API quote benchmark | Exercises durable storage and tenant paths | Does not isolate the warm Tax Engine adapter | Rejected for this boundary |
| Unbounded external load generator | Broad traffic shapes | Non-reproducible, resource-heavy, and difficult to keep synthetic | Rejected |
| **Bounded local Docker concurrency contract** | Measures the real image with explicit workload and cleanup | Local-only evidence and Docker dependency | **Selected in ADR-0232** |

### Real-container Tax Engine mTLS

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Rustls unit tests only | Fast and deterministic | Misses image paths, process startup, listener, and signal delivery | Rejected as the sole gate |
| Plaintext runtime contract only | Simple and portable | Cannot detect non-local plaintext fallback or missing client authentication | Rejected |
| Sidecar/proxy-only mTLS test | Matches one deployment topology | Leaves direct Rustls configuration and image behavior unverified | Rejected as the service contract |
| **Ephemeral CA plus real pinned-container mTLS probe** | Exercises deployed binary, certificate files, client verification, ALPN, and reload | Requires Docker and host OpenSSL | **Selected in ADR-0233** |

### JWKS deployment representation

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Require the legacy single-key pair | Simple shell validation | Rejects provider-managed rotation already supported by API startup | Rejected |
| Accept both structured JWKS and the pair simultaneously | Permissive migration | Creates ambiguous identity authority and rotation drift | Rejected |
| Remove identity-key validation from preflight | Fewer deployment failures | Allows missing or local identity material to reach rollout | Rejected |
| **Accept exactly one supported representation with structural/local-value checks** | Keeps deployment parity, fail-closed mutual exclusion, and secret-safe validation | Cryptographic key validity remains a runtime/provider responsibility | **Selected in ADR-0219** |

### Jurisdiction-provider key deployment representation

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Require the legacy provider-key pair | Simple shell validation | Rejects the structured registry already supported by API startup | Rejected |
| Accept structured registry and pair simultaneously | Permissive migration | Creates ambiguous provider authority and rotation drift | Rejected |
| Remove provider-key validation from preflight | Fewer deployment failures | Allows missing or local jurisdiction evidence verification material to reach rollout | Rejected |
| **Accept exactly one supported representation with structural/local-value checks** | Keeps preflight/runtime parity and fail-closed secret-safe validation | Cryptographic validity and provider availability remain runtime/deployment responsibilities | **Selected in ADR-0220** |

### Validator-backed Split Plan conformance

| Decision | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Runtime proof for bounded recipient fan-out | Extend the pinned external Agave validator suite (ADR-0140) | Allocator unit tests only, `solana-program-test` only, synthetic RPC fixture | Proves the deployed SBF artifact, exact remaining-account order, token CPI movement, finalized conservation, and duplicate-receipt behavior at the real program boundary |

ADR-0214 extends that selected validator boundary with an explicit
extension-free Token-2022 two-recipient case. This preserves the one
`token_interface` implementation while preventing a single-recipient test from
being mistaken for proof of multi-recipient Token-2022 compatibility.

### Validator-backed policy revocation

| Decision | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Runtime proof that revoked policy cannot move tokens | Execute revoke then settlement against the pinned external Agave validator (ADR-0141) | Unit-test revocation helpers only, synthetic RPC fixture | Proves authority binding, revoked-PDA enforcement, deployed SBF behavior, and unchanged token balances at the CPI boundary |

### Bounded load-test resource profiling

| Decision | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Resource evidence alongside quote latency | Bounded Docker JSON stats for the named benchmark containers (ADR-0215) | Latency only, arbitrary host-process sampling, an unpinned monitoring stack | Provides reproducible CPU/memory/runtime evidence without leaking unrelated host data or adding an unbounded dependency to the benchmark |

### Server-owned settlement-route mint precision

| Decision | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Asset precision at the checkout boundary | Persist finalized verifier-derived `mint_decimals` on the immutable route version (ADR-0216) | Assume two decimals, accept browser/caller precision, fetch mint metadata during checkout, parse untyped evidence on every read | Keeps precision evidence-bound and O(1) on the hot path, supports Solana assets with 0–9 decimals, and prevents browser-side amount misrepresentation |

### SPA workspace focus management

| Decision | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Keyboard/screen-reader context after workspace changes | Skip link plus stable focus boundary (ADR-0142) | Leave focus on navigation control, focus first form control, add a full router | Deterministic across lazy/error/empty states with minimal client lifecycle cost and no impact on financial authority |

### Structured Tier-1 rule workbench

| Decision | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Rule-governance authoring surface | Schema-driven structured workbench plus advanced canonical JSON mode (ADR-0143) | Raw JSON only, jurisdiction-specific forms, graph editor only | Makes every supported DSL semantic reviewable and accessible while preserving exact canonical interchange and server authority |

### Persisted-rule simulation boundary

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Caller-supplied isolated calculation preview | Simple and useful for engine development | Does not prove the document being simulated is the governed tenant-owned draft | Retained only for simulation-mode development; rejected for governance UI |
| Browser-side calculation against a returned rule document | Immediate interaction | Duplicated tax semantics, numeric drift, and a client authority surface | Rejected |
| Persist every simulation as financial evidence | Maximum historical volume | Hot-path writes and audit noise; still needs rule binding | Rejected |
| **Server-owned simulation of persisted rule with digest/identity checks and warm cache (ADR-0144)** | Binds the exact governed artifact while preserving bounded low-latency execution | Ephemeral result is not a quote or attestation | **Selected** |

### Settlement adjustments

| Decision | Selected approach | Alternatives considered | Why selected |
| --- | --- | --- | --- |
| Original receipt binding | Tenant-scoped finalized settlement-intent join to the canonical receipt projection | Caller-supplied receipt; mutable adjustment copy; signature-only join | Preserves server ownership, finalized-slot binding, asset binding, duplicate-stream tolerance, and fail-closed conflict handling |

| Adjustment authority binding | Wallet-proof-backed subject-to-Solana-key binding plus durable compiler intent | OIDC subject as key; caller-supplied keys; server-held signer | Provides cryptographic actor binding, non-custodial signing, rotation evidence, and deterministic retry convergence |

### Canonical settlement-receipt identity projection

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Caller-supplied original receipt and chain accounts | Small request shape | Breaks server-owned authority and permits account substitution | Rejected |
| Mutable API UUID-to-receipt mapping | Simple lookup | Adds a second authority disconnected from finalized raw evidence | Rejected |
| Split-settlement projection for all payments | Existing rich identity fields | Does not cover ordinary settlement receipts | Rejected |
| **Enrich `IndexedReceipt` from finalized Anchor event bytes** | Replayable, immutable, shared by reconciliation and adjustments | Requires additive migration and backfill evidence | **Selected in ADR-0164** |

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Mutate the original settlement receipt | Minimal tables and simple customer display | Destroys historical evidence, breaks replay and reconciliation, and obscures who changed money | Rejected |
| Treat refunds as an untyped negative settlement | Reuses payment code | Loses adjustment reason/evidence, weakens bounds, and can bypass approval and duplicate fencing | Rejected |
| Put refund/correction arithmetic on-chain | Strong ledger authority | Tax semantics, chargeback evidence, and unbounded business workflow do not fit a bounded deterministic program | Rejected |
| **Immutable signed-delta adjustment domain with separate governed posting** | Exact conservation, replayable evidence, bounded arithmetic, and clear separation from token authority | Requires durable workflow, read models, and a later posting contract | **Selected in ADR-0150** |

### Durable adjustment workflow

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Mutate the original receipt row | Few reads and no new projection | Breaks immutable audit/replay semantics and hides the adjustment actor | Rejected |
| Browser/local storage workflow | Fast interaction and no API schema | Not durable, not tenant-authoritative, and unsafe after reload or retries | Rejected |
| Redis-only proposal state | Low latency and simple TTLs | Disposable cache cannot be the financial evidence authority | Rejected |
| **PostgreSQL append-only adjustment record plus transactional outbox** | Durable UUIDv7 identity, tenant fencing, atomic evidence/publication, and existing operational controls | Requires migration and bounded governance endpoints | **Selected in ADR-0151** |

### Settlement-adjustment operator workspace

**Criteria:** preserve the settlement-adjustment authority boundary, make
four-eyes evidence operable, support real loading/error/retry states, avoid
financial arithmetic in JavaScript, and fit the existing accessible SPA.

| Option | Strengths | Risks | Decision |
| --- | --- | --- | --- |
| **Dedicated settlement-adjustment workspace** | Clear bounded-context ownership, explicit evidence review, no implied chain posting | Adds one lazy-loaded workspace and API contract | **Selected in ADR-0152** |
| Embed into vault-release control | Reuses treasury UI and lifecycle patterns | Conflates adjustment evidence with token release authority and makes refund semantics ambiguous | Rejected |
| Extend rule-governance workspace | Reuses author/reviewer patterns | Tax-rule publication and financial corrections have different invariants, scopes, and audit meaning | Rejected |
| Generic admin table | Fast list/read surface | Encourages unsafe inline mutation and cannot express exact deltas, evidence, or four-eyes semantics well | Rejected |

The selected workspace is a projection and command client only. PostgreSQL and
the authenticated API remain authoritative; browser state is disposable.

### Adjustment UI validation boundary

**Criteria:** preserve exact financial values, provide immediate feedback, and
avoid introducing a second financial implementation.

| Option | Strengths | Risks | Decision |
| --- | --- | --- | --- |
| **String validation plus conservation check, API revalidation** | Safe browser UX, no floating-point conversion, server remains authority | Duplicates a small amount of validation code | **Selected in ADR-0152** |
| JavaScript `number` arithmetic | Simple controls | Loses precision for Solana atomic units and violates financial contract | Rejected |
| Browser-only validation | Minimal API work | Allows stale or malicious callers to bypass invariants | Rejected |
| Client tax/adjustment engine | Rich immediate calculation | Creates a second authority and can diverge from Rust semantics | Rejected |

### On-chain adjustment boundary

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Mutate the original settlement receipt or reuse `settle` | Few accounts and familiar code path | Destroys original evidence, conflates checkout with correction, and weakens replay/reconciliation | Rejected |
| Transfer directly from the tax vault or original payer source | Appears to complete refunds quickly | No universal lawful funder/beneficiary; unsafe across tax, chargeback, and remittance semantics | Rejected |
| Off-chain `posted` state only | Simple API and database implementation | No independently verifiable finalized chain evidence and easy to overstate payment | Rejected |
| **Separate bounded `record_adjustment` PDA with active merchant/mint authority registry and no token CPI** | Immutable chain evidence, deterministic replay identity, fixed account/compute shape, and fail-closed authority boundary | Does not itself pay a refund; requires registry governance and a later movement contract | **Selected in ADR-0153** |

### Finalized adjustment-registry verification

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Trust the database route and wallet bindings | No extra RPC call | Does not prove the active on-chain authority or program ownership | Rejected |
| Accept a registry account from the browser | Flexible integration | Allows PDA/account substitution | Rejected |
| Read at processed or confirmed commitment | Lower latency | Can admit stale or reorged authority state | Rejected |
| Read once and cache indefinitely | Low steady-state RPC cost | Hides registry rotation and makes cache an authority | Rejected |
| **Derive the PDA, read finalized account data, require provider agreement, and snapshot evidence in the intent** | Chain-bound, replayable, provider-aware, and fail closed | Adds bounded RPC admission and evidence columns | **Selected in ADR-0167** |

### Adjustment submission and finalized recording

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Mutate the immutable evidence intent with transport status | Fewer tables | Blends immutable compiler evidence with retryable transport state | Rejected |
| Reuse generic settlement intents | Existing lifecycle and routes | Settlement-specific message verification cannot prove adjustment accounts or event identity | Rejected |
| Trust client digest/signature or RPC acknowledgement | Lowest implementation cost | Does not prove message binding, finality, or emitted adjustment evidence | Rejected |
| **Separate wallet-bound chain-intent lifecycle plus typed finalized event projection** | Immutable audit boundary, bounded retries, exact message proof, and independent finalized evidence | Adds migration, decoder, and reconciliation work | **Selected in ADR-0170** |

### Adjustment wallet workspace

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Duplicate wallet/signing code in the adjustment panel | Fast local implementation | Splits security review, serialization, timeout, and rejection behavior | Rejected |
| Browser constructs or mutates `record_adjustment` bytes | Flexible UI | Creates an account/data substitution authority and numeric drift surface | Rejected |
| Persist signed bytes in browser storage | Convenient retry after reload | Increases sensitive material exposure and can cross operator/session boundaries | Rejected |
| **Reuse the shared injected-wallet boundary with server-owned compile, exact submission, and explicit reconciliation** | One wallet trust boundary, immutable server authority, bounded UX states | Requires a typed adjustment artifact parser and wallet mismatch checks | **Selected in ADR-0171** |

### Solana `u64` browser wire representation

| Alternative | Decision | Trade-off |
| --- | --- | --- |
| JSON numbers with browser safe-integer rejection | Rejected | Valid Solana counters above `2^53 - 1` cannot be represented. |
| Hex/base64 protocol counters | Rejected | Exact but inconsistent with canonical decimal financial values and less interoperable. |
| JavaScript `BigInt` as the universal client representation | Rejected | Does not solve JSON transport and complicates wallet/UI boundaries. |
| **Canonical decimal strings at API/browser boundaries; Rust `u64` internally** | **Selected** | Exact, deterministic, cross-language, and compatible with existing financial string semantics. |

### Governed adjustment refund movement

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Reuse `execute_vault_release` | Existing claim and vault CPI path | Cannot atomically bind the merchant-funded net leg; over-privileges the tax vault | Rejected |
| Transfer full gross from the tax vault | One source and one CPI | Tax vault is not a universal refund funder and may not contain net funds | Rejected |
| Transfer from the original payer source | Appears to reverse the checkout | Original token account is not stored in the immutable receipt and payer is not necessarily the funder | Rejected |
| Browser/API direct transfer | Fast to expose | Bypasses on-chain account binding, atomicity, replay fencing, and non-custodial authority | Rejected |
| **Immutable refund claim with tax-vault and merchant-funder sources, one payer-bound destination, and two atomic checked CPIs** | Exact authority topology, conservation, replay protection, and explicit first-scope semantics | Adds claim lifecycle and requires separate contracts for other adjustment kinds | **Selected in ADR-0154** |

### Refund-claim wallet orchestration

**Criteria:** preserve non-custodial wallet authority, prevent account
substitution, retain four-eyes governance, make retries idempotent, and prove
finalized token movement independently from event indexing.

| Option | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Browser-supplied refund topology | Small API surface | Client can redirect source or beneficiary; cannot establish authority | Rejected |
| Immediate claim-and-execute flow | Fewer screens and tables | Removes approval separation and makes ambiguous submission unsafe | Rejected |
| Generic settlement-intent reuse | Existing submission machinery | Does not bind claim PDA, two refund signers, or movement proof | Rejected |
| **Separate server-owned claim and execute intents with shared wallet adapter, finalized typed evidence, and transaction-bound proof** | Strong authority separation, replay fencing, and explicit recovery | More durable state and operator workflow | **Selected in ADR-0173** |

### Refund funding configuration

**Criteria:** make the source and authority topology explicit, preserve tenant
isolation and four-eyes governance, support versioned rotation, and prevent an
off-chain row from being mistaken for chain ownership evidence.

| Option | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Caller-supplied funding accounts per refund | No setup workflow | Account substitution and beneficiary ambiguity; no durable rotation evidence | Rejected |
| Reuse the generic settlement route | Existing schema | A refund funder and payer destination have different authority semantics; conflates checkout and refund funding | Rejected |
| Mutable one-row merchant funding settings | Simple reads | Breaks replay of an existing claim when accounts rotate | Rejected |
| **Versioned tenant/mint funding configuration with append-only governance and separate finalized topology verification** | Stable claim binding, four-eyes control, auditable rotation, and explicit chain authority boundary | Adds a control-plane lifecycle and verification work | **Selected in ADR-0173** |

### Refund-funding topology activation

**Criteria:** prove the complete authority topology at one finalized slot,
prevent browser- or database-controlled activation, preserve bounded provider
failover, and make verification evidence replayable.

| Option | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Trust approved database account strings | No activation RPC | Does not prove chain ownership, token layout, PDA, or finality | Rejected |
| Reuse generic settlement-route verification unchanged | Existing implementation | Omits refund-funder wallet/source authority semantics | Rejected |
| Independent account reads | Small individual responses | Can mix slots and create a topology that never existed atomically | Rejected |
| **Complete finalized account-set read with provider agreement and immutable digest** | Exact authority checks, one-slot evidence, bounded failover, and replayability | Adds one provisioning operation and starts legacy-SPL-only | **Selected in ADR-0174** |

### Finalized refund execution evidence

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Keep refund execution as opaque raw bytes | Minimal schema and decoder work | Every consumer reparses event data and can disagree on account/amount binding | Rejected |
| Trust API-submitted signature and amounts | Low implementation cost | Caller-controlled evidence can claim movement without finalized chain proof | Rejected |
| Infer execution from balances only | Direct financial observation | A balance delta cannot prove the exact claim, event, beneficiary, or approver | Rejected |
| **Typed finalized event projection bound to immutable raw evidence, followed by independent balance reconciliation** | Deterministic replay, exact event binding, auditable evidence, and fail-closed conflict handling | Adds a bounded projection and a later reconciliation step | **Selected in ADR-0155** |

### Finalized refund balance proof

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Trust the refund execution event | Lowest latency and no extra account reads | Does not prove committed three-account deltas | Rejected |
| Read only the payer destination | Confirms an inflow | Cannot prove tax-vault and merchant-funder outflows or source identity | Rejected |
| Read the three accounts independently | Reuses simple account readers | Mixed finalized slots can produce a false composite | Rejected |
| **One finalized multi-account snapshot plus exact checked-u64 delta proof** | Atomic observation boundary, deterministic arithmetic, and replayable audit evidence | Requires claim topology/account-layout verification before persistence | **Selected in ADR-0156** |

### Transaction-bound refund balance proof

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Persist a finalized `getMultipleAccounts` post-state | Simple and inexpensive | Cannot prove the pre-state or distinguish later unrelated transfers | Rejected |
| Independently read accounts before and after | Reuses account readers | Different finalized slots and races cannot prove one atomic movement | Rejected |
| Trust event amounts | Lowest latency | Does not prove committed token-account deltas or account-index binding | Rejected |
| **Finalized transaction `preTokenBalances`/`postTokenBalances` plus a separate finalized claim topology read** | Exact transaction slot, atomic pre/post observation, account-index binding, and explicit claim semantics | Requires strict bounded provider decoding and two RPC calls | **Selected in ADR-0157** |

### Refund-proof orchestration authority

| Option | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Client/operator supplies complete account topology | Smallest integration surface | Allows account substitution and cannot establish server authority | Rejected |
| Mutable database topology projection | Fast reads | Adds a second mutable authority and synchronization lifecycle | Rejected |
| **Resolve topology from the finalized claim inside a complete provider attempt** | Server-owned bindings, bounded failover, replayable evidence | Adds one bounded claim read | **Selected in ADR-0158** |

### Durable refund claim and execute intents

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Reuse the generic adjustment chain intent | Smallest schema | Collapses claim creation and execution authority and weakens proof semantics | Rejected |
| Let the browser provide refund accounts, amounts, and nonce | Minimal server orchestration | Makes financial topology and replay protection caller-controlled | Rejected |
| Server custody and immediate submission | Simple retry behavior | Expands custody and removes explicit merchant wallet consent | Rejected |
| **Separate server-owned claim/execute intents with exact signed-message verification** | Four-eyes authority, durable idempotency, replayable evidence, and non-custodial wallet flow | Additional lifecycle, evidence, and reconciliation implementation | **Selected in ADR-0175** |

### Refund claim preparation API

**Criteria:** keep financial inputs server-owned, bind the active finalized
funding topology, preserve wallet consent, and make concurrent browser retries
safe and replayable.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Browser supplies receipt, accounts, amounts, and nonce | Smallest request | Account substitution, stale evidence, and caller-controlled money | Rejected |
| Reuse the generic adjustment compiler | Existing route and response shape | Does not bind refund-funder authority or claim lifecycle | Rejected |
| Custodial server submission | Fewer wallet interactions | Key custody and no explicit merchant authorization | Rejected |
| **Server-owned idempotent claim-preparation endpoint** | Complete review artifact, tenant isolation, deterministic replay, and non-custodial signing | Requires bounded intent persistence and later submission APIs | **Selected in ADR-0176** |

### Signed refund-claim submission

**Criteria:** prevent artifact substitution, preserve wallet consent, fence
concurrent submissions, and make provider ambiguity recoverable.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Accept browser-supplied instruction and artifact | Small request | Account, signer, and digest substitution | Rejected |
| Recompile from browser fields at submit time | Flexible | Reopens stale topology and amount races | Rejected |
| Custodial server submission | Fewer wallet steps | Key custody and weaker consent boundary | Rejected |
| Treat provider acknowledgement as success | Low latency | Does not prove inclusion or finality | Rejected |
| **Persisted-artifact exact verification plus durable submission fence** | Non-custodial, replay-safe, and recoverable | Requires lifecycle storage and reconciliation | **Selected in ADR-0177** |

### Refund claim wallet workspace

**Criteria:** make two-signer authority explicit, preserve server-owned financial
inputs, support bounded recovery, and avoid implying refund completion.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Generic checkout panel | Reuses existing UI | Hides refund-funder co-signing and proof boundaries | Rejected |
| Custodial server signing | One-click experience | Adds key custody and removes wallet consent | Rejected |
| Browser-editable refund transaction | Flexible | Allows stale topology and amount substitution | Rejected |
| **Dedicated lazy two-signer workspace with memory-only artifact recovery** | Explicit review, role binding, accessible failure states, and no financial authority in the browser | Requires sequential wallet actions | **Selected in ADR-0178** |

### Durable refund-execute intent

**Criteria:** preserve claim/execute authority separation, bind execution to
finalized claim evidence and current verified topology, support safe wallet
recovery, and make proof-pending state explicit.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Reuse the claim intent | Less persistence and UI work | Collapses four-eyes authority and lifecycle evidence | Rejected |
| Compile from mutable current configuration | Simple recovery | Configuration rotation can silently change movement topology | Rejected |
| Trust signature/provider acknowledgement/event | Low latency | No finalized transaction-bound conservation proof | Rejected |
| Server custody of the funder key | Fewer wallet prompts | Violates non-custody and signer separation | Rejected |
| **Separate immutable execute intent with exact signed verification and proof-gated reconciliation** | Explicit authority, replayability, bounded retries, and fail-closed completion | Additional storage, API, UI, and reconciliation implementation | **Selected in ADR-0179** |

### Refund-claim finality reconciliation

**Criteria:** bind the persisted submission signature to the claim actually
created on Solana, retain forensic evidence, and unlock execute preparation only
after a bounded finalized proof.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Trust provider acknowledgement | Lowest latency | Not finalized evidence; ambiguous outcomes are unsafe | Rejected |
| Read the finalized claim PDA only | Simple account query | Does not bind state to the persisted transaction signature | Rejected |
| Accept browser-supplied recovery inputs | Flexible UX | Caller-controlled signature/address and substitution risk | Rejected |
| **Bounded finalized recovery plus exact raw claim-created event matching** | Signature binding, replayability, topology checks, forensic bytes, and restart-safe worker convergence | Requires event decoding, bounded RPC work, and operational backlog evidence | **Selected in ADR-0180** |

### Typed finalized refund-claim evidence boundary

**Criteria:** expose useful finalized-event proof to operators without moving
authority into the browser, weakening canonical wire validation, or implying
legal/tax compliance.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Status, signature, and slot only | Small response | The UI cannot show the exact event proof and may overstate finality | Rejected |
| Browser-decoded raw event bytes | Preserves raw data | ABI coupling and client-side authority risk | Rejected |
| Browser-supplied evidence | Flexible recovery | Evidence substitution and broken server ownership | Rejected |
| **Server-derived typed finalized-event projection with strict client validation** | Stable schema, proof binding, progressive disclosure, fail-closed parsing | Additional API contract and test surface | **Selected in ADR-0213** |

### Refund-claim reconciliation concurrency

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| No lease; rely only on finalization CAS | Minimal schema and simple retries | Duplicate replicas repeat RPC recovery and amplify provider load | Rejected |
| Process-local single-flight | Low local overhead | Does not coordinate replicas or survive restart | Rejected |
| Long database lock across RPC recovery | Strong serialization | Holds a database connection/lock across external I/O and can block unrelated work | Rejected |
| **Short PostgreSQL lease with `SKIP LOCKED`, TTL reclaim, and final CAS** | Cross-replica bounded ownership, restart recovery, no lock across RPC, safe duplicate convergence | Requires lease fields, metrics, and stale-lease tests | **Selected in ADR-0180** |

### Refund-funding topology verification concurrency

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Status transition only; allow duplicate provider reads | Minimal schema and simple retries | Replicas amplify finalized RPC traffic and stale workers can race activation | Rejected |
| Hold a PostgreSQL row lock across RPC verification | Serializes one configuration | Holds a pool connection across network I/O and can block governance or activation | Rejected |
| Redis/advisory lock | Low-latency coordination | Non-authoritative or weakly inspectable ownership; activation still needs durable fencing | Rejected |
| **Short PostgreSQL UUIDv7 lease with stale reclaim, explicit failure release, and token-fenced activation** | Durable cross-replica ownership, bounded crash recovery, no lock across RPC, exact evidence binding | Requires migration and lease telemetry | **Selected in ADR-0181** |

### Migration rollback rehearsal

**Criteria:** preserve immutable migration history, recover financial evidence
to a known pre-deploy image, keep the old application/schema compatible, and
prove forward recovery after the rollback decision.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Destructive `down` migrations | Familiar command shape; potentially fast | Can destroy evidence, is hard to make data-safe, and conflicts with immutable migration history | Rejected |
| Restore only | Proves a backup can be opened | Does not prove old-schema compatibility or safe forward recovery | Rejected |
| Corrective forward migration only | Preserves history and supports additive repair | Cannot recover an incompatible deployment without a pre-deploy image | Rejected as the sole contract |
| **Pre-deploy restore plus compatible old-schema read and forward reapply rehearsal** | Covers operator rollback, data preservation, compatibility, and redeployment recovery | Requires an isolated PostgreSQL drill and synthetic migration fixture | **Selected in ADR-0183** |

### Typed finalized adjustment proof envelope

**Criteria:** ensure the browser cannot render finalized evidence from a bare
status, preserve tenant/server authority, keep refresh recovery bounded, and
bind the response to the exact finalized typed event.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Status and signature only | Minimal response | Does not expose proof identity or prevent status-only UI success | Rejected |
| Client-supplied proof fields | Flexible client recovery | Caller-controlled evidence and substitution risk | Rejected |
| Re-query indexed evidence for every browser render | No schema change | Adds latency/availability coupling and no stable proof contract | Rejected |
| **Persist event index/digest and return a server-owned typed proof envelope** | Refresh-safe, replayable, bounded, and explicit | Additive migration and cross-language vectors required | **Selected in ADR-0184** |

### Refund-proof candidate ownership

| Option | Benefits | Costs / risks | Decision |
| --- | --- | --- | --- |
| Read-only unproved cursor | No coordination schema | Duplicate provider work and replica races | Rejected |
| Database lock held across RPC | Strong active exclusion | Couples pool capacity to external latency and stalls | Rejected |
| Redis/distributed lock | Familiar lease primitive | Disposable coordination is not evidence-bound; adds failure authority | Rejected |
| **PostgreSQL UUIDv7 lease with `SKIP LOCKED`, TTL reclaim, and final token CAS** | Durable ownership, bounded crash recovery, no database lock during RPC, stale-worker fencing | Requires additive lease columns, metrics, and contention tests | **Selected in ADR-0185** |
| **Storage-local bounded retries for refund-intent serializable conflicts** | Preserves atomic idempotency/event/outbox writes while absorbing ordinary `40001`/`40P01` contention | Adds bounded retry latency and requires measured retry telemetry | **Selected in ADR-0186** |
| **Refund-execute replay authorization** | Compiled approver public key plus immutable adjustment approver subject | Adds one bounded tenant-scoped lookup on replay/read/submit | **Selected in ADR-0187; avoids comparing Solana keys to OIDC subjects** |
| **Refund-execute contention latency evidence** | Bounded authenticated API samples with p50/p95/p99 and explicit workload metadata | Local-only evidence with a small sample; excludes live wallet/RPC and regional variance | **Selected in ADR-0188** |

### Automated indexer replay recovery contract

**Criteria:** prove that persisted finalized-slot gaps are recovered by the
running indexer, that multiple replicas coordinate through durable leases, that
forward checkpoint state is not mutated by replay, and that provider work stays
bounded and observable.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Unit tests for the pure replay queue only | Fast and deterministic | Does not prove PostgreSQL leases, service wiring, or restart behavior | Rejected as the sole gate |
| Manual SQL plus an operator replay endpoint | Easy to demonstrate | Does not prove automatic recovery or replica coordination | Rejected |
| Broker-owned replay queue | Familiar asynchronous delivery model | Adds another authority, duplicate delivery semantics, and settlement-path coupling | Rejected |
| **Isolated Docker PostgreSQL + RPC fixture + two running indexer replicas** | Exercises durable claim/lease, bounded RPC page recovery, projection persistence, checkpoint isolation, and observable completion | Synthetic chain data and local timing do not replace Linux validator/live-provider SLO evidence | **Selected in ADR-0189** |

### Indexer replay retry-exhaustion contract

**Criteria:** prove that a persistent provider failure cannot create an
unbounded retry loop, that the exact lease is released into explicit
dead-letter state after the configured attempt limit, that the last failure is
observable, and that no forward checkpoint is advanced.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Pure state-machine tests | Fast and deterministic | Misses service polling, SQL lease release, and metric refresh wiring | Rejected as the sole gate |
| Manual insertion of a pre-dead-letter row | Exercises read/alert UI | Bypasses automatic retry and exhaustion behavior | Rejected |
| Unbounded retry with operator cancellation | May eventually recover transient provider faults | Retry storms and no finite incident boundary | Rejected |
| **Two real indexer replicas plus a fixture that fails one finalized RPC method** | Proves bounded automatic attempts, lease cleanup, dead-letter persistence, fixed-cardinality metrics, and checkpoint isolation | Synthetic provider and local timing do not prove production notification delivery | **Selected in ADR-0190** |

### Alertmanager delivery contract

**Criteria:** prove that operational alerts leave Prometheus, preserve bounded
identity and ownership metadata, reach a receiver, remain outside financial
authority, and can be tested reproducibly without a production notification
provider.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Service-local notification calls | Immediate failure proximity | Couples hot paths to notification outages, duplicates policy, and expands the settlement trust boundary | Rejected |
| Prometheus rules with no receiver drill | Lowest operational cost | Proves evaluation only; delivery regressions remain invisible | Rejected |
| Direct Prometheus webhook fan-out | Fewer components | Weak grouping, retry, deduplication, silence, and escalation semantics | Rejected |
| **Prometheus → Alertmanager → approved receiver, with a bounded local Rust fixture for CI** | Standard operational boundary, explicit delivery contract, grouping/deduplication, reproducible evidence, and deployment-owned escalation | Adds an operational component and leaves production receiver selection to deployment | **Selected in ADR-0191** |

### Real-browser vault-reconciliation evidence

**Criteria:** prove the shipped SPA renders server-owned reconciliation
evidence and fails closed on authorization or mismatch without conflating
synthetic UI fixtures with live-chain evidence.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Unit/component tests only | Fast and deterministic | Misses the built SPA, lazy imports, browser focus, and network boundary | Rejected as the sole gate |
| Full live-provider browser test in CI | Closest to production | Requires credentials and unstable external state; mixes UI and chain evidence | Rejected for the deterministic browser gate |
| Fake success page | Low implementation cost | Can hide unsafe success and failure UX | Rejected |
| **Real built SPA with bounded network fixtures and explicit failure assertions** | Reproducible browser behavior, safe synthetic data, and explicit mismatch/authorization evidence | Does not replace validator, wallet, live-provider, or production identity evidence | **Selected in ADR-0192** |

### Server-owned reconciliation freshness

**Criteria:** prevent an old matched observation from being rendered as current
financial evidence while preserving append-only runs, tenant isolation, and a
bounded low-latency read path.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Browser-local timestamp comparison | No API change | Clock skew and inconsistent caller interpretation | Rejected |
| Caller-supplied threshold | Operational flexibility | Each caller can redefine evidence validity | Rejected |
| Mutate historical rows to stale | Simple UI status | Breaks append-only evidence and conflates observation with read-time freshness | Rejected |
| Timestamps with operator interpretation | Minimal schema | Unsafe ambiguity in a financial control surface | Rejected |
| **Database-clock server-owned stale boolean with fixed 90-second policy** | Deterministic, append-only compatible, and explicit | Policy changes require a superseding ADR | **Selected in ADR-0193** |

### Wallet-bound adjustment finality evidence

**Criteria:** prove the local path from server-owned instruction compilation
through real wallet transaction serialization, signed-byte verification,
bounded submission fencing, finalized observation, and exact typed-event proof
binding without credentials or nondeterministic public-RPC state.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Unit verifier tests only | Fast and precise | Does not prove API persistence, provider observation, or lifecycle fencing | Rejected as the sole gate |
| Pre-seeded finalized intent and event rows | Easy to repeat | Bypasses the wallet and submission boundaries that are highest risk | Rejected |
| Live wallet/provider test in public CI | Production-like | Requires secrets and external state; unsafe and nondeterministic | Rejected for the deterministic gate |
| **Real API and PostgreSQL with synthetic finalized RPC/indexer contracts plus real wallet serialization** | Complete local authority-boundary evidence, reproducible and credential-free | Does not close validator, live-provider, KMS, or production-SLO gates | **Selected in ADR-0194** |

### Transaction-bound refund proof evidence

| Criterion | Unit/storage-only tests | Direct proof-row seed | Public/live RPC CI | Real indexer + PostgreSQL + synthetic finalized RPC |
| --- | --- | --- | --- | --- |
| Exercises claim topology resolution | No | No | Yes, but variable | Yes |
| Exercises transaction/event/balance parser | Partial | No | Yes, but variable | Yes |
| Exercises UUIDv7 lease and worker wiring | No | No | No | Yes |
| Reproducible public CI without credentials | Yes | Yes | No | Yes |
| Evidence of live provider agreement/validator execution | No | No | Partial | No |

Selected: the deterministic Docker drill in ADR-0195. It closes the local
service-wiring and proof-persistence boundary without overclaiming live-chain
or legal evidence.

### Document-bound checkout workspace

**Criteria:** preserve the document/aggregate authority firewall, expose the
issuer and scope evidence chain to operators, keep the initial SPA bundle
bounded, and make wallet/finality failure states unambiguous.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Extend aggregate checkout with a document mode | Reuses the existing form and wallet path | Blurs authority boundaries and makes accidental aggregate compilation easier | Rejected |
| Operator-only HTTP workflow | Smallest UI change | Not a supportable, accessible product surface; increases evidence-entry errors | Rejected |
| Browser-side attestation calculation/signing | Low interaction latency | Violates issuer custody and server-owned evidence; unsafe for real money | Rejected |
| **Dedicated lazy document workspace over typed verification, compilation, wallet, and reconciliation boundaries** | Explicit type firewall, evidence-first UX, code-split delivery, and reusable finality controls | Adds a separately tested workspace | **Selected in ADR-0207** |

### Authenticated document-quote workspace

**Criteria:** keep simulation and production quote authority separate, preserve
server-owned jurisdiction and amount semantics, support exact idempotent retry,
and provide a discoverable accessible path into document settlement.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Reuse the simulation form against the production endpoint | Minimal visual duplication | Can accidentally submit caller-owned rule data and blur unattested simulation with persisted evidence | Rejected |
| Hide document quotes behind aggregate quote UI | Fewer workspaces | Conceals document-specific facts, attestation, and line-scope semantics | Rejected |
| API-client-only workflow | No SPA implementation | Undocumented, inaccessible, and operationally error-prone | Rejected |
| **Dedicated authenticated lazy workspace** | Clear authority boundary, exact server evidence, and direct document-checkout handoff | Additional UI and contract tests | **Selected in ADR-0208** |

### Retained Docker integration evidence

**Criteria:** preserve the authoritative integration command semantics, make
success and failure behavior reviewable for a specific commit, avoid leaking
credentials or private evidence, and keep the artifact bounded.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Retain only the CI exit status | Minimal configuration | Loses scenario cardinality and recovery diagnostics | Rejected |
| Upload complete Docker logs and volumes | Maximum raw detail | Noise, sensitive-data risk, and poor public-safety boundary | Rejected |
| Add a second bespoke JSON reporter | Structured output | Duplicates the authoritative gate and can drift | Rejected |
| **Pipe the authoritative bounded integration stream to a retained CI artifact** | Exact command semantics, public-safe output, success/failure review, bounded retention | Requires CI artifact storage | **Selected in ADR-0217** |

### Explicit release integrity hashes

**Criteria:** bind authority-sensitive release bytes to named machine-readable
claims, hash the effective rather than source-only deployment configuration,
preserve one checksum source of truth, and fail before release evidence is
reported complete.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Generic artifact list only | Simple and already available | Consumers must infer program/configuration paths | Rejected |
| Hash source Compose files | Easy to inspect | Does not bind profiles, interpolation, or resolved image configuration | Rejected |
| Detached configuration hash file | Small change | Creates a second source of truth outside the manifest | Rejected |
| **Named manifest integrity fields plus the common checksum set** | Explicit, reproducible, machine-readable, and reviewable | Requires resolved configuration as a bundle input | **Selected in ADR-0218** |

### API JWKS rotation

**Criteria:** preserve zero network and filesystem work on the JWT request
path, rotate identity keys without avoidable process downtime, retain the last
known-good configuration on failure, and make the operational boundary
auditable.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Restart-only key rotation | Simple deployment model | Adds downtime and makes emergency rotation depend on rollout scheduling | Rejected |
| Identity-provider JWKS fetch on every request | Immediate provider changes | Unbounded hot-path latency and external availability dependency | Rejected |
| Periodic background JWKS polling | Automatic rotation | Requires stale-cache, provider-authentication, backoff, and split-brain policy before safe production use | Rejected for this contract |
| Mutable `RwLock` key map | No extra dependency | Adds request-path contention and couples hot-path cost to reloads | Rejected |
| **Immutable key snapshot with atomic file reload on `SIGHUP`** | No request-path I/O, bounded validation, deterministic last-known-good behavior, explicit secret-manager ownership | Requires atomic delivery and signal/runbook discipline | **Selected in ADR-0234** |
