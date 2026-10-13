#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

fail() {
    echo "Solana conformance workflow contract failed: $1" >&2
    exit 1
}

require_text() {
    file=$1
    text=$2
    grep -Fq "$text" "$file" || fail "$file is missing: $text"
}

require_job_text() {
    workflow=$1
    job=$2
    text=$3
    block=$(awk -v job="$job" '
        $0 == "  " job ":" { in_job = 1; next }
        in_job && $0 ~ /^  [A-Za-z0-9_-]+:/ { exit }
        in_job { print }
    ' "$workflow")
    [ -n "$block" ] || fail "$workflow is missing job $job"
    printf '%s\n' "$block" | grep -Fq "$text" ||
        fail "$workflow job $job is missing: $text"
}

require_order() {
    workflow=$1
    job=$2
    first=$3
    second=$4
    block=$(awk -v job="$job" '
        $0 == "  " job ":" { in_job = 1; next }
        in_job && $0 ~ /^  [A-Za-z0-9_-]+:/ { exit }
        in_job { print }
    ' "$workflow")
    first_line=$(printf '%s\n' "$block" | grep -n -F -m1 "$first" | cut -d: -f1 || true)
    second_line=$(printf '%s\n' "$block" | grep -n -F -m1 "$second" | cut -d: -f1 || true)
    [ -n "$first_line" ] && [ -n "$second_line" ] && [ "$first_line" -lt "$second_line" ] ||
        fail "$workflow job $job has invalid order: $first before $second"
}

check_workflow() {
    workflow=$1
    job=$2
    require_job_text "$workflow" "$job" "runs-on: ubuntu-24.04"
    require_job_text "$workflow" "$job" 'test "$(uname -s)" = Linux'
    require_job_text "$workflow" "$job" 'test "$(docker info --format '\''{{.OSType}}'\'')" = linux'
    require_job_text "$workflow" "$job" 'amd64|x86_64'
    require_job_text "$workflow" "$job" 'run: ./scripts/verify_solana_conformance_host.sh'
    require_job_text "$workflow" "$job" 'SOLANA_USE_DOCKER_BUILDER: true'
    require_job_text "$workflow" "$job" 'run: make solana-conformance'
    require_job_text "$workflow" "$job" 'run: ./scripts/verify_solana_conformance_evidence.sh'
    require_job_text "$workflow" "$job" 'if: always()'
    require_job_text "$workflow" "$job" 'path: test-results/solana-conformance/'
    require_job_text "$workflow" "$job" 'if-no-files-found: error'
    require_job_text "$workflow" "$job" 'retention-days:'
    require_order "$workflow" "$job" 'run: ./scripts/verify_solana_conformance_host.sh' 'run: make solana-conformance'
    require_order "$workflow" "$job" 'run: make solana-conformance' 'run: ./scripts/verify_solana_conformance_evidence.sh'
}

ci_workflow="$repo_root/.github/workflows/ci.yml"
release_workflow="$repo_root/.github/workflows/release-candidate.yml"
dedicated_workflow="$repo_root/.github/workflows/solana-conformance.yml"
[ -f "$ci_workflow" ] || fail "missing $ci_workflow"
[ -f "$release_workflow" ] || fail "missing $release_workflow"
[ -f "$dedicated_workflow" ] || fail "missing $dedicated_workflow"

require_text "$ci_workflow" '  workflow_dispatch:'
require_text "$ci_workflow" '      run_extended:'
require_text "$ci_workflow" '        default: false'
require_text "$ci_workflow" '        type: boolean'
for extended_job in \
    tax-engine-load \
    otel-trace \
    solana-conformance \
    indexer-replay \
    indexer-replay-dead-letter \
    alert-delivery \
    refund-funding-verifier \
    refund-proof-lease \
    refund-execute-api-contention \
    quote-load \
    document-quote-load \
    rule-simulation-load \
    postgres-recovery \
    postgres-encrypted-recovery \
    projection-recovery \
    vault-reconciliation \
    rpc-failover \
    postgres-contention \
    postgres-pool \
    postgres-index-plan \
    postgres-pitr \
    migration-integrity \
    migration-rollback; do
    require_job_text "$ci_workflow" "$extended_job" "if: \${{ github.event_name == 'workflow_dispatch' && inputs.run_extended == true }}"
done

check_workflow "$ci_workflow" solana-conformance
check_workflow "$release_workflow" release-candidate
check_workflow "$dedicated_workflow" solana-conformance
require_job_text "$release_workflow" release-candidate 'run: make release-bundle-integrity-contract'
require_job_text "$release_workflow" release-candidate 'run: ./scripts/create_release_bundle.sh'
require_job_text "$release_workflow" release-candidate 'run: make web-browser-check'
require_job_text "$release_workflow" release-candidate 'cp -R test-results/web-e2e/. release-bundle/web-e2e/'
require_job_text "$release_workflow" release-candidate 'git rev-parse HEAD'
require_job_text "$release_workflow" release-candidate 'path: test-results/web-e2e/'
require_job_text "$release_workflow" release-candidate 'make integration-check 2>&1 | tee release-bundle/integration/evidence.txt'
require_job_text "$release_workflow" release-candidate 'id: authenticated_integration'
require_job_text "$release_workflow" release-candidate 'tee release-bundle/integration/evidence.txt'
require_job_text "$release_workflow" release-candidate 'release-bundle/integration/source-revision.txt'
require_job_text "$release_workflow" release-candidate 'path: release-bundle/integration/'
require_job_text "$release_workflow" release-candidate "if: failure() && steps.authenticated_integration.outcome == 'failure'"
require_order "$release_workflow" release-candidate 'run: make web-browser-check' 'run: make release-bundle-integrity-contract'
require_order "$release_workflow" release-candidate 'make integration-check 2>&1 | tee release-bundle/integration/evidence.txt' 'run: make web-browser-check'
require_order "$release_workflow" release-candidate 'run: make release-bundle-integrity-contract' 'run: ./scripts/create_release_bundle.sh'

echo "Solana conformance workflow contract passed"
