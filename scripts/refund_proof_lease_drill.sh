#!/bin/sh

set -eu

project_name="solana-tax-protocol-refund-proof-lease"
export POSTGRES_PORT="25436"
stream_key="drill/refund-proof-lease/v1"
drill_dir="$(mktemp -d)"

cleanup() {
    docker compose -p "$project_name" down --volumes --remove-orphans >/dev/null 2>&1 || true
    rm -rf "$drill_dir"
}
trap cleanup EXIT INT TERM

psql_exec() {
    docker compose -p "$project_name" exec -T postgres \
        psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol "$@"
}

docker compose -p "$project_name" up --detach postgres
ready="false"
for _attempt in $(seq 1 60); do
    if psql_exec -Atc "SELECT 1" >/dev/null 2>&1; then
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

psql_exec <<SQL
INSERT INTO chain_checkpoints (stream_key, next_slot, finalized_slot, generation)
VALUES ('$stream_key', 2, 1, 1)
ON CONFLICT (stream_key) DO NOTHING;

INSERT INTO indexed_adjustment_refund_executions (
    stream_key, signature, slot, event_index, claim, adjustment,
    original_receipt, merchant, mint, nonce, gross_magnitude, tax_magnitude,
    net_magnitude, refund_funder, refund_destination, approver,
    reference_digest, evidence_digest
)
VALUES
('$stream_key', 'refund-proof-lease-a', 777, 0,
 decode(repeat('01', 32), 'hex'), decode(repeat('02', 32), 'hex'),
 decode(repeat('03', 32), 'hex'), decode(repeat('04', 32), 'hex'),
 decode(repeat('05', 32), 'hex'), 9, 75, 25, 50,
 decode(repeat('06', 32), 'hex'), decode(repeat('07', 32), 'hex'),
 decode(repeat('08', 32), 'hex'), decode(repeat('09', 32), 'hex'),
 decode(repeat('0a', 32), 'hex'))
ON CONFLICT (stream_key, signature, event_index) DO NOTHING;
SQL

claim_sql="WITH candidate AS (
    SELECT execution.stream_key, execution.signature, execution.event_index
      FROM indexed_adjustment_refund_executions execution
      LEFT JOIN refund_balance_proofs proof
        ON proof.stream_key = execution.stream_key
       AND proof.signature = execution.signature
       AND proof.event_index = execution.event_index
     WHERE execution.stream_key = '$stream_key'
       AND proof.id IS NULL
       AND (execution.proof_claim_token IS NULL
            OR execution.proof_claimed_at < now() - interval '60 seconds')
     ORDER BY execution.proof_claimed_at NULLS FIRST,
              execution.slot ASC, execution.signature ASC,
              execution.event_index ASC
     FOR UPDATE OF execution SKIP LOCKED
     LIMIT 1
)
UPDATE indexed_adjustment_refund_executions execution
   SET proof_claim_token = uuidv7(),
       proof_claimed_at = now(),
       proof_claim_attempts = proof_claim_attempts + 1,
       proof_claim_last_error = NULL,
       proof_claim_dead_lettered_at = NULL
  FROM candidate
 WHERE execution.stream_key = candidate.stream_key
   AND execution.signature = candidate.signature
   AND execution.event_index = candidate.event_index
RETURNING execution.signature, execution.proof_claim_token;"

docker compose -p "$project_name" exec -T postgres \
    psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol -Atc "$claim_sql" \
    >"$drill_dir/claim-a.log" 2>&1 &
claim_a_pid=$!
docker compose -p "$project_name" exec -T postgres \
    psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol -Atc "$claim_sql" \
    >"$drill_dir/claim-b.log" 2>&1 &
claim_b_pid=$!
wait "$claim_a_pid"
wait "$claim_b_pid"

claimed_count="$(psql_exec -Atc \
    "SELECT count(*) FROM indexed_adjustment_refund_executions WHERE stream_key = '$stream_key' AND proof_claim_token IS NOT NULL;")"
if [ "$claimed_count" != "1" ]; then
    echo "concurrent claimers did not produce exactly one live lease: $claimed_count" >&2
    cat "$drill_dir/claim-a.log" "$drill_dir/claim-b.log" >&2
    exit 1
fi

claimed_signature="$(psql_exec -Atc \
    "SELECT signature FROM indexed_adjustment_refund_executions WHERE stream_key = '$stream_key' AND proof_claim_token IS NOT NULL;")"
old_token="$(psql_exec -Atc \
    "SELECT proof_claim_token FROM indexed_adjustment_refund_executions WHERE stream_key = '$stream_key' AND signature = '$claimed_signature';")"

psql_exec -c \
    "UPDATE indexed_adjustment_refund_executions SET proof_claimed_at = now() - interval '120 seconds' WHERE stream_key = '$stream_key' AND signature = '$claimed_signature';" \
    >/dev/null
new_claim="$(psql_exec -Atc "$claim_sql")"
new_token="$(printf '%s\n' "$new_claim" | awk -F'|' 'NF > 1 { print $2; exit }')"
if [ -z "$new_token" ] || [ "$new_token" = "$old_token" ]; then
    echo "stale lease was not reclaimed with a new UUIDv7 token" >&2
    exit 1
fi

stale_update_count="$(psql_exec -Atc \
    "WITH updated AS (UPDATE indexed_adjustment_refund_executions SET proof_claim_token = uuidv7(), proof_claimed_at = now() WHERE stream_key = '$stream_key' AND signature = '$claimed_signature' AND proof_claim_token = '$old_token' RETURNING signature) SELECT count(*) FROM updated;")"
if [ "$stale_update_count" != "0" ]; then
    echo "stale worker was able to mutate a reclaimed candidate" >&2
    exit 1
fi

psql_exec <<SQL
INSERT INTO indexed_adjustment_refund_executions (
    stream_key, signature, slot, event_index, claim, adjustment,
    original_receipt, merchant, mint, nonce, gross_magnitude, tax_magnitude,
    net_magnitude, refund_funder, refund_destination, approver,
    reference_digest, evidence_digest
)
VALUES (
    '$stream_key', 'refund-proof-lease-b', 778, 0,
    decode(repeat('11', 32), 'hex'), decode(repeat('12', 32), 'hex'),
    decode(repeat('13', 32), 'hex'), decode(repeat('14', 32), 'hex'),
    decode(repeat('15', 32), 'hex'), 10, 75, 25, 50,
    decode(repeat('16', 32), 'hex'), decode(repeat('17', 32), 'hex'),
    decode(repeat('18', 32), 'hex'), decode(repeat('19', 32), 'hex'),
    decode(repeat('1a', 32), 'hex')
);
SQL

psql_exec <<SQL
INSERT INTO refund_balance_proofs (
    stream_key, signature, event_index, slot, claim, adjustment,
    original_receipt, merchant, mint, token_program, tax_vault, refund_source,
    payer_destination, vault_authority, refund_funder, payer, nonce,
    gross_atomic, tax_atomic, net_atomic, tax_vault_before_atomic,
    tax_vault_after_atomic, refund_source_before_atomic,
    refund_source_after_atomic, payer_destination_before_atomic,
    payer_destination_after_atomic, proof_digest
)
VALUES (
    '$stream_key', 'refund-proof-lease-a', 0, 777,
    decode(repeat('01', 32), 'hex'), decode(repeat('02', 32), 'hex'),
    decode(repeat('03', 32), 'hex'), decode(repeat('04', 32), 'hex'),
    decode(repeat('05', 32), 'hex'), decode(repeat('0b', 32), 'hex'),
    decode(repeat('0c', 32), 'hex'), decode(repeat('0d', 32), 'hex'),
    decode(repeat('07', 32), 'hex'), decode(repeat('0f', 32), 'hex'),
    decode(repeat('06', 32), 'hex'), decode(repeat('10', 32), 'hex'), 9,
    75, 25, 50, 100, 75, 200, 150, 0, 75,
    decode(repeat('1b', 32), 'hex')
);
SQL

pending_signature="$(psql_exec -Atc \
    "SELECT execution.signature FROM indexed_adjustment_refund_executions execution LEFT JOIN refund_balance_proofs proof ON proof.stream_key = execution.stream_key AND proof.signature = execution.signature AND proof.event_index = execution.event_index WHERE execution.stream_key = '$stream_key' AND proof.id IS NULL ORDER BY execution.slot LIMIT 1;")"
if [ "$pending_signature" != "refund-proof-lease-b" ]; then
    echo "proof exclusion selected the wrong pending execution: $pending_signature" >&2
    exit 1
fi

psql_exec -Atc "$claim_sql" >/dev/null
for attempt in 1 2 3; do
    failure_token="$(psql_exec -Atc \
        "SELECT proof_claim_token FROM indexed_adjustment_refund_executions WHERE stream_key = '$stream_key' AND signature = 'refund-proof-lease-b';")"
    failure_count="$(psql_exec -Atc \
        "WITH updated AS (UPDATE indexed_adjustment_refund_executions SET proof_claim_token = NULL, proof_claimed_at = NULL, proof_claim_last_error = 'provider_failure', proof_claim_dead_lettered_at = CASE WHEN proof_claim_attempts >= 3 THEN now() ELSE NULL END WHERE stream_key = '$stream_key' AND signature = 'refund-proof-lease-b' AND proof_claim_token = '$failure_token' RETURNING signature) SELECT count(*) FROM updated;")"
    if [ "$failure_count" != "1" ]; then
        echo "bounded proof failure was not recorded on attempt $attempt" >&2
        exit 1
    fi
    if [ "$attempt" -lt 3 ]; then
        psql_exec -Atc "$claim_sql" >/dev/null
    fi
done

dead_letter_count="$(psql_exec -Atc \
    "SELECT count(*) FROM indexed_adjustment_refund_executions WHERE stream_key = '$stream_key' AND signature = 'refund-proof-lease-b' AND proof_claim_dead_lettered_at IS NOT NULL AND proof_claim_attempts = 3 AND proof_claim_token IS NULL;")"
if [ "$dead_letter_count" != "1" ]; then
    echo "third proof failure did not produce bounded dead-letter state" >&2
    exit 1
fi
automatic_retry_count="$(psql_exec -Atc \
    "SELECT count(*) FROM indexed_adjustment_refund_executions execution LEFT JOIN refund_balance_proofs proof ON proof.stream_key = execution.stream_key AND proof.signature = execution.signature AND proof.event_index = execution.event_index WHERE execution.stream_key = '$stream_key' AND proof.id IS NULL AND execution.proof_claim_dead_lettered_at IS NULL;")"
if [ "$automatic_retry_count" != "0" ]; then
    echo "dead-lettered proof candidate remained eligible for automatic retry" >&2
    exit 1
fi

echo "refund-proof-lease-drill"
echo "concurrent_claimers=one-winner"
echo "stale_reclaim=new-token"
echo "stale_fencing=passed"
echo "proof_exclusion=passed"
echo "bounded_failure=dead-lettered-after-three-attempts"
