#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
evidence_dir=${1:-${SOLANA_CONFORMANCE_EVIDENCE_DIR:-$repo_root/test-results/solana-conformance}}

fail() {
    echo "solana conformance evidence verification failed: $1" >&2
    exit 1
}

require_file() {
    [ -s "$evidence_dir/$1" ] || fail "missing or empty $1"
}

require_file metadata.txt
require_file preflight.txt

grep -Fxq 'schema_version=solana-tax-protocol/conformance-host/v1' "$evidence_dir/preflight.txt" \
    || fail "unsupported conformance host preflight schema"
grep -Fxq 'status=ready' "$evidence_dir/preflight.txt" \
    || fail "conformance host preflight did not pass"
grep -Fxq 'failure_reason=none' "$evidence_dir/preflight.txt" \
    || fail "conformance host preflight has a failure reason"
grep -Eq '^host_os=Linux$' "$evidence_dir/preflight.txt" \
    || fail "authoritative conformance must run on a Linux host"
grep -Eq '^host_architecture=(x86_64|amd64)$' "$evidence_dir/preflight.txt" \
    || fail "authoritative conformance must run on an amd64 host"
grep -Eq '^docker_ostype=linux$' "$evidence_dir/preflight.txt" \
    || fail "authoritative conformance must use a Linux Docker engine"
grep -Eq '^docker_server_architecture=(amd64|x86_64)$' "$evidence_dir/preflight.txt" \
    || fail "authoritative conformance must run on an amd64 Docker engine"

require_file artifact.txt
require_file compose-config.yml
require_file container-digests.txt
require_file validator.log
require_file localnet-tests.log
require_file result.env

result_value() {
    key=$1
    value=$(sed -n "s/^${key}=//p" "$evidence_dir/result.env")
    [ "$(printf '%s\n' "$value" | wc -l | tr -d ' ')" = "1" ] || fail "result.env must contain one $key field"
    printf '%s' "$value"
}

[ "$(result_value schema_version)" = "solana-tax-protocol/conformance-result/v1" ] \
    || fail "unsupported result.env schema"
[ "$(result_value phase)" = "completed" ] || fail "conformance did not complete"
[ "$(result_value status)" = "0" ] || fail "conformance status is not successful"
[ "$(result_value failure_reason)" = "none" ] || fail "conformance has a failure reason"

grep -Eq '^git_revision=[0-9a-f]{40}$' "$evidence_dir/metadata.txt" \
    || fail "metadata does not contain a full source revision"
grep -Eq '^platform=Linux ' "$evidence_dir/metadata.txt" \
    || fail "authoritative conformance must run on a Linux host"
grep -Eq '^docker_server_architecture=(amd64|x86_64)$' "$evidence_dir/metadata.txt" \
    || fail "authoritative conformance must run on an amd64 Docker engine"
grep -Fq 'validator' "$evidence_dir/metadata.txt" \
    || fail "metadata does not record the validator image"
grep -Eq '^container_id=.+$' "$evidence_dir/container-digests.txt" \
    || fail "validator container identity is missing"
grep -Eq '^image_id=sha256:[0-9a-f]{64}$' "$evidence_dir/container-digests.txt" \
    || fail "validator image identity is missing"

grep -Fq 'agave-validator 4.3.0' "$evidence_dir/validator.log" \
    || fail "validator log does not prove the pinned Agave version"
if grep -Fq 'io_uring NOT supported' "$evidence_dir/validator.log"; then
    fail "validator reported unavailable io_uring"
fi

[ -s "$repo_root/target/deploy/settlement.so" ] || fail "SBF artifact is missing"
if command -v sha256sum >/dev/null 2>&1; then
    actual_hash=$(sha256sum "$repo_root/target/deploy/settlement.so" | awk '{print $1}')
else
    actual_hash=$(shasum -a 256 "$repo_root/target/deploy/settlement.so" | awk '{print $1}')
fi
grep -Fq "sbf_artifact_sha256=$actual_hash" "$evidence_dir/artifact.txt" \
    || fail "retained SBF hash does not match the built artifact"

for expected_case in \
    'stablecoin account topology' \
    'settles legacy SPL' \
    'rejects disabled or substituted asset admission' \
    'payer-bound refund claim' \
    'governance cancellation' \
    'blocks paused vault release' \
    'real v0 address lookup table' \
    'extension-free Token-2022' \
    'settles a document-bound payment' \
    'immutable two-recipient Split Plan' \
    'rejects wrong recipient order without balance drift' \
    'extension-free Token-2022 two-recipient Split Plan' \
    'real Token-2022 TransferFeeConfig' \
    'non-stable asset classification' \
    'revoked policy attestations' \
    'revenue-stream single-use claim'; do
    grep -Fq "$expected_case" "$evidence_dir/localnet-tests.log" \
        || fail "localnet evidence is missing test case: $expected_case"
done

echo "verified Linux validator conformance evidence: $evidence_dir"
