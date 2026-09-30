# ADR-0204: Authoritative document quote boundary

## Status

Accepted for implementation. This ADR adds production persistence and replay
for the ADR-0203 document-facts bounded context. It does not authorize a
document quote to settle funds; settlement binding remains a separate decision
with explicit line-to-payment semantics.

## Context

ADR-0203 currently exposes a schema-validated simulation route. Reusing the
aggregate quote table and response type would make a line/invoice result look
like an aggregate quote, allowing settlement code to read the wrong shape and
making replay ambiguous. The production path needs authenticated tenant
scope, published-rule and jurisdiction-provider selection, UUIDv7
idempotency, encrypted facts evidence, immutable result evidence, outbox
publication, replay, and attestation binding without weakening the existing
aggregate settlement contract.

## Decision

1. Add `POST /v1/tax/document-quote` as the production document-calculation
   boundary. It requires the authenticated `tax:quote` scope, a merchant
   idempotency key, an exact published rule selector, jurisdiction evidence,
   the exact `solana-tax-protocol/tax-document/v1` schema, and tenant-scoped
   document facts. The merchant is derived from the authenticated principal;
   caller-supplied merchant IDs are rejected or ignored.
2. Persist document quotes in a dedicated `tax_document_calculations` table,
   not in `tax_calculations`. The table stores a UUIDv7 calculation ID,
   merchant/rule bindings, schema, document/facts digest, engine version,
   currency/jurisdiction, gross/tax/net totals, status, immutable result,
   encrypted normalized facts, evidence references, expiry, and timestamps.
   PostgreSQL is a durable projection/evidence store; Solana remains the
   authority for any future movement.
3. Reuse the existing merchant-scoped `idempotency_records` and serializable
   transaction pattern, but use a domain-separated
   `solana-tax-protocol/document-quote-request/v1` digest. The first durable
   document result and `tax.document_calculation.created` outbox event commit
   atomically. Same-key identical requests replay the exact stored response;
   same-key different requests fail closed.
4. Resolve the published rule and jurisdiction evidence inside the transaction
   boundary, calculate with the compiled Rust document engine, bind the
   server-owned jurisdiction-resolution envelope into document facts, and
   persist encrypted facts. No caller-provided rule document or resolved
   evidence becomes authoritative.
5. Add `GET /v1/tax/document-quotes/{calculation_id}/replay`, requiring the
   same tenant scope. Replay loads the immutable published rule and encrypted
   facts, verifies schema, facts digest, rule digest, engine version, complete
   `TaxDocumentResult`, and document replay semantics. It never authorizes
   settlement.
6. Extend attestation ingestion only with a document-specific binding and
   status transition after the document row exists. Aggregate attestation
   verification must reject document calculation IDs, and document
   attestation verification must reject aggregate calculation IDs.
7. Keep `/v1/settlement/compile-from-quote` and split settlement compilation
   aggregate-only until a later ADR defines whether one payment maps to one
   document, a line subset, or a document total, including refund and partial
   payment semantics.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Reuse `tax_calculations` and `StoredQuote` | Less schema/code | Shape confusion, unsafe settlement reads, ambiguous replay and attestation | Rejected |
| Store document results only in JSON/object storage | Flexible payloads | No transactional idempotency/outbox/evidence query boundary | Rejected |
| Let the browser persist or attest document results | Low server work | Caller-controlled authority, replay gaps, key/evidence leakage | Rejected |
| **Dedicated tenant-scoped document calculation table and adapter** | Explicit type boundary, durable idempotency, encrypted replay, clear settlement firewall | Adds migration and adapter surface | **Selected** |

## Invariants and failure behavior

- Aggregate and document calculation IDs are not interchangeable.
- Every successful document quote has one authenticated merchant, one
  published rule digest, one jurisdiction-resolution digest, one document
  facts digest, one engine version, and one immutable result.
- Facts are encrypted at rest when persisted; plaintext facts are never
  returned by the quote endpoint.
- Unknown schema, missing/ambiguous provider evidence, stale policy, rule
  digest mismatch, invalid document scope, idempotency conflict, replay
  mismatch, and unknown provider outcomes fail closed.
- The outbox is at-least-once and non-authoritative; duplicate publication is
  harmless because the aggregate event identity is the document calculation
  UUID.

## Budgets and verification

- The API applies the existing bounded request body and deadline limits before
  invoking the engine; the domain remains O(lines × components) with the
  ADR-0203 256-line/128-component ceilings.
- Document quote persistence adds one bounded serializable transaction and
  no network dependency to the engine hot path beyond the existing published
  rule/provider evidence boundary.
- Failing-first tests cover tenant scope, schema, provider disagreement,
  idempotency replay/conflict, encrypted facts, result immutability, outbox
  atomicity, document replay tampering, and aggregate/document attestation
  separation.
- Docker integration proves PostgreSQL 18 migration, two-replica same-key
  convergence, replay, and unauthorized cross-tenant reads before any
  settlement adapter is considered.
