#!/bin/sh

set -eu

# Bounded local/CI evidence only. This workload uses synthetic facts and an
# ephemeral JWT against an isolated Compose project. Simulation is read-only.
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"
project_name="${COMPOSE_PROJECT_NAME:-solana-tax-protocol-rule-simulation-load}"
load_requests="${SIMULATION_LOAD_REQUESTS:-200}"
load_concurrency="${SIMULATION_LOAD_CONCURRENCY:-32}"
load_timeout_seconds="${SIMULATION_LOAD_TIMEOUT_SECONDS:-10}"
contention_requests="${SIMULATION_CONTENTION_REQUESTS:-64}"
database_pool_size="${API_DATABASE_POOL_SIZE:-16}"
api_port="${SIMULATION_API_PORT:-28082}"
replica_api_port="${SIMULATION_API_REPLICA_PORT:-28083}"
postgres_port="${SIMULATION_POSTGRES_PORT:-25442}"
nats_port="${SIMULATION_NATS_PORT:-25443}"
p95_budget_ms="${SIMULATION_P95_BUDGET_MS:-25}"
allow_budget_miss="${ALLOW_LATENCY_BUDGET_MISS:-false}"
merchant_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d11"
rule_set_uuid="01992f1b-4a70-7e30-8ea2-b6e37a9d9d10"
rule_external_id="operator-simulator-flat-rate"
expected_rule_digest='[38,170,224,75,70,133,191,124,7,115,59,252,148,156,161,238,222,34,160,248,25,155,199,9,68,252,174,11,94,117,7,50]'
integration_dir="$(mktemp -d)"

case "$load_requests:$load_concurrency:$load_timeout_seconds:$contention_requests:$database_pool_size:$api_port:$replica_api_port:$postgres_port:$nats_port:$p95_budget_ms" in
    *[!0-9:]*|0:*|*:[!0-9:]*|*:*:0)
        echo "workload, pool, port, and latency-budget settings must be positive integers" >&2
        exit 2
        ;;
esac
case "$allow_budget_miss" in
    true|false) ;;
    *)
        echo "ALLOW_LATENCY_BUDGET_MISS must be true or false" >&2
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
export API_AUTH_ISSUER="https://rule-simulation-load-identity.invalid"
export API_AUTH_AUDIENCE="tax-api"
export API_AUTH_JWKS_KID="rule-simulation-load-key"
export API_AUTH_JWKS_PUBLIC_KEY="$public_key"
export API_PORT="$api_port"
export POSTGRES_PORT="$postgres_port"
export NATS_PORT="$nats_port"
export API_DATABASE_POOL_SIZE="$database_pool_size"

now_unix="$(date +%s)"
token_header="$(b64url '{"alg":"EdDSA","kid":"rule-simulation-load-key","typ":"JWT"}')"
token_claims="$(b64url "{\"iss\":\"$API_AUTH_ISSUER\",\"sub\":\"simulation-load-user\",\"aud\":\"$API_AUTH_AUDIENCE\",\"exp\":$((now_unix + 300)),\"iat\":$((now_unix - 10)),\"nbf\":$((now_unix - 10)),\"merchant_id\":\"$merchant_id\",\"scope\":\"tax:rule:read\",\"roles\":[\"tax_rule_author\"]}")"
token_signing_input="$token_header.$token_claims"
token_signature="$(printf '%s' "$token_signing_input" | openssl pkeyutl -sign -inkey "$integration_dir/signing-key.pem" -rawin -in /dev/stdin 2>/dev/null | b64url_stdin)"
authorization="Bearer $token_signing_input.$token_signature"

base_url="http://127.0.0.1:$api_port"
replica_base_url="http://127.0.0.1:$replica_api_port"
request_results="$integration_dir/results"
latencies="$integration_dir/latencies"
response_dir="$integration_dir/responses"
contention_response_dir="$integration_dir/contention-responses"
mkdir -p "$response_dir" "$contention_response_dir"

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
    echo "rule-simulation API did not become ready" >&2
    exit 1
fi

replica_container_id="$(docker compose -p "$project_name" run --detach --no-deps \
    --publish "$replica_api_port:8080" api)"
replica_ready=false
for _attempt in $(seq 1 60); do
    if curl --fail --silent "$replica_base_url/readyz" >/dev/null 2>&1; then
        replica_ready=true
        break
    fi
    sleep 1
done
if [ "$replica_ready" != "true" ]; then
    echo "independent rule-simulation API replica did not become ready" >&2
    docker logs "$replica_container_id" >&2 || true
    docker compose -p "$project_name" logs --no-color api postgres >&2 || true
    exit 1
fi

simulation_body='{"facts":{"gross_atomic":"1100","net_atomic":"1000","currency":"USD","jurisdiction":"US-CA"}}'
simulation_path="/v1/tax/rules/$rule_set_uuid/simulate"
warmup_response="$integration_dir/warmup.json"
warmup_status="$(printf '%s' "$simulation_body" | curl --silent --show-error \
    --output "$warmup_response" --write-out '%{http_code}' --max-time "$load_timeout_seconds" \
    --header 'content-type: application/json' --header "authorization: $authorization" \
    --data-binary @- "$base_url$simulation_path")"
if [ "$warmup_status" != "200" ] \
    || ! grep -q '"status":"simulated_unattested"' "$warmup_response" \
    || ! grep -Fq '"rule_set_id":"'$rule_external_id'"' "$warmup_response" \
    || ! grep -Fq '"rule_digest":'$expected_rule_digest "$warmup_response"; then
    echo "rule-simulation warmup failed with HTTP $warmup_status" >&2
    sed -n '1,20p' "$warmup_response" >&2 || true
    docker compose -p "$project_name" logs --no-color api postgres >&2 || true
    exit 1
fi

export load_base_url="$base_url"
export load_replica_base_url="$replica_base_url"
export load_authorization="$authorization"
export load_timeout_seconds
export load_body="$simulation_body"
export load_path="$simulation_path"
export load_response_dir="$response_dir"
export contention_response_dir

seq 1 "$load_requests" | xargs -P "$load_concurrency" -n 1 sh -c '
    ordinal="$1"
    request_url="$load_base_url"
    if [ "$((ordinal % 2))" -eq 0 ]; then request_url="$load_replica_base_url"; fi
    curl --silent --show-error --max-time "$load_timeout_seconds"         --header "content-type: application/json"         --header "authorization: $load_authorization"         --data-binary "$load_body"         --output "$load_response_dir/$ordinal.json"         --write-out "%{http_code} %{time_total}\n"         "$request_url$load_path"
' simulation-load > "$request_results"

sample_count="$(wc -l < "$request_results" | tr -d ' ')"
if [ "$sample_count" -ne "$load_requests" ]; then
    echo "incomplete simulation sample: expected $load_requests responses, got $sample_count" >&2
    exit 1
fi
status_counts="$(awk '{print $1}' "$request_results" | sort | uniq -c | sed 's/^ *//')"
non_success="$(awk '$1 != 200 { count += 1 } END { print count + 0 }' "$request_results")"
if [ "$non_success" -ne 0 ]; then
    echo "rule-simulation benchmark produced non-200 responses:" >&2
    printf '%s\n' "$status_counts" >&2
    for response in "$response_dir"/*.json; do
        if ! grep -q '"status":"simulated_unattested"' "$response" 2>/dev/null; then
            echo "invalid response: $response" >&2
            sed -n '1,8p' "$response" >&2
        fi
    done
    docker compose -p "$project_name" logs --no-color api postgres >&2 || true
    exit 1
fi
for response in "$response_dir"/*.json; do
    if ! grep -Fq '"status":"simulated_unattested"' "$response" \
        || ! grep -Fq '"rule_set_id":"'$rule_external_id'"' "$response" \
        || ! grep -Fq '"rule_digest":'$expected_rule_digest "$response"; then
        echo "simulation response lost the server-owned rule binding: $response" >&2
        sed -n '1,8p' "$response" >&2
        exit 1
    fi
done

awk '$1 == 200 && $2 ~ /^[0-9]+([.][0-9]+)?$/ { printf "%.6f\n", $2 * 1000 }' "$request_results" | sort -n > "$latencies"
latency_count="$(wc -l < "$latencies" | tr -d ' ')"
if [ "$latency_count" -ne "$load_requests" ]; then
    echo "simulation benchmark contained invalid latency samples" >&2
    exit 1
fi

export contention_body="$simulation_body"
seq 1 "$contention_requests" | xargs -P "$load_concurrency" -n 1 sh -c '
    ordinal="$1"
    request_url="$load_base_url"
    if [ "$((ordinal % 2))" -eq 0 ]; then request_url="$load_replica_base_url"; fi
    curl --silent --show-error --max-time "$load_timeout_seconds"         --header "content-type: application/json"         --header "authorization: $load_authorization"         --data-binary "$contention_body"         --output "$contention_response_dir/$ordinal.json"         --write-out "%{http_code}\n"         "$request_url$load_path"
' simulation-contention > "$integration_dir/contention-results" &
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
contention_non_success="$(awk '$1 != 200 { count += 1 } END { print count + 0 }' "$integration_dir/contention-results")"
if [ "$contention_sample_count" -ne "$contention_requests" ] || [ "$contention_non_success" -ne 0 ]; then
    echo "simulation read contention did not remain successful: samples=$contention_sample_count" >&2
    sort "$integration_dir/contention-results" | uniq -c >&2
    docker compose -p "$project_name" logs --no-color api postgres >&2 || true
    exit 1
fi
for response in "$contention_response_dir"/*.json; do
    if ! grep -Fq '"status":"simulated_unattested"' "$response" \
        || ! grep -Fq '"rule_digest":'$expected_rule_digest "$response"; then
        echo "simulation contention response was not a valid unattested result: $response" >&2
        exit 1
    fi
done

percentile() {
    percentile_value="$1"
    awk -v count="$latency_count" -v percentile="$percentile_value" '
        BEGIN { rank = int((count * percentile + 99) / 100); if (rank < 1) rank = 1 }
        NR == rank { printf "%.3f", $1; exit }
    ' "$latencies"
}

p95_ms="$(percentile 95)"
p95_budget_met="$(awk -v observed="$p95_ms" -v budget="$p95_budget_ms" 'BEGIN { print (observed <= budget) ? "true" : "false" }')"
if [ "$p95_budget_met" != "true" ] && [ "$allow_budget_miss" != "true" ]; then
    echo "simulation p95 latency budget missed: observed=${p95_ms}ms budget=${p95_budget_ms}ms" >&2
    echo "set ALLOW_LATENCY_BUDGET_MISS=true only for an explicitly reviewed diagnostic exception" >&2
    exit 1
fi

printf '%s\n' "rule-simulation-load-evidence" \
    "endpoint=$base_url$simulation_path" \
    "replicas=2" \
    "replica_endpoint=$replica_base_url$simulation_path" \
    "requests=$load_requests" \
    "concurrency=$load_concurrency" \
    "contention_requests=$contention_requests" \
    "timeout_seconds=$load_timeout_seconds" \
    "tenant=$merchant_id" \
    "rule=$rule_external_id@1" \
    "status_counts=$status_counts" \
    "error_rate=0" \
    "p50_ms=$(percentile 50)" \
    "p95_ms=$p95_ms" \
    "p99_ms=$(percentile 99)" \
    "p95_budget_ms=$p95_budget_ms" \
    "p95_budget_met=$p95_budget_met" \
    "latency_budget_exception=$allow_budget_miss" \
    "contention_status_counts=$(sort "$integration_dir/contention-results" | uniq -c | sed 's/^ *//')" \
    "database_pool_max_size=$database_pool_size" \
    "database_pool_peak_waiting=$pool_peak_waiting"
