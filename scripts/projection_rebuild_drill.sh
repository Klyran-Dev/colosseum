#!/bin/sh

set -eu

project_name="solana-tax-protocol-projection-rebuild"
export POSTGRES_PORT="25434"
stream_key="drill/settlement/v1"

cleanup() {
    docker compose -p "$project_name" down --volumes --remove-orphans >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

docker compose -p "$project_name" up --build --detach postgres
ready="false"
for _attempt in $(seq 1 60); do
    if docker compose -p "$project_name" exec -T postgres \
        pg_isready -U tax_protocol -d tax_protocol >/dev/null 2>&1; then
        ready="true"
        break
    fi
    sleep 1
done
if [ "$ready" != "true" ]; then
    echo "PostgreSQL did not become ready" >&2
    exit 1
fi

docker compose -p "$project_name" run --build --rm migrator

docker compose -p "$project_name" exec -T postgres psql \
    -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol <<'SQL'
INSERT INTO chain_checkpoints (stream_key, next_slot, finalized_slot, generation)
VALUES ('drill/settlement/v1', 43, 42, 1)
ON CONFLICT (stream_key) DO UPDATE
SET next_slot = EXCLUDED.next_slot,
    finalized_slot = EXCLUDED.finalized_slot,
    generation = EXCLUDED.generation;
DELETE FROM indexed_receipts WHERE stream_key = 'drill/settlement/v1';
DELETE FROM indexed_split_settlements WHERE stream_key = 'drill/settlement/v1';
DELETE FROM raw_chain_evidence WHERE stream_key = 'drill/settlement/v1';
INSERT INTO raw_chain_evidence
    (stream_key, signature, slot, event_index, event_discriminator, event_data, evidence_digest)
VALUES (
    'drill/settlement/v1',
    'rebuild-signature',
    42,
    0,
    decode('c6dd9a1cad087206', 'hex'),
    decode(
        'c6dd9a1cad087206' || repeat('01', 32) || repeat('02', 32) ||
        repeat('03', 32) || repeat('04', 32) ||
        '0900000000000000' || '6400000000000000' ||
        '1900000000000000' || '4b00000000000000' || repeat('05', 32),
        'hex'
    ),
    decode('05cef870c29b83a545fd1820e6fcaac28b50cb62e566591f7f75feae1bf6f6c8', 'hex')
);
INSERT INTO raw_chain_evidence
    (stream_key, signature, slot, event_index, event_discriminator, event_data, evidence_digest)
VALUES (
    'drill/settlement/v1',
    'split-rebuild-signature',
    42,
    0,
    decode('3281666df98e7d1e', 'hex'),
    decode(
        '3281666df98e7d1e' || repeat('0b', 32) || repeat('0c', 32) ||
        repeat('0d', 32) || repeat('0e', 32) ||
        '1300000000000000' || '6400000000000000' ||
        '1900000000000000' || '4b00000000000000' ||
        '0200000000000000' || repeat('0f', 32) || repeat('10', 32),
        'hex'
    ),
    decode('d58aa9a5d4bed20228480ab5eb633548c822894c53a4fbc266a02c9f646ab80b', 'hex')
);
SQL

run_rebuild() {
    docker compose -p "$project_name" run --build --rm --no-deps \
        --entrypoint /usr/local/bin/projection-rebuild \
        -e PROJECTION_REBUILD_DATABASE_URL=postgres://tax_protocol:local-development-only@postgres:5432/tax_protocol \
        -e PROJECTION_REBUILD_STREAM_KEY="$stream_key" \
        -e PROJECTION_REBUILD_START_SLOT=42 \
        -e PROJECTION_REBUILD_END_SLOT=42 \
        -e PROJECTION_REBUILD_BATCH_SIZE=1 \
        indexer
}

first_report="$(run_rebuild)"
first_digest="$(printf '%s\n' "$first_report" | awk -F= '$1 == "projection_digest" { print $2 }')"
first_inserted="$(printf '%s\n' "$first_report" | awk -F= '$1 == "inserted_rows" { print $2 }')"
if [ "$first_inserted" != "3" ] || [ -z "$first_digest" ]; then
    echo "initial projection rebuild did not insert both receipts and the split projection" >&2
    printf '%s\n' "$first_report" >&2
    exit 1
fi

checkpoint_before="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select next_slot || ':' || finalized_slot || ':' || generation from chain_checkpoints where stream_key = '$stream_key';")"
docker compose -p "$project_name" exec -T postgres psql \
    -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    -c "delete from indexed_receipts where stream_key = '$stream_key'; delete from indexed_split_settlements where stream_key = '$stream_key';" >/dev/null
second_report="$(run_rebuild)"
second_digest="$(printf '%s\n' "$second_report" | awk -F= '$1 == "projection_digest" { print $2 }')"
second_inserted="$(printf '%s\n' "$second_report" | awk -F= '$1 == "inserted_rows" { print $2 }')"
checkpoint_after="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select next_slot || ':' || finalized_slot || ':' || generation from chain_checkpoints where stream_key = '$stream_key';")"

if [ "$second_inserted" != "3" ] || [ "$first_digest" != "$second_digest" ]; then
    echo "projection rebuild digest was not byte-identical" >&2
    printf '%s\n' "$first_report" "$second_report" >&2
    exit 1
fi
if [ "$checkpoint_before" != "$checkpoint_after" ]; then
    echo "projection rebuild mutated the forward checkpoint" >&2
    exit 1
fi

echo "projection rebuild drill passed: digest=$second_digest checkpoint=$checkpoint_after"
