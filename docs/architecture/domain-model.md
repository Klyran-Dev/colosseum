# Domain model and bounded contexts

## Bounded contexts

| Context | Owns | Does not own |
| --- | --- | --- |
| Merchant | merchant identity, accepted assets, policy issuer set, default split plan | legal tax interpretation or private keys |
| Policy | canonical tax parameter attestations, issuer keys, validity, sequence, revocation | payment balances or recipient payouts |
| Tax Engine | normalized tax facts, jurisdiction resolution, rule graphs, calculations, explanations, evidence | wallet custody or on-chain authority |
| Settlement | quote-to-receipt lifecycle and atomic accounting rules | fiat settlement or database projections |
| Split plan | ordered recipients, basis points, fixed/variable constraints, version | tax policy |
| Vault | segregated reserves, release claims, release governance | how a tax amount was legally calculated |
| Asset | mint metadata, token program, decimals, stablecoin status, extension risk | arbitrary token acceptance |
| Settlement Route Registry | tenant-scoped merchant/mint/vault/destination/token configuration, route versions, activation lifecycle | payer keys, tax interpretation, funds, on-chain ownership proof |
| Route Verification | finalized account topology proofs, bounded provider agreement, activation evidence | route authoring, tax interpretation, funds, program governance |
| Refund Movement | governed claim/execute intents, wallet artifacts, signer separation, nonce fencing, movement reconciliation | tax-rule authoring, wallet custody, finality without chain proof |
| Reconciliation | raw events, balance snapshots, status projections, gap detection | on-chain authority |

## Core aggregates and accounts

| Aggregate/account | Suggested PDA seeds | Invariants |
| --- | --- | --- |
| MerchantConfig | `merchant`, merchant key | authorized policy issuers, accepted assets, pause mode, config version |
| AssetConfig | `asset`, mint | expected token program, decimals, stablecoin status, enabled flag |
| PolicyAttestation | `policy`, merchant, policy id, sequence | canonical digest, rate bounds, effective window, issuer/sequence validity |
| SplitPlan | `split`, merchant, plan id, version | recipient count bound, basis points sum, no duplicate recipient/asset route |
| TaxVault | `vault`, merchant, asset | PDA authority, known asset, release policy, balance accounting |
| SettlementReceipt | `receipt`, merchant, client nonce | unique nonce, gross/net/tax conservation, policy digest, transaction identity |
| ReleaseClaim | `release`, vault, claim id | single-use claim, authorized destination, amount <= available reserve |
| RuleSet | database identity + immutable content hash | one effective version, provenance, approval state, no overlapping published versions |
| TaxCalculation | calculation id, input/rule digests | deterministic output, complete trace, replayable result |
| SettlementRoute | route id, tenant, account tuple, version, lifecycle/effective window | immutable active account fields, supported programs, distinct token accounts |

## Value objects

- `MintAddress`: validated public key plus expected token-program owner.
- `AtomicAmount`: unsigned integer in mint base units; no floating point.
- `BasisPoints`: bounded integer, with explicit denominator (10,000).
- `PolicyWindow`: `effective_from`, `effective_until`, and issuer sequence.
- `ClientNonce`: merchant-scoped 128-bit or 256-bit random value used once.
- `EntityId`: UUIDv7 for internal distributed identifiers; stored as PostgreSQL `uuid` and never interpreted as business time.
- `PolicyDigest`: SHA-256/Keccak-compatible canonical bytes chosen by the schema version; never hash language-specific serialization.
- `RuleDigest`: immutable digest of the normalized rule graph, source provenance, and semantic version.
- `CalculationDigest`: digest of normalized facts, rule digest, engine version, and output trace.
- `SettlementId`: deterministic correlation identifier derived from merchant and client nonce.

## Accounting invariants

For every successful settlement in one asset:

```text
gross = tax_withheld + protocol_fee + sum(recipient_net_amounts)
0 <= each amount <= u64::MAX
tax_withheld <= gross
protocol_fee <= gross - tax_withheld
sum(split_bps) = 10_000
```

If a split plan cannot represent the remaining atomic units exactly, the instruction rejects. Rounding direction is explicit and stable; no residual is silently retained by the protocol.

## DDD module shape

```text
crates/
  domain/
    merchant/
    policy/
    settlement/
    split_plan/
    vault/
    asset/
    refund_movement/
  application/
    compile_settlement/
    submit_settlement/
    reconcile_settlement/
  adapters/
    solana_rpc/
    postgres/
    policy_signer/
    telemetry/
  api/
  indexer/
programs/
  settlement_program/
```

The same bounded contexts should be reflected in frontend features and API routes. Shared code should contain contracts and primitives, not an all-purpose “utils” module.
