# ADR-0125: Bounded targeted signature-to-evidence recovery

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Indexing, and SRE
- Scope: targeted recovery of finalized transaction evidence

## Context

ADR-0124 added bounded finalized signature discovery, but discovery alone does
not repair an evidence projection. The recovery path needs to fetch the full
finalized transaction, verify that the response is for the requested
signature, decode the canonical settlement event contract, and persist the
result without moving the forward checkpoint.

This is an operational recovery capability, not a checkout or settlement
authority. A caller must already be authorized by the operator control plane;
this bounded domain/service primitive must not infer authority from a merchant
identifier or from the Solana signature.

## Decision

Add a targeted recovery operation that:

1. validates one target address, signature, stream key, and a page budget of
   1 through 64 pages;
2. uses finalized `getSignaturesForAddress` pages with a strict cursor and
   stops at the target signature or the explicit page budget;
3. fetches the target with finalized `getTransaction`, requesting JSON and
   `maxSupportedTransactionVersion: 0`;
4. verifies the response slot, primary signature, bounded payload, and
   canonical `SettlementExecuted`/split-event decoding through the existing
   decoder;
5. persists the resulting raw transaction, raw event evidence, and normalized
   receipt through `persist_replayed_page`; and
6. never reads or mutates `chain_checkpoints`, never submits a transaction,
   and never marks an operator action as repair completion implicitly.

The operation is one target per invocation. It has no internal unbounded loop,
uses bounded provider deadlines/failover, and returns explicit outcomes for
`Recovered`, `AlreadyPresent`, and `TargetNotFoundWithinBudget`. The
authenticated API endpoint wraps it through the existing operator control
plane. The endpoint requires the exact `tax:indexer:operate` scope and
`indexer_operator` role, keeps stream/provider configuration server-owned, and
appends immutable `requested`, `recovered`, or `failed` audit rows. Audit and
projection persistence are intentionally separate transactions so a process
crash leaves a visible requested-without-terminal signal for reconciliation
rather than fabricating repair completion.

## Alternatives considered

### Rebuild only from already-retained raw evidence

Retained as the preferred disaster-recovery path. It cannot recover a
signature whose raw evidence was never persisted.

### Poll `getTransaction` without signature discovery

Rejected as a general recovery contract. It cannot prove that the target
belongs to the requested address and encourages provider-specific/manual
queries.

### Use signature discovery and persist the page directly

Rejected. A signature list contains no transaction message, event bytes, or
amount evidence and must never create a projection row.

### **Bounded signature discovery followed by finalized transaction fetch and
projection-only persistence**

Selected. It links the operational target to canonical finalized evidence while
preserving checkpoint and settlement authority boundaries.

## Invariants and failure behavior

- The target signature is canonical base58 and the target address is a valid
  Solana public key.
- Page count, response bytes, transaction payload, and decoded event data are
  bounded.
- The target must appear in a finalized signature page before `getTransaction`
  is attempted.
- `getTransaction` must return the same primary signature and a finalized
  transaction slot; disagreement fails closed.
- Identical evidence is idempotent; conflicting evidence rolls back.
- No forward checkpoint or settlement intent state is mutated.
- A target absent after the configured page budget is not treated as success.

## Evidence and remaining gates

Unit tests cover cursor budgets, signature mismatch, finalized transaction
request shape, decoder rejection, and failover primitives. The service method
uses the existing storage integration contract for idempotent projection
persistence. The API route and migration enforce operator authentication,
server-owned target scope, and append-only audit evidence. Live provider
evidence, crash-reconciliation alerting, and fleet recovery SLOs remain
separate gates.
