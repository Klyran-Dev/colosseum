#!/bin/sh
# Run the cross-engine smoke suite (Firefox and WebKit) in the isolated Playwright
# Compose project (ADR-0439). Fails if any engine fails.
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"
mkdir -p test-results
project="solana-tax-protocol-web-xbrowser"
cleanup() { docker compose -p "$project" -f compose.yaml -f compose.browser.yaml down -v --remove-orphans >/dev/null 2>&1 || true; }
trap cleanup EXIT INT TERM
docker compose -p "$project" -f compose.yaml -f compose.browser.yaml --profile browser run --build --rm web-e2e \
  /bin/sh -c "/workspace/apps/web/node_modules/.bin/tsc --noEmit -p /workspace/apps/web/tsconfig.e2e.json && node /workspace/apps/web/node_modules/@playwright/test/cli.js test --config /workspace/apps/web/playwright.cross-browser.config.ts"
