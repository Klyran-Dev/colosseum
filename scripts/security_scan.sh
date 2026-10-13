#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
trivy_image=${TRIVY_IMAGE:-aquasec/trivy:0.74.0@sha256:62b1e65e8869bc4b4c6aa4fa2b21595256c7c2f6018a9d9ad61caf87187c1969}

exec docker run --rm \
  --mount "type=bind,src=${repo_root},dst=/src,readonly" \
  "$trivy_image" \
  fs \
  --skip-version-check \
  --timeout 15m \
  --scanners vuln,secret,misconfig \
  --exit-code 1 \
  --severity CRITICAL,HIGH \
  --skip-dirs /src/.git \
  --skip-dirs /src/target \
  --skip-dirs /src/apps/web/node_modules \
  --skip-dirs /src/apps/web/dist \
  --skip-dirs /src/evm-adapter/cache \
  --skip-dirs /src/evm-adapter/out \
  --skip-dirs /src/release-bundle \
  --skip-dirs /src/test-results \
  --skip-dirs /src/tmp \
  --skip-files /src/.DS_Store \
  --skip-files '/src/**/.DS_Store' \
  /src
