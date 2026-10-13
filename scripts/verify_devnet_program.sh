#!/bin/sh

set -eu

fail() {
  echo "Devnet program verification failed: $1" >&2
  exit 1
}

expected_program_id=4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi
program_id=${SOLANA_PROGRAM_ID:-$expected_program_id}
network=${SOLANA_NETWORK:-}
rpc_url=${SOLANA_RPC_URL:-}
artifact_path=${SOLANA_ARTIFACT_PATH:-target/deploy/settlement.so}

[ "$network" = devnet ] || fail "SOLANA_NETWORK must be exactly devnet"
[ "$program_id" = "$expected_program_id" ] || fail "SOLANA_PROGRAM_ID does not match the repository program ID"
[ -n "$rpc_url" ] || fail "SOLANA_RPC_URL is required"
case "$rpc_url" in
  https://*) ;;
  *) fail "SOLANA_RPC_URL must use HTTPS" ;;
esac
case "$rpc_url" in
  *localhost*|*127.0.0.1*|*validator:*|*auth-service*)
    fail "SOLANA_RPC_URL points to a local or internal endpoint"
    ;;
esac
[ -f "$artifact_path" ] || fail "SBF artifact does not exist: $artifact_path"

command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v jq >/dev/null 2>&1 || fail "jq is required"

request=$(printf '%s' '{"jsonrpc":"2.0","id":1,"method":"getAccountInfo","params":["'"$program_id"'",{"encoding":"base64","commitment":"finalized"}]}')
response=$(curl --fail --silent --show-error --max-time 15 --max-filesize 8388608 \
  --header 'content-type: application/json' \
  --data "$request" "$rpc_url") || fail "finalized RPC request failed"

rpc_error=$(printf '%s' "$response" | jq -r '.error.message // empty')
[ -z "$rpc_error" ] || fail "RPC returned an error"

value=$(printf '%s' "$response" | jq -c '.result.value // empty')
[ -n "$value" ] || fail "canonical program account does not exist"

executable=$(printf '%s' "$value" | jq -r '.executable // false')
owner=$(printf '%s' "$value" | jq -r '.owner // empty')
loader="BPFLoaderUpgradeab1e11111111111111111111111"
[ "$executable" = true ] || fail "canonical program account is not executable"
[ "$owner" = "$loader" ] || fail "canonical program account has an unexpected owner"

if command -v sha256sum >/dev/null 2>&1; then
  artifact_sha256=$(sha256sum "$artifact_path" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
  artifact_sha256=$(shasum -a 256 "$artifact_path" | awk '{print $1}')
else
  fail "sha256sum or shasum is required"
fi

finalized_slot=$(printf '%s' "$response" | jq -r '.result.context.slot // empty')
[ -n "$finalized_slot" ] || fail "finalized RPC response did not include a context slot"

printf '%s\n' \
  'schema_version=solana-tax-protocol/devnet-program-verification/v1' \
  "network=$network" \
  "program_id=$program_id" \
  "program_owner=$owner" \
  "executable=$executable" \
  "finalized_slot=$finalized_slot" \
  "artifact_path=$artifact_path" \
  "artifact_sha256=$artifact_sha256"
