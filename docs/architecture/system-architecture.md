# System architecture

## Executive design

The system is a Solana-native settlement protocol with a thin Rust integration plane and a React SPA. The critical path is a signed client transaction that invokes one Solana program instruction. The API prepares quotes, policy references, split plans, tenant-scoped settlement routes, and transaction messages; it does not custody payer funds or decide tax law.

```text
                         control plane
  Policy issuer ---> signed policy envelope ---> Policy Registry PDA
       |                                         Split Plan / Asset Registry PDAs
       v
  Tax Engine ---> signed CalculationAttestation ---> Rust API / transaction compiler
                                                        <----> PostgreSQL projections + outbox
       ^              |                         ^
       |              | RPC / WebSocket / Geyser| event ingestion
       |              v                         |
  React 19 SPA ---> signed Solana transaction --> Solana RPC / gateway
                                                    |
                 settlement_program (Rust/Anchor) |
                  |                                |
          Token Program / Token-2022              |
          Tax Vault PDA + net recipient ATAs <-----+

  NGINX: static SPA, TLS, security headers, API reverse proxy, rate limits.
  It is not a custody layer and is not in the authoritative settlement path.
```

## Components

### 1. `settlement_program`

Rust/Anchor program responsible for:

- merchant configuration and authorized policy issuers;
- asset registry and stablecoin allow-list;
- immutable or versioned split plans;
- policy attestation reference and freshness checks;
- amount conservation and fee/withholding calculation;
- CPI transfers to the tax vault and recipient token accounts;
- replay protection and settlement receipts;
- pause controls limited to emergency containment, with defined behavior for already-signed transactions;
- structured events for indexing and reconciliation.

The program owns no business logic that requires external HTTP, floating-point math, or current market data.

### 2. Rust transaction compiler/API

An Axum/Tokio-style Rust service, organized by DDD bounded contexts, that:

- validates merchant requests and authorization;
- resolves merchant-side settlement accounts through the authenticated
  tenant-scoped route registry and derives the payer's canonical associated
  token account without an RPC lookup;
- resolves a current policy reference from a local cache;
- derives and validates all PDAs and associated token accounts;
- returns a deterministic `prepared_unsubmitted` Anchor instruction with a
  canonical account-meta order;
- compiles a versioned transaction with minimal account metas;
- accepts only canonical wallet-signed bytes at submission, uses the bounded
  non-custodial coordinator, and persists UUIDv7 intent/lifecycle evidence;
- simulates before submission when latency budget permits;
- selects the RPC path, blockhash, compute limit, and priority fee;
- accepts client signatures without ever receiving private keys;
- tracks transaction lifecycle and exposes an idempotent status API.

This service is an adapter and projection layer. It cannot mint, transfer, or release funds by itself.

### 3. Policy issuer and registry

The policy issuer is an independently governed service or signer set. Its output is a canonical policy attestation. The on-chain registry stores the issuer set, policy digest, effective window, and revocation/sequence metadata. The settlement program rejects expired, future, revoked, wrong-asset, wrong-merchant, or wrong-product attestations.

### 4. Tier-1 Tax Engine

The Tax Engine is a separate bounded context and pure Rust domain library. It normalizes transaction facts, resolves jurisdiction inputs, compiles approved rule graphs, evaluates tax components, produces a complete calculation trace, and signs a `CalculationAttestation`. It is the authoritative calculator for the quote; the Solana program is the authoritative settlement executor.

The engine must be deterministic, replayable, bounded, and fail closed on ambiguous rules. It supports progressive brackets, tax-on-tax dependencies, inclusive/exclusive prices, thresholds, exemptions, credits, min/max, caps/floors, truncation, and explicit rounding scope. It never performs arbitrary network calls during calculation.

### 5. Tax vault

Each supported stablecoin has a program-controlled vault authority. Vault balances are segregated by merchant, asset, and policy domain. A release instruction must satisfy the configured release policy, which may include recipient allow-list, claim/reference, time window, governance threshold, and idempotent release nonce.

### 6. Event indexer and reconciliation worker

The indexer consumes program logs/events and confirmed transactions, writes an append-only raw event table, and builds read models. It replays from a checkpoint, detects gaps, and reconciles balances from chain state. WebSocket notifications are an acceleration signal, never the final authority.

### 7. Settlement route verifier

The route verifier is a separate provisioning service. It reads pending or
explicitly re-verifiable routes, queries a bounded RPC provider pool with
finalized `getMultipleAccounts`, validates the merchant System account, mint,
extension-free token-account layouts, token-program owners, mint bindings, and
vault-authority PDA, then atomically persists append-only evidence and promotes
the route. It is never called from checkout, quote, wallet signing, or
settlement submission. Provider failure and conflicting finalized evidence
leave the route inactive.

### 8. React SPA and NGINX

React 19.3 + TypeScript + Vite provides checkout, tax explanations, merchant configuration, rule authoring, simulation, approvals, settlement history, and vault reporting views. The browser signs with a wallet adapter. NGINX serves immutable static assets, terminates TLS, adds security headers, and reverse-proxies the API. No Next.js runtime is required or permitted.

## Design principles applied

- DDD: policy, settlement, vault, split-plan, asset, and reconciliation are separate bounded contexts.
- SRP: the on-chain program settles; the policy issuer attests; the API compiles; the indexer projects; NGINX delivers.
- DRY: canonical policy and amount schemas are generated into Rust and TypeScript from one versioned schema/IDL boundary.
- SOLID: domain ports are traits/interfaces; RPC, storage, signer, and clock implementations are adapters.
- Explicit side effects: chain submission, database writes, and external notifications are isolated behind ports.
- Immutable evidence: settlement receipts and raw chain events are append-only; projections can be rebuilt.
- Fail closed: missing, stale, ambiguous, or mismatched policy/account state rejects settlement.
