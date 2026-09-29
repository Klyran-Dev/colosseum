#!/bin/sh

set -eu

# This is a bounded local evidence harness, not a production SLO claim. It
# drives the real indexer service through a synthetic finalized provider
# contract and never persists private keys or raw signed transaction bytes.
project_name="solana-tax-protocol-refund-proof-integration"
export COMPOSE_PROJECT_NAME="$project_name"
export POSTGRES_PORT="25446"
export RPC_FIXTURE_PORT="28892"
export INDEXER_HEALTH_PORT="28092"
export INDEXER_ENV="local"
export INDEXER_ALLOW_INSECURE_LOCAL_RPC="true"
export INDEXER_RPC_ENDPOINTS="http://rpc-fixture:8899"
export INDEXER_STREAM_KEY="drill/refund-proof/v1"
export INDEXER_POLL_MS="100"
export INDEXER_VAULT_RECONCILIATION_POLL_MS="100"
export RPC_FIXTURE_SIGNATURE_STATUS="finalized"

merchant_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d61"
adjustment_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d62"
funding_configuration_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d63"
claim_intent_id="01992f1b-4a70-8e30-8ea2-b6e37a9d9d64"
execute_intent_id="01992f1b-4a70-8e30-8ea2-b6e37a9d9d65"
original_settlement_id="01992f1b-4a70-8e30-8ea2-b6e37a9d9d66"
source_calculation_id="01992f1b-4a70-8e30-8ea2-b6e37a9d9d67"
stream_key="$INDEXER_STREAM_KEY"

drill_dir="$(mktemp -d)"
cleanup() {
    docker compose -p "$project_name" down --volumes --remove-orphans >/dev/null 2>&1 || true
    rm -rf "$drill_dir"
}
trap cleanup EXIT INT TERM

command -v docker >/dev/null 2>&1 || { echo "Docker is required" >&2; exit 1; }
command -v jq >/dev/null 2>&1 || { echo "jq is required" >&2; exit 1; }

pnpm --dir apps/web exec node scripts/refund_proof_fixture.mjs >"$drill_dir/fixture.json"
fixture_value() {
    jq -er "$1" "$drill_dir/fixture.json"
}

export RPC_FIXTURE_REFUND_PROOF_CLAIM="$(fixture_value '.claim')"
export RPC_FIXTURE_REFUND_PROOF_ADJUSTMENT="$(fixture_value '.adjustment')"
export RPC_FIXTURE_REFUND_PROOF_ORIGINAL_RECEIPT="$(fixture_value '.original_receipt')"
export RPC_FIXTURE_REFUND_PROOF_MERCHANT="$(fixture_value '.merchant')"
export RPC_FIXTURE_REFUND_PROOF_MINT="$(fixture_value '.mint')"
export RPC_FIXTURE_REFUND_PROOF_PAYER="$(fixture_value '.payer')"
export RPC_FIXTURE_REFUND_PROOF_REFUND_FUNDER="$(fixture_value '.refund_funder')"
export RPC_FIXTURE_REFUND_PROOF_TAX_VAULT="$(fixture_value '.tax_vault')"
export RPC_FIXTURE_REFUND_PROOF_REFUND_SOURCE="$(fixture_value '.refund_source')"
export RPC_FIXTURE_REFUND_PROOF_PAYER_DESTINATION="$(fixture_value '.payer_destination')"
export RPC_FIXTURE_REFUND_PROOF_APPROVER="$(fixture_value '.approver')"
export RPC_FIXTURE_REFUND_PROOF_SIGNATURE="$(fixture_value '.signature')"
export RPC_FIXTURE_REFUND_PROOF_REFERENCE_DIGEST_HEX="$(fixture_value '.reference_digest_hex')"
export RPC_FIXTURE_REFUND_PROOF_NONCE="$(fixture_value '.nonce')"
export RPC_FIXTURE_REFUND_PROOF_GROSS="$(fixture_value '.gross')"
export RPC_FIXTURE_REFUND_PROOF_TAX="$(fixture_value '.tax')"
export RPC_FIXTURE_REFUND_PROOF_NET="$(fixture_value '.net')"
export RPC_FIXTURE_REFUND_PROOF_SLOT="$(fixture_value '.slot')"

docker compose -p "$project_name" --profile settlement-submit-drill --profile indexer \
    up --build --detach postgres rpc-fixture indexer >/dev/null

wait_http() {
    url=$1
    attempts=$2
    for _attempt in $(seq 1 "$attempts"); do
        if curl --fail --silent "$url" >/dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    return 1
}

wait_http "http://127.0.0.1:\${RPC_FIXTURE_PORT}/healthz" 90 || {
    docker compose -p "$project_name" logs --no-color rpc-fixture >&2 || true
    echo "refund-proof RPC fixture did not become ready" >&2
    exit 1
}
wait_http "http://127.0.0.1:\${INDEXER_HEALTH_PORT}/readyz" 90 || {
    docker compose -p "$project_name" logs --no-color indexer postgres >&2 || true
    echo "refund-proof indexer did not become ready" >&2
    exit 1
}

claim_hex="$(fixture_value '.claim_hex')"
adjustment_hex="$(fixture_value '.hex.adjustment')"
original_receipt_hex="$(fixture_value '.hex.original_receipt')"
merchant_hex="$(fixture_value '.hex.merchant')"
mint_hex="$(fixture_value '.hex.mint')"
payer_hex="$(fixture_value '.hex.payer')"
refund_funder_hex="$(fixture_value '.hex.refund_funder')"
payer_destination_hex="$(fixture_value '.hex.payer_destination')"
approver_hex="$(fixture_value '.hex.approver')"
reference_digest_hex="$(fixture_value '.reference_digest_hex')"
vault_authority="$(fixture_value '.vault_authority')"
refund_funder="$(fixture_value '.refund_funder')"
refund_source="$(fixture_value '.refund_source')"
merchant="$(fixture_value '.merchant')"
mint="$(fixture_value '.mint')"
tax_vault="$(fixture_value '.tax_vault')"
payer_destination="$(fixture_value '.payer_destination')"
approver="$(fixture_value '.approver')"
signature="$(fixture_value '.signature')"
artifact_json="$(jq -c '.artifact' "$drill_dir/fixture.json")"

psql_args="-v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol"
docker compose -p "$project_name" exec -T postgres psql $psql_args \
    -v artifact_json="$artifact_json" <<SQL
INSERT INTO chain_checkpoints (stream_key, next_slot, finalized_slot, generation)
VALUES ('$stream_key', 901, 900, 0);

INSERT INTO settlement_adjustments (
    id, merchant_id, idempotency_key, request_digest, adjustment_digest,
    original_settlement_id, asset_id, kind, gross_delta, tax_delta, net_delta,
    reason_code, evidence_digest, source_calculation_id, status,
    proposer_subject, approver_subject
) VALUES (
    '$adjustment_id', '$merchant_id', 'refund-proof-drill-adjustment', decode(repeat('31', 32), 'hex'),
    decode(repeat('32', 32), 'hex'), '$original_settlement_id', decode('$mint_hex', 'hex'),
    'refund', -100, -25, -75, 'transaction-bound-proof-drill', decode(repeat('33', 32), 'hex'),
    '$source_calculation_id', 'approved', 'refund-proof-proposer', 'refund-proof-approver'
);

INSERT INTO refund_funding_configurations (
    id, merchant_id, idempotency_key, request_digest, configuration_digest,
    configuration_key, version, status, merchant_account, mint, tax_vault,
    vault_authority, refund_funder, net_source, token_program,
    proposer_subject, approver_subject, verification_digest, effective_from,
    refund_funder_subject, verification_slot, verification_commitment,
    verification_evidence, verifier_version, verified_at
) VALUES (
    '$funding_configuration_id', '$merchant_id', 'refund-proof-drill-funding', decode(repeat('34', 32), 'hex'),
    decode(repeat('35', 32), 'hex'), 'refund-proof-drill-funding', 1, 'active', '$merchant', '$mint',
    '$tax_vault', '$vault_authority', '$refund_funder', '$refund_source',
    'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA', 'refund-proof-proposer', 'refund-proof-approver',
    decode(repeat('36', 32), 'hex'), now() - interval '1 minute', 'refund-proof-funder', 899,
    'finalized', jsonb_build_object('source', 'adr-0195-local-fixture'), 'adr-0195', now()
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
    '$claim_intent_id', '$merchant_id', 'refund-proof-drill-claim', decode(repeat('37', 32), 'hex'),
    '$adjustment_id', '$funding_configuration_id', decode(repeat('35', 32), 'hex'), 'refund-proof-receipt',
    700, decode('$original_receipt_hex', 'hex'), decode('$payer_hex', 'hex'), decode('$merchant_hex', 'hex'),
    decode('$mint_hex', 'hex'), 'TokenkegQfeZyiNwAJbbNbGKPFXCWuBvf9Ss623VQ5DA', '$merchant',
    '$refund_funder', 'refund-proof-funder', '$tax_vault', '$refund_source', '$payer_destination',
    11, 100, 25, 75, decode('$reference_digest_hex', 'hex'), 1, 9999999999,
    decode('$claim_hex', 'hex'), decode(repeat('38', 32), 'hex'), decode(repeat('39', 32), 'hex'),
    :'artifact_json'::jsonb, 'finalized', 'refund-proof-claim', 800, 'finalized'
);

INSERT INTO refund_execute_intents (
    id, merchant_id, idempotency_key, request_digest, claim_intent_id,
    funding_configuration_id, funding_configuration_digest, claim, adjustment_id,
    original_receipt, merchant, mint, token_program, approver, refund_funder,
    refund_funder_subject, instruction_digest, artifact_digest, unsigned_artifact,
    status, signature, submitted_slot, commitment
) VALUES (
    '$execute_intent_id', '$merchant_id', 'refund-proof-drill-execute', decode(repeat('40', 32), 'hex'),
    '$claim_intent_id', '$funding_configuration_id', decode(repeat('35', 32), 'hex'),
    decode('$claim_hex', 'hex'), '$adjustment_id', decode('$original_receipt_hex', 'hex'),
    decode('$merchant_hex', 'hex'), decode('$mint_hex', 'hex'),
    'TokenkegQfeZyiNwAJbbNbGKPFXCWuBv9Ss623VQ5DA',
    '$approver', '$refund_funder', 'refund-proof-funder', decode(repeat('41', 32), 'hex'),
    decode(repeat('42', 32), 'hex'), :'artifact_json'::jsonb, 'submitted',
    '$signature', 900, 'confirmed'
);

INSERT INTO indexed_adjustment_refund_executions (
    stream_key, signature, slot, event_index, claim, adjustment, original_receipt,
    merchant, mint, nonce, gross_magnitude, tax_magnitude, net_magnitude,
    refund_funder, refund_destination, approver, reference_digest, evidence_digest
) VALUES (
    '$stream_key', '$signature', 900, 0, decode('$claim_hex', 'hex'),
    decode('$adjustment_hex', 'hex'), decode('$original_receipt_hex', 'hex'), decode('$merchant_hex', 'hex'),
    decode('$mint_hex', 'hex'), 11, 100, 25, 75, decode('$refund_funder_hex', 'hex'),
    decode('$payer_destination_hex', 'hex'), decode('$approver_hex', 'hex'),
    decode('$reference_digest_hex', 'hex'), decode(repeat('43', 32), 'hex')
);
SQL

proof_ready="false"
for _attempt in $(seq 1 120); do
    proof_state="$(docker compose -p "$project_name" exec -T postgres psql -qAt $psql_args -c \
        "SELECT (SELECT count(*) FROM refund_balance_proofs WHERE stream_key = '$stream_key') || '|' ||
                (SELECT status FROM refund_execute_intents WHERE id = '$execute_intent_id') || '|' ||
                (SELECT count(*) FROM refund_execute_intent_events WHERE intent_id = '$execute_intent_id' AND to_status = 'finalized') || '|' ||
                (SELECT count(*) FROM outbox_events WHERE aggregate_id = '$execute_intent_id' AND event_type = 'tax.refund_execute_intent.finalized');")"
    if [ "$proof_state" = "1|finalized|1|1" ]; then
        proof_ready="true"
        break
    fi
    sleep 1
done
if [ "$proof_ready" != "true" ]; then
    docker compose -p "$project_name" logs --no-color indexer rpc-fixture postgres >&2 || true
    echo "transaction-bound refund proof did not converge: $proof_state" >&2
    exit 1
fi

proof_cardinality="$(docker compose -p "$project_name" exec -T postgres psql -qAt $psql_args -c \
    "SELECT count(*) || '|' || min(slot::text) || '|' || min(execution.proof_claim_attempts::text)
       FROM refund_balance_proofs proof
       JOIN indexed_adjustment_refund_executions execution USING (stream_key, signature, event_index)
      WHERE proof.stream_key = '$stream_key';")"
if [ "$proof_cardinality" != "1|900|1" ]; then
    echo "refund proof cardinality or slot evidence was unexpected: $proof_cardinality" >&2
    exit 1
fi

if docker compose -p "$project_name" exec -T postgres psql $psql_args -c \
    "UPDATE refund_balance_proofs SET gross_atomic = 99 WHERE stream_key = '$stream_key';" >/dev/null 2>&1; then
    echo "refund balance proof append-only mutation unexpectedly succeeded" >&2
    exit 1
fi

sleep 1
replay_cardinality="$(docker compose -p "$project_name" exec -T postgres psql -qAt $psql_args -c \
    "SELECT count(*) FROM refund_balance_proofs WHERE stream_key = '$stream_key';")"
if [ "$replay_cardinality" != "1" ]; then
    echo "refund proof replay created duplicate evidence: $replay_cardinality" >&2
    exit 1
fi
proof_digest="$(docker compose -p "$project_name" exec -T postgres psql -qAt $psql_args -c \
    "SELECT encode(proof_digest, 'hex') FROM refund_balance_proofs WHERE stream_key = '$stream_key';")"
printf '{"stream_key":"%s","slot":900,"proof_count":1,"execute_status":"finalized","finalized_event_count":1,"finalized_outbox_count":1,"proof_claim_attempts":1,"proof_digest":"%s"}\n' \
    "$stream_key" "$proof_digest"
echo "transaction-bound refund proof integration drill passed"
