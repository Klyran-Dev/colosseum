#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
program="$repo_root/programs/settlement/src/lib.rs"

fail() {
    echo "program pause-coverage contract failed: $1" >&2
    exit 1
}

require_text() {
    text=$1
    grep -Fq "$text" "$program" || fail "program is missing: $text"
}

function_body() {
    function_name=$1
    awk -v needle="    pub fn $function_name" '
        index($0, needle) == 1 { in_function = 1; start = NR }
        in_function && NR > start && $0 ~ /^    pub fn / { exit }
        in_function { print }
    ' "$program"
}

require_guard() {
    function_name=$1
    body=$(function_body "$function_name")
    [ -n "$body" ] || fail "missing allowlisted instruction: $function_name"
    printf '%s\n' "$body" | grep -Fq 'validate_protocol_not_paused' ||
        fail "$function_name does not call validate_protocol_not_paused"
}

require_text 'fn validate_protocol_not_paused'
require_guard settle
require_guard settle_document
require_guard settle_split
require_guard execute_adjustment_refund

release_body=$(function_body execute_vault_release)
[ -n "$release_body" ] || fail "missing allowlisted instruction: execute_vault_release"
printf '%s\n' "$release_body" | grep -Fq 'protocol_paused: ctx.accounts.protocol_config.paused' ||
    fail "execute_vault_release does not bind protocol pause state"
require_text 'pub struct ExecuteVaultRelease'
require_text "pub protocol_config: Account<'info, ProtocolConfig>"
require_text 'seeds = [b"protocol-config"]'

echo "program pause-coverage contract passed"
