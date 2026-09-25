# Solana Tax Protocol

Production-grade architecture for a low-latency Solana payment-routing and Tier-1 tax platform. The system calculates complex tax rules off-chain, produces replayable signed attestations, atomically withholds approved tax amounts into governed stablecoin vaults, and distributes net revenue according to versioned split plans.

## Project status

This repository is currently in the architecture and design phase. It does not claim production deployment, legal/tax coverage, audit completion, or mainnet readiness. Implementation is gated by the design, TDD, security, performance, and operational requirements in [AGENTS.md](AGENTS.md).

## Architecture

```text
React + Vite SPA -> NGINX -> Rust API / Tax Engine / Indexer
                                      |
                              signed calculation attestation
                                      |
                              Solana settlement program
                              |       |        |
                         tax vault  splits  receipts/events
```

The Solana core uses Rust/Anchor-compatible interfaces. The Tier-1 Tax Engine is a deterministic Rust bounded context supporting progressive, cascading, tax-on-tax, threshold, exemption, min/max, truncation, and rounding rules. Solidity is isolated to an optional EVM adapter. Next.js is intentionally not used.

## Documentation

- [Documentation index](docs/README.md)
- [Tier-1 product standard](docs/plans/tier-1-product-standard.md)
- [Implementation plan](docs/plans/implementation-plan.md)
- [Production-readiness gates](docs/plans/production-readiness.md)
- [Tax coverage matrix](docs/product/tax-coverage-matrix.md)
- [Architecture and alternatives](docs/architecture/alternative-analysis.md)
- [Architecture decision records](docs/adr/README.md)
- [Source analysis and requirement boundary](docs/source-analysis.md)
- [Security policy](SECURITY.md)
- [Contributing](CONTRIBUTING.md)

## Engineering contract

All implementation work follows [AGENTS.md](AGENTS.md): design before code, alternatives before decisions, TDD, production-quality vertical slices, bounded concurrency, measurable latency, Docker-based development, explicit security controls, and no demo-only paths.

## Public repository status

The repository will publish executable bootstrap and test commands when Phase 1 implementation begins. Until then, documentation is intentionally honest about what is and is not implemented. Do not infer production support from architecture documents alone.
