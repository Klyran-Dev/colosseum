#!/bin/sh
# Capture the real DOM that unit tests render for every surface no journey
# reaches into test-results/style-fixtures (ADR-0429). Run before
# scripts/web_style_snapshot.sh; fixtures depend on markup, not on CSS.
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"
rm -rf test-results/style-fixtures
STYLE_FIXTURE_DIR="$repo_root/test-results/style-fixtures" \
  pnpm --dir apps/web exec vitest run --sequence.hooks=list
echo "fixtures: test-results/style-fixtures ($(ls test-results/style-fixtures | wc -l | tr -d ' ') states)"
