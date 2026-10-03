#!/bin/sh

set -eu

# Bounded local/CI evidence only. This measures the authenticated HTTP
# capability boundary across two API replicas; it is not a production regional
# SLO and never contains financial facts.
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

project_name="${COMPOSE_PROJECT_NAME:-solana-tax-protocol-authorization-load}"
request_count="${AUTHORIZATION_LOAD_REQUESTS:-256}"
concurrency="${AUTHORIZATION_LOAD_CONCURRENCY:-32}"
request_timeout="${AUTHORIZATION_LOAD_TIMEOUT_SECONDS:-10}"
database_pool_size="${API_DATABASE_POOL_SIZE:-16}"
api_port="${AUTHORIZATION_API_PORT:-28084}"
replica_api_port="${AUTHORIZATION_REPLICA_API_PORT:-28085}"
postgres_port="${AUTHORIZATION_POSTGRES_PORT:-25444}"
p95_budget_ms="${AUTHORIZATION_P95_BUDGET_MS:-25}"
allow_budget_miss="${ALLOW_LATENCY_BUDGET_MISS:-false}"
merchant_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d11"
integration_dir=$(mktemp -d)
response_dir="$integration_dir/responses"
mkdir -p "$response_dir"
replica_container_id=""

fail() {
    echo "authorization capability benchmark failed: $1" >&2
    docker compose -p "$project_name" logs --no-color api postgres >&2 2>/dev/null || true
    exit 1
}

cleanup() {
    exit_code=$?
    if [ -n "$replica_container_id" ]; then
        docker rm --force "$replica_container_id" >/dev/null 2>&1 || true
    fi
    docker compose -p "$project_name" down --volumes --remove-orphans >/dev/null 2>&1 || true
    rm -rf "$integration_dir"
    exit "$exit_code"
}
trap cleanup EXIT INT TERM

positive_bounded() {
    value=$1
    name=$2
    maximum=$3
    case "$value" in
        ''|*[!0-9]*|0)
            echo "$name must be a positive integer" >&2
            exit 2
            ;;
    esac
    if [ "$value" -gt "$maximum" ]; then
        echo "$name must not exceed $maximum" >&2
        exit 2
    fi
}

positive_bounded "$request_count" AUTHORIZATION_LOAD_REQUESTS 4096
positive_bounded "$concurrency" AUTHORIZATION_LOAD_CONCURRENCY 256
positive_bounded "$request_timeout" AUTHORIZATION_LOAD_TIMEOUT_SECONDS 60
positive_bounded "$database_pool_size" API_DATABASE_POOL_SIZE 256
positive_bounded "$api_port" AUTHORIZATION_API_PORT 65535
positive_bounded "$replica_api_port" AUTHORIZATION_REPLICA_API_PORT 65535
positive_bounded "$postgres_port" AUTHORIZATION_POSTGRES_PORT 65535
positive_bounded "$p95_budget_ms" AUTHORIZATION_P95_BUDGET_MS 60000
case "$allow_budget_miss" in
    true|false) ;;
    *)
        echo "ALLOW_LATENCY_BUDGET_MISS must be true or false" >&2
        exit 2
        ;;
esac
if [ "$api_port" = "$replica_api_port" ] || [ "$api_port" = "$postgres_port" ] || [ "$replica_api_port" = "$postgres_port" ]; then
    echo "authorization benchmark ports must be distinct" >&2
    exit 2
fi

command -v docker >/dev/null 2>&1 || { echo "Docker is required" >&2; exit 1; }
command -v curl >/dev/null 2>&1 || { echo "curl is required" >&2; exit 1; }
command -v jq >/dev/null 2>&1 || { echo "jq is required" >&2; exit 1; }
command -v openssl >/dev/null 2>&1 || { echo "OpenSSL is required" >&2; exit 1; }
command -v awk >/dev/null 2>&1 || { echo "awk is required" >&2; exit 1; }

b64url() {
    printf '%s' "$1" | base64 | tr '+/' '-_' | tr -d '=\n'
}

b64url_stdin() {
    base64 | tr '+/' '-_' | tr -d '=\n'
}

openssl genpkey -algorithm ED25519 -out "$integration_dir/signing-key.pem" >/dev/null 2>&1
openssl pkey -in "$integration_dir/signing-key.pem" -pubout -outform DER -out "$integration_dir/public-key.der" >/dev/null 2>&1
public_key=$(tail -c 32 "$integration_dir/public-key.der" | b64url_stdin)

export API_AUTH_ISSUER="https://authorization-load-identity.invalid"
export API_AUTH_AUDIENCE="tax-api"
export API_AUTH_JWKS_KID="authorization-load-key"
export API_AUTH_JWKS_PUBLIC_KEY="$public_key"
export API_PORT="$api_port"
export POSTGRES_PORT="$postgres_port"
export API_DATABASE_POOL_SIZE="$database_pool_size"
export API_SIMULATION_MODE=true
export API_ENV=local
export API_ALLOW_INSECURE_LOCAL_RPC=false

now_unix=$(date +%s)
token_header=$(b64url '{"alg":"EdDSA","kid":"authorization-load-key","typ":"JWT"}')
token_claims=$(b64url "{\"iss\":\"$API_AUTH_ISSUER\",\"sub\":\"capability-load-user\",\"aud\":\"$API_AUTH_AUDIENCE\",\"exp\":$((now_unix + 300)),\"iat\":$((now_unix - 10)),\"nbf\":$((now_unix - 10)),\"merchant_id\":\"$merchant_id\",\"scope\":\"tax:quote tax:rule:read tax:settlement:compile tax:vault:read tax:vault:propose tax:adjustment:propose tax:refund:claim:prepare tax:refund:claim:cancel:prepare tax:refund:execute:prepare tax:indexer:read audit:read\",\"roles\":[\"tax_rule_author\",\"vault_release_proposer\",\"settlement_adjustment_proposer\",\"settlement_adjustment_governance\",\"settlement_adjustment_approver\",\"indexer_operator\"]}")
token_signing_input="$token_header.$token_claims"
token_signature=$(printf '%s' "$token_signing_input" | openssl pkeyutl -sign -inkey "$integration_dir/signing-key.pem" -rawin -in /dev/stdin 2>/dev/null | b64url_stdin)
authorization="Bearer $token_signing_input.$token_signature"

base_url="http://127.0.0.1:$api_port"
replica_base_url="http://127.0.0.1:$replica_api_port"
request_results="$integration_dir/results"
latencies="$integration_dir/latencies"

docker compose -p "$project_name" up --build --detach postgres migrator api >/dev/null
ready=false
for _attempt in $(seq 1 60); do
    if curl --fail --silent "$base_url/readyz" >/dev/null 2>&1; then
        ready=true
        break
    fi
    sleep 1
done
[ "$ready" = true ] || fail "primary API did not become ready"

replica_container_id=$(docker compose -p "$project_name" run --detach --no-deps \
    --publish "$replica_api_port:8080" api)
replica_ready=false
for _attempt in $(seq 1 60); do
    if curl --fail --silent "$replica_base_url/readyz" >/dev/null 2>&1; then
        replica_ready=true
        break
    fi
    sleep 1
done
[ "$replica_ready" = true ] || fail "replica API did not become ready"

validate_response() {
    response_file=$1
    jq -e --arg subject capability-load-user --arg tenant "$merchant_id" '
        .schema == "authorization-capabilities/v1" and
        .subject == $subject and
        .tenant_id == $tenant and
        (.expires_at_unix | type == "number") and
        (.expires_at_unix > now and .expires_at_unix <= (now + 60)) and
        ((.capabilities | keys) == [
            "access.control",
            "evidence.replay",
            "refund.claim.cancel",
            "refund.claim.create",
            "refund.payment.release",
            "settlement.adjustment",
            "settlement.document.wallet",
            "settlement.wallet",
            "tax.document.quote.create",
            "tax.preview.run",
            "tax.quote.create",
            "tax.rule.govern",
            "vault.reconciliation",
            "vault.release"
        ]) and
        .capabilities["refund.claim.cancel"].decision == "allow" and
        .capabilities["refund.claim.create"].decision == "allow" and
        .capabilities["refund.payment.release"].decision == "allow" and
        .capabilities["settlement.adjustment"].decision == "allow" and
        .capabilities["settlement.document.wallet"].decision == "allow" and
        .capabilities["tax.quote.create"].decision == "indeterminate" and
        .capabilities["tax.document.quote.create"].decision == "indeterminate" and
        .capabilities["tax.preview.run"].decision == "allow" and
        .capabilities["tax.rule.govern"].decision == "indeterminate" and
        .capabilities["settlement.wallet"].decision == "allow" and
        .capabilities["vault.reconciliation"].decision == "allow" and
        .capabilities["vault.release"].decision == "allow" and
        .capabilities["evidence.replay"].decision == "allow" and
        .capabilities["access.control"].decision == "allow"
    ' "$response_file" >/dev/null
}

warmup_response="$integration_dir/warmup.json"
warmup_status=$(curl --silent --show-error --output "$warmup_response" \
    --write-out '%{http_code}' --max-time "$request_timeout" \
    --header "authorization: $authorization" "$base_url/v1/authorization/capabilities" || true)
[ "$warmup_status" = 200 ] || fail "warm-up returned HTTP $warmup_status"
if ! validate_response "$warmup_response"; then
    sed -n '1,20p' "$warmup_response" >&2 || true
    fail "warm-up response failed semantic validation"
fi

export load_base_url="$base_url"
export load_replica_base_url="$replica_base_url"
export load_authorization="$authorization"
export load_timeout="$request_timeout"
export load_response_dir="$response_dir"

set +e
seq 1 "$request_count" | xargs -P "$concurrency" -n 1 sh -c '
    ordinal="$1"
    request_url="$load_base_url"
    if [ "$((ordinal % 2))" -eq 0 ]; then request_url="$load_replica_base_url"; fi
    curl --silent --show-error --max-time "$load_timeout" \
        --header "authorization: $load_authorization" \
        --output "$load_response_dir/$ordinal.json" \
        --write-out "%{http_code} %{time_total}\n" \
        "$request_url/v1/authorization/capabilities"
' authorization-load > "$request_results"
xargs_status=$?
set -e
[ "$xargs_status" -eq 0 ] || fail "one or more measured requests failed at transport level"

sample_count=$(wc -l < "$request_results" | tr -d ' ')
[ "$sample_count" -eq "$request_count" ] || fail "expected $request_count samples, got $sample_count"
bad_status_count=$(awk '$1 != 200 { count += 1 } END { print count + 0 }' "$request_results")
[ "$bad_status_count" -eq 0 ] || fail "$bad_status_count measured requests were not HTTP 200"
response_count=$(find "$response_dir" -type f -name '*.json' | wc -l | tr -d ' ')
[ "$response_count" -eq "$request_count" ] || fail "expected $request_count response bodies, got $response_count"
for response_file in "$response_dir"/*.json; do
    if ! validate_response "$response_file"; then
        sed -n '1,20p' "$response_file" >&2 || true
        fail "semantic validation failed for $response_file"
    fi
done

awk '$1 == 200 && $2 ~ /^[0-9]+([.][0-9]+)?$/ { printf "%.6f\n", $2 * 1000 }' "$request_results" | sort -n > "$latencies"
latency_count=$(wc -l < "$latencies" | tr -d ' ')
[ "$latency_count" -eq "$request_count" ] || fail "invalid latency sample count"

percentile() {
    percentile_value=$1
    awk -v count="$latency_count" -v percentile="$percentile_value" '
        BEGIN { rank = int((count * percentile + 99) / 100); if (rank < 1) rank = 1 }
        NR == rank { printf "%.3f", $1; exit }
    ' "$latencies"
}

p50_ms=$(percentile 50)
p95_ms=$(percentile 95)
p99_ms=$(percentile 99)
p95_budget_met=$(awk -v observed="$p95_ms" -v budget="$p95_budget_ms" 'BEGIN { print (observed <= budget) ? "true" : "false" }')
if [ "$p95_budget_met" != true ] && [ "$allow_budget_miss" != true ]; then
    fail "p95 latency budget missed: observed=${p95_ms}ms budget=${p95_budget_ms}ms; set ALLOW_LATENCY_BUDGET_MISS=true only for a reviewed diagnostic exception"
fi

printf '%s\n' \
    'scope=local_synthetic' \
    'service=api' \
    'endpoint=/v1/authorization/capabilities' \
    'replicas=2' \
    "requests=$request_count" \
    "concurrency=$concurrency" \
    "timeout_seconds=$request_timeout" \
    "status_200=$sample_count" \
    "status_non_200=$bad_status_count" \
    "p50_ms=$p50_ms" \
    "p95_ms=$p95_ms" \
    "p99_ms=$p99_ms" \
    "p95_budget_ms=$p95_budget_ms" \
    "p95_budget_met=$p95_budget_met" \
    "latency_budget_exception=$allow_budget_miss"
