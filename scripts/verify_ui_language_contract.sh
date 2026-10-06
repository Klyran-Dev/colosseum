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
if ! grep -Fq 'Review and publish a rule' apps/web/src/GovernancePanel.tsx \
  && ! grep -Fq 'Review and publish a rule' apps/web/src/governanceMessages.ts; then
  fail 'missing approved copy in governance panel or its typed localization catalog: Review and publish a rule'
fi
if ! grep -Fq 'Run controlled simulation' apps/web/src/GovernancePanel.tsx \
  && ! grep -Fq 'Run controlled simulation' apps/web/src/governanceMessages.ts; then
  fail 'missing approved copy in governance panel or its typed localization catalog: Run controlled simulation'
fi
require_text apps/web/src/governanceMessages.ts 'Arbeitsbereich für Steuerregel-Governance'
require_text apps/web/src/governanceMessages.ts 'مساحة حوكمة القواعد الضريبية'
require_text apps/web/src/ruleWorkbenchMessages.ts 'So wird die Steuer berechnet'
require_text apps/web/src/ruleWorkbenchMessages.ts 'كيفية حساب الضريبة'
require_text apps/web/src/ruleWorkbenchMessages.ts 'Exclusive: "Steuer wird aufgeschlagen"'
if ! grep -Fq 'Partner payout plans' apps/web/src/RevenueStreamingPanel.tsx \
  && ! grep -Fq 'Partner payout plans' apps/web/src/revenueStreamingMessages.ts; then
  fail 'missing approved copy in revenue-streaming panel or its typed localization catalog: Partner payout plans'
fi
if ! grep -Fq 'خطط مدفوعات الشركاء' apps/web/src/revenueStreamingMessages.ts; then
  fail 'missing Arabic revenue-streaming catalog coverage'
fi
require_text apps/web/src/accessControlMessages.ts 'Wer darf was?'
require_text apps/web/src/accessControlMessages.ts 'من يستطيع تنفيذ ماذا؟'
require_text apps/web/src/accessControlMessages.ts 'View access rules'
require_text apps/web/src/shellOperationalMessages.ts 'Kontomenü öffnen'
require_text apps/web/src/shellOperationalMessages.ts 'فتح قائمة الحساب'
require_text apps/web/src/shellOperationalMessages.ts 'Dieser Arbeitsbereich konnte nicht geladen werden'
require_text apps/web/src/shellOperationalMessages.ts 'تعذر تحميل مساحة العمل هذه'
for approved_copy in \
  'Calculate tax for an invoice' \
  'Location verification' \
  'Provider-signed location record (advanced)' \
  'Retry protection selected automatically' \
  'calculated but not attested'
do
  if ! grep -Fq "$approved_copy" apps/web/src/DocumentQuotePanel.tsx \
    && ! grep -Fq "$approved_copy" apps/web/src/documentQuoteMessages.ts; then
    fail "missing approved copy in document quote panel or its typed localization catalog: $approved_copy"
  fi
done
if ! grep -Fq 'Payment ready for your approval' apps/web/src/DocumentCheckoutPanel.tsx \
  && ! grep -Fq 'Payment ready for your approval' apps/web/src/documentCheckoutMessages.ts; then
  fail 'missing approved copy in document checkout panel or its typed localization catalog: Payment ready for your approval'
fi
if ! grep -Fq 'Protection details' apps/web/src/ProductShell.tsx \
  && ! grep -Fq 'Protection details' apps/web/src/shellOperationalMessages.ts; then
  fail 'missing approved copy in the shell or its typed operational catalog: Protection details'
fi
if ! grep -Fq 'Automated withholding and evidence' apps/web/src/ProductShell.tsx \
  && ! grep -Fq 'Automated withholding and evidence' apps/web/src/shellMessages.ts; then
  fail 'missing approved copy in the shell or its typed localization catalog: Automated withholding and evidence'
fi
for approved_copy in \
  'not tax advice, a filing, or a compliance guarantee.' \
  'Calculated; verification still needed' \
  'Published rule and issuer attestation' \
  'Retry protection selected automatically' \
  'Location verification'
do
  if ! grep -Fq "$approved_copy" apps/web/src/App.tsx \
    && ! grep -Fq "$approved_copy" apps/web/src/quoteMessages.ts; then
    fail "missing approved copy in App or its typed quote localization catalog: $approved_copy"
  fi
done
if ! grep -Fq 'This is withholding authorization—not tax filing, reporting, legal compliance, or tax advice.' apps/web/src/DocumentCheckoutPanel.tsx \
  && ! grep -Fq 'This is withholding authorization—not tax filing, reporting, legal compliance, or tax advice.' apps/web/src/documentCheckoutMessages.ts; then
  fail 'missing approved copy in document checkout panel or its typed localization catalog: withholding language boundary'
fi
if ! grep -Fq 'does not transfer tokens, issue a refund, file taxes, or establish legal compliance.' apps/web/src/SettlementAdjustmentPanel.tsx \
  && ! grep -Fq 'does not transfer tokens, issue a refund, file taxes, or establish legal compliance.' apps/web/src/settlementAdjustmentMessages.ts; then
  fail 'missing settlement-adjustment non-custodial boundary copy in panel or its typed localization catalog'
fi
if ! grep -Fq 'cannot create a quote or authorize settlement' apps/web/src/TaxDocumentSimulationPanel.tsx \
  && ! grep -Fq 'cannot create a quote or authorize settlement' apps/web/src/taxDocumentSimulationMessages.ts; then
  fail 'missing document-simulation non-authority boundary copy in panel or its typed localization catalog'
fi
require_text apps/web/src/taxDocumentSimulationMessages.ts 'Steuersimulation für Zeilen und Rechnung'
require_text apps/web/src/taxDocumentSimulationMessages.ts 'محاكاة ضريبة السطور والفاتورة'
for forbidden in \
  'guaranteed compliance' \
  'tax compliance guaranteed' \
  'legally compliant' \
  'tax filing completed' \
  'taxes filed'
do
  if grep -Fqi "$forbidden" apps/web/src/*.tsx; then
    fail "user-facing source contains prohibited compliance claim: $forbidden"
  fi
done
if grep -Fq 'security-strip' apps/web/src/ProductShell.tsx apps/web/src/styles.css; then
  fail "retired repeated security strip returned to the shell"
fi
if ! grep -Fq 'Check tax balances' apps/web/src/VaultReconciliationPanel.tsx \
  && ! grep -Fq 'Check tax balances' apps/web/src/vaultReconciliationMessages.ts; then
  fail 'missing approved copy in vault-reconciliation panel or its typed localization catalog: Check tax balances'
fi
if ! grep -Fq 'Recovery queue' apps/web/src/ReplayOpsPanel.tsx \
  && ! grep -Fq 'Recovery queue' apps/web/src/replayOpsMessages.ts; then
  fail 'missing approved copy in replay-operations panel or its typed localization catalog: Recovery queue'
fi
if ! grep -Fq 'Create an adjustment request' apps/web/src/SettlementAdjustmentPanel.tsx \
  && ! grep -Fq 'Create an adjustment request' apps/web/src/settlementAdjustmentMessages.ts; then
  fail 'missing approved copy in settlement-adjustment panel or its typed localization catalog: Create an adjustment request'
fi
if ! grep -Fq 'Cancel a refund request' apps/web/src/RefundCancellationPanel.tsx \
  && ! grep -Fq 'Cancel a refund request' apps/web/src/refundCancellationMessages.ts; then
  fail 'missing approved copy in refund-cancellation panel or its typed localization catalog: Cancel a refund request'
fi
for approved_copy in \
  'Review payment before signing' \
  'Prepare payment for review' \
  'Technical payment details' \
  'Payment safeguards selected automatically'
do
  if ! grep -Fq "$approved_copy" apps/web/src/CheckoutPanel.tsx \
    && ! grep -Fq "$approved_copy" apps/web/src/checkoutMessages.ts; then
    fail "missing approved copy in aggregate checkout panel or its typed localization catalog: $approved_copy"
  fi
done
for approved_copy in \
  'Create a release request' \
  'Review or transition a claim' \
  'Two-person control' \
  'What this page can do:'
do
  if ! grep -Fq "$approved_copy" apps/web/src/VaultReleasePanel.tsx \
    && ! grep -Fq "$approved_copy" apps/web/src/vaultReleaseMessages.ts; then
    fail "missing approved copy in Treasury panel or its typed localization catalog: $approved_copy"
  fi
done
for approved_copy in \
  'Prepare a refund request' \
  'Wallet authorization' \
  'You keep custody' \
  'Finalized claim event evidence matched the durable artifact.' \
  'Reconcile finalized claim evidence'
do
  if ! grep -Fq "$approved_copy" apps/web/src/RefundClaimPanel.tsx \
    && ! grep -Fq "$approved_copy" apps/web/src/refundClaimMessages.ts; then
    fail "missing approved copy in refund-claim panel or its typed localization catalog: $approved_copy"
  fi
done
for approved_copy in \
  'Prepare refund release' \
  'Execution authorization' \
  'Proof required' \
  'Proof pending: provider acknowledgement is not a completed refund.' \
  'Finalized movement evidence is recorded.'
do
  if ! grep -Fq "$approved_copy" apps/web/src/RefundExecutePanel.tsx \
    && ! grep -Fq "$approved_copy" apps/web/src/refundExecuteMessages.ts; then
    fail "missing approved copy in refund-execution panel or its typed localization catalog: $approved_copy"
  fi
done
for approved_copy in \
  'Cancel a refund request' \
  'Governance wallet authorization' \
  'No money moved' \
  'A provider response or wallet signature is not proof of a completed cancellation.' \
  'Reconcile finalized cancellation evidence'
do
  if ! grep -Fq "$approved_copy" apps/web/src/RefundCancellationPanel.tsx \
    && ! grep -Fq "$approved_copy" apps/web/src/refundCancellationMessages.ts; then
    fail "missing approved copy in refund-cancellation panel or its typed localization catalog: $approved_copy"
  fi
done
for approved_copy in \
  'Create an adjustment request' \
  'Review an adjustment' \
  'No money moved' \
  'Record the approved correction' \
  'Finalized typed event evidence matched.'
do
  if ! grep -Fq "$approved_copy" apps/web/src/SettlementAdjustmentPanel.tsx \
    && ! grep -Fq "$approved_copy" apps/web/src/settlementAdjustmentMessages.ts; then
    fail "missing approved copy in settlement-adjustment panel or its typed localization catalog: $approved_copy"
  fi
done
for approved_copy in \
  'Review replay gaps' \
  'Recovery queue' \
  'Route-verification evidence' \
  'Recovery never confirms that a payment moved.' \
  'does not activate a route or confirm finalized settlement.'
do
  if ! grep -Fq "$approved_copy" apps/web/src/ReplayOpsPanel.tsx \
    && ! grep -Fq "$approved_copy" apps/web/src/replayOpsMessages.ts; then
    fail "missing approved copy in replay-operations panel or its typed localization catalog: $approved_copy"
  fi
done
for approved_copy in \
  'Check tax balances' \
  'Active tax accounts' \
  'This workspace cannot change balances, release funds, or clear an alert.' \
  'The latest finalized observation is stale and cannot prove the current vault balance.'
do
  if ! grep -Fq "$approved_copy" apps/web/src/VaultReconciliationPanel.tsx \
    && ! grep -Fq "$approved_copy" apps/web/src/vaultReconciliationMessages.ts; then
    fail "missing approved copy in vault-reconciliation panel or its typed localization catalog: $approved_copy"
  fi
done
if [ "$failures" -ne 0 ]; then
  echo "ui language contract failed with $failures issue(s)" >&2
  exit 1
fi

echo "ui language contract passed"
