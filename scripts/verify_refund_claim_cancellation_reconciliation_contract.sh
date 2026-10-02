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

migration=db/migrations/0057_refund_claim_cancellation_reconciliation_leases.sql
storage=crates/storage/src/lib.rs
service=crates/indexer-service/src/lib.rs
main=crates/indexer-service/src/main.rs
fixture=crates/rpc-fixture/src/main.rs
drill=scripts/refund_cancellation_reconciliation_drill.sh
fixture_script=apps/web/scripts/refund_cancellation_fixture.mjs
compose=compose.yaml

[ -f "$migration" ] || fail "missing additive lease migration"
require_text "$migration" 'reconciliation_claim_token uuid'
require_text "$migration" 'uuid_extract_version(reconciliation_claim_token) = 7'
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
require_text "$fixture" 'getSignaturesForAddress'
require_text "$fixture" 'ADJUSTMENT_REFUND_CLAIM_CANCELLED_EVENT_DISCRIMINATOR'
require_text "$fixture_script" 'cancellation_evidence_digest_hex'
require_text "$compose" 'RPC_FIXTURE_REFUND_CANCELLATION_CLAIM'

echo "refund claim cancellation reconciliation contract passed"
