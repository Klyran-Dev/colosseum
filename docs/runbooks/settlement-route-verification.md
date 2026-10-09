# Settlement-route verification runbook

This runbook describes the bounded, non-hot-path verifier for tenant-scoped
Solana settlement routes. It does not authorize token movement; finalized
Solana account evidence and the settlement program remain authoritative.

## Normal operation

The route-verifier service reads at most the configured batch size (maximum
64) of `pending_verification` or `suspended` routes, verifies the fixed
five-account topology at finalized commitment through the bounded provider
pool, and activates only a matching route inside one serializable transaction.
An idle poll creates no run record.

Each non-empty pass creates a row in
`settlement_route_verification_runs`. Review the latest rows by status:

System operators can use the **Route-verification evidence** disclosure in the
Exceptions & recovery workspace. It is a bounded, read-only view requiring
the existing `tax:indexer:read` scope and exact `indexer_operator` role. The
disclosure loads only after an explicit action and does not activate routes or
confirm finalized settlement. Use the SQL below as the controlled backup path
when the operator UI or API is unavailable.

```sql
SELECT id, run_kind, operator_approval_id, status, batch_size, examined_count, activated_count,
       already_active_count, failed_count, failure_code, started_at,
       completed_at
FROM settlement_route_verification_runs
ORDER BY started_at DESC, id DESC
LIMIT 20;
```

Interpretation:

- `completed` means every examined candidate reached an idempotent terminal
  result without a per-route failure.
- `completed_with_failures` means the pass completed but at least one route
  failed verification or activation; those routes remain unavailable.
- `failed` means the pass could not complete as a process/storage operation.
- `running` means the verifier may have stopped or lost its database session;
  it is never evidence of successful activation.

## Backfill and re-verification

Use the separately approved, one-shot backfill mode. The change record ID
must be a UUIDv7 and must be supplied before the process can connect to
PostgreSQL, create RPC transports, or bind a listener:

```sh
ROUTE_VERIFIER_MODE=backfill \
ROUTE_VERIFIER_BACKFILL_APPROVAL_ID="<approved-change-uuidv7>" \
ROUTE_VERIFIER_ENV=staging \
ROUTE_VERIFIER_DATABASE_URL="<secret-managed-url>" \
ROUTE_VERIFIER_RPC_ENDPOINTS="<finalized-rpc-1>,<finalized-rpc-2>" \
ROUTE_VERIFIER_BATCH_SIZE=8 \
docker compose --profile route-verifier run --rm --no-deps route-verifier
```

The command executes exactly one bounded settlement-route batch, records the
approval ID on the `backfill` run, and exits. It does not run refund-funding
or partner-destination verification and does not start the health listener.
Repeat the command only with a new approved invocation or until the bounded
candidate query is empty. Do not change route rows directly. Confirm that the
migration has suspended every route without complete typed asset evidence, then
retain the run IDs, counts, route IDs, verification digests, finalized slots,
and provider-agreement evidence in the release bundle.

Before enabling paid-state movement, require:

1. no non-local route remains active without complete typed asset evidence;
2. every activated route has finalized v2 five-account evidence and a matching
   `AssetConfig` projection;
3. no run is `running`, `failed`, or `completed_with_failures` without an
   approved remediation record; and
4. the retained evidence is independently reviewed against the exact source
   revision and migration ledger.

Scheduled daemon mode must not receive
`ROUTE_VERIFIER_BACKFILL_APPROVAL_ID`; configuration rejects that combination.

## Failure handling

Never mark a failed run complete by editing the ledger. Terminal run rows are
immutable. Investigate the route-level evidence and provider disagreement,
correct the route through its governed lifecycle, and run a new bounded
verification pass. If the verifier is interrupted, restart it and reconcile
the `running` row with the new run; do not infer activation from logs or
metrics.
