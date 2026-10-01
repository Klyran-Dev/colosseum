#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
output=$(COMPOSE_PROJECT_NAME=bootstrap-contract BOOTSTRAP_DRY_RUN=true \
    "$repo_root/scripts/bootstrap_local.sh")

contains() {
    printf '%s\n' "$output" | grep -Fq -- "$1"
}

for required in \
    'build_sbf: scripts/build_sbf.sh' \
    'docker compose -p bootstrap-contract' \
    '--profile cache' \
    '--profile observability' \
    '--profile indexer' \
    '--profile route-verifier' \
    '--profile solana' \
    'up --build --detach --wait --wait-timeout 180' \
    'postgres migrator api tax-engine nats worker redis' \
    'alert-receiver alertmanager prometheus grafana otel-collector' \
    'indexer route-verifier validator edge'; do
    contains "$required" || {
        echo "bootstrap contract missing: $required" >&2
        exit 1
    }
done

compose_file=$(CDPATH= cd -- "$repo_root" && pwd)/compose.yaml
bootstrap_script=$(CDPATH= cd -- "$repo_root" && pwd)/scripts/bootstrap_local.sh
grep -Fq '"${SOLANA_RPC_PORT:-8899}:8899"' "$compose_file" || {
    echo "bootstrap contract missing SOLANA_RPC_PORT override" >&2
    exit 1
}
grep -Fq '"${SOLANA_FAUCET_PORT:-8900}:8900"' "$compose_file" || {
    echo "bootstrap contract missing SOLANA_FAUCET_PORT override" >&2
    exit 1
}

grep -Fq 'Validator health: http://127.0.0.1:${SOLANA_RPC_PORT:-8899}/health' "$bootstrap_script" || {
    echo "bootstrap contract missing configurable validator URL" >&2
    exit 1
}

if contains 'solana-conformance' || contains 'web-e2e'; then
    echo "bootstrap contract must not start one-shot test services" >&2
    exit 1
fi

echo "Docker full-topology bootstrap contract passed"
