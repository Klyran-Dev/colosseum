# Solana Tax Protocol

This directory is the architecture and delivery baseline for a production-grade, low-latency Solana payment-routing protocol. The protocol settles a payment, withholds an attested tax amount into a program-controlled vault, and distributes the net amount to a pre-authorized split plan in one atomic transaction.

## Documents

- [Source analysis and requirement boundary](source-analysis.md)
- [System architecture](architecture/system-architecture.md)
- [Domain model and bounded contexts](architecture/domain-model.md)
- [Tier-1 tax engine](architecture/tax-engine.md)
- [Gold-standard UI/UX](architecture/ui-ux.md)
- [Architecture alternative analysis](architecture/alternative-analysis.md)
- [Settlement transaction and state machine](architecture/settlement-flow.md)
- [Low-latency and reliability architecture](architecture/low-latency.md)
- [Security and threat model](architecture/security-threat-model.md)
- [Operations, observability, and SRE](architecture/operations.md)
- [API contract baseline](api-contract.md)
- [Implementation plan](plans/implementation-plan.md)
- [Tier-1 product standard](plans/tier-1-product-standard.md)
- [Tax coverage matrix](product/tax-coverage-matrix.md)
- [Production-readiness plan](plans/production-readiness.md)
- [Architecture decision records](adr/README.md)

## Status and scope

This is the architecture and implementation baseline for a system still in its foundation phase. It does not claim that tax treatment is legally correct for every jurisdiction, that an oracle is a tax authority, or that escrow funds can be withdrawn without the configured governance and release controls. Legal, tax, custody, and licensing review are release gates. The current executable foundation is deliberately limited to the deterministic tax engine, authenticated persisted published-rule quote path with an atomic outbox event, bounded NATS JetStream dispatcher, finalized-indexer/reconciliation domain and coordinator plus PostgreSQL persistence slice, isolated preview API, authenticated quote UI, PostgreSQL migration, product shell, and a governed token-interface settlement contract with cached policy-attestation binding. It is not yet a production settlement deployment.

The local Agave validator is available through the resource-heavy `solana`
Compose profile: `make solana-up`. It uses a pinned Agave 4.3.0 release
installer and synthetic local state; it is not a testnet or mainnet node. The
profile requires a Linux runtime exposing the `io_uring` capabilities required
by Agave 4.3.0; image build success alone does not prove validator start-up.

Run `make integration-check` for the authenticated Docker quote and delivery
path. The harness generates an ephemeral Ed25519 signing key and removes its
isolated Compose volume on exit; no development private key is stored in the
repository. The check also proves that the outbox worker publishes the single
transactional event and only then marks it published in PostgreSQL.

The first production scope is stablecoin checkout on Solana. Non-stable assets, swaps, fiat off-ramp execution, and jurisdiction-specific legal determinations are separate adapters and are not implicit parts of the settlement hot path.

## Toolchain baseline

The baseline is recorded as of 2026-09-25 and must be reproduced from lockfiles and CI images:

| Area | Baseline | Policy |
| --- | --- | --- |
| Rust | Stable 1.98 | Pin in `rust-toolchain.toml`; upgrade only through a compatibility PR. |
| Solana/Agave CLI | 4.3.0 stable | Pin the exact release; do not use `stable`, `edge`, or alpha channels in CI. |
| Solana programs | Rust + Anchor v1 templates | Pin the Agave/Solana CLI and Anchor with AVM; do not build programs in Solidity. |
| Anchor CLI | 1.2.0 documentation baseline | Verify the exact compatible release during bootstrap and commit the chosen version. |
| React | 19.3 | Use a Vite SPA; no Next.js and no server components. |
| Node.js | 24.21.0 LTS | Build tooling only; pin the exact patch release and package-manager lockfile. |
| PostgreSQL | 18.x | Latest security patch for major 18; native `uuid` storage and UUIDv7 for sortable internal IDs. |
| Solidity | 0.8.37 | Optional EVM adapter only; separate package, chain, threat model, and audit. |
| NGINX | 1.30.5 stable | Serve the compiled SPA and reverse-proxy non-blocking APIs; keep the transaction path out of NGINX. |

The official version references are listed in [implementation plan](plans/implementation-plan.md#version-and-dependency-governance).
