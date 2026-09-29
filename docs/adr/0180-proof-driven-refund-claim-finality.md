# ADR-0180: Proof-driven refund-claim finality reconciliation

## Status

Accepted for implementation. The claim-finality endpoint and bounded indexer
worker are implemented; Docker validator conformance, load evidence, and
production enablement remain release gates.

## Context

Refund claim submission persists a signature before the RPC call and correctly
enters `unknown` when the provider outcome is ambiguous. Execute preparation
must not rely on that signature, an RPC acknowledgement, or a finalized claim
account snapshot alone. An account snapshot proves state, but it does not prove
which transaction created that state.

The settlement program emits `AdjustmentRefundClaimCreated` with the complete
claim topology, amounts, reference digest, proposer, and execution window. The
indexer already retains raw Anchor event bytes during bounded finalized
recovery, making that event the appropriate signature-to-claim evidence.

## Decision

Add a proposer-scoped reconciliation boundary shared by an authenticated
operator action and an automatic worker:

- `POST /v1/settlement/refund-claims/{intent_id}/reconcile` loads the persisted
  signature and immutable artifact; the caller cannot provide a replacement
  signature, address, topology, or page budget;
- bounded targeted recovery searches only the server-derived proposer address,
  fetches the exact transaction at finalized commitment, validates its raw
  evidence, and persists it through the normal indexer transaction;
- the raw `AdjustmentRefundClaimCreated` event is decoded with bounded exact
  lengths, checked for conservation, distinct authorities, nonzero digests,
  and a valid execution window;
- every event field is compared with the durable claim intent and compiled
  artifact before the claim CAS transition to `finalized`; and
- the indexer selects only durable submitted candidates, performs the same
  bounded recovery under a short database lease, performs the same exact
  evidence comparison, and converges through the same CAS/outbox transition
  without accepting caller input; stale leases are reclaimable after a bounded
  TTL; and
- the indexer exports a fixed-cardinality pending-claim gauge and alerts on a
  sustained backlog; the gauge is operational telemetry, never finality
  evidence; and
- the transition records the finalized slot, commitment, event index, event
  digest, recovery budget evidence, lifecycle event, and outbox event in one
  database transaction.

Only this proof-driven transition unlocks refund-execute preparation. A missing
event, signature substitution, provider disagreement, topology mismatch, or
concurrent state change remains a conflict or reconciliation failure.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Trust RPC submission acknowledgement | Low latency | Not durable finality; timeout can hide inclusion | Rejected |
| Read the finalized claim PDA only | Simple and cheap | Does not bind the PDA state to the submitted signature | Rejected |
| Parse a browser-supplied transaction or signature | Flexible recovery UX | Caller-controlled evidence and address substitution risk | Rejected |
| Add claim-event projection without raw evidence | Easy queries | Loses forensic bytes and replay authority | Rejected |
| **Bounded finalized recovery plus immutable raw claim event matching** | Exact signature binding, replayability, tenant-safe CAS, and forensic evidence | Requires event decoder and reconciliation worker/endpoint | **Selected** |

## Invariants

- the API reconciliation action is tenant-scoped and requires the claim
  proposer role; automatic reconciliation has no caller and uses only durable
  intent/artifact evidence;
- the persisted signature and server-derived proposer are the only recovery
  target inputs;
- finalized evidence must contain exactly one matching claim-created event;
- claim PDA, adjustment, receipt, merchant, mint, amounts, authorities,
  reference digest, and execution window must match the durable artifact;
- finality is recorded only by a compare-and-set lifecycle transition; and
- execute preparation remains impossible until the claim is finalized with the
  recovered signature and slot.

## Verification plan

- decoder golden vectors and mutation tests for every event field;
- targeted-recovery tests for signature substitution, missing event, wrong
  proposer, provider non-finality, and event/artifact mismatch;
- PostgreSQL integration tests for idempotent reconciliation, CAS contention,
  outbox atomicity, and unknown-to-finalized convergence;
- API tests for tenant/role isolation and server-owned recovery inputs; and
- worker tests for bounded candidate selection, duplicate convergence, and
  absence of caller-controlled target fields; and
- lease-claim, `SKIP LOCKED`, stale-lease reclaim, and lease-fenced CAS
  integration tests; and
- fixed-cardinality metrics and alert-rule validation for a non-draining
  claim-finality backlog; and
- local-validator conformance proving claim creation, finalized event recovery,
  execute gating, and replay safety.

## Consequences

Claim finality is now an explicit evidence boundary rather than an inferred
transport state. The system performs more bounded work before enabling the
second wallet approval, but the resulting execute intent is tied to a
replayable finalized chain fact and cannot be unlocked by a stale or ambiguous
provider response. The endpoint remains useful for an authorized operator
retry, while the indexer worker provides restart-safe eventual convergence for
unknown and provider-lagged submissions.
