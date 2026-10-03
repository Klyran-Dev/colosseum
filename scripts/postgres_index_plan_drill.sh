#!/bin/sh

set -eu

# Bounded local planner evidence. This is not a production capacity or
# retention claim, and it never deletes evidence rows.
project_name="${COMPOSE_PROJECT_NAME:-solana-tax-protocol-index-plan}"
export POSTGRES_PORT="${POSTGRES_PORT:-25438}"
stream_key="drill/index-plan/v1"
merchant_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d31"
calculation_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d32"
intent_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d33"
drill_dir="$(mktemp -d)"

cleanup() {
    docker compose -p "$project_name" down --volumes --remove-orphans >/dev/null 2>&1 || true
    rm -rf "$drill_dir"
}
trap cleanup EXIT INT TERM

docker compose -p "$project_name" up --detach postgres >/dev/null
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

# The planner assertions must run against the same migrated schema as every
# other PostgreSQL drill; a bare server is not a valid evidence boundary.
docker compose -p "$project_name" run --build --rm migrator >/dev/null

docker compose -p "$project_name" exec -T postgres psql \
    -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol <<SQL
INSERT INTO chain_checkpoints (stream_key, next_slot, finalized_slot, generation)
VALUES ('$stream_key', 25001, 25000, 1);

INSERT INTO indexed_receipts
    (stream_key, signature, slot, gross_atomic, tax_atomic, net_atomic,
     evidence_digest, receipt, payer, merchant, mint, nonce, policy_digest)
SELECT '$stream_key', format('receipt-%s', ordinal), ordinal, 1100, 100, 1000,
       decode(repeat('ab', 32), 'hex'),
       decode(repeat('01', 32), 'hex'),
       decode(repeat('02', 32), 'hex'),
       decode(repeat('03', 32), 'hex'),
       decode(repeat('04', 32), 'hex'),
       ordinal,
       decode(repeat('05', 32), 'hex')
FROM generate_series(1, 25000) AS rows(ordinal);

INSERT INTO raw_chain_evidence
    (stream_key, signature, slot, event_index, event_discriminator, event_data, evidence_digest)
SELECT '$stream_key', format('receipt-%s', ordinal), ordinal, 0,
       decode('0102030405060708', 'hex'), decode('0102030405060708', 'hex'),
       decode(repeat('cd', 32), 'hex')
FROM generate_series(1, 25000) AS rows(ordinal);

INSERT INTO raw_chain_transactions
    (stream_key, signature, slot, transaction_index, payload, payload_digest)
SELECT '$stream_key', format('receipt-%s', ordinal), ordinal, 0,
       decode(repeat('ef', 32), 'hex'), decode(repeat('12', 32), 'hex')
FROM generate_series(1, 25000) AS rows(ordinal);

INSERT INTO settlement_intents
    (merchant_id, idempotency_key, request_digest, instruction_digest)
SELECT '$merchant_id', format('index-plan-intent-%s', ordinal),
       decode(repeat('01', 32), 'hex'), decode(repeat('02', 32), 'hex')
FROM generate_series(1, 24999) AS rows(ordinal);

INSERT INTO settlement_intents
    (id, merchant_id, idempotency_key, request_digest, instruction_digest)
VALUES ('$intent_id', '$merchant_id', 'index-plan-intent',
        decode(repeat('01', 32), 'hex'), decode(repeat('02', 32), 'hex'));

INSERT INTO settlement_intent_events
    (intent_id, from_status, to_status, evidence)
SELECT '$intent_id', 'submitted', 'unknown', jsonb_build_object('ordinal', ordinal)
FROM generate_series(1, 10000) AS rows(ordinal);

INSERT INTO evidence_access_audit_events
    (merchant_id, calculation_id, actor_subject, request_id, purpose, outcome, facts_digest)
SELECT '$merchant_id', '$calculation_id', 'index-plan-operator', uuidv7(),
       'replay', 'allowed', decode(repeat('34', 32), 'hex')
FROM generate_series(1, 25000);

ANALYZE indexed_receipts;
ANALYZE raw_chain_evidence;
ANALYZE raw_chain_transactions;
ANALYZE settlement_intents;
ANALYZE settlement_intent_events;
ANALYZE evidence_access_audit_events;
SQL

explain_query() {
    name="$1"
    sql="$2"
    docker compose -p "$project_name" exec -T postgres psql \
        -U tax_protocol -d tax_protocol -P pager=off -c \
        "EXPLAIN (ANALYZE, BUFFERS, COSTS OFF) $sql" >"$drill_dir/$name.txt"
    cat "$drill_dir/$name.txt"
}

explain_query receipts \
    "SELECT signature FROM indexed_receipts WHERE stream_key = '$stream_key' AND slot BETWEEN 24990 AND 25000 ORDER BY slot, signature"
explain_query raw_events \
    "SELECT signature, event_index FROM raw_chain_evidence WHERE stream_key = '$stream_key' AND slot BETWEEN 24990 AND 25000 ORDER BY slot, signature, event_index"
explain_query raw_transactions \
    "SELECT signature, transaction_index FROM raw_chain_transactions WHERE stream_key = '$stream_key' AND slot BETWEEN 24990 AND 25000 ORDER BY slot, signature, transaction_index"
explain_query intents \
    "SELECT id FROM settlement_intents WHERE merchant_id = '$merchant_id' AND idempotency_key = 'index-plan-intent'"
explain_query intent_events \
    "SELECT id FROM settlement_intent_events WHERE intent_id = '$intent_id' ORDER BY occurred_at, id LIMIT 50"
explain_query audit \
    "SELECT id FROM evidence_access_audit_events WHERE merchant_id = '$merchant_id' AND calculation_id = '$calculation_id' ORDER BY occurred_at DESC, id DESC LIMIT 50"

assert_plan() {
    name="$1"
    index_name="$2"
    if ! grep -Fq "$index_name" "$drill_dir/$name.txt"; then
        echo "planner did not select $index_name for $name" >&2
        cat "$drill_dir/$name.txt" >&2
        exit 1
    fi
}

assert_plan receipts indexed_receipts_slot_idx
assert_plan raw_events raw_chain_evidence_slot_idx
assert_plan raw_transactions raw_chain_transactions_slot_idx
assert_plan intents settlement_intents_merchant_key_idx
assert_plan intent_events settlement_intent_events_intent_time_idx
assert_plan audit evidence_access_audit_lookup_idx

echo "PostgreSQL index-plan drill passed: rows=25000 append-only evidence families; retention policy unchanged"
