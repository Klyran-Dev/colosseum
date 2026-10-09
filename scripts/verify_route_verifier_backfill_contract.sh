#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
lib="$repo_root/crates/route-verifier/src/lib.rs"
main="$repo_root/crates/route-verifier/src/main.rs"

fail() {
    echo "route-verifier backfill contract failed: $1" >&2
    exit 1
}

grep -Fq 'ROUTE_VERIFIER_MODE' "$lib" || fail "mode configuration is not explicit"
grep -Fq 'ROUTE_VERIFIER_BACKFILL_APPROVAL_ID' "$lib" || fail "approval configuration is not explicit"
grep -Fq 'RouteVerifierRunMode::Backfill' "$main" || fail "main does not branch into backfill mode"
grep -Fq 'RouteVerificationRunKind::Backfill' "$main" || fail "main does not persist backfill run kind"
grep -Fq 'verify_once_with_kind' "$main" || fail "main does not invoke the bounded one-shot path"
grep -Fq 'begin_settlement_route_verification_run' "$lib" || fail "route run persistence is not wired"

branch=$(awk '
    /if config\.run_mode == RouteVerifierRunMode::Backfill/ { in_branch = 1 }
    in_branch { print }
    in_branch && /^    let health_state/ { exit }
' "$main")
[ -n "$branch" ] || fail "backfill branch could not be located before health listener setup"
printf '%s\n' "$branch" | grep -Fq 'return Ok(())' || fail "backfill mode does not exit after one pass"
printf '%s\n' "$branch" | grep -Fq 'RouteVerificationRunKind::Backfill' || fail "backfill branch is not explicitly scoped"
if printf '%s\n' "$branch" | grep -Fq 'verify_refund_funding_once'; then
    fail "backfill mode runs the refund-funding verifier"
fi
if printf '%s\n' "$branch" | grep -Fq 'verify_revenue_stream_destinations_once'; then
    fail "backfill mode runs the revenue-destination verifier"
fi

echo "route-verifier backfill contract passed"
