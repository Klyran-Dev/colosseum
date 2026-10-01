#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
project_name="${COMPOSE_PROJECT_NAME:-solana-tax-protocol-local}"
wait_timeout="${BOOTSTRAP_WAIT_TIMEOUT_SECONDS:-180}"

case "$project_name" in
    ''|*[!A-Za-z0-9_.-]*)
        echo "COMPOSE_PROJECT_NAME contains unsupported characters" >&2
        exit 2
        ;;
esac
case "$wait_timeout" in
    ''|*[!0-9]*|0)
        echo "BOOTSTRAP_WAIT_TIMEOUT_SECONDS must be a positive integer" >&2
        exit 2
        ;;
esac

set -- \
    --profile cache \
    --profile observability \
    --profile indexer \
    --profile route-verifier \
    --profile solana \
    up --build --detach --wait --wait-timeout "$wait_timeout" \
    postgres migrator api tax-engine nats worker redis \
    alert-receiver alertmanager prometheus grafana otel-collector \
    indexer route-verifier validator edge

if [ "${BOOTSTRAP_DRY_RUN:-false}" = "true" ]; then
    printf 'build_sbf: scripts/build_sbf.sh\n'
    printf 'docker compose -p %s' "$project_name"
    for argument in "$@"; do
        printf ' %s' "$argument"
    done
    printf '\n'
    exit 0
fi

cd "$repo_root"
echo "Starting full local topology in Compose project $project_name"
echo "Building the pinned Solana SBF artifact"
if ! "$repo_root/scripts/build_sbf.sh"; then
    echo "SBF build failed; local topology was not started" >&2
    exit 1
fi
if ! docker compose -p "$project_name" "$@"; then
    echo "local topology bootstrap failed; preserving project $project_name for inspection" >&2
    docker compose -p "$project_name" ps >&2 || true
    docker compose -p "$project_name" logs --no-color --tail=120 validator api tax-engine edge indexer route-verifier >&2 || true
    exit 1
fi

docker compose -p "$project_name" ps
echo "Local UI: http://127.0.0.1:${EDGE_PORT:-8088}"
echo "API readiness: http://127.0.0.1:${API_PORT:-18080}/readyz"
echo "Validator health: http://127.0.0.1:${SOLANA_RPC_PORT:-8899}/health"
