#!/bin/sh

set -eu

# This is a bounded local evidence harness, not a production SLO claim.
# It deliberately owns an isolated Compose project and synthetic credentials.
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
project_name="${COMPOSE_PROJECT_NAME:-solana-tax-protocol-quote-load}"
benchmark_mode="${BENCHMARK_MODE:-aggregate}"
load_requests="${LOAD_REQUESTS:-200}"
load_concurrency="${LOAD_CONCURRENCY:-16}"
load_timeout_seconds="${LOAD_TIMEOUT_SECONDS:-10}"
contention_requests="${CONTENTION_REQUESTS:-32}"
database_pool_size="${API_DATABASE_POOL_SIZE:-16}"
api_port="${API_PORT:-28080}"
replica_api_port="${API_REPLICA_PORT:-28081}"
postgres_port="${POSTGRES_PORT:-25432}"
nats_port="${NATS_PORT:-25433}"
merchant_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d11"
integration_dir="$(mktemp -d)"

case "$benchmark_mode" in
    aggregate|document) ;;
    *)
        echo "BENCHMARK_MODE must be aggregate or document" >&2
        exit 2
        ;;
esac

if [ "$benchmark_mode" = "document" ]; then
    idempotency_prefix="${IDEMPOTENCY_PREFIX:-document-load-01J8Q4J0M9M2G7YB8F5N3D6K5T}"
    quote_endpoint="/v1/tax/document-quote"
    rule_set_id="operator-document-flat-rate"
    facts_payload='{"id":"document-load-invoice","currency":"USD","jurisdiction":"US-CA","lines":[{"id":"line-a","gross_atomic":"1000","net_atomic":"1000","exemption_codes":[]},{"id":"line-b","gross_atomic":"500","net_atomic":"500","exemption_codes":[]}]}'
    evidence_facts_payload="{\"facts\":$facts_payload}"
    calculation_table="tax_document_calculations"
    outbox_aggregate_type="tax_document_calculation"
    outbox_event_type="tax.document_calculation.created"
    evidence_name="document-quote-service-load"
else
    idempotency_prefix="${IDEMPOTENCY_PREFIX:-load-01J8Q4J0M9M2G7YB8F5N3D6K5T}"
    quote_endpoint="/v1/tax/quote"
    rule_set_id="operator-simulator-flat-rate"
    facts_payload='{"gross_atomic":"1100","net_atomic":"1000","currency":"USD","jurisdiction":"US-CA"}'
    evidence_facts_payload="{\"facts\":$facts_payload}"
    calculation_table="tax_calculations"
    outbox_aggregate_type="tax_calculation"
    outbox_event_type="tax.calculation.created"
    evidence_name="quote-service-load"
fi

case "$load_requests:$load_concurrency:$load_timeout_seconds:$contention_requests:$database_pool_size:$api_port:$replica_api_port" in
    *[!0-9:]*|0:*|*:[!0-9:]*|*:*:0|*:*:*:0)
        echo "workload, pool, and API port settings must be positive integers" >&2
        exit 2
        ;;
esac

replica_container_id=""

cleanup() {
    if [ -n "$replica_container_id" ]; then
        docker rm --force "$replica_container_id" >/dev/null 2>&1 || true
    fi
    docker compose -p "$project_name" down --volumes --remove-orphans >/dev/null 2>&1 || true
    rm -rf "$integration_dir"
}
trap cleanup EXIT INT TERM

command -v docker >/dev/null 2>&1 || { echo "Docker is required" >&2; exit 1; }
command -v curl >/dev/null 2>&1 || { echo "curl is required" >&2; exit 1; }
command -v openssl >/dev/null 2>&1 || { echo "OpenSSL is required" >&2; exit 1; }

b64url() {
    printf '%s' "$1" | base64 | tr '+/' '-_' | tr -d '=\n'
}

b64url_stdin() {
    base64 | tr '+/' '-_' | tr -d '=\n'
}

openssl genpkey -algorithm ED25519 -out "$integration_dir/signing-key.pem" >/dev/null 2>&1
openssl pkey -in "$integration_dir/signing-key.pem" -pubout -outform DER -out "$integration_dir/public-key.der" >/dev/null 2>&1
public_key="$(tail -c 32 "$integration_dir/public-key.der" | b64url_stdin)"
export API_AUTH_ISSUER="https://quote-load-identity.invalid"
export API_AUTH_AUDIENCE="tax-api"
export API_AUTH_JWKS_KID="quote-load-key"
export API_AUTH_JWKS_PUBLIC_KEY="$public_key"
export API_PORT="$api_port"
export POSTGRES_PORT="$postgres_port"
export NATS_PORT="$nats_port"
export API_DATABASE_POOL_SIZE="$database_pool_size"

now_unix="$(date +%s)"
token_header="$(b64url '{"alg":"EdDSA","kid":"quote-load-key","typ":"JWT"}')"
token_claims="$(b64url "{\"iss\":\"$API_AUTH_ISSUER\",\"sub\":\"quote-load-user\",\"aud\":\"$API_AUTH_AUDIENCE\",\"exp\":$((now_unix + 300)),\"iat\":$((now_unix - 10)),\"nbf\":$((now_unix - 10)),\"merchant_id\":\"$merchant_id\",\"scope\":\"tax:quote\",\"roles\":[]}")"
token_signing_input="$token_header.$token_claims"
token_signature="$(printf '%s' "$token_signing_input" | openssl pkeyutl -sign -inkey "$integration_dir/signing-key.pem" -rawin -in /dev/stdin 2>/dev/null | b64url_stdin)"
authorization="Bearer $token_signing_input.$token_signature"

base_url="http://127.0.0.1:${api_port}"
request_results="$integration_dir/results"
latencies="$integration_dir/latencies"
response_dir="$integration_dir/responses"
contention_response_dir="$integration_dir/contention-responses"
mkdir -p "$response_dir"
mkdir -p "$contention_response_dir"

docker compose -p "$project_name" up --build --detach postgres nats api >/dev/null
ready=false
for _attempt in $(seq 1 60); do
    if curl --fail --silent "$base_url/readyz" >/dev/null 2>&1; then
        ready=true
        break
    fi
    sleep 1
done
if [ "$ready" != "true" ]; then
    docker compose -p "$project_name" logs --no-color api postgres >&2 || true
    echo "quote service did not become ready" >&2
    exit 1
fi

# Start an independent process from the same image. It shares only the
# authoritative PostgreSQL/NATS services and its Docker network; no process
# memory, in-process lock, or cache is shared with the Compose-managed API.
replica_container_id="$(docker compose -p "$project_name" run --detach --no-deps \
    --publish "${replica_api_port}:8080" api)"
replica_base_url="http://127.0.0.1:${replica_api_port}"
replica_ready=false
for _attempt in $(seq 1 60); do
    if curl --fail --silent "$replica_base_url/readyz" >/dev/null 2>&1; then
        replica_ready=true
        break
    fi
    sleep 1
done
if [ "$replica_ready" != "true" ]; then
    echo "independent quote-service API replica did not become ready" >&2
    docker logs "$replica_container_id" >&2 || true
    docker compose -p "$project_name" logs --no-color api postgres >&2 || true
    exit 1
fi

export load_base_url="$base_url"
export load_replica_base_url="$replica_base_url"
export load_authorization="$authorization"
export load_timeout_seconds
export load_idempotency_prefix="$idempotency_prefix"
export load_mode="$benchmark_mode"
export load_quote_endpoint="$quote_endpoint"
export load_rule_set_id="$rule_set_id"
export load_facts_payload="$facts_payload"
export load_response_dir="$response_dir"
export contention_response_dir
export contention_idempotency_key="${idempotency_prefix}-same-key"

jurisdiction_request_fixture="$integration_dir/jurisdiction-request.json"
printf '%s' "$evidence_facts_payload" > "$jurisdiction_request_fixture"
issuer_image="$project_name-attestation-fixture"
docker build --file ops/docker/Dockerfile.attestation-fixture --tag "$issuer_image" . >/dev/null
load_jurisdiction_evidence=$(docker run --rm --read-only --network none --cap-drop ALL \
    --security-opt no-new-privileges:true \
    --volume "$jurisdiction_request_fixture:/fixtures/request.json:ro" \
    "$issuer_image" jurisdiction-evidence /fixtures/request.json "$merchant_id" \
    0909090909090909090909090909090909090909090909090909090909090909)
export load_jurisdiction_evidence

# Warm the authenticated route, PostgreSQL connection pool, and immutable rule
# cache before collecting the declared sample. This request is outside the
# measured prefix and is checked independently.
if [ "$benchmark_mode" = "document" ]; then
    warmup_body="{\"schema\":\"solana-tax-protocol/tax-document/v1\",\"idempotency_key\":\"document-quote-load-warmup\",\"rule_set_id\":\"$rule_set_id\",\"rule_version\":1,\"jurisdiction\":\"US-CA\",\"regime\":\"sales-tax\",\"facts\":$facts_payload,\"jurisdiction_evidence\":[$load_jurisdiction_evidence]}"
else
    warmup_body="{\"idempotency_key\":\"quote-load-warmup\",\"rule_set_id\":\"$rule_set_id\",\"rule_version\":1,\"jurisdiction\":\"US-CA\",\"regime\":\"sales-tax\",\"facts\":$facts_payload,\"jurisdiction_evidence\":[$load_jurisdiction_evidence]}"
fi
curl --fail --silent --show-error --max-time "$load_timeout_seconds" \
    --header "content-type: application/json" \
    --header "authorization: $authorization" \
    --data-binary "$warmup_body" \
    --output "$integration_dir/warmup.json" \
    "$base_url$quote_endpoint" || {
    echo "quote-service warmup failed; response and service logs follow" >&2
    sed -n '1,12p' "$integration_dir/warmup.json" >&2 || true
    docker compose -p "$project_name" logs --no-color api postgres >&2 || true
    exit 1
}

seq 1 "$load_requests" | xargs -P "$load_concurrency" -n 1 sh -c '
    ordinal="$1"
    request_url="$load_base_url"
    if [ "$((ordinal % 2))" -eq 0 ]; then request_url="$load_replica_base_url"; fi
    if [ "$load_mode" = "document" ]; then
        body="{\"schema\":\"solana-tax-protocol/tax-document/v1\",\"idempotency_key\":\"${load_idempotency_prefix}-${ordinal}\",\"rule_set_id\":\"$load_rule_set_id\",\"rule_version\":1,\"jurisdiction\":\"US-CA\",\"regime\":\"sales-tax\",\"facts\":$load_facts_payload,\"jurisdiction_evidence\":[$load_jurisdiction_evidence]}"
    else
        body="{\"idempotency_key\":\"${load_idempotency_prefix}-${ordinal}\",\"rule_set_id\":\"$load_rule_set_id\",\"rule_version\":1,\"jurisdiction\":\"US-CA\",\"regime\":\"sales-tax\",\"facts\":$load_facts_payload,\"jurisdiction_evidence\":[$load_jurisdiction_evidence]}"
    fi
    curl --silent --show-error --max-time "$load_timeout_seconds" \
        --header "content-type: application/json" \
        --header "authorization: $load_authorization" \
        --data-binary "$body" \
        --output "$load_response_dir/$ordinal.json" \
        --write-out "%{http_code} %{time_total}\n" \
        "$request_url$load_quote_endpoint"
' benchmark > "$request_results"

sample_count="$(wc -l < "$request_results" | tr -d ' ')"
if [ "$sample_count" -ne "$load_requests" ]; then
    echo "incomplete benchmark sample: expected $load_requests responses, got $sample_count" >&2
    exit 1
fi

status_counts="$(awk '{print $1}' "$request_results" | sort | uniq -c | sed 's/^ *//')"
non_success="$(awk '$1 != 200 { count += 1 } END { print count + 0 }' "$request_results")"
if [ "$non_success" -ne 0 ]; then
    echo "benchmark produced non-200 responses:" >&2
    printf '%s\n' "$status_counts" >&2
    awk '$1 != 200 { print $1 }' "$request_results" | while read -r status; do
        echo "response status=$status" >&2
    done
    for response in "$response_dir"/*.json; do
        if grep -q '"error"' "$response" 2>/dev/null; then
            echo "error response: $response" >&2
            sed -n '1,4p' "$response" >&2
        fi
    done
    docker compose -p "$project_name" logs --no-color api postgres >&2 || true
    exit 1
fi

awk '$1 == 200 && $2 ~ /^[0-9]+([.][0-9]+)?$/ { printf "%.6f\n", $2 * 1000 }' "$request_results" | sort -n > "$latencies"
latency_count="$(wc -l < "$latencies" | tr -d ' ')"
if [ "$latency_count" -ne "$load_requests" ]; then
    echo "benchmark contained invalid latency samples" >&2
    exit 1
fi

seq 1 "$contention_requests" | xargs -P "$load_concurrency" -n 1 sh -c '
    ordinal="$1"
    request_url="$load_base_url"
    if [ "$((ordinal % 2))" -eq 0 ]; then request_url="$load_replica_base_url"; fi
    if [ "$load_mode" = "document" ]; then
        body="{\"schema\":\"solana-tax-protocol/tax-document/v1\",\"idempotency_key\":\"${contention_idempotency_key}\",\"rule_set_id\":\"$load_rule_set_id\",\"rule_version\":1,\"jurisdiction\":\"US-CA\",\"regime\":\"sales-tax\",\"facts\":$load_facts_payload,\"jurisdiction_evidence\":[$load_jurisdiction_evidence]}"
    else
        body="{\"idempotency_key\":\"${contention_idempotency_key}\",\"rule_set_id\":\"$load_rule_set_id\",\"rule_version\":1,\"jurisdiction\":\"US-CA\",\"regime\":\"sales-tax\",\"facts\":$load_facts_payload,\"jurisdiction_evidence\":[$load_jurisdiction_evidence]}"
    fi
    curl --silent --show-error --max-time "$load_timeout_seconds" \
        --header "content-type: application/json" \
        --header "authorization: $load_authorization" \
        --data-binary "$body" \
        --output "$contention_response_dir/$ordinal.json" \
        --write-out "%{http_code}\n" \
        "$request_url$load_quote_endpoint"
' contention > "$integration_dir/contention-results" &
contention_pid=$!
pool_peak_waiting=0
while kill -0 "$contention_pid" >/dev/null 2>&1; do
    current_waiting="$(curl --silent --max-time 1 "$base_url/metrics" \
        | awk -F' ' '/^stp_database_pool_waiting\{service="api"\}/ { print $2; exit }')"
    case "$current_waiting" in
        ''|*[!0-9]*) ;;
        *) [ "$current_waiting" -gt "$pool_peak_waiting" ] && pool_peak_waiting="$current_waiting" ;;
    esac
    sleep 0.005
done
wait "$contention_pid"

contention_sample_count="$(wc -l < "$integration_dir/contention-results" | tr -d ' ')"
contention_status_counts="$(sort "$integration_dir/contention-results" | uniq -c | sed 's/^ *//')"
contention_non_success="$(awk '$1 != 200 { count += 1 } END { print count + 0 }' "$integration_dir/contention-results")"
if [ "$contention_sample_count" -ne "$contention_requests" ] || [ "$contention_non_success" -ne 0 ]; then
    echo "same-key contention did not converge successfully: samples=$contention_sample_count statuses=$contention_status_counts" >&2
    docker compose -p "$project_name" logs --no-color api postgres >&2 || true
    exit 1
fi

percentile() {
    percentile_value="$1"
    awk -v count="$latency_count" -v percentile="$percentile_value" '
        BEGIN { rank = int((count * percentile + 99) / 100); if (rank < 1) rank = 1 }
        NR == rank { printf "%.3f", $1; exit }
    ' "$latencies"
}

idempotency_count="$(docker compose -p "$project_name" exec -T postgres psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from idempotency_records where merchant_id = '$merchant_id' and idempotency_key like '$idempotency_prefix-%' and idempotency_key <> '${contention_idempotency_key}';")"
calculation_count="$(docker compose -p "$project_name" exec -T postgres psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from $calculation_table where merchant_id = '$merchant_id' and id in (select (response_document->>'calculation_id')::uuid from idempotency_records where merchant_id = '$merchant_id' and idempotency_key like '$idempotency_prefix-%' and idempotency_key <> '${contention_idempotency_key}' and response_status = 200 and response_document->>'calculation_id' is not null);")"
outbox_count="$(docker compose -p "$project_name" exec -T postgres psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from outbox_events where aggregate_type = '$outbox_aggregate_type' and event_type = '$outbox_event_type' and aggregate_id in (select (response_document->>'calculation_id')::uuid from idempotency_records where merchant_id = '$merchant_id' and idempotency_key like '$idempotency_prefix-%' and idempotency_key <> '${contention_idempotency_key}' and response_status = 200 and response_document->>'calculation_id' is not null);")"
contention_idempotency_count="$(docker compose -p "$project_name" exec -T postgres psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from idempotency_records where merchant_id = '$merchant_id' and idempotency_key = '${contention_idempotency_key}';")"
contention_calculation_count="$(docker compose -p "$project_name" exec -T postgres psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from $calculation_table where merchant_id = '$merchant_id' and id in (select (response_document->>'calculation_id')::uuid from idempotency_records where merchant_id = '$merchant_id' and idempotency_key = '${contention_idempotency_key}' and response_status = 200 and response_document->>'calculation_id' is not null);")"
contention_outbox_count="$(docker compose -p "$project_name" exec -T postgres psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from outbox_events where aggregate_type = '$outbox_aggregate_type' and event_type = '$outbox_event_type' and aggregate_id in (select (response_document->>'calculation_id')::uuid from idempotency_records where merchant_id = '$merchant_id' and idempotency_key = '${contention_idempotency_key}' and response_status = 200);")"

if [ "$idempotency_count" -ne "$load_requests" ] || [ "$calculation_count" -ne "$load_requests" ] || [ "$outbox_count" -ne "$load_requests" ]; then
    echo "persistence invariant failed: idempotency=$idempotency_count calculations=$calculation_count outbox=$outbox_count expected=$load_requests" >&2
    exit 1
fi
if [ "$contention_idempotency_count" -ne 1 ] || [ "$contention_calculation_count" -ne 1 ] || [ "$contention_outbox_count" -ne 1 ]; then
    echo "same-key contention did not converge to one durable calculation: idempotency=$contention_idempotency_count calculations=$contention_calculation_count outbox=$contention_outbox_count" >&2
    exit 1
fi

printf '%s\n' "$evidence_name" \
    "mode=$benchmark_mode" \
    "endpoint=$base_url$quote_endpoint" \
    "replicas=2" \
    "replica_endpoint=$replica_base_url$quote_endpoint" \
    "requests=$load_requests" \
    "concurrency=$load_concurrency" \
    "contention_requests=$contention_requests" \
    "contention_status_counts=$contention_status_counts" \
    "timeout_seconds=$load_timeout_seconds" \
    "tenant=$merchant_id" \
    "rule=$rule_set_id@1" \
    "status_counts=$status_counts" \
    "error_rate=0" \
    "p50_ms=$(percentile 50)" \
    "p95_ms=$(percentile 95)" \
    "p99_ms=$(percentile 99)" \
    "idempotency_records=$idempotency_count" \
    "tax_calculations=$calculation_count" \
    "outbox_events=$outbox_count" \
    "contention_idempotency_records=$contention_idempotency_count" \
    "contention_tax_calculations=$contention_calculation_count" \
    "contention_outbox_events=$contention_outbox_count" \
    "database_pool_max_size=$database_pool_size" \
    "database_pool_peak_waiting=$pool_peak_waiting"
