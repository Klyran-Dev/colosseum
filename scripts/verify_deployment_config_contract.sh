#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

synthetic_database='postgres://staging_user:synthetic@db.example.invalid:5432/tax_protocol?sslmode=verify-full'
synthetic_rpc='https://rpc-a.example.invalid,https://rpc-b.example.invalid'

run_checker() {
  simulation_mode=${1:-false}
  seed_dir=${2:-}
  insecure_rpc=${3:-false}
  set -- env -i \
    PATH="$PATH" \
    DEPLOYMENT_ENV=staging \
    API_ENV=staging \
    API_SIMULATION_MODE="$simulation_mode" \
    API_ALLOW_INSECURE_LOCAL_RPC="$insecure_rpc" \
    INDEXER_ENV=staging \
    INDEXER_ALLOW_INSECURE_LOCAL_RPC=false \
    ROUTE_VERIFIER_ENV=staging \
    ROUTE_VERIFIER_ALLOW_INSECURE_LOCAL_RPC=false \
    DATABASE_URL="$synthetic_database" \
    API_DATABASE_URL="$synthetic_database" \
    WORKER_DATABASE_URL="$synthetic_database" \
    INDEXER_DATABASE_URL="$synthetic_database" \
    ROUTE_VERIFIER_DATABASE_URL="$synthetic_database" \
    WORKER_NATS_URL=nats://messaging.example.invalid:4222 \
    API_SETTLEMENT_RPC_ENDPOINTS="$synthetic_rpc" \
    API_INDEXER_RPC_ENDPOINTS="$synthetic_rpc" \
    INDEXER_RPC_ENDPOINTS="$synthetic_rpc" \
    ROUTE_VERIFIER_RPC_ENDPOINTS="$synthetic_rpc" \
    API_AUTH_ISSUER=https://identity.example.invalid \
    API_AUTH_AUDIENCE=tax-api-staging \
    API_AUTH_JWKS_KID=staging-auth-key-v1 \
    API_AUTH_JWKS_PUBLIC_KEY=synthetic-public-key \
    API_EVIDENCE_KEY_ID=staging-evidence-key-v1 \
    API_EVIDENCE_KEY_B64=synthetic-evidence-key \
    API_JURISDICTION_PROVIDER_KEY_ID=staging-provider-key-v1 \
    API_JURISDICTION_PROVIDER_PUBLIC_KEY=synthetic-provider-key
  if [ -n "$seed_dir" ]; then
    set -- "$@" SEED_DIR="$seed_dir"
  fi
  "$@" ./scripts/verify_deployment_config.sh
}

run_checker

if run_checker true >/dev/null 2>&1; then
  echo "simulation-mode negative contract did not fail" >&2
  exit 1
fi

if run_checker false '' true >/dev/null 2>&1; then
  echo "insecure-rpc negative contract did not fail" >&2
  exit 1
fi

if run_checker false /seeds >/dev/null 2>&1; then
  echo "seed-directory negative contract did not fail" >&2
  exit 1
fi

echo "deployment configuration contract passed"
