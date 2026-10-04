# ADR-0321: Finalized asset-configuration evidence for route activation

- Status: Accepted
- Date: 2026-10-04
- Deciders: Principal Low-Latency Blockchain Architecture, Settlement Domain, SRE
- Scope: Settlement Route Registry and finalized route-verifier bounded context

## Context

ADR-0320 made the Solana `AssetConfig` PDA the final stablecoin admission
authority, but the route verifier still proved only the merchant system
account, mint, tax vault, and net destination. Activating such a route would
leave PostgreSQL able to advertise a route that had not been proven to carry an
enabled stablecoin configuration. Existing route evidence would also be
ambiguous if its digest did not bind the new account.

The checkout and settlement hot paths must remain a bounded database read and
wallet-signing flow. Asset-account discovery therefore belongs in the already
separate route-verifier provisioning workflow, at finalized commitment, with
no fallback to database configuration or caller-supplied account addresses.

## Decision

Extend settlement-route verification to request exactly five accounts in this
fixed order:

1. merchant system account;
2. configured mint;
3. configured tax-vault token account;
4. configured net-destination token account; and
5. the deterministic asset-admission PDA derived as
   `["asset-config", mint]` under the settlement program.

The fifth account must:

- be non-executable and owned by the settlement program;
- have the exact Anchor `AssetConfig` discriminator and bounded serialized
  layout;
- bind the exact route mint, token program, and mint decimals;
- carry asset kind `stablecoin` and `enabled = true`; and
- carry the PDA bump derived from the canonical seed tuple.

The verifier keeps the existing fixed provider pool, finalized commitment,
deadline, response-size bound, provider-agreement rule, and serializable
activation transaction. `settlement_route_verifications` stores the five
account evidence records. The route-verification digest changes from `v1` to
`v2`, preventing pre-0320 four-account evidence from being replayed as current
activation evidence. Missing, disabled, non-stable, malformed, substituted,
or conflicting asset evidence leaves the route inactive.

This check is provisioning-only. It does not add an RPC call, database write,
loop, or asset classification lookup to quote, compile, wallet-signing, or
settlement-submit paths. The on-chain program remains the final authority and
repeats admission checks during every token movement.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Continue with four-account topology evidence | No verifier change | Active routes could advertise an unproven or disabled asset configuration | Rejected |
| Trust the route mint and database asset flag | Smallest off-chain change | Database state is not Solana authority; direct callers and stale rows bypass it | Rejected |
| Verify `AssetConfig` during quote or compilation | Fresh evidence at use time | Adds RPC latency, provider uncertainty, and an unsafe retry/finality boundary to the hot path | Rejected |
| Store raw `AssetConfig` bytes in PostgreSQL | Easier forensic decoding | Larger sensitive evidence surface and stale bytes can be mistaken for current chain state | Rejected |
| **Verify the exact PDA at finalized commitment and bind bounded metadata/digests into v2 evidence** | Matches on-chain authority, preserves low latency, supports replay/audit, and fails closed | Requires additive verifier/schema evidence update and route re-verification | **Selected** |

## Invariants

- A production-active route has five verified account evidence records.
- Asset evidence binds route mint, token program, decimals, stablecoin kind,
  enabled state, PDA identity, and finalized slot.
- A v1 four-account digest cannot activate or re-activate a route after this
  decision.
- Any missing, disabled, non-stable, malformed, weak-commitment, provider-
  disagreement, or substituted asset account fails closed.
- Route activation and append-only evidence insertion remain one bounded,
  serializable database transition.
- The verifier never authorizes token movement, signs a transaction, or
  replaces the Solana program's movement checks.

## Delivery and verification

- [x] Failing-first indexer tests cover five-account requests, stablecoin
  asset-layout validation, disabled-asset rejection, and v2 digest binding.
- [x] Storage activation requires five account evidence records.
- [x] Route-verification architecture documentation and public traceability
  identify the new evidence contract.
- [ ] Add migration/backfill execution evidence proving every existing active
  route is re-verified before signing is enabled.
- [ ] Add Linux/amd64 validator evidence for initialized, enabled, disabled,
  non-stable, and substituted `AssetConfig` states.
- [ ] Complete independent audit, governance ceremony, and production route
  re-verification before paid-state enablement.

## References

- [ADR-0062: Finalized chain verification before settlement-route activation](0062-finalized-settlement-route-verification.md)
- [ADR-0320: On-chain stablecoin asset admission](0320-on-chain-stablecoin-asset-admission.md)
- [Settlement route registry](../architecture/settlement-route-registry.md)
- [Settlement route verification](../architecture/settlement-route-verification.md)
