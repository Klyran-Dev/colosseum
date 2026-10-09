#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
work_dir=$(mktemp -d)
cleanup() {
  rm -rf "$work_dir"
}
trap cleanup EXIT INT TERM

mkdir -p "$work_dir/docs/plans" "$work_dir/docs/operations" "$work_dir/.github/workflows" "$work_dir/scripts"
cp "$repo_root/README.md" "$work_dir/README.md"
cp "$repo_root/docs/public-repository-release.md" "$work_dir/docs/public-repository-release.md"
cp "$repo_root/docs/plans/production-readiness.md" "$work_dir/docs/plans/production-readiness.md"
cp "$repo_root/docs/operations/decision-register.md" "$work_dir/docs/operations/decision-register.md"
cp "$repo_root/.github/workflows/release-candidate.yml" "$work_dir/.github/workflows/release-candidate.yml"
cp "$repo_root/scripts/create_release_bundle.sh" "$work_dir/scripts/create_release_bundle.sh"

RELEASE_CLAIM_REPO_ROOT="$work_dir" "$repo_root/scripts/verify_release_claim_contract.sh" >/dev/null

printf '\nThis repository is production-ready.\n' >> "$work_dir/README.md"
if RELEASE_CLAIM_REPO_ROOT="$work_dir" "$repo_root/scripts/verify_release_claim_contract.sh" >/dev/null 2>&1; then
  echo "release claim contract accepted an unsupported production claim" >&2
  exit 1
fi

cp "$repo_root/README.md" "$work_dir/README.md"
touch "$work_dir/LICENSE"
if RELEASE_CLAIM_REPO_ROOT="$work_dir" "$repo_root/scripts/verify_release_claim_contract.sh" >/dev/null 2>&1; then
  echo "release claim contract accepted an unapproved license file" >&2
  exit 1
fi

echo "release claim contract mutation tests passed"
