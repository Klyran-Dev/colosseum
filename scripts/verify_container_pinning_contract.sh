#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

fail_unpinned() {
    file=$1
    kind=$2
    lines=$3
    if [ -n "$lines" ]; then
        printf '%s\n' "$lines" >&2
        echo "unpinned $kind found in $file" >&2
        exit 1
    fi
}

for dockerfile in $(find ops/docker -type f -name 'Dockerfile*' -print); do
    unpinned=$(grep -E '^FROM[[:space:]]' "$dockerfile" | \
        grep -Ev '@sha256:[0-9a-fA-F]{64}([[:space:]]|$)' || true)
    fail_unpinned "$dockerfile" Dockerfile "$unpinned"
done

compose_images=$(grep -E '^[[:space:]]*image:[[:space:]]+' compose.yaml | \
    grep -Ev '@sha256:[0-9a-fA-F]{64}([[:space:]]|$)' || true)
fail_unpinned compose.yaml Compose-image "$compose_images"

workflow_actions=$(grep -R -E '^[[:space:]]*uses:[[:space:]]+' .github/workflows | \
    grep -Ev '@[0-9a-fA-F]{40}([[:space:]]|$)' || true)
fail_unpinned .github/workflows GitHub-action "$workflow_actions"

echo "container and workflow pinning contract passed"
