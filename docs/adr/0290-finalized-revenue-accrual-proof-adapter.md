# ADR-0290: Finalized revenue-accrual proof adapter

- Status: Accepted
- Date: 2026-10-04
- Owners: Indexing, Settlement, and Revenue Operations

## Context

The revenue-stream domain accepts `FinalizedAccrual`, but the current storage
boundary can be reached with a caller-constructed value. Structural validation
protects identity, conservation, windows, and caps; it does not prove that the
source split settlement was observed at Solana `finalized` commitment. A
browser or ordinary API principal must not become the source of that proof.

The existing indexer already produces a typed split-settlement projection and
an observed finalized slot from bounded provider responses. The missing
boundary is a small, deterministic adapter that binds those two values before
the revenue domain or storage layer sees an accrual.

## Decision

Introduce a pure Rust proof contract in the revenue-streaming domain. The
adapter accepts a server-owned `FinalizedSettlementAllocationProof` containing
the source signature, receipt/evidence digest, split and tax policy digests,
asset, recipient allocation, source slot, observed finalized slot, and an
explicit finalized-commitment marker. It returns `FinalizedAccrual` only when:

- the commitment marker is finalized;
- the observed finalized slot is at least the source slot;
- the source signature, evidence digest, digests, identities, and amounts are
  non-zero and bounded;
- gross equals tax plus net; and
- the recipient allocation does not exceed net.

The adapter is pure and has no RPC, database, clock, HTTP, or wallet
dependency. An indexer/application adapter maps its already-validated typed
finalized projection into this proof contract. Storage remains responsible for
tenant/plan binding, idempotency, cap accounting, and append-only evidence.
The browser cannot call this contract or supply its proof fields.

## Alternatives considered

1. Let storage accept `FinalizedAccrual` directly. Rejected because structural
   validity is not chain finality and the caller could self-attest amounts.
2. Put RPC calls inside the revenue domain. Rejected because it violates the
   pure-domain boundary, complicates deterministic replay, and adds network
   latency to the write transaction.
3. Trust a provider JSON response in the API handler. Rejected because it
   bypasses the indexer's bounded provider agreement, checkpoint, and gap
   controls.
4. Use a typed finalized projection plus a pure proof adapter. Selected because
   it preserves one finality boundary, keeps the hot path allocation-bounded,
   and makes golden/property tests independent of external RPC state.

## Invariants and failure behavior

- Non-finalized, weakly committed, stale, malformed, or conflicting proof
  input fails closed before storage mutation.
- `observed_finalized_slot` is evidence about the provider observation; it is
  not persisted as a user-controlled business field in the accrual.
- Replaying the same proof yields the same immutable accrual; conflicting
  evidence remains a storage conflict.
- The source signature and receipt/evidence digest remain bound to the exact
  source event; no caller-supplied replacement signature is accepted.
- This adapter proves the source settlement allocation's finality. It does
  not prove a later revenue-stream claim was executed; claim execution still
  requires exact signed-message verification and finalized release evidence.

## Consequences

The revenue context gains an explicit finality gate that can be fuzzed and
benchmarked without Solana or PostgreSQL. The indexer must expose a mapping
from its typed finalized split projection, and the reconciliation worker must
call the adapter before recording accruals. A real multi-provider, Linux/
amd64, validator-backed conformance run remains a separate release gate.
