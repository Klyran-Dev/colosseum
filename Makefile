SHELL := /bin/sh

.PHONY: fmt lint test bench-tax-engine benchmark-tax-engine-service benchmark-quote-service benchmark-document-quote-service benchmark-rule-simulation benchmark-authorization-capabilities authorization-benchmark-contract postgres-pool-drill postgres-index-plan-drill postgres-pitr-drill vault-reconciliation-drill migration-integrity-drill migration-rollback-rehearsal adjustment-authority-drill refund-funding-verifier-drill refund-proof-lease-drill refund-execute-api-contention-drill refund-proof-integration-drill refund-cancellation-reconciliation-drill indexer-replay-drill indexer-replay-dead-letter-drill alert-delivery-drill otel-trace-drill web-install web-check web-browser-check web-csp-contract web-color-contract material-palette-contract forced-colors-contract ui-language-contract non-custodial-wallet-contract rule-release-provenance-contract security-scan observability-config grafana-dashboard-contract api-metric-registry-contract public-docs-check release-claim-contract environment-contract deployment-config-contract migration-bootstrap-contract refund-claim-cancellation-reconciliation-contract rust-panic-safety-contract container-pinning-contract solana-conformance-workflow-contract program-pause-coverage-contract bootstrap-contract reset-local-contract release-bundle-integrity-contract service-lifecycle-contract tax-engine-service-contract tax-engine-runtime-contract tax-engine-mtls-runtime-contract api-auth-jwks-reload-contract api-jurisdiction-provider-reload-contract route-verifier-backfill-contract ci-toolchain-contract evm-adapter-contract evm-adapter-check tier1-local-check tier1-local-check-contract check compose-config migrate backup-restore-drill encrypted-backup-restore-drill projection-rebuild-drill postgres-contention-drill integration-check solana-build solana-conformance solana-up solana-down indexer-up indexer-down route-verifier-up route-verifier-down observability-up observability-down bootstrap reset-local up down logs

fmt:
	cargo fmt --all

lint:
	cargo clippy --locked --workspace --all-targets -- -D warnings

test:
	cargo test --locked --workspace --all-targets

bench-tax-engine:
	cargo bench --locked -p tax-engine --bench tax_engine

benchmark-tax-engine-service:
	./scripts/benchmark_tax_engine_service.sh

benchmark-quote-service:
	./scripts/benchmark_quote_service.sh

benchmark-document-quote-service:
	BENCHMARK_MODE=document COMPOSE_PROJECT_NAME=solana-tax-protocol-document-quote-load ./scripts/benchmark_quote_service.sh

benchmark-rule-simulation:
	./scripts/benchmark_rule_simulation.sh

benchmark-authorization-capabilities:
	./scripts/benchmark_authorization_capabilities.sh

authorization-benchmark-contract:
	./scripts/verify_authorization_benchmark_contract.sh

postgres-pool-drill:
	./scripts/postgres_pool_drill.sh

postgres-index-plan-drill:
	./scripts/postgres_index_plan_drill.sh

postgres-pitr-drill:
	./scripts/postgres_pitr_drill.sh

web-install:
	pnpm --dir apps/web install --frozen-lockfile

web-check: web-install
	pnpm --dir apps/web test
	pnpm --dir apps/web typecheck
	pnpm --dir apps/web build

web-browser-check:
	mkdir -p test-results/web-e2e
	set -eu; \
	project="solana-tax-protocol-web-e2e"; \
	cleanup() { docker compose -p "$$project" -f compose.yaml -f compose.browser.yaml down -v --remove-orphans >/dev/null 2>&1 || true; }; \
	trap cleanup EXIT INT TERM; \
	docker compose -p "$$project" -f compose.yaml -f compose.browser.yaml --profile browser run --build --rm web-e2e

web-csp-contract:
	./scripts/verify_web_csp_contract.sh

web-color-contract:
	./scripts/verify_web_color_contract.sh

release-claim-contract:
	./scripts/verify_release_claim_contract.sh
	./scripts/release_claim_contract_test.sh

material-palette-contract:
	node scripts/verify_material_palette_contract.mjs

forced-colors-contract:
	./scripts/verify_forced_colors_contract.sh

tier1-local-check-contract:
	./scripts/verify_tier1_local_check_contract.sh

tier1-local-check:
	$(MAKE) check
	$(MAKE) integration-check
	$(MAKE) web-browser-check
	$(MAKE) solana-conformance

ui-language-contract:
	./scripts/verify_ui_language_contract.sh

non-custodial-wallet-contract:
	./scripts/verify_non_custodial_wallet_contract.sh

rule-release-provenance-contract:
	./scripts/verify_rule_release_provenance_contract.sh

security-scan:
	./scripts/security_scan.sh

observability-config:
	./scripts/verify_observability_config.sh

grafana-dashboard-contract:
	node scripts/verify_grafana_dashboard_contract.mjs

api-metric-registry-contract:
	./scripts/verify_api_metric_registry_contract.sh

public-docs-check:
	./scripts/verify_public_repository.sh

environment-contract:
	cargo test --locked -p api config::tests
	./scripts/verify_environment_contract.sh

deployment-config-contract:
	./scripts/verify_deployment_config_contract.sh

migration-bootstrap-contract:
	./scripts/verify_migration_bootstrap.sh

refund-claim-cancellation-reconciliation-contract:
	./scripts/verify_refund_claim_cancellation_reconciliation_contract.sh

rust-panic-safety-contract:
	./scripts/verify_rust_panic_safety_contract.sh

container-pinning-contract:
	./scripts/verify_container_pinning_contract.sh

evm-adapter-contract:
	./scripts/verify_evm_adapter_contract.sh

evm-adapter-check: evm-adapter-contract
	docker build --platform linux/amd64 --file ops/docker/Dockerfile.evm-adapter --tag solana-tax-protocol/evm-adapter-check:0.8.37 .

solana-conformance-workflow-contract:
	./scripts/verify_solana_conformance_workflow_contract.sh

program-pause-coverage-contract:
	./scripts/verify_program_pause_coverage.sh

bootstrap-contract:
	./scripts/verify_bootstrap_contract.sh

reset-local-contract:
	./scripts/verify_reset_local_contract.sh

release-bundle-integrity-contract:
	./scripts/release_bundle_integrity_contract.sh

service-lifecycle-contract:
	./scripts/verify_service_lifecycle_contract.sh

tax-engine-service-contract:
	cargo test --locked -p tax-engine-service --all-targets

tax-engine-runtime-contract:
	./scripts/verify_tax_engine_runtime_contract.sh

tax-engine-mtls-runtime-contract:
	./scripts/verify_tax_engine_mtls_runtime_contract.sh

api-auth-jwks-reload-contract:
	./scripts/verify_api_auth_jwks_reload_contract.sh

api-jurisdiction-provider-reload-contract:
	./scripts/verify_api_jurisdiction_provider_reload_contract.sh

route-verifier-backfill-contract:
	./scripts/verify_route_verifier_backfill_contract.sh

ci-toolchain-contract:
	./scripts/verify_ci_toolchain_contract.sh

compose-config:
	docker compose --profile browser --profile cache --profile observability --profile indexer --profile route-verifier --profile solana config --quiet

integration-check:
	./scripts/verify_local_stack.sh

indexer-replay-drill:
	./scripts/indexer_replay_drill.sh

indexer-replay-dead-letter-drill:
	./scripts/indexer_replay_dead_letter_drill.sh

alert-delivery-drill:
	./scripts/alert_delivery_drill.sh

otel-trace-drill:
	./scripts/otel_trace_drill.sh

migrate:
	docker compose run --rm migrator

solana-up:
	./scripts/build_sbf.sh
	docker compose --profile solana up --build -d validator

solana-build:
	./scripts/build_sbf.sh

solana-conformance:
	./scripts/run_solana_conformance.sh

solana-down:
	docker compose --profile solana stop validator

indexer-up:
	docker compose --profile solana --profile indexer up -d postgres
	@until docker compose exec -T postgres pg_isready -U tax_protocol -d tax_protocol >/dev/null 2>&1; do sleep 1; done
	docker compose run --rm migrator

backup-restore-drill:
	./scripts/backup_restore_drill.sh

encrypted-backup-restore-drill:
	BACKUP_DRILL_ENCRYPTED=true ./scripts/backup_restore_drill.sh

projection-rebuild-drill:
	./scripts/projection_rebuild_drill.sh

postgres-contention-drill:
	./scripts/postgres_contention_drill.sh

refund-funding-verifier-drill:
	./scripts/refund_funding_verifier_drill.sh

refund-proof-lease-drill:
	./scripts/refund_proof_lease_drill.sh

refund-execute-api-contention-drill:
	./scripts/refund_execute_api_contention_drill.sh

refund-proof-integration-drill:
	./scripts/refund_proof_integration_drill.sh

refund-cancellation-reconciliation-drill:
	./scripts/refund_cancellation_reconciliation_drill.sh

vault-reconciliation-drill:
	./scripts/vault_reconciliation_drill.sh

migration-integrity-drill:
	./scripts/migration_integrity_drill.sh

migration-rollback-rehearsal:
	./scripts/migration_rollback_rehearsal.sh

adjustment-authority-drill:
	./scripts/adjustment_authority_drill.sh

indexer-down:
	docker compose --profile indexer stop indexer

route-verifier-up:
	docker compose --profile route-verifier up --build -d route-verifier

route-verifier-down:
	docker compose --profile route-verifier stop route-verifier

observability-up:
	docker compose --profile cache --profile observability up -d redis alert-receiver alertmanager prometheus grafana otel-collector

observability-down:
	docker compose --profile observability stop grafana prometheus alertmanager alert-receiver otel-collector redis

check: lint test web-check web-csp-contract web-color-contract material-palette-contract forced-colors-contract release-claim-contract release-bundle-integrity-contract tier1-local-check-contract ui-language-contract non-custodial-wallet-contract rule-release-provenance-contract authorization-benchmark-contract security-scan compose-config observability-config grafana-dashboard-contract api-metric-registry-contract public-docs-check environment-contract deployment-config-contract migration-bootstrap-contract refund-claim-cancellation-reconciliation-contract rust-panic-safety-contract container-pinning-contract solana-conformance-workflow-contract program-pause-coverage-contract bootstrap-contract reset-local-contract service-lifecycle-contract tax-engine-service-contract tax-engine-runtime-contract tax-engine-mtls-runtime-contract api-auth-jwks-reload-contract api-jurisdiction-provider-reload-contract route-verifier-backfill-contract ci-toolchain-contract evm-adapter-check

bootstrap:
	./scripts/bootstrap_local.sh

reset-local:
	./scripts/reset_local.sh

up:
	docker compose up --build

down:
	docker compose down

logs:
	docker compose logs --follow --tail=200
