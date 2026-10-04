#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
makefile="$repo_root/Makefile"

fail() {
    echo "Tier-1 local gate contract failed: $1" >&2
    exit 1
}

block=$(awk '
    $0 == "tier1-local-check:" { in_target = 1; next }
    in_target && $0 !~ /^[[:space:]]/ { exit }
    in_target { print }
' "$makefile")
[ -n "$block" ] || fail "missing tier1-local-check target"

for command in \
    '$(MAKE) check' \
    '$(MAKE) integration-check' \
    '$(MAKE) web-browser-check' \
    '$(MAKE) solana-conformance'; do
    printf '%s\n' "$block" | grep -Fq "$command" || fail "missing ordered command: $command"
done

previous_line=0
for command in \
    '$(MAKE) check' \
    '$(MAKE) integration-check' \
    '$(MAKE) web-browser-check' \
    '$(MAKE) solana-conformance'; do
    line=$(printf '%s\n' "$block" | grep -n -F -m1 "$command" | cut -d: -f1 || true)
    [ -n "$line" ] || fail "missing command line: $command"
    [ "$line" -gt "$previous_line" ] || fail "commands are not ordered: $command"
    previous_line=$line
done

echo "Tier-1 local gate contract passed"
