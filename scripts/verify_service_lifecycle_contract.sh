#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

fail() {
  echo "service lifecycle contract failed: $1" >&2
  exit 1
}

for source in \
  crates/worker/src/main.rs \
  crates/indexer-service/src/main.rs \
  crates/route-verifier/src/main.rs \
  crates/tax-engine-service/src/main.rs
do
  [ -f "$source" ] || fail "missing service entrypoint: $source"
    if [ "$source" != "crates/tax-engine-service/src/main.rs" ]; then
      grep -Fq 'let mut shutdown = Box::pin(tokio::signal::ctrl_c());' "$source" \
        || fail "$source does not own the shutdown signal future"
      grep -Fq 'shutdown_result = &mut shutdown' "$source" \
        || fail "$source does not branch on shutdown-signal failure"
      grep -Fq 'shutdown_result?;' "$source" \
        || fail "$source does not propagate shutdown-signal failure"
    else
      grep -Fq 'let mut shutdown = Box::pin(tokio::signal::ctrl_c());' "$source" \
        || fail "$source does not own the shutdown signal future"
      grep -Fq 'shutdown_result = &mut shutdown' "$source" \
        || fail "$source does not branch on shutdown-signal failure"
      grep -Fq 'shutdown_result?;' "$source" \
        || fail "$source does not propagate shutdown-signal failure"
      grep -Fq 'handle.graceful_shutdown' "$source" \
        || fail "$source does not request bounded graceful shutdown"
      grep -Fq 'tax engine server ended before a shutdown signal was recorded' "$source" \
        || fail "$source does not fail closed on premature server termination"
    fi
  if grep -Fq 'tokio::signal::ctrl_c().await.ok()' "$source"; then
    fail "$source silently ignores shutdown-signal failure"
  fi
done

api_source=crates/api/src/main.rs
[ -f "$api_source" ] || fail "missing service entrypoint: $api_source"
grep -Fq 'let (shutdown_sender, shutdown_receiver) = mpsc::sync_channel(1);' "$api_source" \
  || fail "API does not retain a bounded shutdown-result channel"
grep -Fq '.with_graceful_shutdown(shutdown)' "$api_source" \
  || fail "API does not own its graceful-shutdown future"
grep -Fq 'api server ended before a shutdown signal result was recorded' "$api_source" \
  || fail "API does not fail closed on premature server termination"
if grep -Fq 'tracing::warn!(%error, "api shutdown signal listener failed")' "$api_source"; then
  fail "API still downgrades shutdown-signal failure to a warning"
fi

echo "service lifecycle contract passed"
