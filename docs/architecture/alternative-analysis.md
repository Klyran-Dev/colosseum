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
