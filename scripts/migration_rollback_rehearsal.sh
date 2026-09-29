#!/bin/sh

set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

postgres_image="postgres:18.6-alpine@sha256:77f585114c32fbca283dc835b0596f4e52b51b4c6662d7810b2f4084f60a1873"
runner_image="solana-tax-protocol-migration-runner:rollback-drill"
network_name="solana-tax-protocol-migration-rollback-$$"
source_container="solana-tax-protocol-migration-rollback-source-$$"
restore_container="solana-tax-protocol-migration-rollback-restore-$$"
drill_dir=$(mktemp -d)
base_migrations="$drill_dir/base-migrations"
candidate_migrations="$drill_dir/candidate-migrations"
pre_deploy_dump="$drill_dir/pre-deploy.dump"

cleanup() {
    exit_status=$?
    docker rm --force "$source_container" "$restore_container" >/dev/null 2>&1 || true
    docker network rm "$network_name" >/dev/null 2>&1 || true
    rm -rf "$drill_dir"
    exit "$exit_status"
}
trap cleanup EXIT INT TERM

umask 077
mkdir -p "$base_migrations" "$candidate_migrations"
cp -R db/migrations/. "$base_migrations/"
cp -R db/migrations/. "$candidate_migrations/"

cat > "$base_migrations/9998_rollback_probe_baseline.sql" <<'SQL'
CREATE TABLE migration_rollback_probe (
    id uuid PRIMARY KEY,
    lifecycle text NOT NULL CHECK (lifecycle IN ('pre-deploy', 'post-deploy'))
);
SQL

cat > "$candidate_migrations/9998_rollback_probe_baseline.sql" <<'SQL'
CREATE TABLE migration_rollback_probe (
    id uuid PRIMARY KEY,
    lifecycle text NOT NULL CHECK (lifecycle IN ('pre-deploy', 'post-deploy'))
);
SQL
cat > "$candidate_migrations/9999_rollback_probe_expand.sql" <<'SQL'
BEGIN;
ALTER TABLE migration_rollback_probe
    ADD COLUMN tax_category text NULL;
CREATE INDEX migration_rollback_probe_tax_category_idx
    ON migration_rollback_probe (tax_category)
    WHERE tax_category IS NOT NULL;
COMMIT;
SQL

docker network create --internal "$network_name" >/dev/null
docker run --detach \
    --name "$source_container" \
    --network "$network_name" \
    --network-alias postgres \
    --platform linux/amd64 \
    --env POSTGRES_DB=tax_protocol \
    --env POSTGRES_USER=tax_protocol \
    --env POSTGRES_PASSWORD=rollback-drill-only \
    "$postgres_image" >/dev/null

source_ready=false
for _attempt in $(seq 1 90); do
    if docker exec "$source_container" pg_isready -U tax_protocol -d tax_protocol >/dev/null 2>&1; then
        source_ready=true
        break
    fi
    sleep 1
done
if [ "$source_ready" != "true" ]; then
    echo "rollback rehearsal source PostgreSQL service did not become ready" >&2
    docker logs "$source_container" >&2 || true
    exit 1
fi

docker build --platform linux/amd64 \
    --file ops/docker/Dockerfile.migration-runner \
    --tag "$runner_image" . >/dev/null

runner_env="DATABASE_URL=postgres://tax_protocol:rollback-drill-only@postgres:5432/tax_protocol"
run_migrations() {
    migrations_dir=$1
    docker run --rm --network "$network_name" \
        --env "$runner_env" \
        --env MIGRATIONS_DIR=/migrations \
        --volume "$migrations_dir:/migrations:ro" \
        "$runner_image"
}

base_summary=$(run_migrations "$base_migrations")
case "$base_summary" in
    *"applied="*"skipped=0"*) ;;
    *)
        echo "unexpected base migration summary: $base_summary" >&2
        exit 1
        ;;
esac

sentinel_id="00000000-0000-7000-8000-000000000001"
docker exec "$source_container" psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    -c "INSERT INTO migration_rollback_probe (id, lifecycle) VALUES ('$sentinel_id', 'pre-deploy');" \
    >/dev/null

docker exec "$source_container" pg_dump \
    --format=custom \
    --no-owner \
    --no-acl \
    --dbname=tax_protocol \
    --username=tax_protocol > "$pre_deploy_dump"
if [ ! -s "$pre_deploy_dump" ]; then
    echo "pre-deployment rollback image is empty" >&2
    exit 1
fi

candidate_summary=$(run_migrations "$candidate_migrations")
case "$candidate_summary" in
    *"applied=1"*) ;;
    *)
        echo "expand migration was not applied exactly once: $candidate_summary" >&2
        exit 1
        ;;
esac

expanded_column_count=$(docker exec "$source_container" psql -U tax_protocol -d tax_protocol -Atq \
    -c "SELECT count(*) FROM information_schema.columns WHERE table_name = 'migration_rollback_probe' AND column_name = 'tax_category';")
expanded_index_count=$(docker exec "$source_container" psql -U tax_protocol -d tax_protocol -Atq \
    -c "SELECT count(*) FROM pg_indexes WHERE indexname = 'migration_rollback_probe_tax_category_idx';")
if [ "$expanded_column_count" != "1" ] || [ "$expanded_index_count" != "1" ]; then
    echo "expand migration did not create the expected compatible objects: column=$expanded_column_count index=$expanded_index_count" >&2
    exit 1
fi

docker run --detach \
    --name "$restore_container" \
    --network "$network_name" \
    --network-alias restore-postgres \
    --platform linux/amd64 \
    --env POSTGRES_DB=tax_protocol \
    --env POSTGRES_USER=tax_protocol \
    --env POSTGRES_PASSWORD=rollback-restore-only \
    "$postgres_image" >/dev/null

restore_ready=false
for _attempt in $(seq 1 90); do
    if docker exec "$restore_container" pg_isready -U tax_protocol -d tax_protocol >/dev/null 2>&1; then
        restore_ready=true
        break
    fi
    sleep 1
done
if [ "$restore_ready" != "true" ]; then
    echo "rollback rehearsal restore PostgreSQL service did not become ready" >&2
    docker logs "$restore_container" >&2 || true
    exit 1
fi

docker exec -i "$restore_container" pg_restore \
    --exit-on-error \
    --no-owner \
    --no-acl \
    --username=tax_protocol \
    --dbname=tax_protocol < "$pre_deploy_dump"

restored_column_count=$(docker exec "$restore_container" psql -U tax_protocol -d tax_protocol -Atq \
    -c "SELECT count(*) FROM information_schema.columns WHERE table_name = 'migration_rollback_probe' AND column_name = 'tax_category';")
restored_index_count=$(docker exec "$restore_container" psql -U tax_protocol -d tax_protocol -Atq \
    -c "SELECT count(*) FROM pg_indexes WHERE indexname = 'migration_rollback_probe_tax_category_idx';")
restored_lifecycle=$(docker exec "$restore_container" psql -U tax_protocol -d tax_protocol -Atq \
    -c "SELECT lifecycle FROM migration_rollback_probe WHERE id = '$sentinel_id';")
if [ "$restored_column_count" != "0" ] || [ "$restored_index_count" != "0" ] || [ "$restored_lifecycle" != "pre-deploy" ]; then
    echo "pre-deployment restore did not remove expand objects or preserve sentinel: column=$restored_column_count index=$restored_index_count lifecycle=$restored_lifecycle" >&2
    exit 1
fi

restore_runner_env="DATABASE_URL=postgres://tax_protocol:rollback-restore-only@restore-postgres:5432/tax_protocol"
forward_recovery_summary=$(docker run --rm --network "$network_name" \
    --env "$restore_runner_env" \
    --env MIGRATIONS_DIR=/migrations \
    --volume "$candidate_migrations:/migrations:ro" \
    "$runner_image")
case "$forward_recovery_summary" in
    *"applied=1"*) ;;
    *)
        echo "forward recovery did not reapply the expand migration: $forward_recovery_summary" >&2
        exit 1
        ;;
esac

forward_recovery_repeat=$(docker run --rm --network "$network_name" \
    --env "$restore_runner_env" \
    --env MIGRATIONS_DIR=/migrations \
    --volume "$candidate_migrations:/migrations:ro" \
    "$runner_image")
case "$forward_recovery_repeat" in
    *"applied=0"*"skipped="*) ;;
    *)
        echo "forward recovery was not idempotent: $forward_recovery_repeat" >&2
        exit 1
        ;;
esac

recovered_column_count=$(docker exec "$restore_container" psql -U tax_protocol -d tax_protocol -Atq \
    -c "SELECT count(*) FROM information_schema.columns WHERE table_name = 'migration_rollback_probe' AND column_name = 'tax_category';")
recovered_lifecycle=$(docker exec "$restore_container" psql -U tax_protocol -d tax_protocol -Atq \
    -c "SELECT lifecycle FROM migration_rollback_probe WHERE id = '$sentinel_id';")
if [ "$recovered_column_count" != "1" ] || [ "$recovered_lifecycle" != "pre-deploy" ]; then
    echo "forward recovery did not preserve schema/data: column=$recovered_column_count lifecycle=$recovered_lifecycle" >&2
    exit 1
fi

echo "migration-rollback-rehearsal"
echo "base_migrations=$base_summary"
echo "expand_migration=$candidate_summary"
echo "pre_deploy_restore=passed"
echo "old_schema_read=passed"
echo "forward_recovery=$forward_recovery_summary"
echo "forward_recovery_repeat=$forward_recovery_repeat"
echo "sentinel_preserved=$recovered_lifecycle"
