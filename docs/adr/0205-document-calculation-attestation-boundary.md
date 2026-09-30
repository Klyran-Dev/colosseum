# ADR-0205: Dedicated document-calculation attestation boundary

## Status

Accepted for implementation.

## Context

ADR-0204 introduced an authenticated, persisted document quote, but the
existing `CalculationAttestation` contract is intentionally aggregate and is
used by Solana settlement compilation. Reusing that type for a document would
allow an aggregate attestation endpoint or settlement compiler to accept a
document calculation identifier, even though the current on-chain settlement
instruction has no line/document payment semantics.

The document attestation must therefore be independently typed, independently
persisted, independently replayable, and explicitly rejected by aggregate
attestation and settlement paths. It must bind the full document identity and
calculation evidence without placing document rule graphs or line traces on
chain.

## Decision

1. Add a dedicated `DocumentCalculationAttestation` contract in the `policy`
   crate with its own schema version and domain-separated canonical payload.
   Its signed binding includes the exact document schema, document ID, engine
   version, merchant identity, published rule identity/digest, facts digest,
   asset mint, gross/tax/net totals, validity window, sequence, and issuer key
   identity.
2. Add `POST /v1/tax/document-quotes/{calculation_id}/attestation/verify`.
   It requires `tax:attestation:verify`, derives the document and route binding
   from tenant-scoped database state, verifies the signed document attestation,
   and persists it in a dedicated table with a document-specific outbox event.
   The caller cannot supply document identity, amounts, merchant identity, or
   mint binding as authority fields.
3. Add `tax_document_calculation_attestations` with a composite foreign key to
   `tax_document_calculations (id, merchant_id)`. The document calculation
   transitions to `attested` only after the immutable attestation row commits.
   Replays require byte-equivalent signed evidence and are idempotent; a
   different attestation for the same tenant/calculation conflicts.
4. Aggregate attestation ingestion remains bound to `tax_calculations` and
   `CalculationAttestation`. Aggregate settlement compilers continue loading
   only `tax_calculations`; they must not query or coerce document rows.
5. This ADR does not authorize a document quote for settlement. A future
   payment-mapping ADR must define document-total, line-subset, partial-payment,
   refunds, residual allocation, and on-chain account semantics before any
   document attestation is accepted by a movement compiler.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Reuse aggregate attestation and facts digest | No new signer type or table | Scope confusion; aggregate settlement could accept document IDs; document identity is not signed | Rejected |
| Add document fields to aggregate schema version 1 | One wire type | Breaks existing issuers/verifiers and Solana-compatible payload semantics | Rejected |
| Store a signed JSON envelope without a new canonical contract | Fast to implement | Cross-language canonicalization and domain separation are ambiguous | Rejected |
| **Dedicated document contract, table, route, and outbox event** | Explicit type firewall, independent replay/idempotency, future payment binding | Additional audited schema and signer integration | **Selected** |

## Invariants and failure behavior

- An aggregate calculation ID can never resolve through the document
  attestation route, and a document calculation ID can never resolve through
  aggregate attestation or settlement storage queries.
- The signed document attestation must bind the exact schema, document ID,
  engine version, facts digest, rule digest, rule identity, merchant, mint,
  amounts, issuer, sequence, and validity window.
- Document totals must conserve: `gross = tax + net`, with all values inside
  Solana `u64` bounds.
- Missing, expired, malformed, tampered, cross-tenant, route-mismatched, or
  scope-mismatched evidence fails closed. No status transition or outbox event
  is emitted on failure.
- Repeating identical signed evidence returns the stored attestation. A
  different signed evidence document conflicts and does not overwrite it.

## Budgets and verification

- Verification is one bounded document lookup, one bounded route lookup, one
  signature verification, and one serializable write transaction; no rule
  graph or external provider is invoked on the attestation hot path.
- The signed document remains bounded by the existing text, digest, validity,
  and JSON evidence limits.
- Tests must cover contract canonicalization, tampering, expired evidence,
  document/aggregate scope separation, route/mint mismatch, tenant isolation,
  idempotent replay, conflict, status transition, outbox atomicity, and
  settlement rejection of document IDs.
- Docker integration proves migration, two-replica document attestation
  replay/conflict, tenant isolation, outbox publication, and cross-context
  rejection against PostgreSQL 18. Document-to-payment mapping remains
  intentionally outside this ADR and requires a future payment-mapping ADR.
