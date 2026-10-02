#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

fail() {
  echo "refund claim cancellation reconciliation contract failed: $1" >&2
  exit 1
}

require_text() {
  file=$1
  text=$2
  grep -Fq "$text" "$file" || fail "$file is missing: $text"
}

lease_migration=db/migrations/0057_refund_claim_cancellation_reconciliation_leases.sql
trigger_migration=db/migrations/0059_refund_claim_cancellation_lease_trigger.sql
storage=crates/storage/src/lib.rs
service=crates/indexer-service/src/lib.rs
main=crates/indexer-service/src/main.rs
fixture=crates/rpc-fixture/src/main.rs
drill=scripts/refund_cancellation_reconciliation_drill.sh
fixture_script=apps/web/scripts/refund_cancellation_fixture.mjs
compose=compose.yaml

[ -f "$lease_migration" ] || fail "missing additive lease migration"
require_text "$lease_migration" 'reconciliation_claim_token uuid'
require_text "$lease_migration" 'uuid_extract_version(reconciliation_claim_token) = 7'
[ -f "$trigger_migration" ] || fail "missing lease-safe transition trigger migration"
require_text "$trigger_migration" 'same-status refund claim cancellation updates may only change reconciliation lease fields'
require_text "$trigger_migration" 'OLD.status = NEW.status'
require_text "$storage" 'load_pending_refund_claim_cancellation_reconciliation'
require_text "$storage" 'load_refund_claim_cancellation_evidence_expectation'
require_text "$storage" 'reconciliation_claim_token = $6::uuid'
require_text "$service" 'reconcile_one_refund_claim_cancellation_once'
require_text "$service" 'RefundClaimCancellationReconciliationMetricsSnapshot'
require_text "$main" 'refund_claim_cancellation_reconcile'
[ -x "$drill" ] || fail "missing executable two-replica reconciliation drill"
require_text "$drill" 'run --detach --no-deps indexer'
require_text "$drill" 'reconciliation_claim_token'
require_text "$drill" 'refund_claim_cancellation_intent.finalized'
require_text "$drill" 'FROM raw_chain_evidence'
require_text "$fixture" 'getSignaturesForAddress'
require_text "$fixture" 'ADJUSTMENT_REFUND_CLAIM_CANCELLED_EVENT_DISCRIMINATOR'
require_text "$fixture_script" 'cancellation_evidence_digest_hex'
require_text "$compose" 'RPC_FIXTURE_REFUND_CANCELLATION_CLAIM'

echo "refund claim cancellation reconciliation contract passed"
