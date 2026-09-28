#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
bundle_dir="${RELEASE_BUNDLE_DIR:-$repo_root/release-bundle}"
artifacts_dir="$bundle_dir/artifacts"

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    echo "sha256sum or shasum is required" >&2
    exit 1
  fi
}

json_value() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g; s/	/\\t/g; s/\r/\\r/g'
}

fail_missing() {
  echo "release bundle input is missing: $1" >&2
  exit 1
}

copy_file() {
  source_path=$1
  destination_path=$2
  [ -f "$source_path" ] || fail_missing "$source_path"
  mkdir -p "$(dirname "$destination_path")"
  cp "$source_path" "$destination_path"
}

repo_revision=$(git -C "$repo_root" rev-parse HEAD)
release_ref=${RELEASE_REF:-${GITHUB_REF_NAME:-$(git -C "$repo_root" describe --tags --always --dirty)}}
source_date_epoch=${SOURCE_DATE_EPOCH:-$(git -C "$repo_root" show -s --format=%ct HEAD)}

case "$repo_revision" in
  *[!0123456789abcdef]*|'')
    echo "source revision is not a full hexadecimal commit: $repo_revision" >&2
    exit 1
    ;;
esac
case "$source_date_epoch" in
  *[!0123456789]*|'')
    echo "SOURCE_DATE_EPOCH must be a decimal Unix timestamp" >&2
    exit 1
    ;;
esac

[ -f "$bundle_dir/security-scan.txt" ] || fail_missing "$bundle_dir/security-scan.txt"
[ -f "$bundle_dir/sbom.cdx.json" ] || fail_missing "$bundle_dir/sbom.cdx.json"
[ -f "$bundle_dir/validator-conformance/result.env" ] || fail_missing "$bundle_dir/validator-conformance/result.env"
grep -Fq 'status=0' "$bundle_dir/validator-conformance/result.env" || {
  echo "validator conformance evidence is not successful" >&2
  exit 1
}

rm -rf "$artifacts_dir"
mkdir -p "$artifacts_dir"

copy_file "$repo_root/target/release/api" "$artifacts_dir/bin/api"
copy_file "$repo_root/target/release/indexer-service" "$artifacts_dir/bin/indexer-service"
copy_file "$repo_root/target/release/projection-rebuild" "$artifacts_dir/bin/projection-rebuild"
copy_file "$repo_root/target/release/route-verifier" "$artifacts_dir/bin/route-verifier"
copy_file "$repo_root/target/release/worker" "$artifacts_dir/bin/worker"
copy_file "$repo_root/target/deploy/settlement.so" "$artifacts_dir/solana/settlement.so"
[ -d "$repo_root/apps/web/dist" ] || fail_missing "$repo_root/apps/web/dist"
cp -R "$repo_root/apps/web/dist" "$artifacts_dir/web"

{
  printf 'source_revision=%s\n' "$repo_revision"
  printf 'release_ref=%s\n' "$release_ref"
  printf 'source_date_epoch=%s\n' "$source_date_epoch"
  rustc --version
  cargo --version
  node --version
  pnpm --version
  docker version --format 'docker_server={{.Server.Version}}' 2>/dev/null || true
  docker compose version 2>/dev/null || true
} > "$artifacts_dir/build-environment.txt"

artifact_paths="$bundle_dir/artifact-paths.txt"
find "$artifacts_dir" -type f -print \
  | sed "s#^$bundle_dir/##" \
  | LC_ALL=C sort > "$artifact_paths"

checksums="$bundle_dir/checksums.sha256"
: > "$checksums"
while IFS= read -r relative_path; do
  hash=$(sha256_file "$bundle_dir/$relative_path")
  printf '%s  %s\n' "$hash" "$relative_path" >> "$checksums"
done < "$artifact_paths"

while IFS=' ' read -r expected_hash relative_path; do
  [ -n "$expected_hash" ] || continue
  actual_hash=$(sha256_file "$bundle_dir/$relative_path")
  [ "$actual_hash" = "$expected_hash" ] || {
    echo "artifact checksum changed after generation: $relative_path" >&2
    exit 1
  }
done < "$checksums"

manifest="$bundle_dir/release-manifest.json"
{
  printf '{\n'
  printf '  "schema_version": "solana-tax-protocol/release-candidate/v1",\n'
  printf '  "source": {"revision": "%s", "ref": "%s"},\n' \
    "$(json_value "$repo_revision")" "$(json_value "$release_ref")"
  printf '  "build": {"source_date_epoch": %s, "artifacts_are_not_deployments": true},\n' "$source_date_epoch"
  printf '  "evidence": {"security_scan": "security-scan.txt", "sbom": "sbom.cdx.json", "validator_conformance": "validator-conformance/result.env"},\n'
  printf '  "artifacts": [\n'
  first=true
  while IFS= read -r relative_path; do
    [ -n "$relative_path" ] || continue
    [ "$first" = true ] || printf ',\n'
    first=false
    hash=$(sha256_file "$bundle_dir/$relative_path")
    printf '    {"path": "%s", "sha256": "%s"}' \
      "$(json_value "$relative_path")" "$hash"
  done < "$artifact_paths"
  printf '\n  ]\n}\n'
} > "$manifest"

rm -f "$artifact_paths"
echo "release candidate bundle created at $bundle_dir"
