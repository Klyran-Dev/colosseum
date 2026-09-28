#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

if ! grep -Fq 'dockerfile: ops/docker/Dockerfile.migration-runner' compose.yaml; then
    echo "Compose does not define the versioned migration runner" >&2
    exit 1
fi
if ! grep -Fq -- '- ./db/migrations:/migrations:ro' compose.yaml; then
    echo "Compose does not mount migrations read-only into the runner" >&2
    exit 1
fi
if ! grep -Fq -- '- ./db/fixtures:/seeds:ro' compose.yaml; then
    echo "Compose does not isolate local synthetic seeds from migrations" >&2
    exit 1
fi
if ! grep -Fq 'service_completed_successfully' compose.yaml; then
    echo "application services do not wait for successful migration completion" >&2
    exit 1
fi

echo "versioned migration bootstrap contract passed"
