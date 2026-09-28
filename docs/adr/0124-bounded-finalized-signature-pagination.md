# ADR-0124: Bounded finalized signature pagination for targeted recovery

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Indexing, and SRE
- Scope: indexer recovery and settlement-signature reconciliation

## Context

The forward indexer scans finalized blocks with `getBlock` and advances a
slot checkpoint only after an exact, complete page has been validated. That
remains the authoritative projection path. Operations also need a bounded way
to discover recent signatures for a known address when investigating an
ambiguous submission, repairing a narrow projection, or checking a provider
disagreement. The repository previously had no typed adapter for
`getSignaturesForAddress`, so operators would have to use an unbounded or
provider-specific query outside the evidence contract.

Signature pagination is an acceleration and discovery signal. It cannot
advance the finalized checkpoint, prove a receipt, or authorize settlement.
Every discovered signature must still be fetched and verified through the
finalized transaction/block evidence path before it changes a projection.

## Decision

Add a bounded `SolanaRpcSignaturePageSource` behind the existing
`JsonRpcTransport` port. It:

- accepts one validated address, one optional `before` cursor, and a limit of
  1 through 1,000;
- always requests `commitment: finalized`;
- omits `before` on the first page and sends only the server-returned canonical
  last signature as the next cursor;
- bounds serialized response size and entry count;
- requires non-empty canonical base58 signatures, non-negative slots, and a
  `confirmationStatus` of `finalized`;
- rejects duplicate signatures, malformed error/status fields, cursor
  non-progress, and unexpected JSON shapes;
- returns a typed page with a `complete` flag and an optional next cursor, but
  never loops internally or persists a checkpoint.

The adapter is intentionally a narrow port. A future recovery worker may use
it with a fixed page budget and then obtain finalized transaction evidence by
signature. Such a worker requires its own ADR for authorization, audit, retry,
and projection mutation semantics.

## Alternatives considered

### Continue using only finalized `getBlock`

Rejected as the only operational tool. It is authoritative and remains the
forward path, but narrow signature investigation can be needlessly expensive
when a known address and signature cursor are available.

### Let operators call provider-specific APIs or raw JSON-RPC

Rejected. It bypasses response bounds, commitment enforcement, canonical
cursor handling, and testable failure semantics.

### Scan `getSignaturesForAddress` until exhaustion inside the service

Rejected. It creates unbounded work, can amplify provider cost, and makes
recovery progress difficult to cancel or budget.

### Bounded typed adapter with a separate evidence fetch

Selected. It improves low-latency targeted recovery while preserving finalized
chain evidence and checkpoint authority boundaries.

## Invariants and verification

- No page request uses a commitment weaker than `finalized`.
- Page limits are bounded to 1,000 and response bytes are bounded by the
  existing RPC transport limit.
- A full page always produces a strict next cursor; an empty or short page is
  terminal for that bounded scan.
- The adapter never advances a checkpoint or writes a projection.
- Unit tests cover request shape, canonical cursor progress, duplicate and
  malformed entries, non-finalized status, oversized pages, and provider
  errors.

## Consequences

Positive: targeted recovery has a typed, bounded, provider-independent
contract and can be measured separately from the forward indexer.

Cost: the repository still needs a production recovery worker, live provider
pagination evidence, signature-to-finalized-transaction backfill, and
operator authorization before this becomes a complete operational workflow.
