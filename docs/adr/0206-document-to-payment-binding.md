# ADR-0206: Document-to-payment binding

- Status: Accepted for implementation
- Date: 2026-09-30
- Owners: Tax Platform, Settlement Protocol, Security

## Context

ADR-0204 established an authoritative document quote and ADR-0205 established
a dedicated signed document-calculation attestation. Neither artifact may
currently authorize a Solana movement. The existing `settle` and `settle_split`
instructions consume aggregate facts and write aggregate receipts without
document provenance. Feeding document totals into that path would lose the
document identity and make line-subset or partial-payment semantics ambiguous.

Tax rules can be progressive, thresholded, cascading, capped, exempted, and
rounded at line or invoice scope. A partial payment cannot safely be produced
by multiplying a stored total: the tax engine may need to recalculate brackets,
tax-on-tax dependencies, credits, and residual allocation.

## Decision

Create a dedicated document-payment bounded context with a pure Rust domain
library and an authenticated compiler:

`POST /v1/settlement/document/compile-from-quote`

The request contains only a tenant-scoped document `calculation_id`, active
`route_id`, payer, nonce, policy sequence/digest, and a bounded payment scope:

- `FullDocument`: uses the complete stored document result.
- `LineSubset`: uses one or more complete canonical line IDs from the stored
  result; amounts are summed from stored line results with checked arithmetic.

The domain sorts and deduplicates line IDs, rejects unknown or empty selections,
enforces the document line bound, proves `gross = tax + net`, and computes a
canonical `payment_scope_digest` bound to the document schema, document ID,
facts digest, source attestation payload digest, and selected line IDs. The
server derives all amounts and account bindings. Callers never supply gross,
tax, net, line amounts, or merchant-side accounts.

Partial payment of an individual line, quantity-based payment, arbitrary amount
proration, and tax re-rating are rejected. They require a new document quote
whose facts represent the paid quantity/amount and a new attestation; they must
not approximate a nonlinear result by scaling it.

Add a separate `settle_document` Solana instruction. It reuses the bounded
token-transfer and policy-attestation invariants of `settle`, but uses a
`document-receipt` PDA and stores/emits the source document-attestation payload
digest and payment-scope digest. Existing `settle` and `settle_split` and their
aggregate receipts remain unchanged. The program records provenance digests;
the off-chain verifier proves the signed document and scope before compilation.

Persist a dedicated `document_payment_authorizations` row linked to the
durable settlement intent. Creation of the mapping and intent is one
serializable transaction. The row is immutable evidence containing tenant,
document calculation, source attestation digest, scope digest/document,
route/mint, and derived amounts. Replayed idempotency returns the exact same
artifact; conflicting scope, attestation, route, or instruction evidence
returns a conflict.

Wallet submission accepts a typed document compiled artifact and verifies the
same signed-message invariants as aggregate settlement, including the exact
`settle_document` discriminator, account order, document receipt PDA, amounts,
policy digest, source-attestation digest, and payment-scope digest.

## Alternatives considered

| Alternative | Advantages | Rejection reason |
| --- | --- | --- |
| Reuse aggregate `settle` | Minimal implementation | Loses typed document provenance and permits scope confusion. |
| Opaque document identity in policy digest | No program change | Chain evidence cannot independently identify source and scope. |
| Prorate stored totals | Flexible UI | Wrong for nonlinear and rounded rules; violates deterministic tax semantics. |
| Full document/rule graph on-chain | Direct visibility | Unbounded cost, privacy leakage, and poor global-rule evolution. |
| **Dedicated domain, compiler, `settle_document`, and receipt** | Explicit type firewall, bounded semantics, and on-chain evidence | Additional audited instruction surface; selected. |

## Invariants and failure modes

- The document attestation exists for the authenticated merchant, is unexpired,
  valid, and its payload digest equals the requested policy digest.
- The selected route mint matches the stored document-attestation binding.
- Full-document amounts equal stored document totals exactly.
- Line-subset amounts equal the checked sum of selected stored line results.
- Every amount is a positive Solana `u64`, and `gross = tax + net`.
- Scope IDs are canonical, unique, bounded, and bound to the source digest.
- Authorization and settlement intent commit atomically; retries are bounded and
  idempotent.
- Aggregate compilers reject document IDs; document compilers reject aggregate
  IDs with explicit scope errors.
- Unknown, expired, revoked, stale, conflicted, or partially persisted evidence
  fails closed before wallet signing or RPC submission.

## Budgets and verification

- Scope derivation is O(selected lines), bounded by the document line cap, and
  has no database or network dependency.
- Compilation uses one document quote lookup, one document attestation lookup,
  one route lookup, and one bounded intent/authorization transaction.
- The instruction remains at 12 accounts; only a new discriminator and bounded
  provenance digests are added to the instruction data.
- Tests cover full documents, one-line and multi-line subsets, duplicate /
  unknown / over-bound IDs, conservation, partial-line rejection, attestation
  and route mismatch, idempotent replay/conflict, tenant isolation, signed
  message tampering, PDA substitution, scope rejection, migration rollback,
  and localnet instruction conformance.

## Consequences

Document payments have an explicit evidence chain:

`DocumentCalculationAttestation → DocumentPaymentAuthorization →
settle_document → DocumentSettlementReceipt → finalized chain event`

This does not claim legal compliance, filing, or tax advice. A future ADR is
required for quantity/partial-line re-rating, refund attribution across
document scopes, or multi-document cart aggregation.
