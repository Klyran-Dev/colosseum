# ADR-0203: Versioned line and invoice tax facts with explicit aggregation scope

## Status

Accepted. The pure domain, bounded simulation API, browser workbench, golden
vectors, property tests, and 64-line × 16-component benchmark are implemented.
This does not claim legal coverage for any jurisdiction, product category,
filing, registration, or remittance obligation; production quote authorization,
durable evidence, and settlement binding remain open gates.

## Context

The aggregate `TaxFacts` contract is intentionally bounded to one taxable
amount. Treating it as an invoice or silently summing caller-provided line
amounts would lose line identity, make rounding scope ambiguous, and weaken
replay evidence. A Tier-1 product needs deterministic line and invoice
semantics for carts, invoices, credits, and multi-component tax calculations
without allowing unbounded user-controlled work.

## Decision

1. Add a versioned `TaxDocumentFacts` contract with a document identifier,
   one currency, one server-owned jurisdiction-resolution envelope, and a
   bounded ordered set of `TaxLineFacts`. Each line has a unique canonical
   line identifier, gross/net atomic amounts, and line-scoped exemption codes.
   The document facts digest includes line identifiers and amounts in canonical
   identifier order, not caller array order.
2. Bound documents to 256 lines and line identifiers to the existing 64-byte
   code limit. Reject duplicate/empty IDs, zero or negative amounts, total
   overflow, currency/jurisdiction mismatch, invalid evidence, and empty
   documents. The domain remains pure and receives all evidence before the
   calculation; it never loads a database, clock, RPC, or tax provider.
3. Add `Line` and `Invoice` to the versioned `RoundingScope` enum. Aggregate
   `calculate` rejects those scopes with an explicit document-scope error;
   callers must use `calculate_document`. This prevents an aggregate endpoint
   from accidentally claiming line/invoice semantics.
4. `Line` scope calculates exact independent component values for each line,
   rounds the line total once, and allocates the residual in quantum increments
   across that line's components using canonical component IDs. `Invoice` scope
   calculates all line/component cells exactly, rounds the invoice total once,
   and allocates residual quantum units by canonical `(line_id, component_id)`.
   Both scopes require one mode and quantum for the rule set and reject
   component dependencies; tax-on-tax remains explicitly supported by
   component scope in the aggregate contract.
5. Emit `TaxDocumentResult` with the document/facts/rule/engine bindings,
   total gross/net/tax, and ordered `TaxLineResult` traces. Every cell is an
   integer multiple of the declared quantum, totals conserve the document
   amounts, and replay compares the complete result and canonical facts digest.
6. Publish the transport schema as
   `solana-tax-protocol/tax-document/v1`. Adding fields or changing rounding
   semantics requires a new schema/rule version; unknown schemas and unknown
   fields at API boundaries fail closed. Historical aggregate facts remain
   replayable and are never reinterpreted as document facts.
7. The browser exposes a bounded document simulator beside rule authoring. It
   edits line identity and decimal-string facts, validates duplicate/empty/
   over-bound lines before network access, submits only to the explicit
   simulation route, and renders server-returned canonical lines and trace
   digests. It never computes a tax total locally, accepts a settlement action,
   or presents an unattested result as a quote.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Reuse aggregate `TaxFacts` with a line count | Smallest API change | Loses line identity and allows accidental invoice semantics | Rejected |
| Let the browser calculate and aggregate lines | Responsive preview | Numeric drift, caller-controlled evidence, and no authoritative replay | Rejected |
| Round each line independently for every rule | Easy implementation | Cannot express invoice-level legal rounding and may create conservation drift | Rejected |
| **Versioned document facts with explicit line/invoice scope** | Replayable identity, bounded work, conserving deterministic allocation, clear authority boundary | Adds a new contract and separate result type | **Selected** |
| Unbounded generic rule graph over arbitrary line data | Broad theoretical flexibility | DoS risk, unpredictable latency, difficult audit and on-chain attestation bounds | Rejected |

## Invariants and failure behavior

- Aggregate and document facts have distinct schemas and digest domains.
- Canonical document line order is identifier order; caller ordering never
  decides tax or residual allocation.
- A document contains at most 256 lines and every user-controlled collection
  has an explicit bound.
- Line/invoice scopes cannot use component dependencies, mixed rounding modes,
  or mixed quanta.
- Tax totals are non-negative, bounded by gross, and equal the sum of ordered
  line/component results.
- Any malformed facts, missing/invalid jurisdiction evidence, duplicate ID,
  overflow, unsupported scope, or replay mismatch fails closed before
  persistence or settlement preparation.

## Performance and operational budget

- Domain calculation is O(lines × components × bounded base sources), with a
  hard 256-line and 128-component ceiling.
- The warm Rust document calculation target is p95 ≤ 5 ms and p99 ≤ 15 ms for
  64 lines × 16 components on the documented benchmark host; this is a local
  domain budget, not a regional production SLO.
- No network or storage operation is allowed in the calculation path. API
  adapters must enforce request-size, deadline, cancellation, and concurrency
  budgets before invoking the domain.

## Verification and release gates

- Failing-first Rust tests cover schema validation, canonical line-order
  digesting, duplicate/overflow rejection, line allocation, invoice allocation,
  aggregate/document scope separation, replay, and conservation.
- Golden vectors cover one-line, multi-line, line-scope, invoice-scope,
  residual tie, exemption, progressive, inclusive, and tax-on-tax rejection
  cases where the scope is incompatible.
- Property/fuzz tests cover line permutation, bounded amounts, malformed IDs,
  and residual allocation invariants.
- API/storage/UI adapters are separate implementation slices and must not
  publish document results until their own authorization, persistence,
  accessibility, and load evidence exists. The current API/UI slice is
  simulation-only and has no production quote authority.
