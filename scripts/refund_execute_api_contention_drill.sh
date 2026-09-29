#!/bin/sh

set -eu

# This is a bounded local evidence harness, not a production SLO claim. It
# drives the real authenticated API and PostgreSQL adapter with synthetic,
# server-owned finalized evidence.
project_name="solana-tax-protocol-refund-execute-api-contention"
export API_PORT="28082"
export POSTGRES_PORT="25436"
export NATS_PORT="25437"
export RPC_FIXTURE_PORT="28882"
export API_ENV="local"
export API_ALLOW_INSECURE_LOCAL_RPC="true"
export API_SETTLEMENT_RPC_ENDPOINTS="http://rpc-fixture:8899"
merchant_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d41"
adjustment_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d42"
funding_configuration_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d43"
settlement_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d44"
evidence_intent_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d45"
chain_intent_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d46"
proposer_challenge_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d47"
funder_challenge_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d48"
approver_challenge_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d49"
stream_key="drill/refund-execute-api/v1"
receipt_signature="refund-execute-api-receipt"
adjustment_signature="refund-execute-api-adjustment"
claim_idempotency_key="refund-execute-api-claim-01"
execute_idempotency_key="refund-execute-api-execute-01"
proposer_subject="refund-api-proposer"
funder_subject="refund-api-funder"
approver_subject="refund-api-approver"
token_program="TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
proposer="6JhaGdekBjU2RfiYWSjYdQAibx4LfSfTNFEeMUHnUVz7"
funder="Bow1CGKGDB9mNxeWdw85E2aCthQ1oZX4oFEe7fYT17ew"
merchant="1119DWteoLSdjvrT6g6L8C2PfDD2faiTQUpsjY2RiF"
mint="111BuZ6b86gm7XhxjvTakhRvxSMjXp2GqgifkNUmDK"
original_receipt="111FJo4zLAGU9nzTWa6EnbV4VAmtG4FR8kcokrtZYr"
approver="AB3FQHskSYuWVw4M9EpGdxNzrAjBNiYGpbH4CVzLFene"
tax_vault="111A4R8Vf7xP4Rz7ZkYd8H7f4Q5J4h4Y4V4w4U4T4S"
net_source="111B4R8Vf7xP4Rz7ZkYd8H7f4P5J4h4Y4V4w4U4T4S"
adjustment_address="Fk3Z5sqgXWRYu3dFuQqshGmgGJoSxQSnxTHSFt9CKM5T"
registry_address="4fuGSLS5aka44BPDqoH7RSg1HLvy4bMU4zY551u2Dnts"
payer="qcfm6jd37nhqmbY6JomQWtdD9S4HieqnmRm49Hum6ir"
proposer_seed_hex="2020202020202020202020202020202020202020202020202020202020202020"
funder_seed_hex="2222222222222222222222222222222222222222222222222222222222222222"
fee_payer_seed_hex="2323232323232323232323232323232323232323232323232323232323232323"

drill_dir="$(mktemp -d)"
claim_response="$drill_dir/claim.json"
execute_response_dir="$drill_dir/execute-responses"
execute_statuses="$drill_dir/execute-statuses"
latency_samples="$drill_dir/latencies-ms"
mkdir -p "$execute_response_dir"

cleanup() {
    docker compose -p "$project_name" down --volumes --remove-orphans >/dev/null 2>&1 || true
    rm -rf "$drill_dir"
}
trap cleanup EXIT INT TERM

command -v docker >/dev/null 2>&1 || { echo "Docker is required" >&2; exit 1; }
command -v curl >/dev/null 2>&1 || { echo "curl is required" >&2; exit 1; }
command -v jq >/dev/null 2>&1 || { echo "jq is required" >&2; exit 1; }
command -v openssl >/dev/null 2>&1 || { echo "OpenSSL is required" >&2; exit 1; }

b64url() {
    printf '%s' "$1" | base64 | tr '+/' '-_' | tr -d '=\n'
}

b64url_stdin() {
    base64 | tr '+/' '-_' | tr -d '=\n'
}

openssl genpkey -algorithm ED25519 -out "$drill_dir/signing-key.pem" >/dev/null 2>&1
openssl pkey -in "$drill_dir/signing-key.pem" -pubout -outform DER \
    -out "$drill_dir/public-key.der" >/dev/null 2>&1
public_key="$(tail -c 32 "$drill_dir/public-key.der" | b64url_stdin)"
export API_AUTH_ISSUER="https://refund-execute-drill.invalid"
export API_AUTH_AUDIENCE="tax-api"
export API_AUTH_JWKS_KID="refund-execute-drill-key"
export API_AUTH_JWKS_PUBLIC_KEY="$public_key"
export API_DATABASE_POOL_SIZE="16"

now_unix="$(date +%s)"
token_header="$(b64url '{"alg":"EdDSA","kid":"refund-execute-drill-key","typ":"JWT"}')"
claim_token_claims="$(b64url "{\"iss\":\"$API_AUTH_ISSUER\",\"sub\":\"$proposer_subject\",\"aud\":\"$API_AUTH_AUDIENCE\",\"exp\":$((now_unix + 600)),\"iat\":$((now_unix - 10)),\"nbf\":$((now_unix - 10)),\"merchant_id\":\"$merchant_id\",\"scope\":\"tax:refund:claim:prepare\",\"roles\":[\"settlement_adjustment_proposer\"]}")"
claim_token_signing_input="$token_header.$claim_token_claims"
claim_token_signature="$(printf '%s' "$claim_token_signing_input" | openssl pkeyutl -sign \
    -inkey "$drill_dir/signing-key.pem" -rawin -in /dev/stdin 2>/dev/null | b64url_stdin)"
claim_authorization="Bearer $claim_token_signing_input.$claim_token_signature"

execute_token_claims="$(b64url "{\"iss\":\"$API_AUTH_ISSUER\",\"sub\":\"$approver_subject\",\"aud\":\"$API_AUTH_AUDIENCE\",\"exp\":$((now_unix + 600)),\"iat\":$((now_unix - 10)),\"nbf\":$((now_unix - 10)),\"merchant_id\":\"$merchant_id\",\"scope\":\"tax:refund:execute:prepare\",\"roles\":[\"settlement_adjustment_approver\"]}")"
execute_token_signing_input="$token_header.$execute_token_claims"
execute_token_signature="$(printf '%s' "$execute_token_signing_input" | openssl pkeyutl -sign \
    -inkey "$drill_dir/signing-key.pem" -rawin -in /dev/stdin 2>/dev/null | b64url_stdin)"
execute_authorization="Bearer $execute_token_signing_input.$execute_token_signature"

claim_submit_token_claims="$(b64url "{\"iss\":\"$API_AUTH_ISSUER\",\"sub\":\"$proposer_subject\",\"aud\":\"$API_AUTH_AUDIENCE\",\"exp\":$((now_unix + 600)),\"iat\":$((now_unix - 10)),\"nbf\":$((now_unix - 10)),\"merchant_id\":\"$merchant_id\",\"scope\":\"tax:refund:claim:submit\",\"roles\":[\"settlement_adjustment_proposer\"]}")"
claim_submit_token_signing_input="$token_header.$claim_submit_token_claims"
claim_submit_token_signature="$(printf '%s' "$claim_submit_token_signing_input" | openssl pkeyutl -sign \
    -inkey "$drill_dir/signing-key.pem" -rawin -in /dev/stdin 2>/dev/null | b64url_stdin)"
claim_submit_authorization="Bearer $claim_submit_token_signing_input.$claim_submit_token_signature"

execute_submit_token_claims="$(b64url "{\"iss\":\"$API_AUTH_ISSUER\",\"sub\":\"$approver_subject\",\"aud\":\"$API_AUTH_AUDIENCE\",\"exp\":$((now_unix + 600)),\"iat\":$((now_unix - 10)),\"nbf\":$((now_unix - 10)),\"merchant_id\":\"$merchant_id\",\"scope\":\"tax:refund:execute:submit\",\"roles\":[\"settlement_adjustment_approver\"]}")"
execute_submit_token_signing_input="$token_header.$execute_submit_token_claims"
execute_submit_token_signature="$(printf '%s' "$execute_submit_token_signing_input" | openssl pkeyutl -sign \
    -inkey "$drill_dir/signing-key.pem" -rawin -in /dev/stdin 2>/dev/null | b64url_stdin)"
execute_submit_authorization="Bearer $execute_submit_token_signing_input.$execute_submit_token_signature"

docker compose -p "$project_name" --profile settlement-submit-drill \
    up --build --detach postgres nats rpc-fixture api >/dev/null
base_url="http://127.0.0.1:$API_PORT"
rpc_fixture_url="http://127.0.0.1:$RPC_FIXTURE_PORT"
rpc_fixture_ready="false"
for _attempt in $(seq 1 90); do
    if curl --fail --silent "$rpc_fixture_url/healthz" >/dev/null 2>&1; then
        rpc_fixture_ready="true"
        break
    fi
    sleep 1
done
if [ "$rpc_fixture_ready" != "true" ]; then
    docker compose -p "$project_name" logs --no-color rpc-fixture >&2 || true
    echo "refund execute drill RPC fixture did not become ready" >&2
    exit 1
fi
ready="false"
for _attempt in $(seq 1 90); do
    if curl --fail --silent "$base_url/readyz" >/dev/null 2>&1; then
        ready="true"
        break
    fi
    sleep 1
done
if [ "$ready" != "true" ]; then
    docker compose -p "$project_name" logs --no-color api postgres >&2 || true
    echo "refund execute API did not become ready" >&2
    exit 1
fi

docker compose -p "$project_name" exec -T postgres psql \
    -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol <<SQL
INSERT INTO chain_checkpoints (stream_key, next_slot, finalized_slot, generation)
VALUES ('$stream_key', 703, 702, 1);

INSERT INTO settlement_intents (
    id, merchant_id, idempotency_key, request_digest, instruction_digest,
    status, signature, slot, commitment
) VALUES (
    '$settlement_id', '$merchant_id', 'refund-execute-api-settlement',
    decode(repeat('01', 32), 'hex'), decode(repeat('02', 32), 'hex'),
    'finalized', '$receipt_signature', 700, 'finalized'
);

INSERT INTO indexed_receipts (
    stream_key, signature, slot, gross_atomic, tax_atomic, net_atomic,
    evidence_digest, receipt, payer, merchant, mint, nonce, policy_digest
) VALUES (
    '$stream_key', '$receipt_signature', 700, 1000, 250, 750,
    decode(repeat('03', 32), 'hex'),
    decode('000000056f0002172b8dff4f5a32740e271904d7e188fa8e6ced51ce1506139f', 'hex'),
    decode('0c743586ecbf479ff6085fa87957fd17f5eaa67d26959c0b612d91eb26b2b0cf', 'hex'),
    decode('000000031ecab356b92ca5429c433a0e4b51c5605f0a28f8b2947c9ba24bd710', 'hex'),
    decode('00000004245c80604e6b71818637a409e660630704568cb6de6d5ee2e00f3262', 'hex'),
    7, decode(repeat('04', 32), 'hex')
);

INSERT INTO settlement_adjustments (
    id, merchant_id, idempotency_key, request_digest, adjustment_digest,
    original_settlement_id, asset_id, kind, gross_delta, tax_delta, net_delta,
    reason_code, evidence_digest, source_calculation_id, status,
    proposer_subject, approver_subject
) VALUES (
    '$adjustment_id', '$merchant_id', 'refund-execute-api-adjustment',
    decode(repeat('05', 32), 'hex'), decode('4435737c6d6f75aa72f2d7276dabef56c52bcea964a62da4094d81257fe8e31a', 'hex'),
    '$settlement_id',
    decode('00000004245c80604e6b71818637a409e660630704568cb6de6d5ee2e00f3262', 'hex'),
    'refund', -100, -25, -75, 'api-contention',
    decode(repeat('07', 32), 'hex'),
    '01992f1b-4a70-7e30-8ea2-b6e37a9d9d4a', 'approved',
    '$proposer_subject', '$approver_subject'
);

INSERT INTO refund_funding_configurations (
    id, merchant_id, idempotency_key, request_digest, configuration_digest,
    configuration_key, version, status, merchant_account, mint, tax_vault,
    vault_authority, refund_funder, net_source, token_program,
    proposer_subject, approver_subject, verification_digest, effective_from,
    refund_funder_subject, verification_slot, verification_commitment,
    verification_evidence, verifier_version, verified_at
) VALUES (
    '$funding_configuration_id', '$merchant_id', 'refund-execute-api-funding',
    decode(repeat('08', 32), 'hex'), decode(repeat('09', 32), 'hex'),
    'refund-execute-api-funding', 1, 'active', '$merchant', '$mint',
    '$tax_vault', '$registry_address', '$funder', '$net_source', '$token_program',
    '$proposer_subject', '$approver_subject', decode(repeat('0a', 32), 'hex'),
    now() - interval '1 minute', '$funder_subject', 650, 'finalized',
    jsonb_build_object('source', 'refund-execute-api-contention-drill'),
    'refund-execute-api-drill-v1', now()
);

INSERT INTO settlement_adjustment_authority_challenges (
    id, merchant_id, subject, role, message_digest, expires_at
) VALUES
    ('$proposer_challenge_id', '$merchant_id', '$proposer_subject', 'proposer',
     decode(repeat('0b', 32), 'hex'), now() + interval '1 hour'),
    ('$funder_challenge_id', '$merchant_id', '$funder_subject', 'refund_funder',
     decode(repeat('0c', 32), 'hex'), now() + interval '1 hour'),
    ('$approver_challenge_id', '$merchant_id', '$approver_subject', 'approver',
     decode(repeat('0d', 32), 'hex'), now() + interval '1 hour');

INSERT INTO settlement_adjustment_authority_bindings (
    id, merchant_id, subject, role, public_key, challenge_id, proof_signature
) VALUES
    ('01992f1b-4a70-7e30-8ea2-b6e37a9d9d4a', '$merchant_id', '$proposer_subject',
     'proposer',
     decode('4ed32f63bf35f0eeefcb25f28a2e1fbdc873ae2835671b0c9460f5f12e4556a8', 'hex'),
     '$proposer_challenge_id', decode(repeat('0e', 64), 'hex')),
    ('01992f1b-4a70-7e30-8ea2-b6e37a9d9d4b', '$merchant_id', '$funder_subject',
     'refund_funder',
     decode('a09aa5f47a6759802ff955f8dc2d2a14a5c99d23be97f864127ff9383455a4f0', 'hex'),
     '$funder_challenge_id', decode(repeat('0f', 64), 'hex')),
    ('01992f1b-4a70-7e30-8ea2-b6e37a9d9d4c', '$merchant_id', '$approver_subject',
     'approver',
     decode('884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b4201d9d0b', 'hex'),
     '$approver_challenge_id', decode(repeat('10', 64), 'hex'));

INSERT INTO settlement_adjustment_evidence_intents (
    id, adjustment_id, merchant_id, nonce, proposer, approver,
    original_receipt, merchant, mint, token_program, registry_address,
    registry_slot, registry_commitment, registry_account_digest,
    request_digest, instruction_digest, status
) VALUES (
    '$evidence_intent_id', '$adjustment_id', '$merchant_id', 11,
    decode('4ed32f63bf35f0eeefcb25f28a2e1fbdc873ae2835671b0c9460f5f12e4556a8', 'hex'),
    decode('884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b4201d9d0b', 'hex'),
    decode('000000056f0002172b8dff4f5a32740e271904d7e188fa8e6ced51ce1506139f', 'hex'),
    decode('000000031ecab356b92ca5429c433a0e4b51c5605f0a28f8b2947c9ba24bd710', 'hex'),
    decode('00000004245c80604e6b71818637a409e660630704568cb6de6d5ee2e00f3262', 'hex'),
    '$token_program',
    decode('368a3aa30a1a06e3e4be21bd106dad0c09e9e68cb1daf9d85670931846efc684', 'hex'),
    650, 'finalized', decode(repeat('11', 32), 'hex'),
    decode(repeat('12', 32), 'hex'), decode(repeat('13', 32), 'hex'),
    'prepared_unsubmitted'
);

INSERT INTO settlement_adjustment_chain_intents (
    id, evidence_intent_id, adjustment_id, merchant_id, instruction_digest,
    status, signature, slot, commitment, finalized_event_index,
    finalized_event_evidence_digest
) VALUES (
    '$chain_intent_id', '$evidence_intent_id', '$adjustment_id', '$merchant_id',
    decode(repeat('13', 32), 'hex'), 'finalized',
    '$adjustment_signature', 701, 'finalized', 0, decode(repeat('14', 32), 'hex')
);

INSERT INTO indexed_settlement_adjustments (
    stream_key, signature, slot, event_index, adjustment, original_receipt,
    merchant, mint, nonce, kind, gross_sign, gross_magnitude, tax_sign,
    tax_magnitude, net_sign, net_magnitude, adjustment_digest,
    evidence_digest, adjustment_evidence_digest, approver
) VALUES (
    '$stream_key', '$adjustment_signature', 701, 0,
    decode('db0a7a88472dc6415f919dd66909adc3c6722a4d4bd3df225b0007a22104f612', 'hex'),
    decode('000000056f0002172b8dff4f5a32740e271904d7e188fa8e6ced51ce1506139f', 'hex'),
    decode('000000031ecab356b92ca5429c433a0e4b51c5605f0a28f8b2947c9ba24bd710', 'hex'),
    decode('00000004245c80604e6b71818637a409e660630704568cb6de6d5ee2e00f3262', 'hex'),
    11, 1, 1, 100, 1, 25, 1, 75,
    decode('4435737c6d6f75aa72f2d7276dabef56c52bcea964a62da4094d81257fe8e31a', 'hex'), decode(repeat('14', 32), 'hex'),
    decode(repeat('07', 32), 'hex'),
    decode('884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b4201d9d0b', 'hex')
);

INSERT INTO raw_chain_evidence (
    stream_key, signature, slot, event_index, event_discriminator,
    event_data, evidence_digest
) VALUES (
    '$stream_key', '$adjustment_signature', 701, 0,
    decode('d05645b72ccbec4b', 'hex'),
    decode('d05645b72ccbec4b0000000000000000', 'hex'),
    decode(repeat('15', 32), 'hex')
);
SQL

seed_adjustment_state="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select count(*) || '|' || coalesce(min(merchant_id::text), '') from settlement_adjustments where id = '$adjustment_id';")"
if [ "$seed_adjustment_state" != "1|$merchant_id" ]; then
    echo "refund execute drill seeded an unexpected adjustment row: $seed_adjustment_state" >&2
    exit 1
fi

claim_body="{\"idempotency_key\":\"$claim_idempotency_key\",\"adjustment_id\":\"$adjustment_id\",\"confirm\":true}"
claim_status="$(curl --silent --show-error --max-time 10 \
    --header "content-type: application/json" \
    --header "authorization: $claim_authorization" \
    --data-binary "$claim_body" \
    --output "$claim_response" \
    --write-out "%{http_code}" \
    "$base_url/v1/settlement/refund-claims")"
if [ "$claim_status" != "200" ]; then
    echo "claim preparation failed with HTTP $claim_status" >&2
    cat "$claim_response" >&2
    docker compose -p "$project_name" logs --no-color api postgres >&2 || true
    exit 1
fi
claim_intent_id="$(jq -er '.intent_id' "$claim_response")"
jq -e --arg expected "$claim_idempotency_key" \
    '.status == "prepared_unsubmitted" and .artifact.schema == "refund-movement-artifact/v1"' \
    "$claim_response" >/dev/null

claim_initial_send_count="$(curl --fail --silent "$rpc_fixture_url/admin/send-count" | jq -er '.send_transaction_calls')"
API_BASE="$base_url" \
RPC_FIXTURE_URL="$rpc_fixture_url" \
AUTHORIZATION="$claim_submit_authorization" \
ARTIFACT_FILE="$claim_response" \
INTENT_KIND="claim" \
SIGNER_SEED_HEX="$proposer_seed_hex" \
REFUND_FUNDER_SEED_HEX="$funder_seed_hex" \
FEE_PAYER_SEED_HEX="$fee_payer_seed_hex" \
INITIAL_SEND_COUNT="$claim_initial_send_count" \
pnpm --dir apps/web exec node scripts/refund_submission_drill.mjs >"$drill_dir/claim-submit.json"
cat "$drill_dir/claim-submit.json"

claim_signature="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select signature from refund_claim_intents where id = '$claim_intent_id' and merchant_id = '$merchant_id';")"
if [ -z "$claim_signature" ]; then
    echo "wallet-signed claim submission did not persist a signature" >&2
    exit 1
fi
docker compose -p "$project_name" exec -T postgres psql \
    -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol <<SQL
UPDATE refund_claim_intents
SET status = 'finalized', submitted_slot = 702, commitment = 'finalized'
WHERE id = '$claim_intent_id' AND merchant_id = '$merchant_id'
  AND status = 'submitted';
SQL

claim_state="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select status || '|' || commitment from refund_claim_intents where id = '$claim_intent_id' and merchant_id = '$merchant_id';")"
if [ "$claim_state" != "finalized|finalized" ]; then
    echo "synthetic finalized claim setup failed: $claim_state" >&2
    exit 1
fi

export drill_base_url="$base_url"
export drill_authorization="$execute_authorization"
export drill_body="{\"idempotency_key\":\"$execute_idempotency_key\",\"claim_intent_id\":\"$claim_intent_id\",\"confirm\":true}"
export drill_response_dir="$execute_response_dir"
seq 1 16 | xargs -P 16 -n 1 sh -c '
    ordinal="$1"
    sleep "0.0$ordinal"
    curl --silent --show-error --max-time 15 \
        --header "content-type: application/json" \
        --header "authorization: $drill_authorization" \
        --data-binary "$drill_body" \
        --output "$drill_response_dir/$ordinal.json" \
        --write-out "%{http_code} %{time_total}\n" \
        "$drill_base_url/v1/settlement/refund-executions"
' refund-execute-api-contention >"$execute_statuses"

response_count="$(wc -l <"$execute_statuses" | tr -d ' ')"
non_success="$(awk '$1 != 200 { count += 1 } END { print count + 0 }' "$execute_statuses")"
if [ "$response_count" != "16" ] || [ "$non_success" != "0" ]; then
    echo "execute preparation contention did not converge: responses=$response_count non_success=$non_success" >&2
    cat "$execute_statuses" >&2
    for response in "$execute_response_dir"/*.json; do
        if jq -e '.error' "$response" >/dev/null 2>&1; then
            echo "error response: $response" >&2
            cat "$response" >&2
        else
            echo "success response: $response" >&2
            jq -c '{intent_id,request_digest,claim,artifact_digest}' "$response" >&2
        fi
    done
    docker compose -p "$project_name" logs --no-color api postgres >&2 || true
    exit 1
fi
awk '$2 ~ /^[0-9]+([.][0-9]+)?$/ { printf "%.3f\n", ($2 * 1000) }' \
    "$execute_statuses" | LC_ALL=C sort -n >"$latency_samples"
latency_sample_count="$(wc -l <"$latency_samples" | tr -d ' ')"
positive_latency_sample_count="$(awk '$1 > 0 && $1 == $1 { count += 1 } END { print count + 0 }' "$latency_samples")"
if [ "$latency_sample_count" != "16" ] || [ "$positive_latency_sample_count" != "16" ]; then
    echo "execute preparation latency samples were incomplete or invalid: samples=$latency_sample_count positive=$positive_latency_sample_count" >&2
    cat "$execute_statuses" >&2
    exit 1
fi
percentile_ms() {
    percentile="$1"
    awk -v percentile="$percentile" '
        { values[NR] = $1 }
        END {
            rank = int(percentile * NR)
            if (rank < percentile * NR) rank += 1
            if (rank < 1) rank = 1
            print values[rank]
        }
    ' "$latency_samples"
}
p50_ms="$(percentile_ms 0.50)"
p95_ms="$(percentile_ms 0.95)"
p99_ms="$(percentile_ms 0.99)"

intent_count="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from refund_execute_intents where merchant_id = '$merchant_id' and idempotency_key = '$execute_idempotency_key';")"
event_count="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from refund_execute_intent_events e join refund_execute_intents i on i.id = e.intent_id and i.merchant_id = e.merchant_id where i.merchant_id = '$merchant_id' and i.idempotency_key = '$execute_idempotency_key';")"
outbox_count="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from outbox_events where aggregate_type = 'refund_execute_intent' and event_type = 'tax.refund_execute_intent.prepared' and payload->>'merchant_id' = '$merchant_id';")"
response_intent_count="$(for response in "$execute_response_dir"/*.json; do jq -r '.intent_id' "$response"; done | sort -u | wc -l | tr -d ' ')"
response_digest_count="$(for response in "$execute_response_dir"/*.json; do jq -c '.request_digest' "$response"; done | sort -u | wc -l | tr -d ' ')"
if [ "$intent_count" != "1" ] || [ "$event_count" != "1" ] || [ "$outbox_count" != "1" ] \
    || [ "$response_intent_count" != "1" ] || [ "$response_digest_count" != "1" ]; then
    echo "execute idempotency cardinality failed: intents=$intent_count events=$event_count outbox=$outbox_count response_intents=$response_intent_count response_digests=$response_digest_count" >&2
    exit 1
fi

execute_initial_send_count="$(curl --fail --silent "$rpc_fixture_url/admin/send-count" | jq -er '.send_transaction_calls')"
API_BASE="$base_url" \
RPC_FIXTURE_URL="$rpc_fixture_url" \
AUTHORIZATION="$execute_submit_authorization" \
ARTIFACT_FILE="$execute_response_dir/1.json" \
INTENT_KIND="execute" \
SIGNER_SEED_HEX="2121212121212121212121212121212121212121212121212121212121212121" \
REFUND_FUNDER_SEED_HEX="$funder_seed_hex" \
FEE_PAYER_SEED_HEX="$fee_payer_seed_hex" \
INITIAL_SEND_COUNT="$execute_initial_send_count" \
pnpm --dir apps/web exec node scripts/refund_submission_drill.mjs >"$drill_dir/execute-submit.json"
cat "$drill_dir/execute-submit.json"

execute_state="$(docker compose -p "$project_name" exec -T postgres psql \
    -U tax_protocol -d tax_protocol -Atc \
    "select status || '|' || (signature is not null)::text from refund_execute_intents where merchant_id = '$merchant_id' and idempotency_key = '$execute_idempotency_key';")"
if [ "$execute_state" != "submitted|true" ]; then
    echo "wallet-signed execute submission did not persist submitted state: $execute_state" >&2
    exit 1
fi

echo "Refund execute API contention drill passed: claim=$claim_intent_id requests=16 concurrency=16 pool_size=$API_DATABASE_POOL_SIZE stagger_ms=10-16 http_200=16 execute_intent=1 lifecycle_event=1 outbox=1 p50_ms=$p50_ms p95_ms=$p95_ms p99_ms=$p99_ms"
