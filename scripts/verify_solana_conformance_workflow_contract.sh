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
    require_job_text "$workflow" "$job" 'SOLANA_USE_DOCKER_BUILDER: true'
    require_job_text "$workflow" "$job" 'run: make solana-conformance'
    require_job_text "$workflow" "$job" 'run: ./scripts/verify_solana_conformance_evidence.sh'
    require_job_text "$workflow" "$job" 'if: always()'
    require_job_text "$workflow" "$job" 'path: test-results/solana-conformance/'
    require_job_text "$workflow" "$job" 'if-no-files-found: error'
    require_job_text "$workflow" "$job" 'retention-days:'
    require_order "$workflow" "$job" 'run: make solana-conformance' 'run: ./scripts/verify_solana_conformance_evidence.sh'
}

ci_workflow="$repo_root/.github/workflows/ci.yml"
release_workflow="$repo_root/.github/workflows/release-candidate.yml"
[ -f "$ci_workflow" ] || fail "missing $ci_workflow"
[ -f "$release_workflow" ] || fail "missing $release_workflow"

check_workflow "$ci_workflow" solana-conformance
check_workflow "$release_workflow" release-candidate

echo "Solana conformance workflow contract passed"
