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

# Scan with POSIX grep so the contract cannot silently pass because a scanner is
# missing (an earlier version used ripgrep behind `|| true`, which turned "command
# not found" into "no findings"). grep exits 0 on a match and 1 on none; GNU grep
# exits 2 on an error but BSD grep may still exit 0 or 1 while reporting an error
# (for example an unreadable file) on stderr, so any exit above 1 or any stderr
# output fails the contract.
SCANNER_ERROR="scanner-error:"
scan() {
  directory=$1
  shift
  errors=$(mktemp)
  status=0
  output=$(grep -rEni "$@" "$directory" 2>"$errors") || status=$?
  if [ "$status" -gt 1 ] || [ -s "$errors" ]; then
    # scan runs in a command substitution (a subshell), so it cannot record the
    # failure itself; it reports through its output and the caller fails closed.
    cat "$errors" >&2
    printf '%s\n' "$SCANNER_ERROR (grep exit $status) while scanning $directory"
    rm -f "$errors"
    return 0
  fi
  rm -f "$errors"
  if [ "$status" -eq 0 ]; then printf '%s\n' "$output"; fi
  return 0
}

browser_forbidden=$(scan apps/web/src \
  --include='*.ts' --include='*.tsx' \
  --exclude='*.test.ts' --exclude='*.test.tsx' --exclude-dir=e2e \
  -e '(seed[ _-]?phrase|mnemonic|private[ _-]?key[[:space:]]*[:?]|secret[ _-]?key[[:space:]]*[:?]|secretKey[[:space:]]*[:?]|new[[:space:]]+Keypair|Keypair[.]from)')
if printf '%s' "$browser_forbidden" | grep -Fq "$SCANNER_ERROR"; then
  fail "scanner error while scanning apps/web/src: $browser_forbidden"
elif [ -n "$browser_forbidden" ]; then
  printf '%s\n' "$browser_forbidden" >&2
  fail "browser production source contains user-key material or client keypair construction"
fi

api_forbidden=$(scan crates/api/src \
  --include='*.rs' \
  -e '(seed[ _-]?phrase|mnemonic|private[ _-]?key[[:space:]]*[:]|secret[ _-]?key[[:space:]]*[:]|secret_key[[:space:]]*:)')
if printf '%s' "$api_forbidden" | grep -Fq "$SCANNER_ERROR"; then
  fail "scanner error while scanning crates/api/src: $api_forbidden"
elif [ -n "$api_forbidden" ]; then
  printf '%s\n' "$api_forbidden" >&2
  fail "API request source contains user-key material fields"
fi

if [ "$failures" -ne 0 ]; then
  exit 1
fi

echo "non-custodial wallet contract passed"
