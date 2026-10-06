# ADR-0320: On-chain stablecoin asset admission

- Status: Accepted
- Date: 2026-10-04
- Deciders: Principal Low-Latency Blockchain Architecture and Settlement Domain
- Scope: Asset bounded context, Solana settlement, vault release, revenue-stream release, and refund movement

## Context

ADR-0005 restricts the v1 tax-reserve hot path to approved stablecoins. The
current settlement program validates the token program, mint owner, account
layout, decimals, and account bindings, but those checks do not establish that
the mint is an approved stablecoin. A tenant-scoped database route alone is not
an authority boundary: a direct caller could construct a valid instruction for
another mint, and an off-chain route verifier cannot make the Solana program
trust its own projection.

The attached evaluation describes stablecoin tax reserves as a product
constraint, while its generated competition and regulatory commentary is not a
protocol requirement. The implementation must therefore enforce the narrow
stablecoin decision without accepting the PDF's claims of legal compliance as
authority.

## Decision

Add a mint-scoped `AssetConfig` PDA to the Solana settlement program:

```text
seeds = ["asset-config", mint]
```

The immutable identity fields are the mint, token-program owner, and verified
mint decimals. The configuration also contains an immutable asset-kind value
(`stablecoin` for the v1 movement path), a governance authority, an enabled
flag, and the PDA bump. The protocol authority may disable an existing asset
for emergency containment; it may not mutate the mint, token program,
decimals, or stablecoin classification. Re-enabling uses the same immutable
configuration and remains subject to the normal pause and governance policy.

The program will require the enabled `AssetConfig` in every instruction that
can move reserve or payout tokens: aggregate settlement, document settlement,
Split Plan settlement, vault release, revenue-stream claim execution, and
adjustment-refund execution. The helper will re-check the account's mint,
token-program owner, decimals, stablecoin kind, enabled state, and PDA-derived
identity at the movement boundary. Initialization, policy, and claim-record
instructions may remain preparatory, but no token movement can bypass asset
admission.

The off-chain settlement-route registry and route verifier will carry the
server-owned `asset_config` PDA and its finalized account evidence. The API
compiler and browser envelope will require the exact PDA in canonical account
order; the browser may never supply a replacement asset identity. Existing
route activation remains fail-closed until both route topology and the asset
configuration are verified at the required commitment. A synthetic marker is
allowed only in Docker fixtures.

The stablecoin kind is a governed classification, not a market-price oracle or
a legal determination. Issuer depeg, freeze, redemption, sanctions, and fiat
off-ramp risk remain separate operational and legal controls. Non-stable assets
and swaps require a new bounded adapter and a separate ADR; they cannot reuse
the v1 movement instructions.

## Invariants

- No v1 token movement succeeds without an enabled stablecoin `AssetConfig`.
- `AssetConfig.mint`, token-program owner, and decimals match the actual mint
  account and the token accounts used by the instruction.
- Stablecoin classification is immutable for a PDA identity; changing it means
  a new governed configuration and new route version.
- Disabling an asset blocks new movement but does not fabricate cancellation or
  alter finalized receipts.
- The API, browser, and database are projections and admission aids; the
  Solana program remains the final movement authority.
- Asset admission adds one bounded read-only account and one fixed validation
  step; it adds no network call, loop, floating-point operation, or mutable
  shared counter to the settlement hot path.
- Account order, instruction bytes, wire budget, signed-message verification,
  and cross-language golden vectors are versioned together.

## Alternatives considered

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Off-chain route allow-list only | Smallest change and easy operations | Direct Solana callers bypass it; database state is not movement authority | Rejected |
| Hard-code USDC/USDT mint addresses in the program | Very small runtime check | No governed rotation, network/issuer ambiguity, poor multi-tenant extensibility, upgrade pressure | Rejected |
| Infer stablecoin status from symbol, metadata, freeze authority, or price | No new PDA | Metadata and market state are mutable or unavailable on-chain; unsafe false positives | Rejected |
| Per-merchant mutable asset lists in every settlement account | Flexible tenant policy | Larger hot accounts, mutable shared state, extra contention and account metas | Rejected |
| **Mint-scoped immutable `AssetConfig` PDA plus server-verified route evidence** | Solana-native final authority, bounded account read, governed classification, deterministic compiler and audit trail | Requires additive ABI/IDL update, migration, and Linux validator evidence | **Selected** |

## Delivery and compatibility

1. Add failing-first domain, program, compiler, storage, and browser-envelope
   tests for missing, disabled, wrong-mint, wrong-program, wrong-decimals, and
   non-stable configurations.
2. Add the PDA initializer/disable instruction and bounded account validation;
   update all movement contexts and the typed IDL/compiler contracts.
3. Add the typed PostgreSQL asset projection and route-verifier evidence
   binding with expand/contract migration behavior. Existing active routes
   become blocked until their typed asset evidence is backfilled and verified;
   no silent default is allowed.
4. Add legacy-SPL and extension-free Token-2022 positive vectors, non-stable
   and disabled negative vectors, signed-message substitution tests, and the
   real Linux validator matrix.
5. Enable paid-state movement only after the validator, program audit,
   governance ceremony, and route backfill evidence are retained.

The change is intentionally not a compatibility shim that treats missing asset
data as stable. During rollout, the API returns an explicit unavailable or
re-verification-required state and the UI keeps signing disabled.

## Consequences

The v1 reserve path becomes enforceably stablecoin-only at the ledger boundary,
which matches ADR-0005 and the source problem statement. Every route and
movement compiler must carry one additional read-only PDA, so the wire and
compute budgets need measured updates. The migration is additive, but old
prepared artifacts are not safe to submit after the ABI version changes and
must be rejected by the signed-message verifier.

## Verification evidence

- Rust program tests prove the asset PDA identity and all fail-closed cases.
- Settlement-client and TypeScript vectors prove account order, bytes, and
  stablecoin metadata binding.
- The real-validator TypeScript conformance harness provisions the
  mint-scoped configuration before legacy-SPL and extension-free Token-2022
  movement, supplies the PDA in every movement account list, proves disabled
  movement leaves balances unchanged, rejects non-conserving gross/tax/net
  inputs without balance drift, and rejects a TransferFeeConfig mint before
  route activation. The harness remains disabled unless an explicit local
  validator is available.
- PostgreSQL migration tests prove no active route can omit asset evidence.
- Docker integration proves tenant isolation and re-verification behavior.
- Linux/amd64 validator conformance proves real SPL and extension-free
  Token-2022 movement and unchanged balances on rejection.

## Initial implementation slice

The first reviewable slice is now present in source control. The Solana
program defines `AssetConfig`, its mint-scoped PDA initializer, governed
enable/disable operation, and fail-closed admission checks on aggregate,
document, Split Plan, vault-release, revenue-stream, and adjustment-refund
movement contexts. The settlement-client and browser envelope add the PDA as
the thirteenth bounded read-only account for aggregate/document settlement and
as the fixed account ten for Split Plan settlement; signed-message checking
uses the same account contract. Focused Rust, API, coordinator, and browser
tests pass.

This slice does not yet claim production stablecoin support. Finalized
route-verifier evidence, typed PostgreSQL projection, migration constraints,
and Docker integration checks are present locally. Representative production
backfill/re-verification, successful Linux/amd64 execution of the real
validator harness, non-stable/substitution/tenant-isolation negatives, audit,
and governance evidence remain required. Until those are complete, missing
asset evidence must keep signing unavailable.

## References

- [ADR-0005: Restrict the settlement hot path to approved stablecoins](0005-stablecoin-hot-path.md)
- [ADR-0064: Browser settlement envelope validation](0064-browser-settlement-envelope-validation.md)
- [ADR-0065: Localnet token-interface conformance](0065-localnet-token-interface-conformance.md)
- [Source analysis and requirement boundary](../source-analysis.md)
