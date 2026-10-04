#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

failures=0

fail() {
  echo "non-custodial wallet contract failed: $*" >&2
  failures=$((failures + 1))
}

require_text() {
  file=$1
  text=$2
  if ! grep -Fq "$text" "$file"; then
    fail "missing boundary marker in $file: $text"
  fi
}

require_text apps/web/src/wallet.ts 'NON_CUSTODIAL_WALLET_PORT_V1'
require_text crates/api/src/lib.rs 'NON_CUSTODIAL_WALLET_API_BOUNDARY_V1'
require_text apps/web/src/wallet.ts 'connectInjectedWallet'
require_text apps/web/src/wallet.ts 'ensureConnectedWallet'
require_text apps/web/src/wallet.ts 'signTransaction'

browser_forbidden=$(rg -n -i \
  --glob '*.ts' --glob '*.tsx' \
  --glob '!**/*.test.ts' --glob '!**/*.test.tsx' \
  --glob '!**/e2e/**' \
  '(seed[ _-]?phrase|mnemonic|private[ _-]?key[[:space:]]*[:?]|secret[ _-]?key[[:space:]]*[:?]|secretKey[[:space:]]*[:?]|new[[:space:]]+Keypair|Keypair[.]from)' \
  apps/web/src || true)
if [ -n "$browser_forbidden" ]; then
  printf '%s\n' "$browser_forbidden" >&2
  fail "browser production source contains user-key material or client keypair construction"
fi

api_forbidden=$(rg -n -i \
  --glob '*.rs' \
  '(seed[ _-]?phrase|mnemonic|private[ _-]?key[[:space:]]*[:]|secret[ _-]?key[[:space:]]*[:]|secret_key[[:space:]]*:)' \
  crates/api/src || true)
if [ -n "$api_forbidden" ]; then
  printf '%s\n' "$api_forbidden" >&2
  fail "API request source contains user-key material fields"
fi

if [ "$failures" -ne 0 ]; then
  exit 1
fi

echo "non-custodial wallet contract passed"
