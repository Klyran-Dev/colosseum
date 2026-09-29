SHELL := /bin/sh

.PHONY: fmt lint test bench-tax-engine benchmark-quote-service benchmark-rule-simulation postgres-pool-drill postgres-index-plan-drill postgres-pitr-drill vault-reconciliation-drill migration-integrity-drill migration-rollback-rehearsal adjustment-authority-drill refund-funding-verifier-drill refund-proof-lease-drill refund-execute-api-contention-drill refund-proof-integration-drill indexer-replay-drill indexer-replay-dead-letter-drill alert-delivery-drill web-install web-check web-browser-check security-scan observability-config public-docs-check environment-contract deployment-config-contract migration-bootstrap-contract check compose-config migrate backup-restore-drill encrypted-backup-restore-drill projection-rebuild-drill postgres-contention-drill integration-check solana-build solana-conformance solana-up solana-down indexer-up indexer-down route-verifier-up route-verifier-down observability-up observability-down up down logs

fmt:
	cargo fmt --all

lint:
	cargo clippy --locked --workspace --all-targets -- -D warnings

test:
	cargo test --locked --workspace --all-targets

bench-tax-engine:
	cargo bench --locked -p tax-engine --bench tax_engine

benchmark-quote-service:
	./scripts/benchmark_quote_service.sh

benchmark-rule-simulation:
	./scripts/benchmark_rule_simulation.sh

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
	docker compose --profile browser run --build --rm web-e2e

security-scan:
	./scripts/security_scan.sh

observability-config:
	./scripts/verify_observability_config.sh

public-docs-check:
	./scripts/verify_public_repository.sh

environment-contract:
	cargo test --locked -p api config::tests
	./scripts/verify_environment_contract.sh

deployment-config-contract:
	./scripts/verify_deployment_config_contract.sh

migration-bootstrap-contract:
	./scripts/verify_migration_bootstrap.sh

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

check: lint test web-check security-scan compose-config observability-config public-docs-check environment-contract deployment-config-contract migration-bootstrap-contract

up:
	docker compose up --build

down:
	docker compose down

logs:
	docker compose logs --follow --tail=200
