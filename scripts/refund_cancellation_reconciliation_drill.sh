#!/bin/sh

set -eu

# Bounded local evidence harness for the automatic finalized cancellation
# reconciler. It uses the real indexer, PostgreSQL migrations, targeted
# getSignaturesForAddress/getTransaction recovery, lease fencing, and atomic
# cancellation transition. All identities and evidence are synthetic.
project_name="solana-tax-protocol-refund-cancellation-reconciliation"
stream_key="drill/refund-cancellation/v1"
export COMPOSE_PROJECT_NAME="$project_name"
export POSTGRES_PORT="25448"
export RPC_FIXTURE_PORT="28894"
export INDEXER_HEALTH_PORT="18094"
export INDEXER_ENV="local"
export INDEXER_ALLOW_INSECURE_LOCAL_RPC="true"
export INDEXER_RPC_ENDPOINTS="http://rpc-fixture:8899"
export INDEXER_STREAM_KEY="$stream_key"
export INDEXER_START_SLOT="1"
export INDEXER_START_FINALIZED_SLOT="1"
export INDEXER_PAGE_SLOTS="16"
export INDEXER_POLL_MS="5000"
export INDEXER_RPC_TIMEOUT_MS="500"
export INDEXER_REPLAY_LEASE_MS="30000"
export INDEXER_VAULT_RECONCILIATION_POLL_MS="5000"
export RPC_FIXTURE_SIGNATURE_STATUS="finalized"

merchant_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d51"
adjustment_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d52"
funding_configuration_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d53"
claim_intent_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d54"
cancellation_intent_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d55"
original_settlement_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d56"
source_calculation_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d57"

drill_dir="$(mktemp -d)"
evidence_dir="${REFUND_CANCELLATION_EVIDENCE_DIR:-test-results/refund-cancellation-reconciliation}"
mkdir -p "$evidence_dir"
replica_one=""
replica_two=""

cleanup() {
    exit_status=$?
    if [ -n "$replica_one" ]; then
        docker rm --force "$replica_one" >/dev/null 2>&1 || true
    fi
    if [ -n "$replica_two" ]; then
        docker rm --force "$replica_two" >/dev/null 2>&1 || true
    fi
    if [ "$exit_status" -ne 0 ]; then
        docker compose -p "$project_name" logs --no-color indexer rpc-fixture postgres >&2 || true
    fi
    docker compose -p "$project_name" down --volumes --remove-orphans >/dev/null 2>&1 || true
    rm -rf "$drill_dir"
    exit "$exit_status"
}
trap cleanup EXIT INT TERM

command -v docker >/dev/null 2>&1 || { echo "Docker is required" >&2; exit 1; }
command -v jq >/dev/null 2>&1 || { echo "jq is required" >&2; exit 1; }
docker info >/dev/null 2>&1 || {
    echo "A reachable Docker daemon is required for the reconciliation drill" >&2
    exit 1
}

pnpm --dir apps/web exec node scripts/refund_cancellation_fixture.mjs >"$drill_dir/fixture.json"
fixture_value() {
    jq -er "$1" "$drill_dir/fixture.json"
}

export RPC_FIXTURE_REFUND_CANCELLATION_CLAIM="$(fixture_value '.claim')"
export RPC_FIXTURE_REFUND_CANCELLATION_ADJUSTMENT="$(fixture_value '.adjustment')"
export RPC_FIXTURE_REFUND_CANCELLATION_MERCHANT="$(fixture_value '.merchant')"
export RPC_FIXTURE_REFUND_CANCELLATION_MINT="$(fixture_value '.mint')"
export RPC_FIXTURE_REFUND_CANCELLATION_GOVERNANCE="$(fixture_value '.governance')"
export RPC_FIXTURE_REFUND_CANCELLATION_SIGNATURE="$(fixture_value '.cancellation_signature')"
export RPC_FIXTURE_REFUND_CANCELLATION_REFERENCE_DIGEST_HEX="$(fixture_value '.reference_digest_hex')"
export RPC_FIXTURE_REFUND_CANCELLATION_NONCE="$(fixture_value '.artifact.instruction.claim_nonce')"
export RPC_FIXTURE_REFUND_CANCELLATION_SLOT="$(fixture_value '.cancellation_slot')"

docker compose -p "$project_name" --profile settlement-submit-drill --profile indexer \
    up --build --detach postgres rpc-fixture >/dev/null

wait_for_rpc() {
    for _attempt in $(seq 1 90); do
        if curl --fail --silent "http://127.0.0.1:${RPC_FIXTURE_PORT}/healthz" >/dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    echo "refund-cancellation RPC fixture did not become reachable" >&2
    return 1
}

wait_for_replica() {
    replica_id="$1"
    for _attempt in $(seq 1 90); do
        if docker exec "$replica_id" wget --spider --quiet http://127.0.0.1:8082/readyz >/dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    echo "refund-cancellation indexer replica did not become ready: $replica_id" >&2
    return 1
}

wait_for_rpc
postgres_ready="false"
for _attempt in $(seq 1 90); do
    if docker compose -p "$project_name" exec -T postgres \
        pg_isready -U tax_protocol -d tax_protocol >/dev/null 2>&1; then
        postgres_ready="true"
        break
    fi
    sleep 1
done
if [ "$postgres_ready" != "true" ]; then
    echo "refund-cancellation PostgreSQL did not become ready" >&2
    exit 1
fi
docker compose -p "$project_name" --profile indexer run --rm migrator >/dev/null

claim_hex="$(fixture_value '.claim_hex')"
adjustment_hex="$(fixture_value '.adjustment_hex')"
original_receipt_hex="$(fixture_value '.original_receipt_hex')"
merchant_hex="$(fixture_value '.merchant_hex')"
mint_hex="$(fixture_value '.mint_hex')"
payer_hex="$(fixture_value '.payer_hex')"
refund_funder_hex="$(fixture_value '.refund_funder_hex')"
tax_vault_hex="$(fixture_value '.tax_vault_hex')"
net_source_hex="$(fixture_value '.net_source_hex')"
refund_destination_hex="$(fixture_value '.refund_destination_hex')"
proposer_hex="$(fixture_value '.proposer_hex')"
reference_digest_hex="$(fixture_value '.reference_digest_hex')"
instruction_digest_hex="$(fixture_value '.instruction_digest_hex')"
event_hex="$(fixture_value '.cancellation_event_hex')"
evidence_digest_hex="$(fixture_value '.cancellation_evidence_digest_hex')"
merchant="$(fixture_value '.merchant')"
mint="$(fixture_value '.mint')"
refund_funder="$(fixture_value '.refund_funder')"
tax_vault="$(fixture_value '.tax_vault')"
net_source="$(fixture_value '.net_source')"
refund_destination="$(fixture_value '.refund_destination')"
proposer="$(fixture_value '.proposer')"
vault_authority="$(fixture_value '.vault_authority')"
governance="$(fixture_value '.governance')"
token_program="$(fixture_value '.token_program')"
cancellation_signature="$(fixture_value '.cancellation_signature')"
claim_artifact_json="$(jq -c '.artifact' "$drill_dir/fixture.json")"

psql_args="-v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol"
docker compose -p "$project_name" exec -T postgres psql $psql_args \
    -v claim_artifact_json="$claim_artifact_json" <<SQL
INSERT INTO chain_checkpoints (stream_key, next_slot, finalized_slot, generation)
VALUES ('$stream_key', 1, 1, 0)
ON CONFLICT (stream_key) DO UPDATE
SET next_slot = EXCLUDED.next_slot,
    finalized_slot = EXCLUDED.finalized_slot,
    generation = EXCLUDED.generation;

INSERT INTO settlement_adjustments (
    id, merchant_id, idempotency_key, request_digest, adjustment_digest,
    original_settlement_id, asset_id, kind, gross_delta, tax_delta, net_delta,
    reason_code, evidence_digest, source_calculation_id, status,
    proposer_subject, approver_subject
) VALUES (
    '$adjustment_id', '$merchant_id', 'refund-cancellation-drill-adjustment', decode(repeat('31', 32), 'hex'),
    decode(repeat('32', 32), 'hex'), '$original_settlement_id', decode('$mint_hex', 'hex'),
    'refund', -100, -25, -75, 'refund-cancellation-reconciliation-drill', decode(repeat('33', 32), 'hex'),
    '$source_calculation_id', 'approved', 'refund-cancellation-proposer', 'refund-cancellation-approver'
);

INSERT INTO refund_funding_configurations (
    id, merchant_id, idempotency_key, request_digest, configuration_digest,
    configuration_key, version, status, merchant_account, mint, tax_vault,
    vault_authority, refund_funder, net_source, token_program,
    proposer_subject, approver_subject, verification_digest, effective_from,
    refund_funder_subject, verification_slot, verification_commitment,
    verification_evidence, verifier_version, verified_at
) VALUES (
    '$funding_configuration_id', '$merchant_id', 'refund-cancellation-drill-funding', decode(repeat('34', 32), 'hex'),
    decode(repeat('35', 32), 'hex'), 'refund-cancellation-drill-funding', 1, 'active', '$merchant', '$mint',
    '$tax_vault', '$vault_authority', '$refund_funder', '$net_source', '$token_program',
    'refund-cancellation-proposer', 'refund-cancellation-approver', decode(repeat('36', 32), 'hex'),
    now() - interval '1 minute', 'refund-cancellation-funder', 899, 'finalized',
    jsonb_build_object('source', 'adr-0238-local-fixture'), 'adr-0238', now()
);

INSERT INTO refund_claim_intents (
    id, merchant_id, idempotency_key, request_digest, adjustment_id,
    funding_configuration_id, funding_configuration_digest, receipt_signature,
    receipt_slot, original_receipt, payer, merchant, mint, token_program,
    proposer, refund_funder, refund_funder_subject, tax_vault, net_source,
    refund_destination, claim_nonce, gross_atomic, tax_atomic, net_atomic,
    reference_digest, execute_after_unix, expires_at_unix, claim,
    instruction_digest, artifact_digest, unsigned_artifact, status, signature,
    submitted_slot, commitment
) VALUES (
    '$claim_intent_id', '$merchant_id', 'refund-cancellation-drill-claim', decode(repeat('37', 32), 'hex'),
    '$adjustment_id', '$funding_configuration_id', decode(repeat('35', 32), 'hex'), 'refund-cancellation-receipt',
    700, decode('$original_receipt_hex', 'hex'), decode('$payer_hex', 'hex'), decode('$merchant_hex', 'hex'),
    decode('$mint_hex', 'hex'), '$token_program', '$proposer', '$refund_funder',
    'refund-cancellation-funder', '$tax_vault', '$net_source', '$refund_destination',
    11, 100, 25, 75, decode('$reference_digest_hex', 'hex'), 1, 9999999999,
    decode('$claim_hex', 'hex'), decode('$instruction_digest_hex', 'hex'), decode(repeat('39', 32), 'hex'),
    :'claim_artifact_json'::jsonb, 'finalized', 'refund-cancellation-claim', 800, 'finalized'
);

INSERT INTO refund_claim_cancellation_intents (
    id, merchant_id, idempotency_key, request_digest, claim_intent_id,
    adjustment_id, merchant, mint, adjustment, claim_nonce, claim,
    governance_authority, governance_subject, instruction_digest, artifact_digest,
    unsigned_artifact, status, signature, submitted_slot, commitment,
    reconciliation_claim_token, reconciliation_claimed_at
) VALUES (
    '$cancellation_intent_id', '$merchant_id', 'refund-cancellation-drill-cancel', decode(repeat('40', 32), 'hex'),
    '$claim_intent_id', '$adjustment_id', decode('$merchant_hex', 'hex'), decode('$mint_hex', 'hex'),
    decode('$adjustment_hex', 'hex'), 11, decode('$claim_hex', 'hex'), '$governance',
    'refund-cancellation-governance', decode(repeat('41', 32), 'hex'), decode(repeat('42', 32), 'hex'),
    '{}'::jsonb, 'submitted', '$cancellation_signature', 901, 'confirmed',
    uuidv7(), now() - interval '120 seconds'
);
SQL

docker compose -p "$project_name" --profile indexer --profile settlement-submit-drill build indexer >/dev/null
replica_one=$(docker compose -p "$project_name" --profile indexer --profile settlement-submit-drill \
    run --detach --no-deps indexer)
replica_two=$(docker compose -p "$project_name" --profile indexer --profile settlement-submit-drill \
    run --detach --no-deps indexer)
wait_for_replica "$replica_one"
wait_for_replica "$replica_two"

converged="false"
state=""
for _attempt in $(seq 1 90); do
    state=$(docker compose -p "$project_name" exec -T postgres psql -qAt $psql_args -c \
        "SELECT c.status || '|' || r.status || '|' ||
                (SELECT count(*) FROM refund_claim_cancellation_intent_events WHERE intent_id = '$cancellation_intent_id' AND to_status = 'finalized') || '|' ||
                (SELECT count(*) FROM refund_claim_intent_events WHERE intent_id = '$claim_intent_id' AND to_status = 'cancelled') || '|' ||
                (SELECT count(*) FROM outbox_events WHERE aggregate_id IN ('$cancellation_intent_id', '$claim_intent_id') AND event_type IN ('tax.refund_claim_cancellation_intent.finalized', 'tax.refund_claim_intent.cancelled')) || '|' ||
                (c.reconciliation_claim_token IS NULL)::text
         FROM refund_claim_cancellation_intents c
         JOIN refund_claim_intents r ON r.id = c.claim_intent_id
         WHERE c.id = '$cancellation_intent_id';" | sed -n '1p')
    if [ "$state" = "finalized|cancelled|1|1|2|true" ]; then
        converged="true"
        break
    fi
    sleep 1
done
if [ "$converged" != "true" ]; then
    echo "refund cancellation reconciliation did not converge: ${state:-missing}" >&2
    exit 1
fi

evidence_state=$(docker compose -p "$project_name" exec -T postgres psql -qAt $psql_args -c \
    "SELECT count(*) || '|' || min(slot::text) || '|' || min(event_index::text) || '|' ||
            (SELECT count(*) FROM raw_chain_transactions WHERE stream_key = '$stream_key' AND signature = '$cancellation_signature') || '|' ||
            (SELECT encode(evidence_digest, 'hex') FROM raw_chain_evidence WHERE stream_key = '$stream_key' AND signature = '$cancellation_signature')
     FROM raw_chain_evidence
     WHERE stream_key = '$stream_key' AND signature = '$cancellation_signature';")
if [ "$evidence_state" != "1|901|0|1|$evidence_digest_hex" ]; then
    echo "finalized cancellation evidence cardinality or digest was unexpected: $evidence_state" >&2
    exit 1
fi

metrics_file="$evidence_dir/metrics.txt"
docker exec "$replica_one" wget -qO- http://127.0.0.1:8082/metrics >"$metrics_file"
if ! grep -q 'stp_indexer_refund_claim_cancellations_pending{service="indexer"} 0' "$metrics_file"; then
    echo "cancellation backlog metric did not converge to zero" >&2
    exit 1
fi

printf '{"stream_key":"%s","status":"finalized|cancelled","cancellation_event_count":1,"claim_cancelled_event_count":1,"outbox_event_count":2,"raw_transaction_count":1,"raw_evidence_count":1,"slot":901,"replicas":2,"evidence_digest":"%s"}\n' \
    "$stream_key" "$evidence_digest_hex" >"$evidence_dir/result.json"
cat "$evidence_dir/result.json"
echo "refund claim cancellation reconciliation drill passed: replicas=2 events=1 outbox=2"
