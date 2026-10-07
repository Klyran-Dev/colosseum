#!/bin/sh
# Capture e2e style snapshots into test-results/style-snapshots/<label> (ADR-0428).
#   scripts/web_style_snapshot.sh <label>
# Runs the real-browser journeys in an isolated Compose project with
# STYLE_SNAPSHOT_DIR set. Compare labels with scripts/diff_style_snapshots.mjs.
set -eu
label=${1:?usage: web_style_snapshot.sh <label>}
case "$label" in *[!A-Za-z0-9._-]*) echo "label must be [A-Za-z0-9._-]" >&2; exit 2;; esac
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"
project="solana-tax-protocol-web-style"
mkdir -p "test-results/style-snapshots"
rm -rf "test-results/style-snapshots/$label"
cleanup() { docker compose -p "$project" -f compose.yaml -f compose.browser.yaml down -v --remove-orphans >/dev/null 2>&1 || true; }
trap cleanup EXIT INT TERM
fixture_args=""
if [ -d "test-results/style-fixtures" ]; then
  # Markup captured from unit tests by scripts/web_style_fixtures.sh (ADR-0429).
  fixture_args="-e STYLE_FIXTURE_INPUT_DIR=/workspace/artifacts/style-fixtures"
fi
# Optional parallelism (STYLE_SNAPSHOT_WORKERS=N): journeys and fixture states are
# independent (every API call is mocked per test), so they can run concurrently.
# Default is the repository configuration (one worker, serial).
workers="${STYLE_SNAPSHOT_WORKERS:-}"
case "$workers" in *[!0-9]*) echo "STYLE_SNAPSHOT_WORKERS must be a number" >&2; exit 2;; esac
run_e2e() {
  docker compose -p "$project" -f compose.yaml -f compose.browser.yaml --profile browser run --build --rm \
    -e "STYLE_SNAPSHOT_DIR=/workspace/artifacts/style-snapshots/$label" \
    -e "STYLE_SNAPSHOT_SCREENSHOTS=${STYLE_SNAPSHOT_SCREENSHOTS:-}" \
    -e "STYLE_SNAPSHOT_NO_STATES=${STYLE_SNAPSHOT_NO_STATES:-}" \
    $fixture_args web-e2e "$@"
}
if [ -n "$workers" ]; then
  run_e2e /bin/sh -c "/workspace/apps/web/node_modules/.bin/tsc --noEmit -p /workspace/apps/web/tsconfig.e2e.json && node /workspace/apps/web/node_modules/@playwright/test/cli.js test --config /workspace/apps/web/playwright.config.ts --workers=$workers --fully-parallel"
else
  run_e2e
fi
echo "snapshots: test-results/style-snapshots/$label ($(ls "test-results/style-snapshots/$label" | wc -l | tr -d ' ') journeys)"
