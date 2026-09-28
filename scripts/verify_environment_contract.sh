#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

failures=0

require_text() {
  file=$1
  text=$2
  if ! grep -Fq "$text" "$file"; then
    echo "missing environment contract text in $file: $text" >&2
    failures=$((failures + 1))
  fi
}

require_text compose.yaml 'API_ENV: ${API_ENV:-local}'
require_text compose.yaml 'API_SIMULATION_MODE: ${API_SIMULATION_MODE:-true}'
require_text crates/api/src/config.rs 'API_SIMULATION_MODE=true is permitted only in API_ENV=local'
require_text crates/api/src/config.rs 'API_ALLOW_INSECURE_LOCAL_RPC=true is permitted only in API_ENV=local'
require_text compose.yaml 'INDEXER_ENV: ${INDEXER_ENV:-local}'
require_text compose.yaml 'ROUTE_VERIFIER_ENV: ${ROUTE_VERIFIER_ENV:-local}'
require_text crates/indexer-service/src/lib.rs 'INDEXER_ENV must be exactly local, staging, or production'
require_text crates/route-verifier/src/lib.rs 'ROUTE_VERIFIER_ENV must be exactly local, staging, or production'

if grep -Fq 'API_ENV: ${API_ENV:-}' compose.yaml; then
  echo "Compose must not use an empty deployment-environment default" >&2
  failures=$((failures + 1))
fi

if [ "$failures" -ne 0 ]; then
  echo "environment contract failed with $failures issue(s)" >&2
  exit 1
fi

echo "environment contract passed"
