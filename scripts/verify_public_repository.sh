#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

failures=0

require_file() {
  if [ ! -f "$1" ]; then
    echo "missing required public file: $1" >&2
    failures=$((failures + 1))
  fi
}

require_text() {
  file=$1
  text=$2
  if ! grep -Fq "$text" "$file"; then
    echo "missing required text in $file: $text" >&2
    failures=$((failures + 1))
  fi
}

for file in \
  AGENTS.md \
  README.md \
  CONTRIBUTING.md \
  SECURITY.md \
  CODE_OF_CONDUCT.md \
  CHANGELOG.md \
  docs/README.md \
  docs/api-contract.md \
  docs/architecture/alternative-analysis.md \
  docs/plans/implementation-plan.md \
  docs/plans/production-readiness.md \
  docs/product/tax-coverage-matrix.md \
  docs/public-repository-release.md \
  docs/adr/README.md \
  docs/adr/0135-slo-error-budget-incident-and-dependency-controls.md \
  docs/adr/0138-versioned-database-migration-runner.md \
  docs/adr/0139-non-local-deployment-configuration-contract.md \
  docs/adr/0140-validator-backed-split-settlement-conformance.md \
  docs/adr/0141-validator-backed-policy-revocation-conformance.md \
  docs/adr/0142-spa-workspace-focus-management.md \
  docs/adr/0143-structured-tier-1-rule-workbench.md \
  docs/adr/0144-server-owned-persisted-rule-simulation.md \
  docs/adr/0145-reproducible-rule-simulation-latency-evidence.md \
  docs/adr/0146-centralized-locale-timezone-and-rtl-display-contract.md \
  docs/adr/0147-lazy-workspace-error-containment.md \
  docs/adr/0148-real-browser-ui-journey-contract.md \
  docs/adr/0149-secure-client-idempotency-identity.md \
  docs/adr/0150-immutable-settlement-adjustments.md \
  docs/adr/0151-durable-settlement-adjustment-workflow.md \
  docs/adr/0152-settlement-adjustment-operator-workspace.md \
  docs/adr/0153-on-chain-adjustment-evidence-boundary.md \
  docs/adr/0154-governed-adjustment-refund-movement.md \
  docs/adr/0155-finalized-refund-execution-evidence.md \
  docs/adr/0156-finalized-refund-balance-proof.md \
  docs/adr/0157-transaction-bound-refund-balance-proof.md \
  docs/adr/0158-server-owned-refund-proof-orchestration.md \
  docs/adr/0168-dockerized-solana-conformance-runner.md \
  docs/adr/0169-sbf-stack-frame-safety.md \
  docs/adr/0170-adjustment-submission-and-finalized-recording.md \
  docs/adr/0171-adjustment-wallet-workspace.md \
  docs/adr/0172-exact-chain-u64-wire-encoding.md \
  docs/adr/0173-refund-claim-wallet-orchestration.md \
  docs/adr/0174-finalized-refund-funding-topology-verification.md \
  docs/adr/0175-durable-refund-claim-execute-intents.md \
  test-vectors/settlement-adjustment-evidence-v1.json \
  test-vectors/settlement-refund-claim-v1.json \
  docs/operations/slo-error-budget-policy.md \
  docs/operations/dependency-register.md \
  docs/runbooks/incident-severity-and-error-budget.md \
  docs/adr/0129-release-artifact-provenance-and-dependency-automation.md \
  .github/dependabot.yml \
  .github/workflows/release-candidate.yml \
  scripts/create_release_bundle.sh \
  ops/docker/Dockerfile.solana-conformance \
  scripts/generate_release_sbom.sh \
  scripts/vault_reconciliation_drill.sh \
  scripts/benchmark_rule_simulation.sh \
  scripts/migration_integrity_drill.sh \
  scripts/verify_deployment_config.sh \
  scripts/verify_deployment_config_contract.sh
do
  require_file "$file"
done

for link in \
  'docs/README.md' \
  'docs/plans/tier-1-product-standard.md' \
  'docs/plans/implementation-plan.md' \
  'docs/plans/production-readiness.md' \
  'docs/product/tax-coverage-matrix.md' \
  'docs/architecture/alternative-analysis.md' \
  'docs/api-contract.md' \
  'docs/adr/README.md' \
  'docs/public-repository-release.md' \
  'SECURITY.md' \
  'CONTRIBUTING.md' \
  'docs/runbooks/README.md'
do
  if ! grep -Fq "($link" README.md; then
    echo "README.md does not link to $link" >&2
    failures=$((failures + 1))
  fi
done

for adr in docs/adr/[0-9][0-9][0-9][0-9]-*.md; do
  [ -f "$adr" ] || continue
  name=${adr#docs/adr/}
  if ! grep -Fq "($name)" docs/adr/README.md; then
    echo "ADR is not indexed: $name" >&2
    failures=$((failures + 1))
  fi
done

require_text README.md 'It does not claim production deployment'
require_text README.md 'Public repository status'
require_text docs/public-repository-release.md 'must not be represented as licensed'
require_text docs/plans/production-readiness.md 'License choice is explicit before public source release.'
require_text docs/public-repository-release.md 'ADR-0129'
require_text docs/README.md 'operations/slo-error-budget-policy.md'
require_text docs/README.md 'operations/dependency-register.md'
require_text docs/runbooks/README.md 'incident-severity-and-error-budget.md'
require_text docs/plans/production-readiness.md 'SLOs, error budgets, RTO/RPO, support ownership, and incident severity policy are published under ADR-0135'

tracked_secret_names=$(git ls-files | grep -E '(^|/)(\.env|\.env\.|id_rsa|id_ed25519|credentials\.json|service-account.*\.json|.*\.pem|.*\.key)$' || true)
if [ -n "$tracked_secret_names" ]; then
  echo "tracked filename resembles a secret-bearing file:" >&2
  echo "$tracked_secret_names" >&2
  failures=$((failures + 1))
fi

if [ "$failures" -ne 0 ]; then
  echo "public repository contract failed with $failures issue(s)" >&2
  exit 1
fi

echo "public repository contract passed"
