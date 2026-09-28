#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
bundle_dir="${RELEASE_BUNDLE_DIR:-$repo_root/release-bundle}"
trivy_image=${TRIVY_IMAGE:-aquasec/trivy:0.74.0@sha256:62b1e65e8869bc4b4c6aa4fa2b21595256c7c2f6018a9d9ad61caf87187c1969}

command -v docker >/dev/null 2>&1 || {
  echo "Docker is required to generate the pinned release SBOM" >&2
  exit 1
}

mkdir -p "$bundle_dir"
docker run --rm \
  --mount "type=bind,src=$repo_root,dst=/src" \
  "$trivy_image" \
  fs \
  --skip-version-check \
  --scanners vuln,license \
  --skip-dirs /src/target \
  --skip-dirs /src/apps/web/node_modules \
  --skip-dirs /src/release-bundle \
  --format cyclonedx \
  --output /src/release-bundle/sbom.cdx.json \
  /src
