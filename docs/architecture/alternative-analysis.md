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

### Chain indexing and reconciliation

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Finalized RPC subscription signal plus bounded slot-range backfill** | Works with standard Solana RPC, repairs missed notifications, explicit commitment semantics, straightforward local testing | Requires checkpoint persistence, provider failover, and bounded backfill queues | **Selected for the first indexer slice** |
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

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Docker Compose with pinned stable images/digests** | Reproducible local topology, low onboarding cost, production-like boundaries | Desktop resource use, Compose is not production orchestration | **Selected for local development** |
| Kubernetes locally | Production-like orchestration | High complexity and slow feedback | Rejected for default local workflow |
| Host-installed toolchains | Fast for experts | Drift, onboarding failure, inconsistent versions | Rejected as default |
| Floating `latest` image tags | Easy updates | Non-reproducible and unsafe | Rejected; resolve latest stable then pin digest |

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
