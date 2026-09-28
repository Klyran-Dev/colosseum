#!/bin/sh

set -eu

project_name="solana-tax-protocol-postgres-contention"
export POSTGRES_PORT="25435"
stream_key="drill/contention/v1"
drill_dir="$(mktemp -d)"
holder_log="$drill_dir/holder.log"

cleanup() {
    if [ -n "${holder_pid:-}" ]; then
        kill "$holder_pid" >/dev/null 2>&1 || true
        wait "$holder_pid" >/dev/null 2>&1 || true
    fi
    docker compose -p "$project_name" down --volumes --remove-orphans >/dev/null 2>&1 || true
    rm -rf "$drill_dir"
}
trap cleanup EXIT INT TERM

docker compose -p "$project_name" up --detach postgres
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
VALUES ('drill/contention/v1', 2, 1, 1)
ON CONFLICT (stream_key) DO UPDATE
SET next_slot = EXCLUDED.next_slot,
    finalized_slot = EXCLUDED.finalized_slot,
    generation = EXCLUDED.generation;
SQL

docker compose -p "$project_name" exec -T postgres psql \
    -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol >"$holder_log" 2>&1 <<'SQL' &
BEGIN;
UPDATE chain_checkpoints
SET generation = generation + 1
WHERE stream_key = 'drill/contention/v1';
SELECT pg_sleep(1);
COMMIT;
SQL
holder_pid=$!
sleep 0.2

set +e
contender_output="$(docker compose -p "$project_name" exec -T postgres psql \
    -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol 2>&1 <<'SQL'
BEGIN;
SET LOCAL lock_timeout = '250ms';
UPDATE chain_checkpoints
SET generation = generation + 10
WHERE stream_key = 'drill/contention/v1';
COMMIT;
SQL
)"
contender_status=$?
set -e

if [ "$contender_status" -eq 0 ] || ! printf '%s\n' "$contender_output" | grep -Fqi "lock timeout"; then
    echo "lock contention did not fail at the bounded lock timeout" >&2
    printf '%s\n' "$contender_output" >&2
    cat "$holder_log" >&2 || true
    exit 1
fi
wait "$holder_pid"
holder_pid=""

generation="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select generation from chain_checkpoints where stream_key = '$stream_key';")"
if [ "$generation" != "2" ]; then
    echo "lock-holder commit or failed-contender rollback was not preserved" >&2
    exit 1
fi

set +e
statement_output="$(docker compose -p "$project_name" exec -T postgres psql \
    -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol 2>&1 <<'SQL'
BEGIN;
SET LOCAL statement_timeout = '900ms';
SELECT pg_sleep(2);
COMMIT;
SQL
)"
statement_status=$?
set -e
if [ "$statement_status" -eq 0 ] || ! printf '%s\n' "$statement_output" | grep -Fqi "statement timeout"; then
    echo "long statement did not fail at the bounded statement timeout" >&2
    printf '%s\n' "$statement_output" >&2
    exit 1
fi

required_indexes="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from pg_indexes where indexname in ('chain_checkpoints_pkey', 'indexed_receipts_pkey', 'indexed_receipts_slot_idx', 'raw_chain_evidence_pkey', 'raw_chain_evidence_slot_idx', 'indexer_replay_gaps_claim_idx', 'indexer_replay_gaps_dead_letter_idx', 'reconciliation_runs_stream_idx');")"
if [ "$required_indexes" != "8" ]; then
    echo "required evidence and replay indexes are incomplete: $required_indexes" >&2
    exit 1
fi

fence_merchant_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d21"
fence_intent_id="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "insert into settlement_intents (merchant_id, idempotency_key, request_digest, instruction_digest, status) values ('$fence_merchant_id', 'contention-fence-01', decode(repeat('11', 32), 'hex'), decode(repeat('22', 32), 'hex'), 'signed') returning id;")"
fence_intent_id="$(printf '%s\n' "$fence_intent_id" | sed -n '1p')"
if [ -z "$fence_intent_id" ]; then
    echo "failed to create a signed settlement intent for the submission-fence drill" >&2
    exit 1
fi
fence_signature="$(printf '%*s' 88 '' | tr ' ' '5')"
fence_first_log="$drill_dir/fence-first.log"
fence_second_log="$drill_dir/fence-second.log"
docker compose -p "$project_name" exec -T postgres psql \
    -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol >"$fence_first_log" 2>&1 <<SQL &
BEGIN;
UPDATE settlement_intents
SET status = 'submitting', signature = '$fence_signature'
WHERE id = '$fence_intent_id' AND merchant_id = '$fence_merchant_id' AND status = 'signed';
COMMIT;
SQL
fence_first_pid=$!
docker compose -p "$project_name" exec -T postgres psql \
    -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol >"$fence_second_log" 2>&1 <<SQL &
BEGIN;
UPDATE settlement_intents
SET status = 'submitting', signature = '$fence_signature'
WHERE id = '$fence_intent_id' AND merchant_id = '$fence_merchant_id' AND status = 'signed';
COMMIT;
SQL
fence_second_pid=$!
wait "$fence_first_pid"
wait "$fence_second_pid"
fence_claim_count="$(grep -hE '^UPDATE 1$' "$fence_first_log" "$fence_second_log" | wc -l | tr -d ' ')"
fence_loser_count="$(grep -hE '^UPDATE 0$' "$fence_first_log" "$fence_second_log" | wc -l | tr -d ' ')"
if [ "$fence_claim_count" != "1" ] || [ "$fence_loser_count" != "1" ]; then
    echo "durable settlement submission fence did not produce exactly one winner and one loser" >&2
    cat "$fence_first_log" "$fence_second_log" >&2
    exit 1
fi
fence_state="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select status || '|' || length(signature)::text from settlement_intents where id = '$fence_intent_id' and merchant_id = '$fence_merchant_id';")"
if [ "$fence_state" != "submitting|88" ]; then
    echo "durable settlement submission fence persisted an invalid state: $fence_state" >&2
    exit 1
fi

echo "PostgreSQL contention drill passed: lock_timeout=250ms statement_timeout=900ms generation=$generation indexes=$required_indexes submission_fence=1-winner-1-loser"
