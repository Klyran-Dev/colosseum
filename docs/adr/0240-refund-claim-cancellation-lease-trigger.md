# ADR-0240: Lease-safe refund-claim cancellation transition trigger

## Status

Accepted for implementation. Migration `0059` repairs the database lifecycle
guard used by the ADR-0238 automatic cancellation reconciler. Local Docker
evidence passes on arm64; Linux/amd64 validator, provider, and production
operational evidence remain release gates.

## Context

ADR-0238 uses a short PostgreSQL UUIDv7 lease so multiple indexer replicas can
coordinate finalized cancellation reconciliation without holding a database
lock across RPC. Lease acquisition updates only `reconciliation_claim_token`
and `reconciliation_claimed_at`; it intentionally does not advance the
cancellation status.

The original status trigger treated every same-status update as an invalid
lifecycle transition. That made a correctly persisted submitted cancellation
unreconcilable: the worker could select it, but the lease update was rejected
before finalized evidence recovery. The safe repair must preserve the immutable
artifact boundary and must not weaken lifecycle or authority checks.

## Decision

Add a forward-only migration that replaces the cancellation transition trigger
function. The trigger now has three explicit branches:

1. immutable authority and artifact fields remain unchanged;
2. a same-status update may change only the reconciliation lease fields and
   the trigger-managed timestamp; or
3. a status-changing update must satisfy the existing allow-list of lifecycle
   transitions.

Lifecycle fields such as signature, submitted slot, commitment, and error code
cannot change during a same-status lease update. Finalization remains a
status-changing, evidence-gated, token-fenced transaction in the storage
adapter. The migration is additive and preserves immutable migration history.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`.

1. Remove the trigger: rejected because database lifecycle fencing would be
   lost.
2. Allow every same-status mutation: rejected because signature, commitment,
   error, and chain-slot fields could be changed outside the lifecycle path.
3. Have the worker advance status while acquiring the lease: rejected because
   coordination ownership is not chain evidence and would blur the durable
   state machine.
4. **Allow only lease-field changes at the same status and retain the existing
   status allow-list:** selected because it repairs lease acquisition while
   keeping authority, evidence, and lifecycle invariants fail-closed.

## Verification

- The previously failing `make refund-cancellation-reconciliation-drill`
  remains the executable regression test: two real indexer replicas must
  acquire leases, recover finalized evidence, converge the cancellation and
  claim projections, emit exactly two outbox events, and leave zero backlog.
- The static cancellation contract must require migration `0059` and its
  same-status lease guard.
- Migration integrity and backup/restore drills must apply the forward
  migration and retain checksum evidence.

## Consequences

The cancellation reconciler can make progress without an artificial status
transition. The database remains the lifecycle authority for off-chain state,
while finalized Solana evidence remains the authority for cancellation. The
new migration adds no request-path work and has bounded DDL/function-replace
cost during deployment.
