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
  .gitignore \
  AGENTS.md \
  rust-toolchain.toml \
  Cargo.lock \
  README.md \
  CONTRIBUTING.md \
  SECURITY.md \
  CODE_OF_CONDUCT.md \
  CHANGELOG.md \
  docs/README.md \
  docs/api-contract.md \
  docs/architecture/alternative-analysis.md \
  docs/architecture/fx-evidence.md \
  docs/architecture/toolchain-compatibility.md \
  docs/plans/implementation-plan.md \
  docs/plans/production-readiness.md \
  docs/product/tax-coverage-matrix.md \
  docs/public-repository-release.md \
  db/README.md \
  docs/adr/README.md \
  docs/adr/0135-slo-error-budget-incident-and-dependency-controls.md \
  docs/adr/0138-versioned-database-migration-runner.md \
  docs/adr/0139-non-local-deployment-configuration-contract.md \
  docs/adr/0140-validator-backed-split-settlement-conformance.md \
  docs/adr/0141-validator-backed-policy-revocation-conformance.md \
  docs/adr/0214-token-2022-split-conformance.md \
  docs/adr/0215-bounded-load-resource-profile.md \
  docs/adr/0216-server-owned-mint-precision-at-the-route-boundary.md \
  docs/adr/0217-retained-docker-integration-evidence.md \
  docs/adr/0218-explicit-release-integrity-hashes.md \
  docs/adr/0219-jwks-json-preflight-parity.md \
  docs/adr/0220-jurisdiction-provider-json-preflight-parity.md \
  docs/adr/0221-structured-migration-connection-lifecycle.md \
  docs/adr/0222-fail-closed-shutdown-signal-handling.md \
  docs/adr/0223-observable-non-authoritative-metrics-failures.md \
  docs/adr/0224-cross-language-calculation-attestation-vector.md \
  docs/adr/0225-dedicated-warm-tax-engine-service.md \
  docs/adr/0226-tax-engine-deployment-preflight.md \
  docs/adr/0227-tax-engine-rustls-mtls-transport.md \
  docs/adr/0228-tax-engine-mtls-certificate-reload.md \
  docs/adr/0229-tax-engine-observability-identity.md \
  docs/adr/0230-tax-engine-runtime-contract.md \
  docs/adr/0231-tax-engine-request-rejection-semantics.md \
  docs/adr/0232-tax-engine-service-concurrency-evidence.md \
  docs/adr/0233-tax-engine-container-mtls-contract.md \
  docs/adr/0236-tax-engine-mtls-client-identity.md \
  docs/adr/0237-tax-engine-service-token-rotation.md \
  docs/adr/0238-governed-refund-claim-cancellation-workflow.md \
  docs/adr/0239-catalog-driven-uuidv7-invariant-closure.md \
  docs/adr/0240-refund-claim-cancellation-lease-trigger.md \
  docs/adr/0234-api-jwks-atomic-reload.md \
  docs/adr/0235-jurisdiction-provider-key-atomic-reload.md \
  ops/docker/Dockerfile.tax-engine \
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
  docs/adr/0192-real-browser-vault-reconciliation-evidence.md \
  docs/adr/0193-server-owned-reconciliation-freshness.md \
  docs/adr/0194-adjustment-wallet-finality-drill.md \
  docs/adr/0195-transaction-bound-refund-proof-drill.md \
  docs/adr/0196-client-quote-expiry-boundary.md \
  docs/adr/0197-strict-browser-tax-evidence-responses.md \
  docs/adr/0198-strict-vault-release-response-contract.md \
  test-vectors/settlement-adjustment-evidence-v1.json \
  test-vectors/settlement-refund-claim-v1.json \
  test-vectors/calculation-attestation-v1.json \
  docs/operations/slo-error-budget-policy.md \
  docs/operations/dependency-register.md \
  docs/runbooks/incident-severity-and-error-budget.md \
  docs/runbooks/tax-engine-mtls-rotation.md \
  docs/adr/0129-release-artifact-provenance-and-dependency-automation.md \
  .github/dependabot.yml \
  .github/workflows/release-candidate.yml \
  scripts/create_release_bundle.sh \
  ops/docker/Dockerfile.solana-conformance \
  scripts/generate_release_sbom.sh \
  scripts/release_bundle_integrity_contract.sh \
  scripts/vault_reconciliation_drill.sh \
  scripts/refund_proof_integration_drill.sh \
  scripts/benchmark_rule_simulation.sh \
  scripts/benchmark_tax_engine_service.sh \
  scripts/verify_tax_engine_mtls_runtime_contract.sh \
  scripts/verify_api_auth_jwks_reload_contract.sh \
  scripts/verify_api_jurisdiction_provider_reload_contract.sh \
  scripts/migration_integrity_drill.sh \
  db/verification/settlement_route_mint_precision.sql \
  scripts/verify_deployment_config.sh \
  scripts/verify_deployment_config_contract.sh \
  scripts/verify_service_lifecycle_contract.sh \
  scripts/verify_tax_engine_runtime_contract.sh \
  scripts/verify_rust_panic_safety_contract.sh \
  scripts/verify_container_pinning_contract.sh \
  scripts/verify_refund_claim_cancellation_reconciliation_contract.sh \
  scripts/refund_cancellation_reconciliation_drill.sh \
  apps/web/scripts/refund_cancellation_fixture.mjs \
  docs/plans/tier-1-product-standard.md \
  db/migrations/0058_catalog_driven_uuidv7_invariant_closure.sql \
  db/migrations/0059_refund_claim_cancellation_lease_trigger.sql \
  db/verification/uuidv7_contract.sql \
  apps/web/package.json \
  apps/web/pnpm-lock.yaml
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
require_text docs/runbooks/README.md 'tax-engine-mtls-rotation.md'
require_text docs/plans/production-readiness.md 'SLOs, error budgets, RTO/RPO, support ownership, and incident severity policy are published under ADR-0135'
require_text Makefile 'public-docs-check'
require_text Makefile 'rust-panic-safety-contract'
require_text Makefile 'container-pinning-contract'
require_text Makefile 'solana-conformance-workflow-contract'
require_text Makefile 'program-pause-coverage-contract'
require_text Makefile 'refund-cancellation-reconciliation-drill'
require_text README.md 'refund-cancellation-reconciliation-drill'
require_text docs/adr/README.md '0238-governed-refund-claim-cancellation-workflow.md'
require_text docs/adr/README.md '0241-linux-conformance-workflow-contract.md'
require_text docs/adr/README.md '0242-protocol-pause-vault-release-execution.md'
require_text docs/adr/README.md '0243-executable-program-pause-coverage.md'
require_text docs/adr/README.md '0244-exact-fx-evidence-boundary.md'
require_text docs/adr/README.md '0245-server-owned-signed-fx-evidence.md'
require_text docs/adr/README.md '0246-fx-provider-policy-and-issuance.md'
require_text docs/adr/README.md '0247-fx-provider-policy-snapshot-reload.md'
require_text CHANGELOG.md 'container and workflow pinning contract'
require_text CHANGELOG.md 'conformance workflow contract'
require_text CHANGELOG.md 'emergency pause cover governed vault'
require_text CHANGELOG.md 'program pause-coverage contract'
require_text CHANGELOG.md 'exact foreign-exchange evidence boundary'
require_text CHANGELOG.md 'server-owned signed FX evidence bounded context'
require_text CHANGELOG.md 'atomic digest-keyed FX provider policy snapshot'
require_text apps/web/package.json '"packageManager": "pnpm@12.3.4"'
require_text docs/architecture/toolchain-compatibility.md '| pnpm | 12.3.4 |'

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
