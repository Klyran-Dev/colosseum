#!/bin/sh

set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

postgres_image="postgres:18.6-alpine@sha256:77f585114c32fbca283dc835b0596f4e52b51b4c6662d7810b2f4084f60a1873"
runner_image="solana-tax-protocol-migration-runner:drill"
expected_migration_count=$(find db/migrations -type f -name '[0-9][0-9][0-9][0-9]_*.sql' | wc -l | tr -d ' ')
network_name="solana-tax-protocol-migration-drill-$$"
postgres_container="solana-tax-protocol-migration-postgres-$$"
drill_dir=$(mktemp -d)
mutated_dir="$drill_dir/migrations"

cleanup() {
    exit_status=$?
    docker rm --force "$postgres_container" >/dev/null 2>&1 || true
    docker network rm "$network_name" >/dev/null 2>&1 || true
    rm -rf "$drill_dir"
    exit "$exit_status"
}
trap cleanup EXIT INT TERM

umask 077
docker network create --internal "$network_name" >/dev/null
docker run --detach \
    --name "$postgres_container" \
    --network "$network_name" \
    --network-alias postgres \
    --env POSTGRES_DB=tax_protocol \
    --env POSTGRES_USER=tax_protocol \
    --env POSTGRES_PASSWORD=migration-drill-only \
    "$postgres_image" >/dev/null

ready=false
for _attempt in $(seq 1 90); do
    if docker exec "$postgres_container" pg_isready -U tax_protocol -d tax_protocol >/dev/null 2>&1; then
        ready=true
        break
    fi
    sleep 1
done
if [ "$ready" != "true" ]; then
    echo "migration drill PostgreSQL service did not become ready" >&2
    docker logs "$postgres_container" >&2 || true
    exit 1
fi

docker build --platform linux/amd64 \
    --file ops/docker/Dockerfile.migration-runner \
    --tag "$runner_image" . >/dev/null

runner_env="DATABASE_URL=postgres://tax_protocol:migration-drill-only@postgres:5432/tax_protocol"
docker run --rm --network "$network_name" \
    --env "$runner_env" \
    --env MIGRATIONS_DIR=/migrations \
    --volume "$project_root/db/migrations:/migrations:ro" \
    "$runner_image" > "$drill_dir/concurrent-a.out" &
runner_a_pid=$!
docker run --rm --network "$network_name" \
    --env "$runner_env" \
    --env MIGRATIONS_DIR=/migrations \
    --volume "$project_root/db/migrations:/migrations:ro" \
    "$runner_image" > "$drill_dir/concurrent-b.out" &
runner_b_pid=$!
runner_a_status=0
runner_b_status=0
wait "$runner_a_pid" || runner_a_status=$?
wait "$runner_b_pid" || runner_b_status=$?
if [ "$runner_a_status" -ne 0 ] || [ "$runner_b_status" -ne 0 ]; then
    echo "concurrent migration runners failed: a=$runner_a_status b=$runner_b_status" >&2
    exit 1
fi
concurrent_a=$(tr -d '\r\n' < "$drill_dir/concurrent-a.out")
concurrent_b=$(tr -d '\r\n' < "$drill_dir/concurrent-b.out")
case "$concurrent_a" in
    *"applied=$expected_migration_count skipped=0"*) concurrent_a_applied=1 ;;
    *"applied=0 skipped=$expected_migration_count"*) concurrent_a_applied=0 ;;
    *)
        echo "unexpected concurrent runner summary: a=$concurrent_a" >&2
        exit 1
        ;;
esac
case "$concurrent_b" in
    *"applied=$expected_migration_count skipped=0"*) concurrent_b_applied=1 ;;
    *"applied=0 skipped=$expected_migration_count"*) concurrent_b_applied=0 ;;
    *)
        echo "unexpected concurrent runner summary: b=$concurrent_b" >&2
        exit 1
        ;;
esac
if [ "$concurrent_a_applied" -eq "$concurrent_b_applied" ]; then
    echo "concurrent migration runners did not serialize: a=$concurrent_a b=$concurrent_b" >&2
    exit 1
fi

second_run=$(docker run --rm --network "$network_name" \
    --env "$runner_env" \
    --env MIGRATIONS_DIR=/migrations \
    --volume "$project_root/db/migrations:/migrations:ro" \
    "$runner_image")
case "$second_run" in
    *"applied=0 skipped=$expected_migration_count"*) ;;
    *)
        echo "unexpected idempotent migration-runner summary: $second_run" >&2
        exit 1
        ;;
esac

atomic_failure_dir="$drill_dir/atomic-failure-migrations"
cp -R db/migrations "$atomic_failure_dir"
printf 'CREATE TABLE migration_partial_probe (id integer PRIMARY KEY);\nSELECT 1 / 0;\n' \
    > "$atomic_failure_dir/0031_atomic_failure.sql"
if docker run --rm --network "$network_name" \
    --env "$runner_env" \
    --env MIGRATIONS_DIR=/migrations \
    --volume "$atomic_failure_dir:/migrations:ro" \
    "$runner_image" >/dev/null 2>&1; then
    echo "migration-runner accepted a failed migration" >&2
    exit 1
fi
partial_probe_count=$(docker exec "$postgres_container" psql -U tax_protocol -d tax_protocol -Atq \
    -c "SELECT count(*) FROM pg_class WHERE relname = 'migration_partial_probe';")
if [ "$partial_probe_count" != "0" ]; then
    echo "failed migration left partial schema state: $partial_probe_count" >&2
    exit 1
fi

cp -R db/migrations "$mutated_dir"
printf '\n-- deliberate checksum drift for ADR-0138\n' >> "$mutated_dir/0001_initial_schema.sql"
if docker run --rm --network "$network_name" \
    --env "$runner_env" \
    --env MIGRATIONS_DIR=/migrations \
    --volume "$mutated_dir:/migrations:ro" \
    "$runner_image" >/dev/null 2>&1; then
    echo "migration-runner accepted historical checksum drift" >&2
    exit 1
fi

ledger_count=$(docker exec "$postgres_container" psql -U tax_protocol -d tax_protocol -Atq \
    -c "SELECT count(*) FROM schema_migrations;")
if [ "$ledger_count" != "$expected_migration_count" ]; then
    echo "migration ledger count is not $expected_migration_count: $ledger_count" >&2
    exit 1
fi

docker exec -i "$postgres_container" psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    < db/verification/refund_funding_verification_leases.sql >/dev/null

docker exec -i "$postgres_container" psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    < db/verification/settlement_route_mint_precision.sql >/dev/null

docker exec -i "$postgres_container" psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    < db/verification/settlement_route_asset_projection.sql >/dev/null

docker exec -i "$postgres_container" psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    < db/verification/settlement_route_verification_runs.sql >/dev/null

docker exec -i "$postgres_container" psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    < db/verification/settlement_route_verification_approval.sql >/dev/null

docker exec -i "$postgres_container" psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    < db/verification/tax_applicability_contexts.sql >/dev/null

docker exec -i "$postgres_container" psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    < db/verification/applicability_binding_verifications.sql >/dev/null

docker exec -i "$postgres_container" psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    < db/verification/applicability_binding_verification_jobs.sql >/dev/null

echo "migration-integrity-drill"
echo "concurrent_run_a=$concurrent_a"
echo "concurrent_run_b=$concurrent_b"
echo "second_run=$second_run"
echo "failed_migration=rolled_back"
echo "checksum_drift=failed_closed"
echo "ledger_count=$ledger_count"
echo "refund_funding_verification_lease_contract=passed"
echo "settlement_route_mint_precision_contract=passed"
echo "settlement_route_asset_projection_contract=passed"
echo "settlement_route_verification_runs_contract=passed"
echo "settlement_route_verification_approval_contract=passed"
echo "tax_applicability_context_contract=passed"
echo "applicability_binding_verification_contract=passed"
echo "applicability_binding_verification_job_contract=passed"
