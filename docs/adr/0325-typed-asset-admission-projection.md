# ADR-0325: Typed asset-admission projection for settlement routes

- Status: Accepted
- Date: 2026-10-04
- Deciders: Principal Low-Latency Blockchain Architecture, Settlement Domain, SRE
- Scope: Settlement route registry and route-verifier persistence

## Context

ADR-0320 and ADR-0321 make the mint-scoped Solana `AssetConfig` PDA the
movement authority and require finalized five-account route evidence. The
route table currently persists `asset_config` only inside the opaque JSON
evidence document. That is adequate for archival inspection, but it is not a
typed operational projection: SQL constraints and bounded route reads cannot
prove that the stored asset identity, stablecoin classification, enabled
state, and PDA bump are present and consistent without decoding untrusted JSON.

The projection must improve queryability and fail-closed database behavior
without becoming a second authority. It must also be safe during rolling
deployment: old v2 evidence that lacks the new fields must not remain
signable, historical evidence must not be rewritten, and the quote/compile
hot path must not add an RPC call or a JSON parse.

## Decision

Add four nullable, typed columns to `settlement_routes`:

- `asset_config` — the canonical mint-scoped PDA address;
- `asset_kind` — the bounded protocol classification, currently
  `stablecoin = 1`;
- `asset_enabled` — the finalized on-chain enabled state;
- `asset_config_bump` — the PDA bump proved by the finalized account.

Route activation writes these columns and the raw bounded evidence in one
serializable transaction. The route-verifier result exposes the same values
as typed fields, while the existing account evidence and verification digest
remain retained for replay and forensic review. The digest algorithm is not
changed because the account data digest already binds the complete
`AssetConfig` bytes; the typed fields are a projection of that proof.

The migration backfills only the `asset_config` address when it is present in
canonical v2 evidence, then suspends every active route that cannot prove all
four typed fields. New constraints require an active route to have the
stablecoin kind, `asset_enabled = true`, a bounded PDA address, and evidence
fields equal to the typed projection. Legacy evidence remains append-only and
is never rewritten to manufacture missing facts. New activation is the only
path that can populate the complete projection.

The Solana program remains authoritative. Database columns are a bounded
admission projection and operational read model; they never authorize token
movement or replace the on-chain `AssetConfig` checks. Local synthetic rows
may use the explicit `local_synthetic` commitment only in Docker fixtures.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep all asset fields only in JSON evidence | No migration or code changes | Weak SQL invariants, expensive/ambiguous operational queries, easier direct-SQL drift | Rejected |
| Normalize a separate asset table keyed by mint | Reusable across contexts | Creates a second lifecycle and join authority; complicates tenant/version binding and migration | Rejected |
| Store raw `AssetConfig` bytes as the projection | Strong forensic fidelity | Larger sensitive surface; operators can confuse stale bytes with current authority; no typed route checks | Rejected |
| **Typed route columns plus retained immutable evidence** | Bounded queries, direct constraints, simple rollback, preserves audit evidence, no hot-path dependency | Additive migration and re-verification of rows lacking explicit fields | **Selected** |

## Invariants

- A production-active v2 route has non-null typed asset fields.
- An active route has `asset_kind = 1` and `asset_enabled = true`.
- `asset_config` equals the deterministic PDA derived from the route mint and
  settlement program; its evidence field equals the typed column.
- Typed fields equal the corresponding fields in the route's immutable
  verification evidence.
- Existing evidence is never rewritten or upgraded by inference.
- A route without complete typed proof is suspended or remains pending and is
  unavailable to all signing and release selectors.
- The quote, compile, wallet-signing, and submit paths perform no RPC call and
  no unbounded evidence parse because they consume the already validated
  route projection.
- Solana movement instructions still re-check the live `AssetConfig` PDA.

## Delivery and verification

- [x] Add alternatives, invariants, and the expand/contract migration design.
- [x] Add typed verifier fields and storage activation binding.
- [x] Add database constraints, mutation guards, and synthetic fixture data.
- [x] Add failing-first migration and route-verifier tests for missing,
  mismatched, disabled, and non-stable projections.
- [ ] Run representative production snapshot backfill and retain suspended /
  re-verification counts.
- [ ] Complete Linux/amd64 validator evidence, independent audit, governance,
  and non-local route re-verification before paid-state enablement.

## References

- [ADR-0320: On-chain stablecoin asset admission](0320-on-chain-stablecoin-asset-admission.md)
- [ADR-0321: Finalized asset-configuration route evidence](0321-finalized-asset-config-route-evidence.md)
- [ADR-0322: Expand/contract route evidence migration](0322-route-evidence-version-migration.md)
