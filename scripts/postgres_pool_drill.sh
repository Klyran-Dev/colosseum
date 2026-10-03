#!/bin/sh

set -eu

# Bounded local capacity evidence. This is not a production SLO claim.
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
project_name="${COMPOSE_PROJECT_NAME:-solana-tax-protocol-pool-drill}"

output="$(
    COMPOSE_PROJECT_NAME="$project_name" \
    API_DATABASE_POOL_SIZE=1 \
    LOAD_REQUESTS=256 \
    LOAD_CONCURRENCY=64 \
    CONTENTION_REQUESTS=256 \
    POOL_HOLD_TABLE_LOCK_MILLISECONDS=50 \
    API_PORT=28082 \
    POSTGRES_PORT=25436 \
    NATS_PORT=25437 \
    "$repo_root/scripts/benchmark_quote_service.sh"
)"
printf '%s\n' "$output"

pool_max_size="$(printf '%s\n' "$output" | sed -n 's/^database_pool_max_size=//p')"
pool_peak_waiting="$(printf '%s\n' "$output" | sed -n 's/^database_pool_peak_waiting=//p')"
if [ "$pool_max_size" != "1" ]; then
    echo "pool drill did not run with the declared pool bound: $pool_max_size" >&2
    exit 1
fi
case "$pool_peak_waiting" in
    ''|*[!0-9]*) echo "pool drill did not produce a bounded waiter sample" >&2; exit 1 ;;
esac
if [ "$pool_peak_waiting" -lt 1 ]; then
    echo "pool drill did not observe connection-pool queueing" >&2
    exit 1
fi

echo "PostgreSQL pool saturation drill passed: max_size=$pool_max_size peak_waiting=$pool_peak_waiting"
