#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
bundle_dir=$(mktemp -d)
evidence_dir="$bundle_dir/validator-conformance"
cleanup() {
    rm -rf "$bundle_dir"
}
trap cleanup EXIT INT TERM

mkdir -p "$evidence_dir"
revision=$(git -C "$repo_root" rev-parse HEAD)
program_hash=$(shasum -a 256 "$repo_root/target/deploy/settlement.so" | awk '{print $1}')

cat > "$evidence_dir/metadata.txt" <<EOF
git_revision=$revision
platform=Linux test-runner
docker_server_architecture=amd64
validator_image=agave-validator:4.3.0
EOF
cat > "$evidence_dir/preflight.txt" <<'EOF'
schema_version=solana-tax-protocol/conformance-host/v1
status=ready
failure_reason=none
host_os=Linux
host_architecture=x86_64
docker_ostype=linux
docker_server_architecture=amd64
EOF
printf 'sbf_artifact_sha256=%s\n' "$program_hash" > "$evidence_dir/artifact.txt"
printf '%s\n' 'container_id=integration-validator' 'image_id=sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' > "$evidence_dir/container-digests.txt"
printf '%s\n' 'agave-validator 4.3.0' > "$evidence_dir/validator.log"
printf '%s\n' 'validator-compose: pinned' > "$evidence_dir/compose-config.yml"
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
    'extension-free Token-2022 two-recipient Split Plan' \
    'real Token-2022 TransferFeeConfig' \
    'revoked policy attestations' \
    'non-stable asset classification' \
    'rejects wrong recipient order without balance drift' \
    'revenue-stream single-use claim'; do
    printf '%s\n' "$expected_case" >> "$evidence_dir/localnet-tests.log"
done
cat > "$evidence_dir/result.env" <<'EOF'
schema_version=solana-tax-protocol/conformance-result/v1
phase=completed
status=0
failure_reason=none
EOF
printf '%s\n' 'synthetic security scan: no findings' > "$bundle_dir/security-scan.txt"
printf '%s\n' '{"bomFormat":"CycloneDX","specVersion":"1.5","components":[]}' > "$bundle_dir/sbom.cdx.json"
printf '%s\n' 'services: api, postgres18, nginx' > "$bundle_dir/compose-config.yml"
mkdir -p "$bundle_dir/web-e2e"
printf '%s\n' '{"status":"passed","synthetic":true}' > "$bundle_dir/web-e2e/results.json"
printf 'git_revision=%s\n' "$revision" > "$bundle_dir/web-e2e/source-revision.txt"
mkdir -p "$bundle_dir/integration"
printf '%s\n' '{"status":"passed","synthetic":true,"scenario":"authenticated-integration"}' > "$bundle_dir/integration/evidence.txt"
printf 'git_revision=%s\n' "$revision" > "$bundle_dir/integration/source-revision.txt"

RELEASE_BUNDLE_DIR="$bundle_dir" \
    "$repo_root/scripts/create_release_bundle.sh" >/dev/null

manifest="$bundle_dir/release-manifest.json"
test "$(jq -r '.integrity.program_binary.path' "$manifest")" = 'artifacts/solana/settlement.so'
test "$(jq -r '.integrity.program_binary.sha256' "$manifest")" = "$program_hash"
test "$(jq -r '.integrity.effective_configuration.path' "$manifest")" = 'artifacts/config/compose-config.yml'
test "$(jq -r '.integrity.effective_configuration.sha256' "$manifest")" = "$(shasum -a 256 "$bundle_dir/artifacts/config/compose-config.yml" | awk '{print $1}')"
test "$(jq -r '.evidence.browser_ui' "$manifest")" = 'artifacts/evidence/web-e2e/results.json'
test "$(jq -r '.evidence.authenticated_integration' "$manifest")" = 'artifacts/evidence/integration/evidence.txt'
test -f "$bundle_dir/artifacts/evidence/web-e2e/source-revision.txt"
test -f "$bundle_dir/artifacts/evidence/integration/source-revision.txt"
test "$(jq '[.artifacts[] | select(.path == "artifacts/solana/settlement.so")] | length' "$manifest")" = 1
test "$(jq '[.artifacts[] | select(.path == "artifacts/config/compose-config.yml")] | length' "$manifest")" = 1
grep -Fq 'artifacts/config/compose-config.yml' "$bundle_dir/checksums.sha256"
grep -Fq 'artifacts/solana/settlement.so' "$bundle_dir/checksums.sha256"
grep -Fq 'artifacts/evidence/web-e2e/results.json' "$bundle_dir/checksums.sha256"
grep -Fq 'artifacts/evidence/integration/evidence.txt' "$bundle_dir/checksums.sha256"

printf 'git_revision=%s\n' "$(printf '%040d' 0)" > "$bundle_dir/web-e2e/source-revision.txt"
if RELEASE_BUNDLE_DIR="$bundle_dir" \
    "$repo_root/scripts/create_release_bundle.sh" >/dev/null 2>&1; then
    echo "release bundle creation unexpectedly accepted browser evidence from another revision" >&2
    exit 1
fi
printf 'git_revision=%s\n' "$revision" > "$bundle_dir/web-e2e/source-revision.txt"

printf 'git_revision=%s\n' "$(printf '%040d' 0)" > "$bundle_dir/integration/source-revision.txt"
if RELEASE_BUNDLE_DIR="$bundle_dir" \
    "$repo_root/scripts/create_release_bundle.sh" >/dev/null 2>&1; then
    echo "release bundle creation unexpectedly accepted integration evidence from another revision" >&2
    exit 1
fi
printf 'git_revision=%s\n' "$revision" > "$bundle_dir/integration/source-revision.txt"

metadata_tmp="$evidence_dir/metadata.txt.tmp"
sed "s/^git_revision=.*/git_revision=$(printf '%040d' 0)/" "$evidence_dir/metadata.txt" > "$metadata_tmp"
mv "$metadata_tmp" "$evidence_dir/metadata.txt"
if RELEASE_BUNDLE_DIR="$bundle_dir" \
    "$repo_root/scripts/create_release_bundle.sh" >/dev/null 2>&1; then
    echo "release bundle creation unexpectedly accepted validator evidence from another revision" >&2
    exit 1
fi
printf 'git_revision=%s\n' "$revision" > "$evidence_dir/metadata.txt"

rm -f "$bundle_dir/compose-config.yml"
if RELEASE_BUNDLE_DIR="$bundle_dir" \
    "$repo_root/scripts/create_release_bundle.sh" >/dev/null 2>&1; then
    echo "release bundle creation unexpectedly accepted missing resolved configuration" >&2
    exit 1
fi

echo "release bundle integrity contract passed"
