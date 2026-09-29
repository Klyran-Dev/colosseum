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

refund_merchant_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d31"
refund_adjustment_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d32"
refund_configuration_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d33"
refund_settlement_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d34"
refund_calculation_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d35"
refund_token_program="TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
refund_proposer="11157t3sqMV725NVRLrVQbAu98Jjfk1uCKehJnXXQs"
refund_funder="1117mWrzzrZr312ebPDHu8tbfMwFNvCvMbr6WepCNG"
refund_merchant="1119DWteoLSdjvrT6g6L8C2PfDD2faiTQUpsjY2RiF"
refund_mint="111BuZ6b86gm7XhxjvTakhRvxSMjXp2GqgifkNUmDK"
refund_tax_vault="111A4R8Vf7xP4Rz7ZkYd8H7f4Q5J4h4Y4V4w4U4T4S"
refund_net_source="111B4R8Vf7xP4Rz7ZkYd8H7f4P5J4h4Y4V4w4U4T4S"
refund_destination="111C4R8Vf7xP4Rz7ZkYd8H7f4Q5J4h4Y4V4w4U4T4S"
refund_claim_key="contention-refund-claim-01"
refund_claim_digest="33"
refund_signature="$(printf '%*s' 88 '' | tr ' ' '6')"

docker compose -p "$project_name" exec -T postgres psql \
    -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol <<SQL
INSERT INTO settlement_adjustments (
    id, merchant_id, idempotency_key, request_digest, adjustment_digest,
    original_settlement_id, asset_id, kind, gross_delta, tax_delta, net_delta,
    reason_code, evidence_digest, source_calculation_id, status, proposer_subject
) VALUES (
    '$refund_adjustment_id', '$refund_merchant_id', 'contention-refund-adjustment-01',
    decode(repeat('44', 32), 'hex'), decode(repeat('45', 32), 'hex'),
    '$refund_settlement_id', decode(repeat('46', 32), 'hex'), 'refund',
    -100, -25, -75, 'contention-drill', decode(repeat('47', 32), 'hex'),
    '$refund_calculation_id', 'approved', 'refund-drill-proposer'
)
ON CONFLICT (id, merchant_id) DO NOTHING;

INSERT INTO refund_funding_configurations (
    id, merchant_id, idempotency_key, request_digest, configuration_digest,
    configuration_key, version, status, merchant_account, mint, tax_vault,
    vault_authority, refund_funder, net_source, token_program, proposer_subject,
    approver_subject, verification_digest, effective_from, refund_funder_subject,
    verification_slot, verification_commitment, verification_evidence,
    verifier_version, verified_at
) VALUES (
    '$refund_configuration_id', '$refund_merchant_id', 'contention-refund-config-01',
    decode(repeat('48', 32), 'hex'), decode(repeat('49', 32), 'hex'),
    'contention-refund-config', 1, 'active', '$refund_merchant', '$refund_mint',
    '$refund_tax_vault', '$refund_proposer', '$refund_funder', '$refund_net_source',
    '$refund_token_program', 'refund-drill-proposer', 'refund-drill-approver',
    decode(repeat('4a', 32), 'hex'), now(), 'refund-drill-funder-subject',
    100, 'finalized', jsonb_build_object('drill', 'postgres-contention'),
    'refund-drill-verifier-1', now()
)
ON CONFLICT (id, merchant_id) DO NOTHING;

CREATE TABLE refund_nonce_drill_allocations (
    worker_id integer PRIMARY KEY,
    allocated_nonce numeric(20, 0) NOT NULL
);
SQL

nonce_dir="$drill_dir/refund-nonces"
mkdir -p "$nonce_dir"
nonce_worker() {
    worker_id="$1"
    worker_log="$nonce_dir/worker-$worker_id.log"
    worker_attempt=1
    sleep "0.0$worker_id"
    while [ "$worker_attempt" -le 3 ]; do
        set +e
        nonce_output="$(docker compose -p "$project_name" exec -T postgres psql \
            -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol -At 2>"$worker_log" <<SQL
BEGIN;
SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;
WITH allocation AS (
    INSERT INTO refund_claim_nonce_counters (merchant_id, adjustment_id, next_nonce)
    VALUES ('$refund_merchant_id', '$refund_adjustment_id', 2)
    ON CONFLICT (merchant_id, adjustment_id) DO UPDATE
    SET next_nonce = refund_claim_nonce_counters.next_nonce + 1,
        updated_at = now()
    WHERE refund_claim_nonce_counters.next_nonce < 18446744073709551615
    RETURNING (next_nonce - 1) AS allocated_nonce
)
INSERT INTO refund_nonce_drill_allocations (worker_id, allocated_nonce)
SELECT $worker_id, allocated_nonce FROM allocation
RETURNING allocated_nonce;
COMMIT;
SQL
        )"
        nonce_status=$?
        set -e
        if [ "$nonce_status" -eq 0 ] && [ -n "$nonce_output" ]; then
            printf '%s\n' "$nonce_output" | tail -n 1 >"$worker_log"
            return 0
        fi
        worker_attempt=$((worker_attempt + 1))
        sleep "0.0$((worker_attempt + worker_id % 3))"
    done
    echo "nonce worker $worker_id exhausted bounded serialization retries" >&2
    cat "$worker_log" >&2 || true
    return 1
}
nonce_pids=""
for worker_id in $(seq 1 16); do
    nonce_worker "$worker_id" &
    nonce_pids="$nonce_pids $!"
done
for worker_pid in $nonce_pids; do
    wait "$worker_pid"
done
nonce_unique_count="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select count(distinct allocated_nonce) from refund_nonce_drill_allocations;")"
nonce_total_count="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select coalesce(sum(allocated_nonce), 0) from refund_nonce_drill_allocations;")"
nonce_allocation_count="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from refund_nonce_drill_allocations;")"
nonce_counter="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select next_nonce from refund_claim_nonce_counters where merchant_id = '$refund_merchant_id' and adjustment_id = '$refund_adjustment_id';")"
if [ "$nonce_allocation_count" != "16" ] || [ "$nonce_unique_count" != "16" ] || [ "$nonce_total_count" != "136" ] || [ "$nonce_counter" != "17" ]; then
    echo "refund nonce counter did not preserve 16 unique allocations: allocations=$nonce_allocation_count unique=$nonce_unique_count sum=$nonce_total_count next=$nonce_counter" >&2
    exit 1
fi

claim_dir="$drill_dir/refund-claims"
mkdir -p "$claim_dir"

claim_worker() {
    worker_id="$1"
    worker_log="$claim_dir/worker-$worker_id.log"
    worker_attempt=1
    sleep "0.0$worker_id"
    while [ "$worker_attempt" -le 3 ]; do
        set +e
        claim_output="$(docker compose -p "$project_name" exec -T postgres psql \
            -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol -At 2>"$worker_log" <<SQL
BEGIN;
SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;
DO \$refund\$
DECLARE
    inserted_id uuid;
BEGIN
    INSERT INTO refund_claim_intents (
        merchant_id, idempotency_key, request_digest, adjustment_id,
        funding_configuration_id, funding_configuration_digest, receipt_signature,
        receipt_slot, original_receipt, payer, merchant, mint, token_program,
        proposer, refund_funder, refund_funder_subject, tax_vault, net_source,
        refund_destination, claim_nonce, gross_atomic, tax_atomic, net_atomic,
        reference_digest, execute_after_unix, expires_at_unix, claim,
        instruction_digest, artifact_digest, unsigned_artifact
    ) VALUES (
        '$refund_merchant_id', '$refund_claim_key',
        decode(repeat('$refund_claim_digest', 32), 'hex'),
        '$refund_adjustment_id', '$refund_configuration_id',
        decode(repeat('51', 32), 'hex'), '$refund_signature', 900,
        decode(repeat('52', 32), 'hex'), decode(repeat('53', 32), 'hex'),
        decode(repeat('54', 32), 'hex'), decode(repeat('55', 32), 'hex'),
        '$refund_token_program', '$refund_proposer', '$refund_funder',
        'refund-drill-funder-subject', '$refund_tax_vault', '$refund_net_source',
        '$refund_destination', 1, 100, 25, 75, decode(repeat('56', 32), 'hex'),
        1900000000, 1900003600, decode(repeat('57', 32), 'hex'),
        decode(repeat('58', 32), 'hex'), decode(repeat('59', 32), 'hex'),
        jsonb_build_object('drill', 'postgres-contention')
    )
    ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
    RETURNING id INTO inserted_id;

    IF inserted_id IS NOT NULL THEN
        INSERT INTO refund_claim_intent_events (
            intent_id, merchant_id, from_status, to_status, evidence
        ) VALUES (
            inserted_id, '$refund_merchant_id', 'none',
            'prepared_unsubmitted',
            jsonb_build_object('source', 'postgres-contention-drill')
        );
        INSERT INTO outbox_events (
            aggregate_type, aggregate_id, event_type, payload
        ) VALUES (
            'refund_claim_intent', inserted_id, 'refund.claim.prepared',
            jsonb_build_object('merchant_id', '$refund_merchant_id', 'source', 'postgres-contention-drill')
        );
    END IF;
END
\$refund\$;
SELECT 'ok';
COMMIT;
SQL
        )"
        claim_status=$?
        set -e
        if [ "$claim_status" -eq 0 ] && printf '%s\n' "$claim_output" | grep -Fxq "ok"; then
            printf '%s\n' "ok" >"$worker_log"
            return 0
        fi
        worker_attempt=$((worker_attempt + 1))
        sleep "0.0$((worker_attempt + worker_id % 3))"
    done
    echo "claim worker $worker_id exhausted bounded serialization retries" >&2
    printf '%s\n' "$claim_output" >&2
    cat "$worker_log" >&2 || true
    return 1
}
claim_pids=""
for worker_id in $(seq 1 16); do
    claim_worker "$worker_id" &
    claim_pids="$claim_pids $!"
done
for worker_pid in $claim_pids; do
    wait "$worker_pid"
done
claim_count="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from refund_claim_intents where merchant_id = '$refund_merchant_id' and idempotency_key = '$refund_claim_key';")"
claim_event_count="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from refund_claim_intent_events e join refund_claim_intents i on i.id = e.intent_id and i.merchant_id = e.merchant_id where i.merchant_id = '$refund_merchant_id' and i.idempotency_key = '$refund_claim_key';")"
claim_outbox_count="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from outbox_events where aggregate_type = 'refund_claim_intent' and payload->>'merchant_id' = '$refund_merchant_id' and event_type = 'refund.claim.prepared';")"
if [ "$claim_count" != "1" ] || [ "$claim_event_count" != "1" ] || [ "$claim_outbox_count" != "1" ]; then
    echo "refund claim idempotency cardinality failed: intents=$claim_count events=$claim_event_count outbox=$claim_outbox_count" >&2
    exit 1
fi

echo "PostgreSQL contention drill passed: lock_timeout=250ms statement_timeout=900ms generation=$generation indexes=$required_indexes submission_fence=1-winner-1-loser refund_nonce=16-unique-sum-136 refund_claim=1-intent-1-event-1-outbox"
