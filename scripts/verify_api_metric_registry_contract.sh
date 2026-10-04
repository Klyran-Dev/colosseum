#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
api_source="$repo_root/crates/api/src/lib.rs"
tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/stp-api-metrics.XXXXXX")
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

registry_file="$tmp_dir/registry"
observed_file="$tmp_dir/observed"

awk '
    /pub const API_METRIC_OPERATIONS:/ { capture = 1; next }
    capture && /^\];/ { exit }
    capture {
        line = $0
        while (match(line, /"[a-z0-9_]+"/)) {
            print substr(line, RSTART + 1, RLENGTH - 2)
            line = substr(line, RSTART + RLENGTH)
        }
    }
' "$api_source" | LC_ALL=C sort -u > "$registry_file"

awk '
    /metrics\.observe_or_log\(/ { capture = 1 }
    capture {
        if (match($0, /"[a-z0-9_]+"/)) {
            print substr($0, RSTART + 1, RLENGTH - 2)
            capture = 0
        }
    }
' "$api_source" | LC_ALL=C sort -u > "$observed_file"

if ! diff -u "$registry_file" "$observed_file"; then
    echo "API metric registry contract failed: every observe_or_log operation must be registered exactly once" >&2
    exit 1
fi

echo "API metric registry contract passed"
