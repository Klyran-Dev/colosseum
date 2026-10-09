#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

failures=0

require_file() {
  if [ ! -f "$1" ]; then
    echo "missing EVM adapter file: $1" >&2
    failures=$((failures + 1))
  fi
}

require_text() {
  file=$1
  text=$2
  if ! grep -Fq "$text" "$file"; then
    echo "missing EVM adapter contract text in $file: $text" >&2
    failures=$((failures + 1))
  fi
}

contract=evm-adapter/src/SolanaSettlementReceiptMirror.sol
dockerfile=ops/docker/Dockerfile.evm-adapter

for file in \
  evm-adapter/foundry.toml \
  "$contract" \
  evm-adapter/test/SolanaSettlementReceiptMirror.t.sol \
  "$dockerfile"
do
  require_file "$file"
done

require_text "$contract" 'pragma solidity 0.8.37;'
require_text "$contract" 'contract SolanaSettlementReceiptMirror'
require_text "$contract" 'function recordReceipt('
require_text "$contract" 'bytes32 public immutable sourceDomain'
require_text "$contract" 'error ConflictingReceipt()'
require_text "$contract" 'netAtomic != grossAtomic - taxAtomic'
require_text evm-adapter/foundry.toml 'solc_version = "0.8.37"'

if grep -Eq '\b(transferFrom|approve|safeTransferFrom)\s*\(' "$contract"; then
  echo "EVM adapter must not expose token movement interfaces" >&2
  failures=$((failures + 1))
fi
if grep -Eq 'function[[:space:]]+(receive|fallback)\b|function[[:space:]][^\n]*payable' "$contract"; then
  echo "EVM adapter must not receive or forward value" >&2
  failures=$((failures + 1))
fi
if grep -Eq '^[[:space:]]*import[[:space:]]' "$contract"; then
  echo "EVM adapter must have no external Solidity dependencies" >&2
  failures=$((failures + 1))
fi
if grep -Eq '\.(call|delegatecall|staticcall)\b|[[:space:]]new[[:space:]]+[A-Za-z_]' "$contract"; then
  echo "EVM adapter must not make external calls or instantiate contracts" >&2
  failures=$((failures + 1))
fi
if ! grep -Eq '^FROM --platform=linux/amd64 ghcr\.io/foundry-rs/foundry:v1\.8\.1@sha256:[0-9a-f]{64}$' "$dockerfile"; then
  echo "EVM adapter Docker image must be an immutable amd64 Foundry pin" >&2
  failures=$((failures + 1))
fi
if ! grep -Fq 'solc-linux-amd64-v0.8.37+commit.f401782d' "$dockerfile" ||
   ! grep -Fq 'sha256:5de843c2c93563cc66425c99a4fb13fdbf32b4c4ae07469480faaf126e14404a' "$dockerfile"; then
  echo "EVM adapter Docker build must pin the Solidity 0.8.37 compiler artifact" >&2
  failures=$((failures + 1))
fi

if [ "$failures" -ne 0 ]; then
  echo "EVM adapter contract failed with $failures issue(s)" >&2
  exit 1
fi

echo "EVM adapter contract passed"
