#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

fail() {
  echo "hosted Solana configuration contract failed: $1" >&2
  exit 1
}

require_text() {
  file=$1
  text=$2
  grep -Fq "$text" "$repo_root/$file" || fail "$file is missing the required contract text: $text"
}

require_text compose.yaml 'VITE_SOLANA_RPC_URL: ${VITE_SOLANA_RPC_URL:-http://localhost:8899}'
require_text compose.yaml 'pg_isready -U \"$${POSTGRES_USER}\" -d \"$${POSTGRES_DB}\"'
require_text ops/docker/Dockerfile.web 'ARG VITE_SOLANA_RPC_URL'
require_text ops/docker/Dockerfile.web 'ENV VITE_SOLANA_RPC_URL=${VITE_SOLANA_RPC_URL}'
require_text ops/docker/Dockerfile.api 'COPY scripts/verify_deployment_config.sh /usr/local/bin/verify_deployment_config'
require_text Anchor.toml '[programs.devnet]'
require_text Anchor.toml 'settlement = "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi"'
require_text docs/runbooks/gcp-cloudflare-deployment.md 'VITE_SOLANA_RPC_URL'

echo "hosted Solana configuration contract passed"
