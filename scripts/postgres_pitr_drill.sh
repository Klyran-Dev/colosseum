#!/bin/sh

set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

postgres_image="postgres:18.6-alpine@sha256:77f585114c32fbca283dc835b0596f4e52b51b4c6662d7810b2f4084f60a1873"
source_container="solana-tax-protocol-pitr-source-$$"
target_container="solana-tax-protocol-pitr-target-$$"
drill_dir=$(mktemp -d)
archive_dir="$drill_dir/archive"
base_dir="$drill_dir/base"
mkdir -p "$archive_dir" "$base_dir"
chmod 0777 "$archive_dir" "$base_dir"

cleanup() {
    docker rm --force "$source_container" >/dev/null 2>&1 || true
    docker rm --force "$target_container" >/dev/null 2>&1 || true
    rm -rf "$drill_dir"
}
trap cleanup EXIT INT TERM

wait_for_sql() {
    container="$1"
    for _attempt in $(seq 1 90); do
        if docker exec "$container" psql -U tax_protocol -d tax_protocol -Atq -c "SELECT 1;" >/dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    echo "PostgreSQL container did not become queryable: $container" >&2
    docker logs "$container" >&2 || true
    return 1
}

wait_for_archive() {
    segment="$1"
    for _attempt in $(seq 1 90); do
        if [ -s "$archive_dir/$segment" ]; then
            return 0
        fi
        sleep 1
    done
    echo "WAL segment was not archived: $segment" >&2
    docker exec "$source_container" psql -U tax_protocol -d tax_protocol -Atq \
        -c "SELECT archived_count, failed_count, last_archived_wal FROM pg_stat_archiver;" >&2 || true
    return 1
}

docker run --detach \
    --name "$source_container" \
    --platform linux/amd64 \
    --env POSTGRES_DB=tax_protocol \
    --env POSTGRES_USER=tax_protocol \
    --env POSTGRES_PASSWORD=pitr-drill-only \
    --volume "$archive_dir:/archive" \
    --volume "$base_dir:/base" \
    "$postgres_image" \
    postgres \
    -c wal_level=replica \
    -c archive_mode=on \
    -c archive_timeout=1s \
    -c max_wal_senders=2 \
    -c "archive_command=test ! -f /archive/%f && cp %p /archive/%f" \
    >/dev/null

wait_for_sql "$source_container"

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

docker exec "$source_container" psql \
    -v ON_ERROR_STOP=1 \
    -U tax_protocol \
    -d tax_protocol \
    -c "CREATE TABLE pitr_drill_marker (id integer PRIMARY KEY, label text NOT NULL);" \
    >/dev/null

rm -rf "$base_dir"/*
docker exec -e PGPASSWORD=pitr-drill-only "$source_container" pg_basebackup \
    --username=tax_protocol \
    --pgdata=/base \
    --format=plain \
    --wal-method=stream \
    --checkpoint=fast \
    --no-password \
    >/dev/null

docker exec "$source_container" psql \
    -v ON_ERROR_STOP=1 \
    -U tax_protocol \
    -d tax_protocol \
    -c "INSERT INTO pitr_drill_marker (id, label) VALUES (1, 'target');" \
    >/dev/null
target_lsn=$(docker exec "$source_container" psql \
    -v ON_ERROR_STOP=1 \
    -U tax_protocol \
    -d tax_protocol \
    -Atq \
    -c "SELECT pg_current_wal_lsn();")
if [ -z "$target_lsn" ]; then
    echo "target WAL LSN was empty" >&2
    exit 1
fi

target_segment=$(docker exec "$source_container" psql \
    -v ON_ERROR_STOP=1 \
    -U tax_protocol \
    -d tax_protocol \
    -Atq \
    -c "SELECT pg_walfile_name('$target_lsn'::pg_lsn);")
docker exec "$source_container" psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    -c "SELECT pg_switch_wal();" >/dev/null
wait_for_archive "$target_segment"

docker exec "$source_container" psql \
    -v ON_ERROR_STOP=1 \
    -U tax_protocol \
    -d tax_protocol \
    -c "INSERT INTO pitr_drill_marker (id, label) VALUES (2, 'post-target');" \
    >/dev/null
post_target_lsn=$(docker exec "$source_container" psql \
    -v ON_ERROR_STOP=1 \
    -U tax_protocol \
    -d tax_protocol \
    -Atq \
    -c "SELECT pg_current_wal_lsn();")
post_target_segment=$(docker exec "$source_container" psql \
    -v ON_ERROR_STOP=1 \
    -U tax_protocol \
    -d tax_protocol \
    -Atq \
    -c "SELECT pg_walfile_name('$post_target_lsn'::pg_lsn);")
docker exec "$source_container" psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    -c "SELECT pg_switch_wal();" >/dev/null
wait_for_archive "$post_target_segment"

cat > "$base_dir/postgresql.auto.conf" <<EOF
restore_command = 'cp /archive/%f %p'
recovery_target_lsn = '$target_lsn'
recovery_target_action = 'pause'
EOF
touch "$base_dir/recovery.signal"
chmod 0644 "$base_dir/postgresql.auto.conf" "$base_dir/recovery.signal"

recovery_started=$(date +%s)
docker run --detach \
    --name "$target_container" \
    --platform linux/amd64 \
    --env PGDATA=/var/lib/postgresql/18/docker \
    --env POSTGRES_DB=tax_protocol \
    --env POSTGRES_USER=tax_protocol \
    --env POSTGRES_PASSWORD=pitr-drill-only \
    --volume "$base_dir:/var/lib/postgresql/18/docker" \
    --volume "$archive_dir:/archive:ro" \
    "$postgres_image" \
    >/dev/null

wait_for_sql "$target_container"
recovery_elapsed=$(( $(date +%s) - recovery_started ))

in_recovery=$(docker exec "$target_container" psql -U tax_protocol -d tax_protocol -Atq \
    -c "SELECT pg_is_in_recovery();")
target_marker_count=$(docker exec "$target_container" psql -U tax_protocol -d tax_protocol -Atq \
    -c "SELECT count(*) FROM pitr_drill_marker WHERE label = 'target';")
post_target_marker_count=$(docker exec "$target_container" psql -U tax_protocol -d tax_protocol -Atq \
    -c "SELECT count(*) FROM pitr_drill_marker WHERE label = 'post-target';")
replayed_lsn=$(docker exec "$target_container" psql -U tax_protocol -d tax_protocol -Atq \
    -c "SELECT pg_last_wal_replay_lsn();")

if [ "$in_recovery" != "t" ] || [ "$target_marker_count" != "1" ] || [ "$post_target_marker_count" != "0" ]; then
    echo "PITR boundary assertion failed: recovery=$in_recovery target=$target_marker_count post_target=$post_target_marker_count" >&2
    exit 1
fi

archive_files=$(find "$archive_dir" -type f | wc -l | tr -d ' ')
echo "PostgreSQL PITR drill passed: target_lsn=$target_lsn replayed_lsn=$replayed_lsn archive_files=$archive_files local_recovery_seconds=$recovery_elapsed"
