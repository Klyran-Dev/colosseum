#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
benchmark="$repo_root/scripts/benchmark_authorization_capabilities.sh"

fail() {
    echo "authorization benchmark contract failed: $1" >&2
    exit 1
}

[ -f "$benchmark" ] || fail "benchmark script is missing"

# Keep the benchmark's semantic response contract aligned with the canonical
# server-owned catalog. A benchmark that omits a workspace can report green
# while never validating that workspace's authorization decision.
for capability in \
    'access.control' \
    'evidence.audit.read' \
    'evidence.replay' \
    'refund.claim.cancel' \
    'refund.claim.create' \
    'refund.payment.release' \
    'revenue_stream.claim.prepare' \
    'revenue_stream.govern' \
    'revenue_stream.propose' \
    'revenue_stream.read' \
    'settlement.adjustment' \
    'settlement.document.wallet' \
    'settlement.wallet' \
    'tax.document.quote.create' \
    'tax.preview.run' \
    'tax.quote.create' \
    'tax.rule.govern' \
    'vault.reconciliation' \
    'vault.release'; do
    grep -Fq "\"$capability\"" "$benchmark" || fail "benchmark does not validate $capability"
done

grep -Fq '((.capabilities | keys) == [' "$benchmark" \
    || fail 'benchmark does not validate the complete capability key set'
grep -Fq 'decision == "indeterminate"' "$benchmark" \
    || fail 'benchmark does not validate indeterminate ABAC decisions'

echo "authorization benchmark contract passed"
