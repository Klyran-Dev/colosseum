# Runbook: indexer gap and projection repair

## Trigger and owner

Use for checkpoint discontinuity, replay-gap dead letter, raw evidence conflict,
projection lag, or reconciliation mismatch. The chain-data owner coordinates;
the system operator role controls replay actions.

## Contain and inspect

1. Preserve the stream key, slot range, observed finalized slot, gap reason,
   attempts, provider evidence, and correlation ID.
2. Inspect through the authenticated Replay operations workspace or its bounded
   API. Do not edit replay rows with ad hoc SQL.
3. Acknowledge a dead letter only after an operator has reviewed the evidence;
   acknowledgement is not repair completion.

## Docker RPC startup failure

If the indexer repeatedly reports transport failures for
`http://validator:8899/`, inspect the validator first:

```sh
docker compose --profile solana ps -a validator
docker compose --profile solana logs --no-color --tail=160 validator
```

Agave 4.3.0 requires an io_uring-capable Linux runtime. A Docker Desktop
validator log containing `io_uring NOT supported` followed by
`assertion failed: io_uring_supported()` means the validator exited before RPC
was available; it is not an indexer decoding defect. The indexer now performs
a bounded startup probe and exits with the provider error instead of creating
an unbounded replay-gap loop.

Run the pinned validator on a supported Linux/amd64 runtime for validator-backed
development and conformance. For non-validator development, use the explicit
synthetic RPC-fixture drills, which set `INDEXER_RPC_ENDPOINTS` themselves;
never point a production or staging indexer at a fixture.

## Repair

- Verify the range against at least the approved finalized provider policy.
- Requeue only after acknowledgement and only with a bounded retry budget.
- For a projection rebuild, use the pinned `make projection-rebuild-drill`
  contract as the local verification baseline and invoke the authenticated
  production recovery job with an incident/change reference.
- Rebuild from immutable raw evidence; never accept caller-supplied amounts,
  signatures, policy digests, or checkpoint advancement.
- Confirm duplicate/conflict behavior, projection digest, checkpoint
  invariance, and reconciliation results before closing the incident.

## Targeted-recovery audit timeout

When `STPTargetedRecoveryAuditStale` fires, query the append-only
`indexer_targeted_recovery_actions` ledger by the alert count and incident
window. A `requested` row older than five minutes without a matching terminal
`recovered` or `failed` row indicates a crash, timeout, or lost response; it is
not proof of a failed chain operation.

1. Use the authenticated targeted-recovery workspace to inspect the target,
   request correlation ID, operator subject, and existing evidence. Never put
   signatures, addresses, or bearer tokens into metric labels or ad hoc logs.
2. Check raw transaction/evidence and normalized receipt projections by the
   target signature. If finalized evidence is present and consistent, record
   the approved reconciliation outcome through the operator workflow and cite
   the incident.
3. If evidence is absent or ambiguous, rerun only within the configured page
   budget after review. The endpoint will rebind the target to finalized
   evidence and append a new immutable audit attempt; it never advances the
   checkpoint or submits funds.
4. Escalate on provider disagreement, conflicting evidence, decoder failure,
   or a missing terminal row after the bounded retry. Do not update/delete
   audit rows or declare repair complete from the metric alone.

## Stop conditions

Stop on provider disagreement, invalid raw digest/discriminator, amount
non-conservation, stale finality, or any conflicting normalized row. Escalate
to chain operations and do not mark the gap repaired by changing a status.
