#!/bin/sh

set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

postgres_image="postgres:18.6-alpine@sha256:77f585114c32fbca283dc835b0596f4e52b51b4c6662d7810b2f4084f60a1873"
source_container="solana-tax-protocol-backup-source-$$"
target_container="solana-tax-protocol-restore-drill-$$"
drill_dir=$(mktemp -d)
dump_file="$drill_dir/tax_protocol.dump"

cleanup() {
    docker rm --force "$source_container" >/dev/null 2>&1 || true
    docker rm --force "$target_container" >/dev/null 2>&1 || true
    rm -rf "$drill_dir"
}
trap cleanup EXIT INT TERM

umask 077

source_ready=false
docker run --detach \
    --name "$source_container" \
    --platform linux/amd64 \
    --env POSTGRES_DB=tax_protocol \
    --env POSTGRES_USER=tax_protocol \
    --env POSTGRES_PASSWORD=backup-drill-only \
    "$postgres_image" >/dev/null

for _attempt in $(seq 1 60); do
    if docker exec "$source_container" psql -U tax_protocol -d tax_protocol -Atc "SELECT 1;" >/dev/null 2>&1; then
        source_ready=true
        break
    fi
    sleep 1
done
if [ "$source_ready" != "true" ]; then
    echo "backup source PostgreSQL service did not become ready" >&2
    exit 1
fi

for migration in db/migrations/*.sql; do
    docker exec -i "$source_container" psql \
        -v ON_ERROR_STOP=1 \
        -U tax_protocol \
        -d tax_protocol < "$migration" >/dev/null
done
for fixture in db/fixtures/*.sql; do
    docker exec -i "$source_container" psql \
        -v ON_ERROR_STOP=1 \
        -U tax_protocol \
        -d tax_protocol < "$fixture" >/dev/null
done

docker exec "$source_container" pg_dump \
    --format=custom \
    --no-owner \
    --no-acl \
    --dbname=tax_protocol \
    --username=tax_protocol > "$dump_file"
if [ ! -s "$dump_file" ]; then
    echo "logical backup is empty" >&2
    exit 1
fi

docker run --detach \
    --name "$target_container" \
    --platform linux/amd64 \
    --env POSTGRES_DB=tax_protocol \
    --env POSTGRES_USER=tax_protocol \
    --env POSTGRES_PASSWORD=restore-drill-only \
    "$postgres_image" >/dev/null

target_ready=false
for _attempt in $(seq 1 60); do
    if docker exec "$target_container" psql -U tax_protocol -d tax_protocol -Atc "SELECT 1;" >/dev/null 2>&1; then
        target_ready=true
        break
    fi
    sleep 1
done
if [ "$target_ready" != "true" ]; then
    echo "restore target PostgreSQL service did not become ready" >&2
    exit 1
fi

docker exec -i "$target_container" pg_restore \
    --exit-on-error \
    --no-owner \
    --no-acl \
    --username=tax_protocol \
    --dbname=tax_protocol < "$dump_file"

facts_column_count=$(docker exec "$target_container" psql -U tax_protocol -d tax_protocol -Atc \
    "SELECT count(*) FROM information_schema.columns WHERE table_name = 'tax_calculations' AND column_name = 'facts_document';")
if [ "$facts_column_count" != "1" ]; then
    echo "restored schema is missing tax_calculations.facts_document" >&2
    exit 1
fi

if ! docker exec -i "$target_container" psql \
    -v ON_ERROR_STOP=1 \
    -U tax_protocol \
    -d tax_protocol < db/verification/uuidv7_contract.sql >/dev/null; then
    echo "restored PostgreSQL UUIDv7 schema contract failed" >&2
    exit 1
fi

uuidv7_version=$(docker exec "$target_container" psql -U tax_protocol -d tax_protocol -Atc \
    "SELECT uuid_extract_version(uuidv7());")
if [ "$uuidv7_version" != "7" ]; then
    echo "restored PostgreSQL UUIDv7 contract is unavailable" >&2
    exit 1
fi

required_tables=$(docker exec "$target_container" psql -U tax_protocol -d tax_protocol -Atc \
    "SELECT count(*) FROM pg_class WHERE relkind = 'r' AND relname IN ('tax_calculations', 'outbox_events', 'settlement_intents', 'settlement_route_verifications');")
if [ "$required_tables" != "4" ]; then
    echo "restored evidence schema is incomplete" >&2
    exit 1
fi

echo "PostgreSQL logical backup and isolated restore drill passed"
