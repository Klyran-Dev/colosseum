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
docker compose -p "$project" -f compose.yaml -f compose.browser.yaml --profile browser run --build --rm \
  -e "STYLE_SNAPSHOT_DIR=/workspace/artifacts/style-snapshots/$label" \
  -e "STYLE_SNAPSHOT_SCREENSHOTS=${STYLE_SNAPSHOT_SCREENSHOTS:-}" web-e2e
echo "snapshots: test-results/style-snapshots/$label ($(ls "test-results/style-snapshots/$label" | wc -l | tr -d ' ') journeys)"
