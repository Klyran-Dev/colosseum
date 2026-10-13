#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

for benchmark in \
    "$repo_root/scripts/benchmark_quote_service.sh" \
    "$repo_root/scripts/benchmark_rule_simulation.sh"; do
    grep -Fq 'export SEED_DIR="/seeds"' "$benchmark" || {
        echo "benchmark fixture contract failed: $benchmark does not enable synthetic seeds" >&2
        exit 1
    }
done

pool_drill="$repo_root/scripts/postgres_pool_drill.sh"
grep -Fq 'LOAD_CONCURRENCY=16' "$pool_drill" || {
    echo "benchmark contract failed: pool drill concurrency is not bounded" >&2
    exit 1
}
grep -Fq 'CONTENTION_REQUESTS=64' "$pool_drill" || {
    echo "benchmark contract failed: pool drill contention is not bounded" >&2
    exit 1
}

echo "benchmark fixture contract passed"
