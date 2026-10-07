#!/bin/sh
# Regression test for scripts/verify_non_custodial_wallet_contract.sh: it must fail
# closed. Builds a throwaway tree so the real repository is never modified.
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
work=$(mktemp -d)
trap 'chmod -R u+rwx "$work" 2>/dev/null || true; rm -rf "$work"' EXIT

mkdir -p "$work/scripts" "$work/apps/web/src" "$work/crates/api/src"
cp "$repo_root/scripts/verify_non_custodial_wallet_contract.sh" "$work/scripts/"
cat > "$work/apps/web/src/wallet.ts" <<'TS'
export const NON_CUSTODIAL_WALLET_PORT_V1 = true;
export function connectInjectedWallet() {}
export function ensureConnectedWallet() {}
export function signTransaction() {}
TS
echo 'pub const NON_CUSTODIAL_WALLET_API_BOUNDARY_V1: bool = true;' > "$work/crates/api/src/lib.rs"

run() { (cd "$work" && sh scripts/verify_non_custodial_wallet_contract.sh) >"$work/out" 2>&1; }
failures=0
expect_pass() { if run; then echo "ok   $1"; else echo "FAIL $1 (expected pass)"; cat "$work/out"; failures=$((failures + 1)); fi; }
expect_fail() { if run; then echo "FAIL $1 (expected failure)"; failures=$((failures + 1)); else echo "ok   $1"; fi; }

expect_pass "clean tree passes"

echo 'const key = new Keypair();' > "$work/apps/web/src/leak.ts"
expect_fail "client keypair construction in browser source fails"
rm "$work/apps/web/src/leak.ts"

echo 'const wallet = { mnemonic: "x" };' > "$work/apps/web/src/seed.ts"
expect_fail "mnemonic material in browser source fails"
rm "$work/apps/web/src/seed.ts"

echo 'const fixture = new Keypair();' > "$work/apps/web/src/wallet.test.ts"
expect_pass "test files may construct keypairs"
rm "$work/apps/web/src/wallet.test.ts"

echo 'pub struct Req { pub secret_key: String }' > "$work/crates/api/src/req.rs"
expect_fail "secret key field in API source fails"
rm "$work/crates/api/src/req.rs"

echo 'const ok = 1;' > "$work/apps/web/src/unreadable.ts"
chmod 000 "$work/apps/web/src/unreadable.ts"
if [ "$(id -u)" -ne 0 ]; then
  expect_fail "a scanner error fails closed instead of passing"
else
  echo "skip scanner-error case (running as root)"
fi
chmod 644 "$work/apps/web/src/unreadable.ts"

# The scan must not depend on ripgrep being installed.
if grep -q '\brg ' "$repo_root/scripts/verify_non_custodial_wallet_contract.sh"; then
  echo "FAIL contract still depends on ripgrep"; failures=$((failures + 1))
else
  echo "ok   contract does not depend on ripgrep"
fi

if [ "$failures" -ne 0 ]; then exit 1; fi
echo "non-custodial wallet contract regression test passed"
