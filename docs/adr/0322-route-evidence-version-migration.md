# ADR-0322: Expand/contract migration for stablecoin route evidence

- Status: Accepted
- Date: 2026-10-04
- Deciders: Principal Low-Latency Blockchain Architecture, Data Architecture, SRE
- Scope: Settlement Route Registry schema and activation read model

## Context

ADR-0321 changes the finalized settlement-route topology from four accounts to
five by adding the on-chain `AssetConfig` PDA. Existing deployments may
contain active rows and append-only evidence created before that boundary. A
new verifier alone is insufficient: legacy active rows could remain visible
to checkout while carrying no proof of the enabled stablecoin configuration.

The migration must preserve historical evidence, avoid silently treating old
evidence as current, and keep the checkout read path bounded. It must also be
safe to roll forward in a rolling deployment where old and new service
processes briefly coexist.

## Decision

Use an expand/contract migration with an explicit route evidence schema:

1. Add nullable `settlement-route-verification/v2` schema metadata to route
   projections and a legacy-compatible schema marker to historical evidence.
2. Change the activation constraint so only an active route with v2 evidence
   can be returned or persisted as active.
3. In the same migration, transition existing active rows without v2 evidence
   to `suspended` while preserving their immutable historical evidence. This is
   a fail-closed operational state, not a deletion or data rewrite.
4. Deploy the v2 verifier/compiler reader and writer. New activation records
   persist the v2 schema and explicit derived `asset_config` PDA identity in
   one serializable transaction with evidence insertion.
5. Re-verify suspended routes through the bounded finalized verifier. Only
   after that operation may a route return to active.

The old reader is allowed to continue reading pending/suspended rows during a
rolling deployment, but it must not expose an active route that the database
marks as lacking v2 evidence. The migration is therefore additive first,
fail-closed second, and cleanup/`NOT NULL` tightening is deferred until the
backfill evidence has been retained and reviewed.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Leave old active rows untouched | No migration downtime | Unsafe legacy routes remain signable without asset proof | Rejected |
| Rewrite old evidence as v2 | Keeps rows active | Forges evidence and destroys provenance; cannot prove the asset account existed | Rejected |
| Delete legacy evidence/routes | Simple state model | Irrecoverable audit loss and breaks replay/support | Rejected |
| Block only in the browser/API | Fast rollout | Direct API/SQL and stale service instances can still advertise unsafe state | Rejected |
| **Add schema metadata, suspend legacy active rows, and re-verify into v2** | Preserves history, fails closed, supports rolling deployment and auditable backfill | Requires explicit operator re-verification and temporary suspended routes | **Selected** |

## Invariants

- An active route has `settlement-route-verification/v2` evidence.
- Legacy evidence remains append-only and queryable but cannot authorize
  checkout or settlement compilation.
- Migration never invents asset-account evidence or changes historical bytes.
- Route activation writes schema, asset PDA identity, digest, slot, commitment,
  and bounded evidence atomically.
- A failed or incomplete backfill leaves the route suspended and signing
  unavailable.
- The hot path performs no RPC discovery and no migration work.

## Delivery and verification

- [x] Add the v2 schema and asset-PDA fields to the verifier domain evidence.
- [x] Add the PostgreSQL expand/contract migration and fail-closed active-route
  read/activation checks.
- [ ] Run the migration against a representative production snapshot and retain
  the suspended-row and re-verification counts.
- [ ] Re-verify every non-local route at finalized commitment and retain the
  evidence bundle before enabling signing.
- [ ] Complete backup/restore, rollback rehearsal, independent audit, and
  operator runbook review.

## References

- [ADR-0320: On-chain stablecoin asset admission](0320-on-chain-stablecoin-asset-admission.md)
- [ADR-0321: Finalized asset-configuration evidence](0321-finalized-asset-config-route-evidence.md)
- [ADR-0138: Versioned database migration runner](0138-versioned-database-migration-runner.md)
- [Settlement route registry](../architecture/settlement-route-registry.md)
