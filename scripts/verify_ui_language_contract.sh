#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

failures=0

fail() {
  echo "ui language contract: $*" >&2
  failures=$((failures + 1))
}

navigation=$(awk '
  /const NAVIGATION:/ { capture = 1 }
  /const GROUPS:/ { capture = 0 }
  capture { print }
' apps/web/src/ProductShell.tsx)

for forbidden in \
  'UUIDv7' \
  'PDA' \
  'atomic units' \
  'server-owned' \
  'control plane' \
  'policy decision point' \
  'RBAC' \
  'PBAC' \
  'ABAC'
do
  if printf '%s\n' "$navigation" | grep -Fqi "$forbidden"; then
    fail "navigation contains forbidden primary term: $forbidden"
  fi
done

require_text() {
  file=$1
  text=$2
  if ! grep -Fq "$text" "$file"; then
    fail "missing approved copy in $file: $text"
  fi
}

require_text apps/web/src/ProductShell.tsx 'Daily work'
require_text apps/web/src/ProductShell.tsx 'Rules & access'
require_text apps/web/src/ProductShell.tsx 'Money movement'
require_text apps/web/src/ProductShell.tsx 'Records & issues'
require_text apps/web/src/GovernancePanel.tsx 'Review and publish a rule'
require_text apps/web/src/GovernancePanel.tsx 'Run controlled simulation'
require_text apps/web/src/DocumentQuotePanel.tsx 'Calculate tax for an invoice'
require_text apps/web/src/DocumentQuotePanel.tsx 'Location verification'
require_text apps/web/src/DocumentQuotePanel.tsx 'Provider-signed location record (advanced)'
require_text apps/web/src/DocumentQuotePanel.tsx 'Retry protection selected automatically'
require_text apps/web/src/DocumentCheckoutPanel.tsx 'Payment ready for review'
require_text apps/web/src/VaultReconciliationPanel.tsx 'Check tax balances'
require_text apps/web/src/ReplayOpsPanel.tsx 'Recovery queue'
require_text apps/web/src/SettlementAdjustmentPanel.tsx 'Create an adjustment request'
require_text apps/web/src/RefundCancellationPanel.tsx 'Cancel a refund request'
require_text apps/web/src/CheckoutPanel.tsx 'Review payment before signing'
require_text apps/web/src/CheckoutPanel.tsx 'Prepare payment for review'
require_text apps/web/src/CheckoutPanel.tsx 'Technical payment details'
require_text apps/web/src/CheckoutPanel.tsx 'Payment safeguards selected automatically'
require_text apps/web/src/App.tsx 'Retry protection selected automatically'
require_text apps/web/src/App.tsx 'Location verification'

if [ "$failures" -ne 0 ]; then
  echo "ui language contract failed with $failures issue(s)" >&2
  exit 1
fi

echo "ui language contract passed"
