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
printf 'sbf_artifact_sha256=%s\n' "$program_hash" > "$evidence_dir/artifact.txt"
printf '%s\n' 'container_id=integration-validator' 'image_id=sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' > "$evidence_dir/container-digests.txt"
printf '%s\n' 'agave-validator 4.3.0' > "$evidence_dir/validator.log"
printf '%s\n' 'validator-compose: pinned' > "$evidence_dir/compose-config.yml"
for expected_case in \
    'settles legacy SPL' \
    'real v0 address lookup table' \
    'extension-free Token-2022' \
    'settles a document-bound payment' \
    'immutable two-recipient Split Plan' \
    'extension-free Token-2022 two-recipient Split Plan' \
    'real Token-2022 TransferFeeConfig' \
    'revoked policy attestations'; do
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

RELEASE_BUNDLE_DIR="$bundle_dir" \
    "$repo_root/scripts/create_release_bundle.sh" >/dev/null

manifest="$bundle_dir/release-manifest.json"
test "$(jq -r '.integrity.program_binary.path' "$manifest")" = 'artifacts/solana/settlement.so'
test "$(jq -r '.integrity.program_binary.sha256' "$manifest")" = "$program_hash"
test "$(jq -r '.integrity.effective_configuration.path' "$manifest")" = 'artifacts/config/compose-config.yml'
test "$(jq -r '.integrity.effective_configuration.sha256' "$manifest")" = "$(shasum -a 256 "$bundle_dir/artifacts/config/compose-config.yml" | awk '{print $1}')"
test "$(jq '[.artifacts[] | select(.path == "artifacts/solana/settlement.so")] | length' "$manifest")" = 1
test "$(jq '[.artifacts[] | select(.path == "artifacts/config/compose-config.yml")] | length' "$manifest")" = 1
grep -Fq 'artifacts/config/compose-config.yml' "$bundle_dir/checksums.sha256"
grep -Fq 'artifacts/solana/settlement.so' "$bundle_dir/checksums.sha256"

rm -f "$bundle_dir/compose-config.yml"
if RELEASE_BUNDLE_DIR="$bundle_dir" \
    "$repo_root/scripts/create_release_bundle.sh" >/dev/null 2>&1; then
    echo "release bundle creation unexpectedly accepted missing resolved configuration" >&2
    exit 1
fi

echo "release bundle integrity contract passed"
