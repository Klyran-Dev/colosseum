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
  docs/operations/requirements-traceability.md \
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
  docs/adr/0249-fx-provider-identity-allowlist.md \
  docs/adr/0250-deterministic-fx-provider-agreement.md \
  docs/adr/0255-bounded-http-fx-provider-adapter.md \
  docs/adr/0256-bounded-http-fx-signer-adapter.md \
  docs/adr/0257-server-owned-fx-issuance-coordinator.md \
  docs/adr/0261-fx-issuer-process-bootstrap.md \
  docs/adr/0262-production-saas-authorization-aware-ui-shell.md \
  docs/adr/0263-task-oriented-product-language-and-navigation.md \
  docs/adr/0264-server-owned-capability-snapshot-contract.md \
  docs/adr/0265-responsive-labeled-navigation-disclosure.md \
  docs/adr/0266-quote-to-payment-workspace-handoff.md \
  docs/adr/0267-honest-runtime-and-account-context.md \
  docs/adr/0268-browser-oidc-pkce-session-boundary.md \
  docs/adr/0269-oidc-origin-csp-allowlist.md \
  docs/adr/0271-executable-ui-language-contract.md \
  docs/adr/0272-payment-review-progressive-disclosure.md \
  docs/adr/0273-explicit-financial-inputs.md \
  docs/adr/0274-explicit-currency-input.md \
  docs/adr/0275-capability-snapshot-latency-evidence.md \
  docs/adr/0401-complete-authorization-benchmark-catalog.md \
  docs/adr/0276-multi-tenant-capability-session-reset.md \
  docs/adr/0277-complete-workspace-capability-catalog.md \
  docs/adr/0278-addressable-workspace-navigation.md \
  docs/adr/0298-material-3-expressive-visual-system.md \
  docs/adr/0299-release-evidence-source-revision-binding.md \
  docs/adr/0300-semantic-color-token-migration.md \
  docs/adr/0301-validator-backed-revenue-stream-conformance.md \
  docs/adr/0302-versioned-local-display-preferences.md \
  docs/adr/0303-release-candidate-browser-evidence.md \
  docs/adr/0304-material-role-contrast-contract.md \
  docs/adr/0305-tax-engine-local-gate-orchestration.md \
  docs/adr/0306-collapsed-advanced-evidence-disclosures.md \
  docs/adr/0307-project-scoped-local-reset.md \
  docs/adr/0308-release-candidate-authenticated-integration-evidence.md \
  docs/adr/0309-contextual-status-disclosure.md \
  docs/adr/0314-non-custodial-wallet-source-contract.md \
  docs/adr/0315-executable-tax-responsibility-language.md \
  docs/adr/0316-versioned-rule-release-provenance.md \
  docs/adr/0353-material-3-expressive-aurora-v2-shell.md \
  docs/adr/0354-versioned-rule-governance-authoring-localization.md \
  docs/adr/0355-versioned-revenue-streaming-localization.md \
  docs/adr/0359-explicit-fee-payer-binding.md \
  docs/adr/0360-versioned-document-simulation-localization.md \
  docs/adr/0373-tenant-scoped-evidence-access-explorer.md \
  docs/adr/0374-collapsible-material-navigation-rail.md \
  docs/adr/0375-bounded-linear-tax-base-operations.md \
  docs/adr/0378-non-custodial-solidity-receipt-mirror.md \
  docs/adr/0379-bounded-tax-base-clamps.md \
  docs/adr/0380-policy-issuer-and-asset-dependency-evidence.md \
  docs/architecture/evm-adapter.md \
  evm-adapter/foundry.toml \
  evm-adapter/src/SolanaSettlementReceiptMirror.sol \
  evm-adapter/test/SolanaSettlementReceiptMirror.t.sol \
  ops/docker/Dockerfile.evm-adapter \
  scripts/verify_evm_adapter_contract.sh \
  test-vectors/tax-engine-v2.json \
  docs/adr/0323-material-expressive-operational-workspace-composition.md \
  docs/adr/0248-registry-bound-fx-issuance.md \
  docs/adr/0234-api-jwks-atomic-reload.md \
  docs/adr/0235-jurisdiction-provider-key-atomic-reload.md \
  ops/docker/Dockerfile.tax-engine \
  docs/adr/0142-spa-workspace-focus-management.md \
  docs/adr/0143-structured-tax-engine-rule-workbench.md \
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
  docs/runbooks/settlement-route-verification.md \
  docs/adr/0329-approved-route-verification-backfill-mode.md \
  docs/adr/0330-server-owned-document-evidence-status.md \
  docs/adr/0331-canonical-authorization-policy-catalog.md \
  contracts/authorization-capabilities-v1.json \
  docs/adr/0129-release-artifact-provenance-and-dependency-automation.md \
  .github/dependabot.yml \
  .github/workflows/release-candidate.yml \
  .github/workflows/solana-conformance.yml \
  scripts/create_release_bundle.sh \
  ops/docker/Dockerfile.solana-conformance \
  scripts/generate_release_sbom.sh \
  scripts/release_bundle_integrity_contract.sh \
  scripts/vault_reconciliation_drill.sh \
  scripts/refund_proof_integration_drill.sh \
  scripts/benchmark_rule_simulation.sh \
  scripts/benchmark_authorization_capabilities.sh \
  scripts/verify_authorization_benchmark_contract.sh \
  scripts/benchmark_tax_engine_service.sh \
  scripts/verify_tax_engine_mtls_runtime_contract.sh \
  scripts/verify_api_auth_jwks_reload_contract.sh \
  scripts/verify_api_jurisdiction_provider_reload_contract.sh \
  scripts/migration_integrity_drill.sh \
  db/verification/settlement_route_mint_precision.sql \
  db/verification/settlement_route_asset_projection.sql \
  db/verification/settlement_route_verification_runs.sql \
  db/verification/settlement_route_verification_approval.sql \
  scripts/verify_route_verifier_backfill_contract.sh \
  scripts/verify_deployment_config.sh \
  scripts/verify_deployment_config_contract.sh \
  scripts/verify_service_lifecycle_contract.sh \
  scripts/verify_tax_engine_runtime_contract.sh \
  scripts/verify_api_metric_registry_contract.sh \
  scripts/verify_rust_panic_safety_contract.sh \
  scripts/verify_container_pinning_contract.sh \
  scripts/verify_solana_conformance_host.sh \
  scripts/verify_refund_claim_cancellation_reconciliation_contract.sh \
  scripts/refund_cancellation_reconciliation_drill.sh \
  apps/web/scripts/refund_cancellation_fixture.mjs \
  docs/plans/tax-engine-product-standard.md \
  db/migrations/0058_catalog_driven_uuidv7_invariant_closure.sql \
  db/migrations/0059_refund_claim_cancellation_lease_trigger.sql \
  db/migrations/0065_rule_release_provenance_readiness.sql \
  db/verification/uuidv7_contract.sql \
  apps/web/package.json \
  apps/web/pnpm-lock.yaml \
  scripts/render_web_nginx_config.mjs \
  scripts/verify_web_csp_contract.sh \
  scripts/verify_ui_language_contract.sh \
  scripts/verify_non_custodial_wallet_contract.sh \
  scripts/verify_rule_release_provenance_contract.sh \
  scripts/verify_release_claim_contract.sh \
  scripts/release_claim_contract_test.sh
do
  require_file "$file"
done

for link in \
  'docs/README.md' \
  'docs/plans/tax-engine-product-standard.md' \
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
require_text docs/runbooks/README.md 'settlement-route-verification.md'
require_text docs/plans/production-readiness.md 'SLOs, error budgets, RTO/RPO, support ownership, and incident severity policy are published under ADR-0135'
require_text Makefile 'public-docs-check'
require_text Makefile 'release-claim-contract'
require_text Makefile 'rust-panic-safety-contract'
require_text Makefile 'container-pinning-contract'
require_text Makefile 'api-metric-registry-contract'
require_text Makefile 'solana-conformance-workflow-contract'
require_text scripts/verify_solana_conformance_workflow_contract.sh 'verify_solana_conformance_host.sh'
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
require_text docs/adr/README.md '0248-registry-bound-fx-issuance.md'
require_text docs/adr/README.md '0249-fx-provider-identity-allowlist.md'
require_text docs/adr/README.md '0250-deterministic-fx-provider-agreement.md'
require_text docs/adr/README.md '0261-fx-issuer-process-bootstrap.md'
require_text docs/adr/README.md '0262-production-saas-authorization-aware-ui-shell.md'
require_text docs/adr/README.md '0263-task-oriented-product-language-and-navigation.md'
require_text docs/adr/README.md '0265-responsive-labeled-navigation-disclosure.md'
require_text docs/adr/README.md '0266-quote-to-payment-workspace-handoff.md'
require_text docs/adr/README.md '0267-honest-runtime-and-account-context.md'
require_text docs/adr/README.md '0268-browser-oidc-pkce-session-boundary.md'
require_text docs/adr/README.md '0269-oidc-origin-csp-allowlist.md'
require_text docs/adr/README.md '0271-executable-ui-language-contract.md'
require_text Makefile 'ui-language-contract'
require_text Makefile 'non-custodial-wallet-contract'
require_text docs/adr/0314-non-custodial-wallet-source-contract.md 'source contract'
require_text docs/adr/0314-non-custodial-wallet-source-contract.md 'private-key material'
require_text docs/adr/0315-executable-tax-responsibility-language.md 'language contract'
require_text docs/adr/0315-executable-tax-responsibility-language.md 'compliance guarantee'
require_text docs/adr/README.md '0316-versioned-rule-release-provenance.md'
require_text docs/adr/0316-versioned-rule-release-provenance.md 'tax-rule-provenance/v1'
require_text docs/adr/README.md '0323-material-expressive-operational-workspace-composition.md'
require_text docs/adr/0323-material-expressive-operational-workspace-composition.md 'Native grouped disclosure'
require_text docs/adr/0323-material-expressive-operational-workspace-composition.md 'Solana Aurora'
require_text docs/adr/README.md '0317-material-expressive-surface-hierarchy-and-navigation-density.md'
require_text docs/adr/0317-material-expressive-surface-hierarchy-and-navigation-density.md 'surface-container-lowest'
require_text docs/adr/README.md '0318-workflow-handoff-url-integrity.md'
require_text docs/adr/0318-workflow-handoff-url-integrity.md 'history.pushState'
require_text docs/adr/0318-workflow-handoff-url-integrity.md 'wallet material, tenant identifiers'
require_text docs/adr/README.md '0319-bounded-browser-error-disclosure.md'
require_text docs/adr/0319-bounded-browser-error-disclosure.md '512 characters'
require_text docs/adr/0319-bounded-browser-error-disclosure.md 'strict UUIDv7 support-reference check'
require_text docs/adr/README.md '0329-approved-route-verification-backfill-mode.md'
require_text docs/adr/0329-approved-route-verification-backfill-mode.md 'ROUTE_VERIFIER_MODE=backfill'
require_text docs/adr/README.md '0320-on-chain-stablecoin-asset-admission.md'
require_text docs/adr/0320-on-chain-stablecoin-asset-admission.md 'AssetConfig'
require_text docs/adr/0320-on-chain-stablecoin-asset-admission.md 'No v1 token movement succeeds without an enabled stablecoin'
require_text docs/adr/0320-on-chain-stablecoin-asset-admission.md 'Initial implementation slice'
require_text apps/web/src/solana.localnet.test.ts 'initializeAssetConfigInstruction'
require_text apps/web/src/solana.localnet.test.ts 'setAssetEnabledInstruction'
require_text apps/web/src/solana.localnet.test.ts 'TransferFeeConfig'
require_text scripts/verify_local_stack.sh 'cross-tenant aggregate settlement compilation'
require_text scripts/verify_local_stack.sh 'server-owned document attestation status lookup'
require_text scripts/verify_solana_conformance_evidence.sh 'rejects disabled or substituted asset admission'
require_text docs/adr/README.md '0321-finalized-asset-config-route-evidence.md'
require_text docs/adr/0321-finalized-asset-config-route-evidence.md 'five accounts'
require_text docs/adr/0321-finalized-asset-config-route-evidence.md 'v2'
require_text docs/adr/README.md '0322-route-evidence-version-migration.md'
require_text docs/adr/0322-route-evidence-version-migration.md 'suspended'
require_text docs/plans/implementation-plan.md 'thirteenth aggregate/document'
require_text docs/plans/implementation-plan.md 'ADR-0321'
require_text docs/plans/implementation-plan.md 'ADR-0322'
require_text docs/plans/implementation-plan.md 'ADR-0329'
require_text docs/adr/README.md '0330-server-owned-document-evidence-status.md'
require_text docs/adr/0330-server-owned-document-evidence-status.md 'route-bound status read'
require_text docs/api-contract.md '/v1/tax/document-quotes/{calculation_id}/attestation/status'
require_text docs/plans/implementation-plan.md 'ADR-0330'
require_text docs/plans/implementation-plan.md 'ADR-0331'
require_text docs/adr/README.md '0331-canonical-authorization-policy-catalog.md'
require_text docs/adr/0331-canonical-authorization-policy-catalog.md 'authorization-capabilities-v1.json'
require_text docs/api-contract.md 'contracts/authorization-capabilities-v1.json'
require_text ops/docker/Dockerfile.api 'COPY contracts ./contracts'
require_text ops/docker/Dockerfile.web 'COPY contracts ./contracts'
require_text ops/docker/Dockerfile.web-e2e 'COPY --chown=pwuser:pwuser contracts ./contracts'
require_text docs/operations/requirements-traceability.md 'On-chain/client/browser boundary and typed finalized asset-account proof are verified'
require_text Makefile 'rule-release-provenance-contract'
require_text scripts/verify_rule_release_provenance_contract.sh 'rule release provenance contract passed'
require_text Makefile 'web-color-contract'
require_text Makefile 'material-palette-contract'
require_text Makefile 'tax-engine-local-check-contract'
require_text Makefile 'evm-adapter-check'
require_text scripts/verify_evm_adapter_contract.sh 'EVM adapter contract passed'
require_text ops/docker/Dockerfile.evm-adapter 'ghcr.io/foundry-rs/foundry:v1.8.1@sha256:'
require_text docs/adr/README.md '0378-non-custodial-solidity-receipt-mirror.md'
require_text docs/adr/README.md '0379-bounded-tax-base-clamps.md'
require_text docs/adr/README.md '0380-policy-issuer-and-asset-dependency-evidence.md'
require_text docs/adr/0379-bounded-tax-base-clamps.md 'bounded optional clamps'
require_text docs/adr/0380-policy-issuer-and-asset-dependency-evidence.md 'typed authorities'
require_text docs/adr/0380-policy-issuer-and-asset-dependency-evidence.md 'unknown'
require_text docs/adr/0380-policy-issuer-and-asset-dependency-evidence.md 'redemption, legal, or tax-compliance approval'
require_text docs/product/tax-coverage-matrix.md 'Bounded taxable bases'
require_text test-vectors/tax-engine-v2.json 'capped-adjusted-base'
require_text apps/web/src/ruleWorkbench.ts 'baseFloorAtomic'
require_text apps/web/src/RuleWorkbenchPanel.tsx 'baseMinimum'
require_text docs/architecture/evm-adapter.md 'non-custodial projection'
require_file scripts/verify_web_color_contract.sh
require_text docs/adr/0271-executable-ui-language-contract.md 'progressive disclosure'
require_text docs/adr/0272-payment-review-progressive-disclosure.md 'Technical payment details'
require_text docs/adr/0273-explicit-financial-inputs.md 'Start financial fields empty'
require_text docs/adr/0274-explicit-currency-input.md 'required `Currency` field'
require_text docs/adr/0275-capability-snapshot-latency-evidence.md 'two API replicas'
require_text docs/adr/0401-complete-authorization-benchmark-catalog.md 'nineteen-capability catalog'
require_text docs/adr/0401-complete-authorization-benchmark-catalog.md 'authorization-benchmark-contract'
require_text Makefile 'authorization-benchmark-contract'
require_text docs/adr/0276-multi-tenant-capability-session-reset.md 'tenant A'
require_text docs/adr/0277-complete-workspace-capability-catalog.md 'refund.payment.release'
require_text docs/adr/0278-addressable-workspace-navigation.md 'history.pushState'
require_text docs/adr/README.md '0298-material-3-expressive-visual-system.md'
require_text docs/adr/0298-material-3-expressive-visual-system.md 'Material 3 Expressive'
require_text docs/adr/0299-release-evidence-source-revision-binding.md 'source revision'
require_text docs/adr/0299-release-evidence-source-revision-binding.md 'mismatch fails'
require_text docs/adr/README.md '0300-semantic-color-token-migration.md'
require_text docs/adr/0300-semantic-color-token-migration.md 'complete apps/web/src/styles.css'
require_text docs/adr/0300-semantic-color-token-migration.md 'raw hex, rgb, or rgba literals'
require_text docs/adr/README.md '0301-validator-backed-revenue-stream-conformance.md'
require_text docs/adr/0301-validator-backed-revenue-stream-conformance.md 'revenue-stream single-use claim'
require_text docs/adr/README.md '0302-versioned-local-display-preferences.md'
require_text docs/adr/0302-versioned-local-display-preferences.md 'presentation-only'
require_text docs/adr/0302-versioned-local-display-preferences.md 'credentials'
require_text docs/adr/README.md '0303-release-candidate-browser-evidence.md'
require_text docs/adr/0303-release-candidate-browser-evidence.md 'source-revision.txt'
require_text docs/adr/README.md '0304-material-role-contrast-contract.md'
require_text docs/adr/0304-material-role-contrast-contract.md '4.5:1'
require_text docs/adr/README.md '0305-tax-engine-local-gate-orchestration.md'
require_text docs/adr/README.md '0306-collapsed-advanced-evidence-disclosures.md'
require_text docs/adr/0306-collapsed-advanced-evidence-disclosures.md 'collapsed by default'
require_text docs/adr/README.md '0307-project-scoped-local-reset.md'
require_text docs/adr/0307-project-scoped-local-reset.md 'down --volumes --remove-orphans'
require_text docs/adr/README.md '0308-release-candidate-authenticated-integration-evidence.md'
require_text docs/adr/0308-release-candidate-authenticated-integration-evidence.md 'make integration-check'
require_text docs/adr/0308-release-candidate-authenticated-integration-evidence.md 'source-revision.txt'
require_text docs/adr/README.md '0309-contextual-status-disclosure.md'
require_text docs/adr/0309-contextual-status-disclosure.md 'System status'
require_text docs/adr/0309-contextual-status-disclosure.md 'Sign-in unavailable'
require_text Makefile 'tax-engine-local-check'
require_file scripts/verify_tax-engine_local_check_contract.sh
require_text docs/plans/implementation-plan.md 'ADR-0298'
require_text docs/plans/implementation-plan.md 'ADR-0309'
require_text docs/plans/implementation-plan.md 'ADR-0317'
require_text docs/plans/implementation-plan.md 'ADR-0318'
require_text docs/plans/implementation-plan.md 'ADR-0319'
require_text docs/plans/implementation-plan.md 'ADR-0320'
require_text docs/plans/implementation-plan.md 'ADR-0338'
require_text docs/operations/requirements-traceability.md 'make forced-colors-contract'
require_text docs/adr/README.md '0338-forced-colors-accessibility-layer.md'
require_text docs/adr/0338-forced-colors-accessibility-layer.md 'forced-color-adjust: auto'
require_text docs/adr/README.md '0339-polite-workspace-navigation-announcement.md'
require_text docs/plans/implementation-plan.md 'ADR-0339'
require_text docs/adr/0339-polite-workspace-navigation-announcement.md 'role="status"'
require_text docs/adr/README.md '0340-versioned-shell-navigation-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0340'
require_text docs/adr/0340-versioned-shell-navigation-localization.md 'typed, immutable, versioned shell message catalog'
require_text CHANGELOG.md 'typed, bounded shell localization catalog'
require_text docs/adr/README.md '0341-versioned-shell-chrome-and-session-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0341'
require_text docs/adr/0341-versioned-shell-chrome-and-session-localization.md 'typed, immutable, versioned catalog'
require_text CHANGELOG.md 'shell chrome and session localization'
require_text docs/adr/README.md '0342-versioned-quote-workspace-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0342'
require_text docs/adr/0342-versioned-quote-workspace-localization.md 'typed, immutable, versioned quote workspace catalog'
require_text CHANGELOG.md 'typed quote-workspace catalog'
require_text docs/adr/README.md '0343-versioned-document-quote-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0343'
require_text docs/adr/0343-versioned-document-quote-localization.md 'typed, immutable, versioned document-quote catalog'
require_text docs/adr/README.md '0344-versioned-document-checkout-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0344'
require_text docs/adr/0344-versioned-document-checkout-localization.md 'typed, immutable, versioned document-checkout catalog'
require_text docs/adr/README.md '0345-versioned-checkout-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0345'
require_text docs/adr/0345-versioned-checkout-localization.md 'typed immutable catalog'
require_text docs/adr/README.md '0346-versioned-vault-release-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0346'
require_text docs/adr/0346-versioned-vault-release-localization.md 'typed, immutable in-bundle catalog'
require_text CHANGELOG.md 'Treasury and vault-release localization'
require_text docs/adr/README.md '0347-versioned-refund-claim-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0347'
require_text docs/adr/0347-versioned-refund-claim-localization.md 'typed immutable in-bundle catalog'
require_text CHANGELOG.md 'refund-claim localization at the wallet boundary'
require_text docs/adr/README.md '0348-versioned-refund-execution-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0348'
require_text docs/adr/0348-versioned-refund-execution-localization.md 'typed immutable in-bundle catalog'
require_text CHANGELOG.md 'refund-execution localization at the final release'
require_text docs/adr/README.md '0349-versioned-refund-cancellation-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0349'
require_text docs/adr/0349-versioned-refund-cancellation-localization.md 'typed, immutable in-bundle catalog'
require_text CHANGELOG.md 'refund-cancellation localization at the governance'
require_text docs/adr/README.md '0350-versioned-settlement-adjustment-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0350'
require_text docs/adr/0350-versioned-settlement-adjustment-localization.md 'typed immutable catalog'
require_text CHANGELOG.md 'settlement-adjustment localization'
require_text docs/adr/README.md '0351-versioned-replay-operations-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0351'
require_text docs/adr/0351-versioned-replay-operations-localization.md 'typed immutable catalog'
require_text CHANGELOG.md 'replay-operations localization'
require_text docs/adr/README.md '0352-versioned-vault-reconciliation-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0352'
require_text docs/adr/0352-versioned-vault-reconciliation-localization.md 'typed immutable catalog'
require_text docs/adr/README.md '0353-material-3-expressive-aurora-v2-shell.md'
require_text docs/plans/implementation-plan.md 'ADR-0353'
require_text docs/adr/0353-material-3-expressive-aurora-v2-shell.md 'seed-derived M3 role matrix'
require_text docs/adr/README.md '0354-versioned-rule-governance-authoring-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0354'
require_text docs/adr/0354-versioned-rule-governance-authoring-localization.md 'immutable, typed, versioned catalogs'
require_text CHANGELOG.md 'ADR-0354 and typed localization'
require_text docs/adr/README.md '0355-versioned-revenue-streaming-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0355'
require_text docs/adr/0355-versioned-revenue-streaming-localization.md 'typed, immutable, versioned presentation catalog'
require_text CHANGELOG.md 'governed partner-payout'
require_text docs/adr/README.md '0356-versioned-access-control-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0356'
require_text docs/adr/0356-versioned-access-control-localization.md 'immutable, typed presentation catalogs'
require_text CHANGELOG.md 'ADR-0356 and typed access-control workspace localization'
require_text docs/adr/README.md '0357-versioned-shell-operational-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0357'
require_text docs/adr/0357-versioned-shell-operational-localization.md 'immutable typed catalog'
require_text CHANGELOG.md 'ADR-0357 and typed shell-operational localization'
require_text docs/adr/README.md '0358-versioned-lazy-workspace-recovery-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0358'
require_text docs/adr/0358-versioned-lazy-workspace-recovery-localization.md 'shell-operational typed catalog'
require_text CHANGELOG.md 'ADR-0358 and localized lazy-workspace loading'
require_text docs/adr/README.md '0359-explicit-fee-payer-binding.md'
require_text docs/plans/implementation-plan.md 'ADR-0359'
require_text docs/adr/0359-explicit-fee-payer-binding.md 'first static account'
require_text CHANGELOG.md 'ADR-0359 and an explicit fee-payer binding check'
require_text docs/adr/README.md '0360-versioned-document-simulation-localization.md'
require_text docs/plans/implementation-plan.md 'ADR-0360'
require_text docs/adr/0360-versioned-document-simulation-localization.md 'immutable, typed presentation catalog'
require_text CHANGELOG.md 'ADR-0360 and typed localization for the governance document-simulation'
require_text docs/adr/README.md '0373-tenant-scoped-evidence-access-explorer.md'
require_text docs/adr/README.md '0374-collapsible-material-navigation-rail.md'
require_text docs/adr/README.md '0375-bounded-linear-tax-base-operations.md'
require_text docs/plans/implementation-plan.md 'ADR-0373'
require_text docs/adr/0373-tenant-scoped-evidence-access-explorer.md 'derives the merchant boundary'
require_text CHANGELOG.md 'ADR-0373 and a tenant-scoped evidence-access explorer'
require_text CHANGELOG.md 'ADR-0374 and an expanded-by-default'
require_text docs/plans/implementation-plan.md 'ADR-0375'
require_text docs/adr/0375-bounded-linear-tax-base-operations.md 'integral atomic units'
require_text CHANGELOG.md 'ADR-0375 and the bounded linear tax-base v2 envelope'
require_text CHANGELOG.md 'vault-reconciliation localization'
require_text CHANGELOG.md 'typed document-quote catalog'
require_text CHANGELOG.md 'polite, bounded workspace navigation status'
require_text Makefile 'forced-colors-contract'
require_text docs/operations/requirements-traceability.md 'Evidence levels'
require_text docs/operations/requirements-traceability.md 'Linux/amd64 Agave validator conformance'
require_text docs/operations/requirements-traceability.md 'apps/web/src/authorization.ts`'
if grep -Fq 'apps/web/src/authorization.tsx' docs/operations/requirements-traceability.md; then
  echo "requirements traceability references nonexistent authorization.tsx" >&2
  failures=$((failures + 1))
fi
require_text docs/review/architecture-review-guide.md 'apps/web/src/wallet.ts`'
if grep -Fq 'apps/web/src/solana.ts' docs/review/architecture-review-guide.md; then
  echo "architecture review guide references nonexistent solana.ts" >&2
  failures=$((failures + 1))
fi
require_text docs/product/tax-coverage-matrix.md 'Engine semantics evidence'
require_text docs/product/tax-coverage-matrix.md 'progressive-marginal'
require_text docs/product/tax-coverage-matrix.md 'Deliberate non-claims'
require_text CHANGELOG.md 'executable UI language contract'
require_text CHANGELOG.md 'container and workflow pinning contract'
require_text CHANGELOG.md 'conformance workflow contract'
require_text CHANGELOG.md 'emergency pause cover governed vault'
require_text CHANGELOG.md 'program pause-coverage contract'
require_text CHANGELOG.md 'exact foreign-exchange evidence boundary'
require_text CHANGELOG.md 'server-owned signed FX evidence bounded context'
require_text CHANGELOG.md 'exact-release authenticated Docker integration evidence'
require_text CHANGELOG.md 'atomic digest-keyed FX provider policy snapshot'
require_text CHANGELOG.md 'registry-bound FX evidence issuance'
require_text CHANGELOG.md 'policy-scoped FX provider identity authorization'
require_text CHANGELOG.md 'deterministic FX provider agreement'
require_text CHANGELOG.md 'accelerated Tax Engine delivery model'
require_text CHANGELOG.md 'bounded concurrent FX provider acquisition'
require_text CHANGELOG.md 'bounded async FX signing boundary'
require_text CHANGELOG.md 'deterministic FX acquisition outcome order'
require_text CHANGELOG.md 'bounded vendor-neutral HTTP FX provider adapter'
require_text CHANGELOG.md 'bounded vendor-neutral HTTP FX signer adapter'
require_text CHANGELOG.md 'server-owned FX issuance coordinator'
require_text CHANGELOG.md 'historical FX-envelope integrity verification'
require_text CHANGELOG.md 'authenticated server-owned FX quote composition boundary'
require_text CHANGELOG.md 'vendor-neutral FX issuer process bootstrap'
require_text CHANGELOG.md 'public Tax Engine decision register'
require_text docs/adr/README.md 'ADR-0251: Accelerated Tax Engine delivery and decision freeze'
require_text docs/adr/README.md 'ADR-0252: Bounded concurrent FX provider acquisition'
require_text docs/adr/README.md 'ADR-0253: Bounded async FX signing boundary'
require_text docs/adr/README.md 'ADR-0254: Deterministic FX acquisition outcome order'
require_text docs/adr/README.md 'ADR-0255: Bounded HTTP FX provider adapter'
require_text docs/adr/README.md 'ADR-0256: Bounded HTTP FX signer adapter'
require_text docs/adr/README.md 'ADR-0257: Server-owned FX issuance coordinator'
require_text docs/adr/README.md 'ADR-0258: FX evidence replay integrity and shared signer ownership'
require_text docs/adr/README.md 'ADR-0259: Durable FX envelope quote binding'
require_text docs/adr/README.md 'ADR-0260: Authenticated server-owned FX quote composition'
require_text docs/README.md 'Accelerated Tax Engine delivery and decision freeze'
require_text docs/README.md 'Bounded concurrent FX provider acquisition'
require_text docs/README.md 'Bounded async FX signing boundary'
require_text docs/README.md 'Deterministic FX acquisition outcome order'
require_text docs/README.md 'FX evidence replay integrity and shared signer ownership'
require_text docs/README.md 'Durable FX envelope quote binding'
require_text docs/README.md 'Authenticated server-owned FX quote composition'
require_text docs/README.md 'Tax Engine decision register'
require_text README.md 'Tax Engine decision register'
require_text docs/operations/decision-register.md 'DEC-0251-01'
require_text docs/operations/decision-register.md 'state: FROZEN'
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
