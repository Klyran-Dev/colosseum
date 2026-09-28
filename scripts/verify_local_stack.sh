#!/bin/sh

set -eu

project_name="solana-tax-protocol-integration"
export POSTGRES_PORT="25432"
export API_PORT="28080"
export API_REPLICA_PORT="28081"
export EDGE_PORT="28088"
export NATS_PORT="25433"
export RPC_FIXTURE_PORT="28899"
export API_SIMULATION_MODE="false"
export API_ENV="local"
export API_ALLOW_INSECURE_LOCAL_RPC="true"
export API_SETTLEMENT_RPC_ENDPOINTS="http://rpc-fixture:8899"
export RPC_FIXTURE_REGISTRY_MERCHANT="11111111111111111111111111111114"
export RPC_FIXTURE_REGISTRY_MINT="SysvarC1ock11111111111111111111111111111111"
export RPC_FIXTURE_REGISTRY_PROPOSER="AKnL4NNf3DGWZJS6cPknBuEGnVsV4A4m5tgebLHaRSZ9"
export RPC_FIXTURE_REGISTRY_APPROVER="9hSR6S7WPtxmTojgo6GG3k4yDPecgJY292j7xrsUGWBu"
export RPC_FIXTURE_REGISTRY_SLOT="777"
export RUST_LOG="info"
integration_dir="$(mktemp -d)"
openssl genpkey -algorithm ED25519 -out "$integration_dir/signing-key.pem" >/dev/null 2>&1
openssl pkey -in "$integration_dir/signing-key.pem" -pubout -outform DER -out "$integration_dir/public-key.der" >/dev/null 2>&1

b64url() {
    printf '%s' "$1" | base64 | tr '+/' '-_' | tr -d '=\n'
}

b64url_stdin() {
    base64 | tr '+/' '-_' | tr -d '=\n'
}

public_key="$(tail -c 32 "$integration_dir/public-key.der" | b64url_stdin)"
export API_AUTH_ISSUER="https://integration-identity.invalid"
export API_AUTH_AUDIENCE="tax-api"
export API_AUTH_JWKS_KID="integration-key"
export API_AUTH_JWKS_PUBLIC_KEY="$public_key"
base_url="http://127.0.0.1:${API_PORT}"
replica_base_url="http://127.0.0.1:${API_REPLICA_PORT}"
edge_url="http://127.0.0.1:${EDGE_PORT}"
replica_container_id=""

cleanup() {
    exit_status=$?
    if [ -n "$replica_container_id" ]; then
        docker rm --force "$replica_container_id" >/dev/null 2>&1 || true
    fi
    if [ "$exit_status" -ne 0 ]; then
        docker compose -p "$project_name" logs --no-color api >&2 || true
        if [ -n "$replica_container_id" ]; then
            docker logs "$replica_container_id" >&2 || true
        fi
    fi
    if [ "$exit_status" -ne 0 ] && [ "${VERIFY_LOCAL_STACK_KEEP_ON_FAILURE:-false}" = "true" ]; then
        echo "preserving local stack for failure inspection (project: $project_name)" >&2
        exit "$exit_status"
    fi
    docker compose -p "$project_name" down --volumes --remove-orphans >/dev/null 2>&1 || true
    exit "$exit_status"
}
trap cleanup EXIT INT TERM

compose_build_option="--build"
if [ "${VERIFY_LOCAL_STACK_SKIP_BUILD:-false}" = "true" ]; then
    compose_build_option="--no-build"
fi
docker compose -p "$project_name" --profile settlement-submit-drill \
    up "$compose_build_option" --detach rpc-fixture postgres nats api worker edge

rpc_fixture_ready="false"
for _attempt in $(seq 1 60); do
    if curl --fail --silent "http://127.0.0.1:${RPC_FIXTURE_PORT}/healthz" >/dev/null 2>&1; then
        rpc_fixture_ready="true"
        break
    fi
    sleep 1
done
if [ "$rpc_fixture_ready" != "true" ]; then
    echo "settlement RPC fixture did not become reachable" >&2
    docker compose -p "$project_name" --profile settlement-submit-drill logs --no-color rpc-fixture >&2 || true
    exit 1
fi

ready="false"
for _attempt in $(seq 1 60); do
    if curl --fail --silent "$base_url/readyz" >/dev/null 2>&1; then
        ready="true"
        break
    fi
    sleep 1
done

if [ "$ready" != "true" ]; then
    docker compose -p "$project_name" logs --no-color
    exit 1
fi

# Run a second API process from the same built image. It shares only the
# durable services and Docker network; settlement fencing must not depend on
# process-local state or edge stickiness.
replica_container_id="$(docker compose -p "$project_name" run --detach --no-deps \
    --publish "${API_REPLICA_PORT}:8080" api)"
replica_ready="false"
for _attempt in $(seq 1 60); do
    if curl --fail --silent "$replica_base_url/readyz" >/dev/null 2>&1; then
        replica_ready="true"
        break
    fi
    sleep 1
done
if [ "$replica_ready" != "true" ]; then
    echo "secondary settlement API replica did not become ready" >&2
    docker logs "$replica_container_id" >&2 || true
    docker compose -p "$project_name" logs --no-color api postgres >&2 || true
    exit 1
fi

worker_ready="false"
for _attempt in $(seq 1 60); do
    if docker compose -p "$project_name" exec -T worker \
        wget --spider --quiet http://127.0.0.1:8081/readyz >/dev/null 2>&1; then
        worker_ready="true"
        break
    fi
    sleep 1
done
if [ "$worker_ready" != "true" ]; then
    docker compose -p "$project_name" logs --no-color worker nats
    exit 1
fi

if ! docker compose -p "$project_name" exec -T postgres \
    psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    < db/verification/uuidv7_contract.sql >/dev/null; then
    echo "PostgreSQL UUIDv7 schema contract failed" >&2
    exit 1
fi

if ! docker compose -p "$project_name" exec -T postgres \
    psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    < db/verification/vault_reconciliation.sql >/dev/null; then
    echo "PostgreSQL vault reconciliation contract failed" >&2
    exit 1
fi

if ! docker compose -p "$project_name" exec -T postgres \
    psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    < db/verification/targeted_recovery_audit.sql >/dev/null; then
    echo "targeted-recovery audit reconciliation contract failed" >&2
    exit 1
fi

if ! docker compose -p "$project_name" exec -T postgres \
    psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    < db/verification/protocol_pause_events.sql >/dev/null; then
    echo "protocol pause evidence append-only contract failed" >&2
    exit 1
fi

if ! docker compose -p "$project_name" exec -T postgres \
    psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    < db/verification/refund_proof_metrics.sql >/dev/null; then
    echo "refund-proof pending metric contract failed" >&2
    exit 1
fi

pause_event_table=$(docker compose -p "$project_name" exec -T postgres \
    psql -qAt -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    -c "SELECT to_regclass('public.protocol_pause_events') IS NOT NULL" | tr -d '[:space:]')
if [ "$pause_event_table" != "t" ]; then
    echo "protocol pause evidence migration is missing from the clean PostgreSQL bootstrap" >&2
    exit 1
fi

merchant_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d11"
other_merchant_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d12"
make_authorization() {
    subject="$1"
    scope="$2"
    roles_json="$3"
    claimed_merchant_id="${4:-$merchant_id}"
    now_unix="$(date +%s)"
    token_header="$(b64url '{"alg":"EdDSA","kid":"integration-key","typ":"JWT"}')"
    token_claims="$(b64url "{\"iss\":\"$API_AUTH_ISSUER\",\"sub\":\"$subject\",\"aud\":\"$API_AUTH_AUDIENCE\",\"exp\":$((now_unix + 300)),\"iat\":$((now_unix - 10)),\"nbf\":$((now_unix - 10)),\"merchant_id\":\"$claimed_merchant_id\",\"scope\":\"$scope\",\"roles\":$roles_json}")"
    token_signing_input="$token_header.$token_claims"
    token_signature="$(printf '%s' "$token_signing_input" | openssl pkeyutl -sign -inkey "$integration_dir/signing-key.pem" -rawin -in /dev/stdin 2>/dev/null | b64url_stdin)"
    printf 'Bearer %s.%s' "$token_signing_input" "$token_signature"
}

authorization="$(make_authorization integration-user 'tax:quote tax:attestation:verify tax:settlement:compile tax:settlement:submit tax:settlement:read tax:settlement:reconcile' '[]')"

# Exercise the tenant-scoped, read-only vault reconciliation projection. The
# target is synthetic local evidence; the endpoint must still return exact
# decimal-string balances, latest run state, and no browser mutation surface.
reconciliation_target_id=$(docker compose -p "$project_name" exec -T postgres \
    psql -qAt -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    -c "INSERT INTO vault_reconciliation_targets
        (merchant_id, route_id, stream_key, mint, token_program, tax_vault,
         opening_balance_atomic, opening_balance_slot, opening_reference)
       VALUES
        ('$merchant_id',
         '01992f1b-4a70-7e30-8ea2-b6e37a9d9d12',
         'integration/vault-reconciliation',
         'SysvarC1ock11111111111111111111111111111111',
         'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
         'Sysvar1nstructions1111111111111111111111111',
         1000, 1, 'integration-opening-balance-INC-42')
       RETURNING id" | tr -d '[:space:]')
docker compose -p "$project_name" exec -T postgres \
    psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    -c "INSERT INTO vault_reconciliation_runs
        (target_id, route_id, merchant_id, stream_key, mint, token_program,
         tax_vault, opening_balance_atomic, receipt_tax_atomic, release_atomic,
         expected_balance_atomic, observed_balance_atomic, observed_slot, status)
       VALUES
        ('$reconciliation_target_id',
         '01992f1b-4a70-7e30-8ea2-b6e37a9d9d12', '$merchant_id',
         'integration/vault-reconciliation',
         'SysvarC1ock11111111111111111111111111111111',
         'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
         'Sysvar1nstructions1111111111111111111111111',
         1000, 0, 0, 1000, 1000, 100, 'matched')"
reconciliation_authorization="$(make_authorization reconciliation-auditor 'tax:vault:read' '["vault_release_auditor"]')"
reconciliation_targets_response_file="$integration_dir/reconciliation-targets-response.json"
reconciliation_targets_status=$(curl --silent --show-error \
    --output "$reconciliation_targets_response_file" --write-out '%{http_code}' \
    -H "Authorization: $reconciliation_authorization" \
    "$base_url/v1/vault/reconciliation/targets?limit=64")
if [ "$reconciliation_targets_status" != "200" ] || \
    ! grep -q '"opening_balance_atomic":"1000"' "$reconciliation_targets_response_file" || \
    ! grep -q '"status":"matched"' "$reconciliation_targets_response_file"; then
    echo "vault reconciliation target read model failed (HTTP $reconciliation_targets_status)" >&2
    sed -n '1,160p' "$reconciliation_targets_response_file" >&2
    exit 1
fi
reconciliation_runs_status=$(curl --silent --show-error \
    --output "$integration_dir/reconciliation-runs-response.json" --write-out '%{http_code}' \
    -H "Authorization: $reconciliation_authorization" \
    "$base_url/v1/vault/reconciliation/targets/$reconciliation_target_id/runs?limit=64")
if [ "$reconciliation_runs_status" != "200" ] || \
    ! grep -q '"observed_slot":"100"' "$integration_dir/reconciliation-runs-response.json"; then
    echo "vault reconciliation run read model failed (HTTP $reconciliation_runs_status)" >&2
    sed -n '1,160p' "$integration_dir/reconciliation-runs-response.json" >&2
    exit 1
fi

# Exercise the governed vault-release command lifecycle. These API calls only
# persist auditable commands; the local validator conformance suite proves the
# separate Solana registry/claim/CPI behavior on a Linux runner.
release_proposer_authorization="$(make_authorization release-proposer 'tax:vault:propose tax:vault:read' '["vault_release_proposer"]')"
release_approver_authorization="$(make_authorization release-approver 'tax:vault:approve tax:vault:execute tax:vault:read' '["vault_release_approver"]')"
release_operator_authorization="$(make_authorization release-operator 'tax:vault:record tax:vault:read' '["vault_release_operator"]')"
release_claim_mint="So11111111111111111111111111111111111111112"
release_claim_token_program="TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
release_claim_vault="11111111111111111111111111111111"
release_claim_destination="SysvarRent111111111111111111111111111111111"
release_now_unix="$(date +%s)"
release_reference_digest='[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32]'
release_claim_request="{\"idempotency_key\":\"integration-release-01\",\"mint\":\"$release_claim_mint\",\"token_program\":\"$release_claim_token_program\",\"tax_vault\":\"$release_claim_vault\",\"destination\":\"$release_claim_destination\",\"nonce\":\"42\",\"amount_atomic\":\"250000\",\"reference_digest\":$release_reference_digest,\"execute_after_unix\":$((release_now_unix + 60)),\"expires_at_unix\":$((release_now_unix + 3600))}"
release_claim_response_file="$integration_dir/release-claim-response"
release_claim_http_status=$(printf '%s' "$release_claim_request" | curl --silent --show-error \
    --output "$release_claim_response_file" --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $release_proposer_authorization" \
    --data-binary @- "$base_url/v1/vault/release-claims")
if [ "$release_claim_http_status" -lt 200 ] || [ "$release_claim_http_status" -ge 300 ]; then
    echo "vault release proposal returned HTTP $release_claim_http_status" >&2
    sed -n '1,160p' "$release_claim_response_file" >&2
    exit 1
fi
release_claim_response=$(sed -n '1p' "$release_claim_response_file")
release_claim_id=$(printf '%s' "$release_claim_response" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
if [ -z "$release_claim_id" ] || ! printf '%s' "$release_claim_response" | grep -q '"status":"proposed"'; then
    echo "vault release proposal did not return a proposed UUIDv7 claim" >&2
    exit 1
fi
release_claim_replay=$(printf '%s' "$release_claim_request" | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $release_proposer_authorization" \
    --data-binary @- "$base_url/v1/vault/release-claims")
if [ "$release_claim_response" != "$release_claim_replay" ]; then
    echo "vault release idempotent replay returned a different claim" >&2
    exit 1
fi
release_conflict_status=$(printf '%s' "${release_claim_request/250000/250001}" | curl --silent --show-error \
    --output /dev/null --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $release_proposer_authorization" \
    --data-binary @- "$base_url/v1/vault/release-claims")
if [ "$release_conflict_status" != "409" ]; then
    echo "expected vault release idempotency conflict HTTP 409, received $release_conflict_status" >&2
    exit 1
fi
release_same_actor_status=$(printf '%s' '{"to_status":"approved","evidence":{"ticket":"same-actor"},"claim_pda":null,"transaction_signature":null,"slot":null,"last_error_code":null}' | curl --silent --show-error \
    --output /dev/null --write-out '%{http_code}' --request POST \
    --header 'content-type: application/json' \
    --header "authorization: $(make_authorization release-proposer 'tax:vault:approve' '["vault_release_approver"]')" \
    --data-binary @- "$base_url/v1/vault/release-claims/$release_claim_id/transition")
if [ "$release_same_actor_status" != "409" ]; then
    echo "expected same-subject vault release approval to return HTTP 409, received $release_same_actor_status" >&2
    exit 1
fi
release_approve_response=$(printf '%s' '{"to_status":"approved","evidence":{"ticket":"TREASURY-RELEASE-001"},"claim_pda":null,"transaction_signature":null,"slot":null,"last_error_code":null}' | curl --fail --silent --show-error --request POST \
    --header 'content-type: application/json' \
    --header "authorization: $release_approver_authorization" \
    --data-binary @- "$base_url/v1/vault/release-claims/$release_claim_id/transition")
if ! printf '%s' "$release_approve_response" | grep -q '"to_status":"approved"'; then
    echo "vault release approval did not record the approved state" >&2
    exit 1
fi
release_chain_created_response=$(printf '%s' '{"to_status":"onchain_created","evidence":{"signature":"integration-chain-create"},"claim_pda":"8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR","transaction_signature":"5VfY7WmYcR7h9o9E9nP7x5qC2oW6v8m1f3H4jK6pQ8rS","slot":42,"last_error_code":null}' | curl --fail --silent --show-error --request POST \
    --header 'content-type: application/json' \
    --header "authorization: $release_operator_authorization" \
    --data-binary @- "$base_url/v1/vault/release-claims/$release_claim_id/transition")
if ! printf '%s' "$release_chain_created_response" | grep -q '"to_status":"onchain_created"'; then
    echo "vault release chain-created evidence did not transition the claim" >&2
    exit 1
fi
release_executed_response=$(printf '%s' '{"to_status":"executed","evidence":{"finalized":true},"claim_pda":null,"transaction_signature":"5VfY7WmYcR7h9o9E9nP7x5qC2oW6v8m1f3H4jK6pQ8rS","slot":43,"last_error_code":null}' | curl --fail --silent --show-error --request POST \
    --header 'content-type: application/json' \
    --header "authorization: $release_approver_authorization" \
    --data-binary @- "$base_url/v1/vault/release-claims/$release_claim_id/transition")
if ! printf '%s' "$release_executed_response" | grep -q '"to_status":"executed"'; then
    echo "vault release execution evidence did not transition the claim" >&2
    exit 1
fi
release_loaded=$(curl --fail --silent --show-error \
    --header "authorization: $release_approver_authorization" \
    "$base_url/v1/vault/release-claims/$release_claim_id")
if ! printf '%s' "$release_loaded" | grep -q '"status":"executed"'; then
    echo "vault release tenant-scoped read did not return finalized projection" >&2
    exit 1
fi
release_event_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from vault_release_claim_events where claim_id = '$release_claim_id';")
if [ "$release_event_count" != "4" ]; then
    echo "vault release lifecycle did not persist exactly four append-only events" >&2
    exit 1
fi
release_outbox_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from outbox_events where aggregate_type = 'vault_release_claim' and aggregate_id = '$release_claim_id';")
if [ "$release_outbox_count" != "4" ]; then
    echo "vault release lifecycle did not persist exactly four transactional outbox events" >&2
    exit 1
fi
if docker compose -p "$project_name" exec -T postgres \
    psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol -Atc \
    "update vault_release_claims set destination = '11111111111111111111111111111111' where id = '$release_claim_id';" \
    >/dev/null 2>&1; then
    echo "approved vault release claim unexpectedly allowed destination mutation" >&2
    exit 1
fi
if docker compose -p "$project_name" exec -T postgres \
    psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol -Atc \
    "update vault_release_claims set claim_pda = 'SysvarRent111111111111111111111111111111111' where id = '$release_claim_id';" \
    >/dev/null 2>&1; then
    echo "bound vault release claim PDA unexpectedly allowed identity mutation" >&2
    exit 1
fi

request_fixture="$integration_dir/jurisdiction-request.json"
printf '%s' '{"facts":{"gross_atomic":"1100","net_atomic":"1000","currency":"USD","jurisdiction":"US-CA"}}' > "$request_fixture"
issuer_image="$project_name-attestation-fixture"
docker build --file ops/docker/Dockerfile.attestation-fixture --tag "$issuer_image" . >/dev/null
jurisdiction_evidence=$(docker run --rm --read-only --network none --cap-drop ALL \
    --security-opt no-new-privileges:true \
    --volume "$request_fixture:/fixtures/request.json:ro" \
    "$issuer_image" jurisdiction-evidence /fixtures/request.json "$merchant_id" \
    0909090909090909090909090909090909090909090909090909090909090909)
request="{\"idempotency_key\":\"integration-01J8Q4J0M9M2G7YB8F5N3D6K5T\",\"rule_set_id\":\"operator-simulator-flat-rate\",\"rule_version\":1,\"jurisdiction\":\"US-CA\",\"regime\":\"sales-tax\",\"facts\":{\"gross_atomic\":\"1100\",\"net_atomic\":\"1000\",\"currency\":\"USD\",\"jurisdiction\":\"US-CA\"},\"jurisdiction_evidence\":[$jurisdiction_evidence]}"
first_response=$(printf '%s' "$request" | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/tax/quote")

api_headers="$integration_dir/api-headers"
printf '%s' "$request" | curl --fail --silent --show-error --dump-header "$api_headers" --output /dev/null \
    --header 'content-type: application/json' \
    --header 'x-request-id: caller-controlled-value' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/tax/quote"
if ! grep -Eqi '^x-request-id: [0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[0-9a-f]{4}-[0-9a-f]{12}[[:space:]]*$' "$api_headers"; then
    echo "direct API response did not contain an API-owned UUIDv7 request ID" >&2
    cat "$api_headers" >&2
    exit 1
fi
if grep -Fqi 'x-request-id: caller-controlled-value' "$api_headers"; then
    echo "API accepted a caller-controlled request ID" >&2
    exit 1
fi

other_jurisdiction_evidence=$(docker run --rm --read-only --network none --cap-drop ALL \
    --security-opt no-new-privileges:true \
    --volume "$request_fixture:/fixtures/request.json:ro" \
    "$issuer_image" jurisdiction-evidence /fixtures/request.json "$other_merchant_id" \
    0909090909090909090909090909090909090909090909090909090909090909)
cross_tenant_request="{\"idempotency_key\":\"integration-cross-tenant-01J8Q4J0M9M2G7YB8F5N3D6K5U\",\"rule_set_id\":\"operator-simulator-flat-rate\",\"rule_version\":1,\"jurisdiction\":\"US-CA\",\"regime\":\"sales-tax\",\"facts\":{\"gross_atomic\":\"1100\",\"net_atomic\":\"1000\",\"currency\":\"USD\",\"jurisdiction\":\"US-CA\"},\"jurisdiction_evidence\":[$other_jurisdiction_evidence]}"
cross_tenant_quote_status=$(printf '%s' "$cross_tenant_request" | \
    curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $(make_authorization cross-tenant-quote 'tax:quote' '[]' '01992f1b-4a70-7e30-8ea2-b6e37a9d9d12')" \
    --data-binary @- "$base_url/v1/tax/quote")
if [ "$cross_tenant_quote_status" != "404" ]; then
    echo "expected cross-tenant quote rule resolution to return HTTP 404, received $cross_tenant_quote_status" >&2
    exit 1
fi

other_rule_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d14"
docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -v ON_ERROR_STOP=1 -Atc \
    "insert into rule_sets (id, merchant_id, external_id, version, jurisdiction, regime, status, rule_digest, rule_document, provenance, effective_from) values ('$other_rule_id', '$other_merchant_id', 'operator-simulator-flat-rate', 1, 'US-CA', 'sales-tax', 'published', decode(repeat('0a', 32), 'hex'), '{}'::jsonb, '{}'::jsonb, now() - interval '30 seconds');" \
    >/dev/null
same_key_rule_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from rule_sets where external_id = 'operator-simulator-flat-rate' and version = 1;")
if [ "$same_key_rule_count" != "2" ]; then
    echo "expected same rule key to coexist across two merchants, found $same_key_rule_count" >&2
    exit 1
fi

operator_rule_id=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select id from rule_sets where merchant_id = '$merchant_id' and external_id = 'operator-simulator-flat-rate' and version = 1;")
if docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -v ON_ERROR_STOP=1 -Atc \
    "insert into tax_calculations (rule_set_id, merchant_id, facts_digest, engine_version, currency, jurisdiction, total_tax_atomic, total_net_atomic, status, result_document, evidence_refs, expires_at, facts_document) values ('$operator_rule_id', '$other_merchant_id', decode(repeat('0b', 32), 'hex'), 'integration', 'USD', 'US-CA', 1, 1, 'calculated_unattested', '{}'::jsonb, '{}'::jsonb, now() + interval '15 minutes', '{}'::jsonb);" \
    >/dev/null 2>&1; then
    echo "composite rule-owner foreign key unexpectedly allowed a cross-tenant calculation" >&2
    exit 1
fi

calculation_id=$(printf '%s' "$first_response" | sed -n 's/.*"calculation_id":"\([^"]*\)".*/\1/p')
if [ -z "$calculation_id" ]; then
    echo "quote response did not contain a calculation_id" >&2
    exit 1
fi

replay_evidence_response=$(curl --fail --silent --show-error \
    --header "authorization: $authorization" \
    "$base_url/v1/tax/quotes/$calculation_id/replay")
if ! printf '%s' "$replay_evidence_response" | grep -q '"status":"replay_verified"'; then
    echo "persisted quote replay did not verify the complete evidence envelope" >&2
    exit 1
fi
replay_audit_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from evidence_access_audit_events where merchant_id = '$merchant_id' and calculation_id = '$calculation_id' and purpose = 'tax_quote_replay' and outcome = 'allowed';")
if [ "$replay_audit_count" != "1" ]; then
    echo "expected one successful encrypted-evidence access audit event, found $replay_audit_count" >&2
    exit 1
fi
cross_tenant_replay_status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header "authorization: $(make_authorization cross-tenant-replay 'tax:quote' '[]' "$other_merchant_id")" \
    "$base_url/v1/tax/quotes/$calculation_id/replay")
if [ "$cross_tenant_replay_status" != "404" ]; then
    echo "expected cross-tenant quote replay to return HTTP 404, received $cross_tenant_replay_status" >&2
    exit 1
fi
denied_replay_audit_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from evidence_access_audit_events where merchant_id = '$other_merchant_id' and calculation_id = '$calculation_id' and purpose = 'tax_quote_replay' and outcome = 'denied';")
if [ "$denied_replay_audit_count" != "1" ]; then
    echo "expected one denied cross-tenant evidence access audit event, found $denied_replay_audit_count" >&2
    exit 1
fi

route_response=$(curl --fail --silent --show-error \
    --header "authorization: $authorization" \
    "$base_url/v1/settlement/routes")
route_id=$(printf '%s' "$route_response" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
split_plans_response_file="$integration_dir/split-plans-response.json"
split_plans_http_status=$(curl --silent --show-error \
    --output "$split_plans_response_file" --write-out '%{http_code}' \
    --header "authorization: $authorization" \
    "$base_url/v1/settlement/split-plans")
split_plans_response=$(sed -n '1p' "$split_plans_response_file")
if [ "$split_plans_http_status" -lt 200 ] || [ "$split_plans_http_status" -ge 300 ]; then
    echo "authenticated Split Plan discovery failed (HTTP $split_plans_http_status): $split_plans_response" >&2
    exit 1
fi
if ! printf '%s' "$split_plans_response" | grep -q '"id":"01992f1b-4a70-7e30-8ea2-b6e37a9d9d13"'; then
    echo "authenticated Split Plan discovery did not return the tenant-owned active plan" >&2
    exit 1
fi
if ! printf '%s' "$split_plans_response" | grep -q '"basis_points":7000'; then
    echo "Split Plan discovery did not return the canonical recipient allocation" >&2
    exit 1
fi
if [ -z "$route_id" ]; then
    echo "route response did not contain an active route" >&2
    exit 1
fi

settlement_payer="cGfHiC6Kgg3FpFZvgwGcswsCRtp4aBP2fzuXRQPizuN"

pre_attestation_compile_request="{\"calculation_id\":\"$calculation_id\",\"route_id\":\"$route_id\",\"payer\":\"$settlement_payer\",\"nonce\":9,\"policy_sequence\":3,\"policy_digest\":[7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7]}"
pre_attestation_compile_response_file="$integration_dir/pre-attestation-compile-response.json"
pre_attestation_compile_status=$(printf '%s' "$pre_attestation_compile_request" | curl --silent --show-error \
    --output "$pre_attestation_compile_response_file" --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/settlement/compile-from-quote")
if [ "$pre_attestation_compile_status" != "404" ]; then
    echo "expected non-simulation settlement compilation to require an attestation, received HTTP $pre_attestation_compile_status: $(sed -n '1p' "$pre_attestation_compile_response_file")" >&2
    exit 1
fi

quote_fixture="$integration_dir/quote.json"
printf '%s' "$first_response" > "$quote_fixture"
route_mint=$(printf '%s' "$route_response" | sed -n 's/.*"mint":"\([^"]*\)".*/\1/p')
if [ -z "$route_mint" ]; then
    echo "route response did not contain a mint for the external issuer fixture" >&2
    exit 1
fi
issuer_seed="0101010101010101010101010101010101010101010101010101010101010101"
attestation_request=$(docker run --rm --read-only --network none --cap-drop ALL \
    --security-opt no-new-privileges:true \
    --volume "$quote_fixture:/fixtures/quote.json:ro" \
    "$issuer_image" /fixtures/quote.json "$route_mint" "$merchant_id" "$route_id" "$issuer_seed")
attestation_response_file="$integration_dir/attestation-response.json"
attestation_http_status=$(printf '%s' "$attestation_request" | curl --silent --show-error \
    --output "$attestation_response_file" --write-out '%{http_code}' \
    --request POST \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/tax/quotes/$calculation_id/attestation/verify")
attestation_status=$(sed -n '1p' "$attestation_response_file")
if [ "$attestation_http_status" -lt 200 ] || [ "$attestation_http_status" -ge 300 ] || \
    ! printf '%s' "$attestation_status" | grep -q '"status":"attested_unsettled"'; then
    echo "quote-bound attestation ingestion did not return attested_unsettled (HTTP $attestation_http_status)" >&2
    sed -n '1,160p' "$attestation_response_file" >&2
    exit 1
fi
attestation_replay=$(printf '%s' "$attestation_request" | curl --fail --silent --show-error \
    --request POST \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/tax/quotes/$calculation_id/attestation/verify")
if [ "$attestation_status" != "$attestation_replay" ]; then
    echo "identical calculation-attestation ingestion did not replay exactly" >&2
    exit 1
fi

conflicting_attestation_request=$(docker run --rm --read-only --network none --cap-drop ALL \
    --security-opt no-new-privileges:true \
    --volume "$quote_fixture:/fixtures/quote.json:ro" \
    "$issuer_image" /fixtures/quote.json "$route_mint" "$merchant_id" "$route_id" \
    "0202020202020202020202020202020202020202020202020202020202020202")
conflicting_attestation_status=$(printf '%s' "$conflicting_attestation_request" | curl --silent --show-error \
    --output /dev/null --write-out '%{http_code}' \
    --request POST \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/tax/quotes/$calculation_id/attestation/verify")
if [ "$conflicting_attestation_status" != "409" ]; then
    echo "expected conflicting calculation-attestation ingestion to return HTTP 409, received $conflicting_attestation_status" >&2
    exit 1
fi

cross_tenant_attestation_status=$(printf '%s' "$attestation_request" | curl --silent --show-error \
    --output /dev/null --write-out '%{http_code}' \
    --request POST \
    --header 'content-type: application/json' \
    --header "authorization: $(make_authorization cross-tenant-attestation 'tax:attestation:verify' '[]' "$other_merchant_id")" \
    --data-binary @- "$base_url/v1/tax/quotes/$calculation_id/attestation/verify")
if [ "$cross_tenant_attestation_status" != "404" ]; then
    echo "expected cross-tenant attestation ingestion to return HTTP 404, received $cross_tenant_attestation_status" >&2
    exit 1
fi

policy_digest=$(printf '%s' "$attestation_request" | sed -n 's/.*"payload_digest":\[\([^]]*\)\].*/\1/p')
if [ -z "$policy_digest" ]; then
    echo "external issuer fixture did not return a payload digest" >&2
    exit 1
fi
compile_request="{\"calculation_id\":\"$calculation_id\",\"route_id\":\"$route_id\",\"payer\":\"$settlement_payer\",\"nonce\":9,\"policy_sequence\":3,\"policy_digest\":[$policy_digest]}"
compile_response_file="$integration_dir/compile-response.json"
compile_http_status=$(printf '%s' "$compile_request" | curl --silent --show-error \
    --output "$compile_response_file" --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/settlement/compile-from-quote")
compile_response=$(sed -n '1p' "$compile_response_file")
if [ "$compile_http_status" -lt 200 ] || [ "$compile_http_status" -ge 300 ]; then
    echo "quote-bound settlement compilation failed (HTTP $compile_http_status): $compile_response" >&2
    exit 1
fi
if ! printf '%s' "$compile_response" | grep -q '"status":"prepared_unsubmitted"'; then
    echo "quote-bound settlement compilation did not return a prepared instruction" >&2
    exit 1
fi
if ! printf '%s' "$compile_response" | grep -q "\"route_id\":\"$route_id\""; then
    echo "quote-bound settlement compilation did not bind the selected route" >&2
    exit 1
fi
settlement_intent_id=$(printf '%s' "$compile_response" | sed -n 's/.*"intent_id":"\([^"]*\)".*/\1/p')
if [ -z "$settlement_intent_id" ]; then
    echo "quote-bound settlement compilation did not return a durable intent ID" >&2
    exit 1
fi
cross_tenant_intent_status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header "authorization: $(make_authorization cross-tenant-intent 'tax:settlement:read' '[]' "$other_merchant_id")" \
    "$base_url/v1/settlement/intents/$settlement_intent_id")
if [ "$cross_tenant_intent_status" != "404" ]; then
    echo "expected cross-tenant settlement intent lookup to return HTTP 404, received $cross_tenant_intent_status" >&2
    exit 1
fi

split_plan_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d13"
split_compile_request="{\"calculation_id\":\"$calculation_id\",\"route_id\":\"$route_id\",\"split_plan_id\":\"$split_plan_id\",\"payer\":\"$settlement_payer\",\"nonce\":10,\"policy_sequence\":3,\"policy_digest\":[$policy_digest]}"
split_compile_response_file="$integration_dir/split-compile-response.json"
split_compile_http_status=$(printf '%s' "$split_compile_request" | curl --silent --show-error \
    --output "$split_compile_response_file" --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/settlement/split/compile-from-quote")
split_compile_response=$(sed -n '1p' "$split_compile_response_file")
if [ "$split_compile_http_status" -lt 200 ] || [ "$split_compile_http_status" -ge 300 ]; then
    echo "authenticated persisted Split Plan compilation failed (HTTP $split_compile_http_status): $split_compile_response" >&2
    exit 1
fi
if ! printf '%s' "$split_compile_response" | grep -q '"status":"prepared_unsubmitted"'; then
    echo "authenticated persisted Split Plan compilation did not return a prepared instruction" >&2
    exit 1
fi
if ! printf '%s' "$split_compile_response" | grep -q "\"split_plan_id\":\"$split_plan_id\""; then
    echo "split compilation did not bind the persisted Split Plan ID" >&2
    exit 1
fi
if ! printf '%s' "$split_compile_response" | grep -q '"split_plan_digest":\[248'; then
    echo "split compilation did not expose the canonical server-owned plan digest" >&2
    exit 1
fi

submission_drill_output=$(API_BASE="$base_url" \
    API_BASE_SECONDARY="$replica_base_url" \
    RPC_FIXTURE_URL="http://127.0.0.1:${RPC_FIXTURE_PORT}" \
    AUTHORIZATION="$authorization" \
    CALCULATION_ID="$calculation_id" \
    ROUTE_ID="$route_id" \
    POLICY_DIGEST_JSON="[$policy_digest]" \
    pnpm --dir apps/web exec node scripts/settlement_submission_fence_drill.mjs)
printf '%s\n' "$submission_drill_output"
if ! printf '%s' "$submission_drill_output" | grep -q '"send_transaction_calls":1'; then
    echo "durable settlement submission fence did not prove exactly one RPC send" >&2
    exit 1
fi
if ! printf '%s' "$submission_drill_output" | grep -q '"durable_status":"submitted"'; then
    echo "durable settlement submission fence did not converge to submitted" >&2
    exit 1
fi
if ! printf '%s' "$submission_drill_output" | grep -q '"replicas":2'; then
    echo "settlement submission fence did not exercise the independent API replica" >&2
    exit 1
fi

# The cross-replica assertion is complete. Remove the ad-hoc second process
# before the later crash-recovery scenario so Compose has no orphan warning and
# the crash drill has one unambiguous primary process to interrupt.
docker rm --force "$replica_container_id" >/dev/null
replica_container_id=""

ambiguous_reconciliation_output=$(API_BASE="$base_url" \
    RPC_FIXTURE_URL="http://127.0.0.1:${RPC_FIXTURE_PORT}" \
    AUTHORIZATION="$authorization" \
    CALCULATION_ID="$calculation_id" \
    ROUTE_ID="$route_id" \
    POLICY_DIGEST_JSON="[$policy_digest]" \
    DRILL_SCENARIO="ambiguous-reconcile" \
    NONCE="13" \
    pnpm --dir apps/web exec node scripts/settlement_submission_fence_drill.mjs)
printf '%s\n' "$ambiguous_reconciliation_output"
if ! printf '%s' "$ambiguous_reconciliation_output" | grep -q '"durable_status_before_reconcile":"unknown"'; then
    echo "ambiguous settlement submission did not persist unknown state" >&2
    exit 1
fi
if ! printf '%s' "$ambiguous_reconciliation_output" | grep -q '"reconciled_status":"confirmed"'; then
    echo "ambiguous settlement submission did not reconcile to confirmed" >&2
    exit 1
fi
if ! printf '%s' "$ambiguous_reconciliation_output" | grep -q '"send_transaction_calls":2'; then
    echo "signature reconciliation unexpectedly resubmitted the transaction" >&2
    exit 1
fi

process_crash_output=$(API_BASE="$base_url" \
    RPC_FIXTURE_URL="http://127.0.0.1:${RPC_FIXTURE_PORT}" \
    AUTHORIZATION="$authorization" \
    CALCULATION_ID="$calculation_id" \
    ROUTE_ID="$route_id" \
    POLICY_DIGEST_JSON="[$policy_digest]" \
    COMPOSE_PROJECT="$project_name" \
    NONCE="14" \
    pnpm --dir apps/web exec node scripts/settlement_process_crash_drill.mjs)
printf '%s\n' "$process_crash_output"
if ! printf '%s' "$process_crash_output" | grep -q '"status_before_crash":"submitting"'; then
    echo "process-crash drill did not kill the API after durable submission claim" >&2
    exit 1
fi
if ! printf '%s' "$process_crash_output" | grep -q '"recovered_status":"submitting"'; then
    echo "process-crash drill did not recover the durable submitting intent" >&2
    exit 1
fi
if ! printf '%s' "$process_crash_output" | grep -q '"reconciled_status":"confirmed"'; then
    echo "process-crash drill did not reconcile the recovered intent" >&2
    exit 1
fi
if ! printf '%s' "$process_crash_output" | grep -q '"send_transaction_calls":3'; then
    echo "process-crash recovery unexpectedly resubmitted the transaction" >&2
    exit 1
fi

expired_calculation_id=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "insert into tax_calculations (
         id, correlation_id, rule_set_id, facts_digest, engine_version,
         currency, jurisdiction, total_tax_atomic, total_net_atomic, status,
         result_document, evidence_refs, created_at, expires_at, merchant_id,
         facts_document, facts_ciphertext, facts_nonce, facts_key_id
     )
     select uuidv7(), uuidv7(), rule_set_id, facts_digest, engine_version,
            currency, jurisdiction, total_tax_atomic, total_net_atomic, status,
            result_document, evidence_refs, now() - interval '2 minutes',
            now() - interval '1 second', merchant_id, facts_document,
            facts_ciphertext, facts_nonce, facts_key_id
       from tax_calculations
      where id = '$calculation_id'
     returning id;")
expired_calculation_id=$(printf '%s\n' "$expired_calculation_id" | sed -n '1p')
if [ -z "$expired_calculation_id" ]; then
    echo "failed to seed an evidence-backed expired quote" >&2
    exit 1
fi
expired_compile_request=$(printf '%s' "$compile_request" \
    | sed "s/\"calculation_id\":\"$calculation_id\"/\"calculation_id\":\"$expired_calculation_id\"/" \
    | sed 's/"nonce":9/"nonce":11/')
expired_compile_status=$(printf '%s' "$expired_compile_request" | curl --silent --show-error \
    --output /dev/null --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/settlement/compile-from-quote")
if [ "$expired_compile_status" != "409" ]; then
    echo "expected expired quote settlement compilation to return HTTP 409, received $expired_compile_status" >&2
    exit 1
fi

unknown_field_request=$(printf '%s' "$compile_request" | sed 's/}$/,"merchant":"11111111111111111111111111111111"}/')
unknown_field_status=$(printf '%s' "$unknown_field_request" | curl --silent --show-error \
    --output /dev/null --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/settlement/compile-from-quote")
if [ "$unknown_field_status" != "400" ]; then
    echo "expected free-form settlement account field to be rejected with HTTP 400, received $unknown_field_status" >&2
    exit 1
fi

replay_response=$(printf '%s' "$request" | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/tax/quote")

if [ "$first_response" != "$replay_response" ]; then
    echo "idempotent replay returned a different response" >&2
    exit 1
fi

edge_response_file="$integration_dir/edge-response"
edge_status=$(printf '%s' "$request" | curl --silent --show-error --output "$edge_response_file" --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$edge_url/api/v1/tax/quote")
if [ "$edge_status" -lt 200 ] || [ "$edge_status" -ge 300 ]; then
    echo "NGINX quote proxy returned HTTP $edge_status" >&2
    sed -n '1,160p' "$edge_response_file" >&2 || true
    docker compose -p "$project_name" logs --no-color api edge >&2 || true
    exit 1
fi
edge_response=$(sed -n '1p' "$edge_response_file")
if [ "$edge_response" != "$first_response" ]; then
    echo "NGINX response differs from direct API response" >&2
    exit 1
fi

edge_headers="$integration_dir/edge-headers"
curl --fail --silent --show-error --dump-header "$edge_headers" --output /dev/null \
    --header 'x-request-id: caller-controlled-value' \
    "$edge_url/api/healthz"
if ! grep -Eqi '^x-request-id: [0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[0-9a-f]{4}-[0-9a-f]{12}[[:space:]]*$' "$edge_headers"; then
    echo "NGINX edge response did not contain an API-owned UUIDv7 request ID" >&2
    cat "$edge_headers" >&2
    exit 1
fi
if grep -Fqi 'x-request-id: caller-controlled-value' "$edge_headers"; then
    echo "NGINX edge forwarded a caller-controlled request ID" >&2
    exit 1
fi
expected_csp="content-security-policy: default-src 'self'; connect-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'; base-uri 'none'; object-src 'none'; form-action 'self'; frame-ancestors 'none'"
for required_header in \
    'x-content-type-options: nosniff' \
    'x-frame-options: DENY' \
    'referrer-policy: strict-origin-when-cross-origin' \
    'permissions-policy: camera=(), microphone=(), geolocation=()' \
    "$expected_csp"
do
    if ! grep -Fqi "${required_header}" "$edge_headers"; then
        echo "required NGINX security header is missing: $required_header" >&2
        cat "$edge_headers" >&2
        exit 1
    fi
done

unauthenticated_status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header 'content-type: application/json' --data-binary @- \
    "$edge_url/api/v1/tax/quote" <<'EOF'
{}
EOF
)
if [ "$unauthenticated_status" != "401" ]; then
    echo "expected unauthenticated quote request to return HTTP 401, received $unauthenticated_status" >&2
    exit 1
fi

cross_origin_headers="$integration_dir/cross-origin-headers"
curl --silent --show-error --dump-header "$cross_origin_headers" --output /dev/null \
    --header 'Origin: https://attacker.invalid' \
    "$edge_url/healthz"
if grep -Eiq '^access-control-allow-origin:' "$cross_origin_headers"; then
    echo "unexpected permissive CORS header was emitted for an untrusted origin" >&2
    cat "$cross_origin_headers" >&2
    exit 1
fi

# Exercise the edge admission boundary with a concurrent burst. This must
# produce explicit 429 responses before the API receives all requests; it is
# intentionally separate from the tenant-aware authorization and idempotency
# assertions above.
rate_statuses="$integration_dir/rate-limit-statuses"
export edge_admission_url="$edge_url/api/healthz"
seq 1 80 | xargs -P 80 -n 1 sh -c \
    'curl --silent --show-error --max-time 5 --output /dev/null --write-out "%{http_code}\n" "$edge_admission_url"' \
    sh > "$rate_statuses"
rate_limited_count=$(grep -c '^429$' "$rate_statuses" || true)
if [ "$rate_limited_count" -lt 1 ]; then
    echo "expected the NGINX burst admission test to produce at least one HTTP 429" >&2
    cat "$rate_statuses" >&2
    exit 1
fi

conflict_request_fixture="$integration_dir/conflict-jurisdiction-request.json"
printf '%s' '{"facts":{"gross_atomic":"1200","net_atomic":"1000","currency":"USD","jurisdiction":"US-CA"}}' > "$conflict_request_fixture"
conflict_jurisdiction_evidence=$(docker run --rm --read-only --network none --cap-drop ALL \
    --security-opt no-new-privileges:true \
    --volume "$conflict_request_fixture:/fixtures/request.json:ro" \
    "$issuer_image" jurisdiction-evidence /fixtures/request.json "$merchant_id" \
    0909090909090909090909090909090909090909090909090909090909090909)
conflict_request="{\"idempotency_key\":\"integration-01J8Q4J0M9M2G7YB8F5N3D6K5T\",\"rule_set_id\":\"operator-simulator-flat-rate\",\"rule_version\":1,\"jurisdiction\":\"US-CA\",\"regime\":\"sales-tax\",\"facts\":{\"gross_atomic\":\"1200\",\"net_atomic\":\"1000\",\"currency\":\"USD\",\"jurisdiction\":\"US-CA\"},\"jurisdiction_evidence\":[$conflict_jurisdiction_evidence]}"
conflict_status=$(printf '%s' "$conflict_request" | curl --silent --show-error \
    --output /dev/null --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/tax/quote")
if [ "$conflict_status" != "409" ]; then
    echo "expected idempotency conflict HTTP 409, received $conflict_status" >&2
    exit 1
fi

stored_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from tax_calculations where status in ('calculated_unattested', 'attested');")
if [ "$stored_count" != "2" ]; then
    echo "expected two persisted calculations (one attested and one expired evidence fixture), found $stored_count" >&2
    exit 1
fi

facts_ciphertext_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from tax_calculations where merchant_id = '$merchant_id' and status in ('calculated_unattested', 'attested') and facts_ciphertext is not null and facts_nonce is not null and facts_key_id is not null;")
if [ "$facts_ciphertext_count" != "2" ]; then
    echo "expected two encrypted normalized facts records, found $facts_ciphertext_count" >&2
    exit 1
fi
plaintext_facts_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from tax_calculations where merchant_id = '$merchant_id' and status in ('calculated_unattested', 'attested') and facts_document is not null;")
if [ "$plaintext_facts_count" != "0" ]; then
    echo "new quote path unexpectedly persisted plaintext normalized facts: $plaintext_facts_count" >&2
    exit 1
fi

attestation_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from tax_calculation_attestations where merchant_id = '$merchant_id' and calculation_id = '$calculation_id';")
if [ "$attestation_count" != "1" ]; then
    echo "expected one immutable tenant-scoped calculation attestation, found $attestation_count" >&2
    exit 1
fi

attested_outbox_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from outbox_events where event_type='tax.calculation.attested' and aggregate_id = '$calculation_id';")
if [ "$attested_outbox_count" != "1" ]; then
    echo "expected one transactional attestation outbox event, found $attested_outbox_count" >&2
    exit 1
fi

route_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from settlement_routes where merchant_id = '$merchant_id' and status = 'active';")
if [ "$route_count" != "1" ]; then
    echo "expected one active tenant settlement route, found $route_count" >&2
    exit 1
fi

split_plan_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from settlement_split_plans where merchant_id = '$merchant_id' and status = 'active';")
if [ "$split_plan_count" != "1" ]; then
    echo "expected one active tenant settlement Split Plan, found $split_plan_count" >&2
    exit 1
fi

route_verification_tables=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from pg_class where relkind='r' and relname='settlement_route_verifications';")
if [ "$route_verification_tables" != "1" ]; then
    echo "expected settlement route verification evidence table, found $route_verification_tables" >&2
    exit 1
fi

route_evidence_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from settlement_routes where merchant_id = '$merchant_id' and status = 'active' and verification_commitment = 'local_synthetic' and verification_evidence is not null;")
if [ "$route_evidence_count" != "1" ]; then
    echo "expected one explicitly synthetic local route evidence record, found $route_evidence_count" >&2
    exit 1
fi

quote_expiry_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from tax_calculations where merchant_id is not null and expires_at > now();")
if [ "$quote_expiry_count" != "1" ]; then
    echo "expected one tenant-scoped unexpired quote, found $quote_expiry_count" >&2
    exit 1
fi

outbox_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from outbox_events where event_type='tax.calculation.created';")
if [ "$outbox_count" != "1" ]; then
    echo "expected one transactional quote outbox event, found $outbox_count" >&2
    exit 1
fi

indexer_tables=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from pg_class where relkind='r' and relname in ('chain_checkpoints','indexed_receipts','indexed_split_settlements','raw_chain_evidence','raw_chain_transactions','reconciliation_runs','indexer_replay_gaps','replay_gap_operator_actions','settlement_intents','settlement_intent_events','evidence_access_audit_events');")
if [ "$indexer_tables" != "11" ]; then
    echo "expected finalized indexer and settlement intent tables, found $indexer_tables" >&2
    exit 1
fi

replay_gap_id=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "insert into indexer_replay_gaps (stream_key, start_slot, end_slot, observed_finalized_slot, reason, attempts, status, last_error_code, dead_lettered_at) values ('integration/settlement', 900, 901, 901, 'operator_requested', 8, 'dead_lettered', 'integration_retry_exhausted', now()) returning id;")
replay_gap_id=$(printf '%s\n' "$replay_gap_id" | sed -n '1p')
if [ -z "$replay_gap_id" ]; then
    echo "expected a seeded dead-lettered replay gap" >&2
    exit 1
fi

replay_operator_authorization="$(make_authorization integration-indexer 'tax:indexer:read tax:indexer:operate' '["indexer_operator"]')"
replay_gaps_response=$(curl --fail --silent --show-error \
    --header "authorization: $replay_operator_authorization" \
    "$base_url/v1/indexer/replay-gaps?stream_key=integration%2Fsettlement&status=dead_lettered&limit=10")
if ! printf '%s' "$replay_gaps_response" | grep -q "\"id\":\"$replay_gap_id\""; then
    echo "replay-gap list did not return the seeded dead-lettered gap" >&2
    exit 1
fi

requeue_before_ack_status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --request POST \
    --header 'content-type: application/json' \
    --header "authorization: $replay_operator_authorization" \
    --data-binary '{"note":"requeue-before-ack"}' \
    "$base_url/v1/indexer/replay-gaps/$replay_gap_id/requeue")
if [ "$requeue_before_ack_status" != "409" ]; then
    echo "expected requeue-before-ack to return HTTP 409, received $requeue_before_ack_status" >&2
    exit 1
fi

acknowledge_response=$(printf '%s' '{"note":"incident INC-REPLAY-001 reviewed by operator"}' | \
    curl --fail --silent --show-error \
    --request POST \
    --header 'content-type: application/json' \
    --header "authorization: $replay_operator_authorization" \
    --data-binary @- "$base_url/v1/indexer/replay-gaps/$replay_gap_id/acknowledge")
if ! printf '%s' "$acknowledge_response" | grep -q '"status":"dead_lettered"'; then
    echo "replay-gap acknowledgement changed the dead-letter state" >&2
    exit 1
fi

operator_action_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from replay_gap_operator_actions where gap_id = '$replay_gap_id' and action = 'acknowledge';")
if [ "$operator_action_count" != "1" ]; then
    echo "expected one replay-gap acknowledgement action, found $operator_action_count" >&2
    exit 1
fi

requeue_response=$(printf '%s' '{"note":"INC-REPLAY-001 repair window approved"}' | \
    curl --fail --silent --show-error \
    --request POST \
    --header 'content-type: application/json' \
    --header "authorization: $replay_operator_authorization" \
    --data-binary @- "$base_url/v1/indexer/replay-gaps/$replay_gap_id/requeue")
if ! printf '%s' "$requeue_response" | grep -q '"status":"queued"'; then
    echo "replay-gap requeue did not return queued state" >&2
    exit 1
fi

operator_action_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from replay_gap_operator_actions where gap_id = '$replay_gap_id';")
if [ "$operator_action_count" != "2" ]; then
    echo "expected acknowledgement and requeue action evidence, found $operator_action_count" >&2
    exit 1
fi

if docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -v ON_ERROR_STOP=1 -Atc \
    "update replay_gap_operator_actions set note='tampered' where gap_id = '$replay_gap_id';" \
    >/dev/null 2>&1; then
    echo "replay-gap operator action mutation unexpectedly succeeded" >&2
    exit 1
fi

requeue_without_new_ack_status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --request POST \
    --header 'content-type: application/json' \
    --header "authorization: $replay_operator_authorization" \
    --data-binary '{"note":"requeue-again"}' \
    "$base_url/v1/indexer/replay-gaps/$replay_gap_id/requeue")
if [ "$requeue_without_new_ack_status" != "409" ]; then
    echo "expected requeue without fresh acknowledgement to return HTTP 409, received $requeue_without_new_ack_status" >&2
    exit 1
fi

published_count="0"
for _attempt in $(seq 1 30); do
    published_count=$(docker compose -p "$project_name" exec -T postgres \
        psql -U tax_protocol -d tax_protocol -Atc \
        "select count(*) from outbox_events where event_type in ('tax.calculation.created', 'tax.calculation.attested') and published_at is not null;")
    if [ "$published_count" = "2" ]; then
        break
    fi
    sleep 1
done
if [ "$published_count" != "2" ]; then
    echo "expected two JetStream-acknowledged calculation outbox events, found $published_count" >&2
    docker compose -p "$project_name" logs --no-color worker nats
    exit 1
fi

if docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -v ON_ERROR_STOP=1 -Atc \
    "update rule_sets set rule_document='{}'::jsonb where status='published';" \
    >/dev/null 2>&1; then
    echo "published rule mutation unexpectedly succeeded" >&2
    exit 1
fi

if docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -v ON_ERROR_STOP=1 -Atc \
    "insert into rule_sets (merchant_id, external_id, version, jurisdiction, regime, status, rule_digest, rule_document, provenance, effective_from) values ('$merchant_id', 'operator-simulator-flat-rate', 2, 'US-CA', 'sales-tax', 'published', decode(repeat('08', 32), 'hex'), '{}'::jsonb, '{}'::jsonb, now() - interval '30 seconds');" \
    >/dev/null 2>&1; then
    echo "overlapping published rule window unexpectedly succeeded" >&2
    exit 1
fi

draft_idempotency_key="integration-rule-draft-01"
draft_effective_from="$(date +%s)"
draft_body="{\"idempotency_key\":\"$draft_idempotency_key\",\"external_id\":\"integration-governed-rule\",\"version\":1,\"jurisdiction\":\"US-CA\",\"regime\":\"sales-tax\",\"effective_from_unix\":$draft_effective_from,\"effective_until_unix\":null,\"rule_set\":{\"id\":\"integration-governed-rule\",\"version\":1,\"components\":[{\"id\":\"sales-tax\",\"label\":\"Sales tax\",\"base\":{\"sources\":[\"Net\"]},\"rate\":{\"Flat\":{\"numerator\":\"10\",\"denominator\":\"100\"}},\"price_mode\":\"Exclusive\",\"credit_atomic\":\"0\",\"floor_atomic\":null,\"cap_atomic\":null,\"rounding\":{\"mode\":\"HalfUp\"},\"threshold\":null,\"exemption_codes\":[]}]},\"provenance\":{\"source\":\"integration-test\",\"ticket\":\"RULE-001\"}}"
authorization="$(make_authorization integration-author 'tax:rule:author' '["tax_rule_author"]')"
draft_response_file="$integration_dir/draft-response"
draft_status=$(printf '%s' "$draft_body" | curl --silent --show-error --output "$draft_response_file" --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/tax/rules")
if [ "$draft_status" -lt 200 ] || [ "$draft_status" -ge 300 ]; then
    echo "rule draft endpoint returned HTTP $draft_status" >&2
    sed -n '1,160p' "$draft_response_file" >&2 || true
    docker compose -p "$project_name" logs --no-color api >&2 || true
    exit 1
fi
created_draft_response=$(sed -n '1p' "$draft_response_file")
if ! printf '%s' "$created_draft_response" | grep -q '"status":"draft"'; then
    echo "rule draft endpoint did not return a server-owned draft" >&2
    exit 1
fi
replayed_draft_response=$(printf '%s' "$draft_body" | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/tax/rules")
if [ "$created_draft_response" != "$replayed_draft_response" ]; then
    echo "rule draft idempotency replay returned different evidence" >&2
    exit 1
fi
governed_rule_id=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select id from rule_sets where merchant_id = '$merchant_id' and external_id = 'integration-governed-rule' and version = 1;")
if [ -z "$governed_rule_id" ]; then
    echo "rule draft was not persisted under the authenticated merchant" >&2
    exit 1
fi

simulation_authorization="$(make_authorization integration-author 'tax:rule:read' '["tax_rule_author"]')"
simulation_request='{"facts":{"gross_atomic":"1100","net_atomic":"1000","currency":"USD","jurisdiction":"US-CA"}}'
simulation_response_file="$integration_dir/rule-simulation-response"
simulation_status=$(printf '%s' "$simulation_request" | curl --silent --show-error \
    --output "$simulation_response_file" --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $simulation_authorization" \
    --data-binary @- "$base_url/v1/tax/rules/$governed_rule_id/simulate")
if [ "$simulation_status" != "200" ] \
    || ! grep -q '"status":"simulated_unattested"' "$simulation_response_file" \
    || ! grep -q '"rule_set_id":"integration-governed-rule"' "$simulation_response_file"; then
    echo "server-owned persisted-rule simulation did not return the expected trace" >&2
    sed -n '1,160p' "$simulation_response_file" >&2 || true
    exit 1
fi

simulation_jurisdiction_status=$(printf '%s' \
    '{"facts":{"gross_atomic":"1100","net_atomic":"1000","currency":"USD","jurisdiction":"GB"}}' | \
    curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $simulation_authorization" \
    --data-binary @- "$base_url/v1/tax/rules/$governed_rule_id/simulate")
if [ "$simulation_jurisdiction_status" != "422" ]; then
    echo "expected persisted-rule simulation jurisdiction mismatch to return HTTP 422, received $simulation_jurisdiction_status" >&2
    exit 1
fi

simulation_unknown_field_status=$(printf '%s' \
    '{"facts":{"gross_atomic":"1100","net_atomic":"1000","currency":"USD","jurisdiction":"US-CA","unsupported":"must-reject"}}' | \
    curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $simulation_authorization" \
    --data-binary @- "$base_url/v1/tax/rules/$governed_rule_id/simulate")
if [ "$simulation_unknown_field_status" != "400" ]; then
    echo "expected persisted-rule simulation unknown-field rejection to return HTTP 400, received $simulation_unknown_field_status" >&2
    exit 1
fi

cross_tenant_simulation_status=$(printf '%s' "$simulation_request" | curl --silent --show-error \
    --output /dev/null --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $(make_authorization cross-tenant-reader 'tax:rule:read' '["tax_rule_auditor"]' "$other_merchant_id")" \
    --data-binary @- "$base_url/v1/tax/rules/$governed_rule_id/simulate")
if [ "$cross_tenant_simulation_status" != "404" ]; then
    echo "expected cross-tenant persisted-rule simulation to return HTTP 404, received $cross_tenant_simulation_status" >&2
    exit 1
fi

# Deliberately corrupt the local draft row to prove digest/document validation
# fails closed, then restore the exact canonical envelope before lifecycle tests.
docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -v ON_ERROR_STOP=1 -Atc \
    "update rule_sets set rule_document='{}'::jsonb where id = '$governed_rule_id';" \
    >/dev/null
simulation_tamper_status=$(printf '%s' "$simulation_request" | curl --silent --show-error \
    --output /dev/null --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $simulation_authorization" \
    --data-binary @- "$base_url/v1/tax/rules/$governed_rule_id/simulate")
if [ "$simulation_tamper_status" != "500" ]; then
    echo "expected tampered persisted-rule simulation to return HTTP 500, received $simulation_tamper_status" >&2
    exit 1
fi
draft_rule_document='{"schema":"solana-tax-protocol/tax-rule/v1","rule_set":{"id":"integration-governed-rule","version":1,"components":[{"id":"sales-tax","label":"Sales tax","base":{"sources":["Net"]},"rate":{"Flat":{"numerator":"10","denominator":"100"}},"price_mode":"Exclusive","credit_atomic":"0","floor_atomic":null,"cap_atomic":null,"rounding":{"mode":"HalfUp"},"threshold":null,"exemption_codes":[]}]}}'
docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -v ON_ERROR_STOP=1 -Atc \
    "update rule_sets set rule_document='$draft_rule_document'::jsonb where id = '$governed_rule_id';" \
    >/dev/null
simulation_recovery_status=$(printf '%s' "$simulation_request" | curl --silent --show-error \
    --output /dev/null --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $simulation_authorization" \
    --data-binary @- "$base_url/v1/tax/rules/$governed_rule_id/simulate")
if [ "$simulation_recovery_status" != "200" ]; then
    echo "persisted-rule simulation did not recover after restoring the canonical document" >&2
    exit 1
fi

if docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -v ON_ERROR_STOP=1 -Atc \
    "update rule_sets set merchant_id = '$other_merchant_id' where id = '$governed_rule_id';" \
    >/dev/null 2>&1; then
    echo "draft rule ownership unexpectedly changed across merchants" >&2
    exit 1
fi

cross_tenant_status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header "authorization: $(make_authorization cross-tenant-reader 'tax:rule:read' '["tax_rule_auditor"]' "$other_merchant_id")" \
    "$base_url/v1/tax/rules/$governed_rule_id/governance")
if [ "$cross_tenant_status" != "404" ]; then
    echo "expected cross-tenant governance read to return HTTP 404, received $cross_tenant_status" >&2
    exit 1
fi

cross_tenant_transition_status=$(printf '%s' '{"to_status":"simulated","evidence":{"source":"cross-tenant-test"}}' | \
    curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $(make_authorization cross-tenant-author 'tax:rule:author' '["tax_rule_author"]' "$other_merchant_id")" \
    --data-binary @- "$base_url/v1/tax/rules/$governed_rule_id/transition")
if [ "$cross_tenant_transition_status" != "404" ]; then
    echo "expected cross-tenant governance transition to return HTTP 404, received $cross_tenant_transition_status" >&2
    exit 1
fi

governance_body='{"to_status":"simulated","evidence":{"source":"integration-test","review_ticket":"GOV-001"}}'
authorization="$(make_authorization integration-author 'tax:rule:author' '["tax_rule_author"]')"
printf '%s' "$governance_body" | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/tax/rules/$governed_rule_id/transition" \
    | grep -q '"to_status":"simulated"'

governance_read_response=$(curl --fail --silent --show-error \
    --header "authorization: $(make_authorization integration-author 'tax:rule:read' '["tax_rule_author"]')" \
    "$base_url/v1/tax/rules/$governed_rule_id/governance")
if ! printf '%s' "$governance_read_response" | grep -q '"status":"simulated"'; then
    echo "governance read endpoint did not return the server-owned lifecycle state" >&2
    exit 1
fi
if ! printf '%s' "$governance_read_response" | grep -q '"to_status":"simulated"'; then
    echo "governance read endpoint did not return append-only transition evidence" >&2
    exit 1
fi

governance_body='{"to_status":"reviewed","evidence":{"source":"integration-test"}}'
authorization="$(make_authorization integration-author 'tax:rule:review' '["tax_rule_reviewer"]')"
same_author_status=$(printf '%s' "$governance_body" | curl --silent --show-error \
    --output /dev/null --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/tax/rules/$governed_rule_id/transition")
if [ "$same_author_status" != "409" ]; then
    echo "expected same-subject rule review to be rejected with HTTP 409, received $same_author_status" >&2
    exit 1
fi

authorization="$(make_authorization integration-reviewer 'tax:rule:review' '["tax_rule_reviewer"]')"
printf '%s' "$governance_body" | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/tax/rules/$governed_rule_id/transition" \
    | grep -q '"to_status":"reviewed"'

governance_body='{"to_status":"approved","evidence":{"source":"integration-test","approval":"recorded"}}'
authorization="$(make_authorization integration-approver 'tax:rule:approve' '["tax_rule_approver"]')"
printf '%s' "$governance_body" | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/tax/rules/$governed_rule_id/transition" \
    | grep -q '"to_status":"approved"'

governance_body='{"to_status":"published","evidence":{"source":"integration-test","release":"recorded"}}'
authorization="$(make_authorization integration-approver 'tax:rule:publish' '["tax_rule_approver"]')"
printf '%s' "$governance_body" | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $authorization" \
    --data-binary @- "$base_url/v1/tax/rules/$governed_rule_id/transition" \
    | grep -q '"to_status":"published"'

governance_event_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from rule_governance_events where rule_set_id = '$governed_rule_id';")
if [ "$governance_event_count" != "5" ]; then
    echo "expected five append-only rule governance events before policy binding, found $governance_event_count" >&2
    exit 1
fi

if docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -v ON_ERROR_STOP=1 -Atc \
    "update rule_governance_events set evidence='{}'::jsonb where rule_set_id = '$governed_rule_id';" \
    >/dev/null 2>&1; then
    echo "rule governance event mutation unexpectedly succeeded" >&2
    exit 1
fi

policy_effective_from="$(date +%s)"
policy_body="{\"idempotency_key\":\"integration-jurisdiction-policy-01\",\"rule_set_id\":\"$governed_rule_id\",\"policy\":{\"id\":\"integration-policy\",\"version\":1,\"ordered_sources\":[\"ShippingDestination\",\"CustomerResidence\",\"MerchantRegistration\"],\"conflict_mode\":\"HighestPrecedence\"},\"effective_from_unix\":$policy_effective_from,\"effective_until_unix\":null,\"provenance\":{\"source\":\"integration-test\",\"ticket\":\"JURISDICTION-001\"}}"
policy_authorization="$(make_authorization policy-author 'tax:jurisdiction:author' '["tax_jurisdiction_author"]')"
policy_response_file="$integration_dir/jurisdiction-policy-response"
policy_status=$(printf '%s' "$policy_body" | curl --silent --show-error --output "$policy_response_file" --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $policy_authorization" \
    --data-binary @- "$base_url/v1/tax/jurisdiction-policies")
if [ "$policy_status" -lt 200 ] || [ "$policy_status" -ge 300 ]; then
    echo "jurisdiction policy draft endpoint returned HTTP $policy_status" >&2
    sed -n '1,160p' "$policy_response_file" >&2 || true
    exit 1
fi
created_policy_response=$(sed -n '1p' "$policy_response_file")
if ! printf '%s' "$created_policy_response" | grep -q '"status":"draft"'; then
    echo "jurisdiction policy draft endpoint did not return a server-owned draft" >&2
    exit 1
fi
replayed_policy_response=$(printf '%s' "$policy_body" | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $policy_authorization" \
    --data-binary @- "$base_url/v1/tax/jurisdiction-policies")
if [ "$created_policy_response" != "$replayed_policy_response" ]; then
    echo "jurisdiction policy idempotency replay returned different evidence" >&2
    exit 1
fi
jurisdiction_policy_id=$(printf '%s' "$created_policy_response" | sed -n 's/.*"policy_id":"\([^"]*\)".*/\1/p')
if [ -z "$jurisdiction_policy_id" ]; then
    echo "jurisdiction policy response did not contain a policy_id" >&2
    exit 1
fi

policy_transition_body='{"to_status":"reviewed","evidence":{"source":"integration-test","review_ticket":"JURISDICTION-GOV-001"}}'
same_policy_author_status=$(printf '%s' "$policy_transition_body" | curl --silent --show-error \
    --output /dev/null --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $(make_authorization policy-author 'tax:jurisdiction:review' '["tax_jurisdiction_reviewer"]')" \
    --data-binary @- "$base_url/v1/tax/jurisdiction-policies/$jurisdiction_policy_id/transition")
if [ "$same_policy_author_status" != "409" ]; then
    echo "expected same-subject jurisdiction policy review to be rejected with HTTP 409, received $same_policy_author_status" >&2
    exit 1
fi

printf '%s' "$policy_transition_body" | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $(make_authorization policy-reviewer 'tax:jurisdiction:review' '["tax_jurisdiction_reviewer"]')" \
    --data-binary @- "$base_url/v1/tax/jurisdiction-policies/$jurisdiction_policy_id/transition" \
    | grep -q '"to_status":"reviewed"'

policy_transition_body='{"to_status":"approved","evidence":{"source":"integration-test","approval":"recorded"}}'
printf '%s' "$policy_transition_body" | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $(make_authorization policy-approver 'tax:jurisdiction:approve' '["tax_jurisdiction_approver"]')" \
    --data-binary @- "$base_url/v1/tax/jurisdiction-policies/$jurisdiction_policy_id/transition" \
    | grep -q '"to_status":"approved"'

policy_approved_read=$(curl --fail --silent --show-error \
    --header "authorization: $(make_authorization policy-auditor 'tax:jurisdiction:read' '["tax_jurisdiction_auditor"]')" \
    "$base_url/v1/tax/jurisdiction-policies/$jurisdiction_policy_id/governance")
if ! printf '%s' "$policy_approved_read" | grep -q '"status":"approved"'; then
    echo "jurisdiction policy did not reach approved before publication" >&2
    printf '%s\n' "$policy_approved_read" >&2
    exit 1
fi

policy_transition_body='{"to_status":"published","evidence":{"source":"integration-test","release":"recorded"}}'
policy_publish_response_file="$integration_dir/jurisdiction-policy-publish-response"
policy_publish_status=$(printf '%s' "$policy_transition_body" | curl --silent --show-error \
    --output "$policy_publish_response_file" --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $(make_authorization policy-approver 'tax:jurisdiction:publish' '["tax_jurisdiction_approver"]')" \
    --data-binary @- "$base_url/v1/tax/jurisdiction-policies/$jurisdiction_policy_id/transition")
if [ "$policy_publish_status" -lt 200 ] || [ "$policy_publish_status" -ge 300 ]; then
    echo "jurisdiction policy publication returned HTTP $policy_publish_status" >&2
    sed -n '1,160p' "$policy_publish_response_file" >&2 || true
    exit 1
fi
if ! grep -q '"to_status":"published"' "$policy_publish_response_file"; then
    echo "jurisdiction policy publication response omitted the published status" >&2
    exit 1
fi

policy_governance_response=$(curl --fail --silent --show-error \
    --header "authorization: $(make_authorization policy-auditor 'tax:jurisdiction:read' '["tax_jurisdiction_auditor"]')" \
    "$base_url/v1/tax/jurisdiction-policies/$jurisdiction_policy_id/governance")
if ! printf '%s' "$policy_governance_response" | grep -q '"status":"published"'; then
    echo "jurisdiction policy governance read did not return the published lifecycle state" >&2
    exit 1
fi
if ! printf '%s' "$policy_governance_response" | grep -q '"to_status":"published"'; then
    echo "jurisdiction policy governance read did not return append-only transition evidence" >&2
    exit 1
fi

policy_event_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from jurisdiction_policy_events where policy_id = '$jurisdiction_policy_id';")
if [ "$policy_event_count" != "4" ]; then
    echo "expected four append-only jurisdiction policy events including creation, found $policy_event_count" >&2
    exit 1
fi

if docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -v ON_ERROR_STOP=1 -Atc \
    "update jurisdiction_policy_events set evidence='{}'::jsonb where policy_id = '$jurisdiction_policy_id';" \
    >/dev/null 2>&1; then
    echo "jurisdiction policy event mutation unexpectedly succeeded" >&2
    exit 1
fi

if docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -v ON_ERROR_STOP=1 -Atc \
    "update jurisdiction_policies set provenance='{}'::jsonb where id = '$jurisdiction_policy_id';" \
    >/dev/null 2>&1; then
    echo "published jurisdiction policy mutation unexpectedly succeeded" >&2
    exit 1
fi

governance_body='{"to_status":"retired","evidence":{"source":"integration-test","reason":"superseded"}}'
printf '%s' "$governance_body" | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $(make_authorization policy-approver 'tax:jurisdiction:publish' '["tax_jurisdiction_approver"]')" \
    --data-binary @- "$base_url/v1/tax/jurisdiction-policies/$jurisdiction_policy_id/transition" \
    | grep -q '"to_status":"retired"'

retired_policy_governance_response=$(curl --fail --silent --show-error \
    --header "authorization: $(make_authorization policy-auditor 'tax:jurisdiction:read' '["tax_jurisdiction_auditor"]')" \
    "$base_url/v1/tax/jurisdiction-policies/$jurisdiction_policy_id/governance")
if ! printf '%s' "$retired_policy_governance_response" | grep -q '"status":"retired"'; then
    echo "jurisdiction policy governance read did not return the retired lifecycle state" >&2
    exit 1
fi

policy_event_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from jurisdiction_policy_events where policy_id = '$jurisdiction_policy_id';")
if [ "$policy_event_count" != "5" ]; then
    echo "expected five append-only jurisdiction policy events after retirement, found $policy_event_count" >&2
    exit 1
fi

if docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -v ON_ERROR_STOP=1 -Atc \
    "update jurisdiction_policies set policy_document='{}'::jsonb where id = '$jurisdiction_policy_id';" \
    >/dev/null 2>&1; then
    echo "retired jurisdiction policy mutation unexpectedly succeeded" >&2
    exit 1
fi

governance_body='{"to_status":"retired","evidence":{"source":"integration-test","reason":"superseded"}}'
printf '%s' "$governance_body" | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $(make_authorization integration-approver 'tax:rule:publish' '["tax_rule_approver"]')" \
    --data-binary @- "$base_url/v1/tax/rules/$governed_rule_id/transition" \
    | grep -q '"to_status":"retired"'

governed_status=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select status from rule_sets where id = '$governed_rule_id';")
if [ "$governed_status" != "retired" ]; then
    echo "expected governed rule to be retired, found $governed_status" >&2
    exit 1
fi

retired_simulation_status=$(printf '%s' "$simulation_request" | curl --silent --show-error \
    --output /dev/null --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $simulation_authorization" \
    --data-binary @- "$base_url/v1/tax/rules/$governed_rule_id/simulate")
if [ "$retired_simulation_status" != "409" ]; then
    echo "expected retired persisted-rule simulation to return HTTP 409, received $retired_simulation_status" >&2
    exit 1
fi

governance_event_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from rule_governance_events where rule_set_id = '$governed_rule_id';")
if [ "$governance_event_count" != "6" ]; then
    echo "expected six append-only rule governance events after retirement, found $governance_event_count" >&2
    exit 1
fi

# Exercise the durable settlement-adjustment proposal and evidence compiler
# boundary. This records an immutable signed refund delta and server-verified
# finalized registry evidence; it deliberately does not submit a token
# transaction or mark the refund as paid.
adjustment_proposer_authorization="$(make_authorization adjustment-proposer 'tax:adjustment:propose tax:adjustment:read tax:adjustment:authority:bind' '["settlement_adjustment_proposer"]')"
adjustment_approver_authorization="$(make_authorization adjustment-approver 'tax:adjustment:approve tax:adjustment:read tax:adjustment:authority:bind tax:adjustment:compile' '["settlement_adjustment_approver"]')"
adjustment_reviewer_authorization="$(make_authorization adjustment-reviewer 'tax:adjustment:reject tax:adjustment:read' '["settlement_adjustment_reviewer"]')"
adjustment_original_settlement_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d13"
adjustment_source_calculation_id="$calculation_id"
adjustment_asset_id='[6,167,213,23,24,199,116,201,40,86,99,152,105,29,94,182,139,94,184,163,155,75,109,92,115,85,91,33,0,0,0,0]'
adjustment_evidence_digest='[5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5]'
authority_binding_output=$(API_BASE="$base_url" \
    PROPOSER_AUTHORIZATION="$adjustment_proposer_authorization" \
    APPROVER_AUTHORIZATION="$adjustment_approver_authorization" \
    pnpm --dir apps/web exec node scripts/adjustment_authority_integration.mjs)
if ! printf '%s' "$authority_binding_output" | grep -q '"status":"wallet_proofs_bound"'; then
    echo "adjustment wallet-proof binding integration failed: $authority_binding_output" >&2
    exit 1
fi

# Bind the server-owned finalized receipt projection to the approved
# adjustment's original settlement intent. The bytes below are the canonical
# decoded values of the synthetic local route and are intentionally isolated
# from production migrations.
docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -v ON_ERROR_STOP=1 -Atc \
    "INSERT INTO chain_checkpoints (stream_key, next_slot, finalized_slot, generation)
     VALUES ('integration/settlement-adjustment', 78, 77, 0)
     ON CONFLICT (stream_key) DO NOTHING;
     INSERT INTO settlement_intents
        (id, merchant_id, idempotency_key, request_digest, instruction_digest,
         status, signature, slot, commitment)
     VALUES
        ('$adjustment_original_settlement_id', '$merchant_id',
         'integration-adjustment-original-settlement', decode(repeat('31', 32), 'hex'),
         decode(repeat('32', 32), 'hex'), 'finalized',
         'integration-adjustment-finalized-signature', 77, 'finalized')
     ON CONFLICT (id) DO NOTHING;
     INSERT INTO indexed_receipts
        (stream_key, signature, slot, gross_atomic, tax_atomic, net_atomic,
         evidence_digest, receipt, payer, merchant, mint, nonce, policy_digest)
     VALUES
        ('integration/settlement-adjustment',
         'integration-adjustment-finalized-signature', 77, 250, 25, 225,
         decode(repeat('44', 32), 'hex'),
         decode('06a7d517192c568ee08a845f73d29788cf035c3145b21ab344d8062ea9400000', 'hex'),
         decode(repeat('09', 32), 'hex'),
         decode('0000000000000000000000000000000000000000000000000000000000000003', 'hex'),
         decode('06a7d51718c774c928566398691d5eb68b5eb8a39b4b6d5c73555b2100000000', 'hex'),
         42, decode(repeat('33', 32), 'hex'))
     ON CONFLICT (stream_key, signature) DO NOTHING;"
adjustment_body="{\"idempotency_key\":\"integration-adjustment-01\",\"original_settlement_id\":\"$adjustment_original_settlement_id\",\"asset_id\":$adjustment_asset_id,\"kind\":\"Refund\",\"delta\":{\"gross_atomic\":\"-250\",\"tax_atomic\":\"-25\",\"net_atomic\":\"-225\"},\"reason_code\":\"customer_refund\",\"evidence_digest\":$adjustment_evidence_digest,\"source_calculation_id\":\"$adjustment_source_calculation_id\",\"replacement_calculation_id\":null}"
adjustment_response_file="$integration_dir/adjustment-response"
adjustment_status=$(printf '%s' "$adjustment_body" | curl --silent --show-error \
    --output "$adjustment_response_file" --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $adjustment_proposer_authorization" \
    --data-binary @- "$base_url/v1/settlement/adjustments")
if [ "$adjustment_status" != "200" ] || \
    ! grep -q '"status":"proposed"' "$adjustment_response_file" || \
    ! grep -q '"gross_atomic":"-250"' "$adjustment_response_file"; then
    echo "settlement adjustment proposal failed (HTTP $adjustment_status)" >&2
    sed -n '1,160p' "$adjustment_response_file" >&2
    exit 1
fi
adjustment_replay=$(printf '%s' "$adjustment_body" | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $adjustment_proposer_authorization" \
    --data-binary @- "$base_url/v1/settlement/adjustments")
if [ "$(sed -n '1p' "$adjustment_response_file")" != "$adjustment_replay" ]; then
    echo "settlement adjustment idempotency replay returned different evidence" >&2
    exit 1
fi
adjustment_id=$(sed -n 's/.*"id":"\([^"]*\)".*/\1/p' "$adjustment_response_file")
if [ -z "$adjustment_id" ]; then
    echo "settlement adjustment response did not contain an ID" >&2
    exit 1
fi

same_adjustment_approval_status=$(printf '%s' '{"to_status":"approved","evidence":{"source":"integration"}}' | curl --silent --show-error \
    --output /dev/null --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $(make_authorization adjustment-proposer 'tax:adjustment:approve' '["settlement_adjustment_approver"]')" \
    --data-binary @- "$base_url/v1/settlement/adjustments/$adjustment_id/transition")
if [ "$same_adjustment_approval_status" != "409" ]; then
    echo "expected same-subject adjustment approval to be rejected with HTTP 409, received $same_adjustment_approval_status" >&2
    exit 1
fi

printf '%s' '{"to_status":"approved","evidence":{"source":"integration","approval":"recorded"}}' | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $adjustment_approver_authorization" \
    --data-binary @- "$base_url/v1/settlement/adjustments/$adjustment_id/transition" \
    | grep -q '"to_status":"approved"'
adjustment_read=$(curl --fail --silent --show-error \
    --header "authorization: $adjustment_approver_authorization" \
    "$base_url/v1/settlement/adjustments/$adjustment_id")
if ! printf '%s' "$adjustment_read" | grep -q '"status":"approved"'; then
    echo "settlement adjustment read did not return approved state" >&2
    exit 1
fi

adjustment_compile_response_file="$integration_dir/adjustment-compile-response"
adjustment_compile_status=$(printf '%s' '{"confirm":true}' | curl --silent --show-error \
    --output "$adjustment_compile_response_file" --write-out '%{http_code}' \
    --header 'content-type: application/json' \
    --header "authorization: $adjustment_approver_authorization" \
    --data-binary @- "$base_url/v1/settlement/adjustments/$adjustment_id/compile")
adjustment_compile_response=$(sed -n '1p' "$adjustment_compile_response_file")
if [ "$adjustment_compile_status" != "200" ] || \
    ! printf '%s' "$adjustment_compile_response" | grep -q '"status":"prepared_unsubmitted"' || \
    ! printf '%s' "$adjustment_compile_response" | grep -q '"registry_commitment":"finalized"'; then
    echo "finalized adjustment evidence compilation failed (HTTP $adjustment_compile_status): $adjustment_compile_response" >&2
    exit 1
fi
adjustment_compile_replay=$(printf '%s' '{"confirm":true}' | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $adjustment_approver_authorization" \
    --data-binary @- "$base_url/v1/settlement/adjustments/$adjustment_id/compile")
if [ "$adjustment_compile_response" != "$adjustment_compile_replay" ]; then
    echo "adjustment evidence compiler did not return a retry-stable response" >&2
    exit 1
fi
adjustment_intent_evidence_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from settlement_adjustment_evidence_intents
      where adjustment_id = '$adjustment_id' and merchant_id = '$merchant_id'
        and registry_commitment = 'finalized' and registry_slot = 777
        and instruction_digest is not null;")
if [ "$adjustment_intent_evidence_count" != "1" ]; then
    echo "expected one immutable finalized adjustment evidence intent, found $adjustment_intent_evidence_count" >&2
    exit 1
fi
adjustment_event_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from settlement_adjustment_events where adjustment_id = '$adjustment_id';")
if [ "$adjustment_event_count" != "2" ]; then
    echo "expected two append-only adjustment events, found $adjustment_event_count" >&2
    exit 1
fi
if docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -v ON_ERROR_STOP=1 -Atc \
    "update settlement_adjustments set gross_delta=0 where id='$adjustment_id';" \
    >/dev/null 2>&1; then
    echo "settlement adjustment financial-field mutation unexpectedly succeeded" >&2
    exit 1
fi
adjustment_outbox_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from outbox_events where aggregate_type='settlement_adjustment' and aggregate_id='$adjustment_id';")
if [ "$adjustment_outbox_count" != "2" ]; then
    echo "expected proposal and approval outbox events, found $adjustment_outbox_count" >&2
    exit 1
fi

rejected_adjustment_body="${adjustment_body/integration-adjustment-01/integration-adjustment-02}"
rejected_adjustment_response=$(printf '%s' "$rejected_adjustment_body" | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $adjustment_proposer_authorization" \
    --data-binary @- "$base_url/v1/settlement/adjustments")
rejected_adjustment_id=$(printf '%s' "$rejected_adjustment_response" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
printf '%s' '{"to_status":"rejected","evidence":{"source":"integration","reason":"unsupported"}}' | curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --header "authorization: $adjustment_reviewer_authorization" \
    --data-binary @- "$base_url/v1/settlement/adjustments/$rejected_adjustment_id/transition" \
    | grep -q '"to_status":"rejected"'

echo "local stack integration checks passed"
