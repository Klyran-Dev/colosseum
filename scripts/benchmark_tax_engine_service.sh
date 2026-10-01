#!/bin/sh

set -eu

# Bounded local evidence only. This measures the warm HTTP adapter, not a
# production regional SLO and not the settlement-critical in-process path.
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

project_name=${COMPOSE_PROJECT_NAME:-solana-tax-protocol-tax-engine-benchmark}
host_port=${TAX_ENGINE_BENCHMARK_PORT:-18085}
request_count=${TAX_ENGINE_BENCHMARK_REQUESTS:-256}
concurrency=${TAX_ENGINE_BENCHMARK_CONCURRENCY:-32}
request_timeout=${TAX_ENGINE_BENCHMARK_TIMEOUT_SECONDS:-10}
cache_capacity=${TAX_ENGINE_CACHE_CAPACITY:-64}
max_concurrency=${TAX_ENGINE_MAX_CONCURRENCY:-32}
auth_token=tax-engine-service-benchmark-token
container_id=''
work_dir=$(mktemp -d)
response_dir="$work_dir/responses"
mkdir -p "$response_dir"

fail() {
  echo "tax-engine service benchmark failed: $1" >&2
  if [ -n "$container_id" ]; then
    docker logs "$container_id" 2>&1 | tail -40 >&2 || true
  fi
  exit 1
}

cleanup() {
  exit_code=$?
  if [ -n "$container_id" ]; then
    docker rm -f "$container_id" >/dev/null 2>&1 || true
  fi
  docker compose -p "$project_name" down --volumes --remove-orphans >/dev/null 2>&1 || true
  rm -rf "$work_dir"
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

positive_bounded "$host_port" TAX_ENGINE_BENCHMARK_PORT 65535
positive_bounded "$request_count" TAX_ENGINE_BENCHMARK_REQUESTS 4096
positive_bounded "$concurrency" TAX_ENGINE_BENCHMARK_CONCURRENCY 256
positive_bounded "$request_timeout" TAX_ENGINE_BENCHMARK_TIMEOUT_SECONDS 60
positive_bounded "$cache_capacity" TAX_ENGINE_CACHE_CAPACITY 256
positive_bounded "$max_concurrency" TAX_ENGINE_MAX_CONCURRENCY 256

command -v docker >/dev/null 2>&1 || { echo "Docker is required" >&2; exit 1; }
command -v curl >/dev/null 2>&1 || { echo "curl is required" >&2; exit 1; }
command -v awk >/dev/null 2>&1 || { echo "awk is required" >&2; exit 1; }
command -v sort >/dev/null 2>&1 || { echo "sort is required" >&2; exit 1; }

docker compose -p "$project_name" build tax-engine
container_id=$(docker compose -p "$project_name" run \
  --detach --rm --no-deps \
  --name "${project_name}-tax-engine" \
  --publish "127.0.0.1:${host_port}:8084" \
  --env TAX_ENGINE_ENV=local \
  --env TAX_ENGINE_AUTH_TOKEN="$auth_token" \
  --env TAX_ENGINE_CACHE_CAPACITY="$cache_capacity" \
  --env TAX_ENGINE_MAX_CONCURRENCY="$max_concurrency" \
  --env OTEL_ENVIRONMENT=local \
  --env OTEL_EXPORTER_OTLP_TRACES_ENDPOINT= \
  tax-engine)

base_url="http://127.0.0.1:${host_port}"
healthy=false
attempt=0
while [ "$attempt" -lt 60 ]; do
  health_status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    "$base_url/healthz" || true)
  if [ "$health_status" = '204' ]; then
    healthy=true
    break
  fi
  state=$(docker inspect --format '{{.State.Status}}' "$container_id" 2>/dev/null || true)
  if [ "$state" = 'exited' ] || [ "$state" = 'dead' ]; then
    fail "container exited before health became ready"
  fi
  attempt=$((attempt + 1))
  sleep 1
done
[ "$healthy" = true ] || fail "health endpoint did not return 204 within the bounded startup window"

printf '%s' '{"rule_set":{"schema":"solana-tax-protocol/tax-rule/v1","rule_set":{"id":"service-benchmark-flat-rate","version":1,"components":[{"id":"sales-tax","label":"Sales tax","base":{"sources":["Net"]},"rate":{"Flat":{"numerator":"10","denominator":"100"}},"price_mode":"Exclusive","credit_atomic":"0","floor_atomic":null,"cap_atomic":null,"rounding":{"mode":"HalfUp"},"threshold":null,"exemption_codes":[]}] }},"facts":{"gross_atomic":"1100","net_atomic":"1000","currency":"USD","jurisdiction":"US-CA","exemption_codes":[],"cumulative_period":null}}' > "$work_dir/calculate.json"

warm_status=$(curl --silent --show-error --output "$work_dir/warm.json" --write-out '%{http_code}' \
  --request POST --header 'content-type: application/json' \
  --header "authorization: Bearer $auth_token" --max-time "$request_timeout" \
  --data-binary "@$work_dir/calculate.json" "$base_url/v1/calculate" || true)
[ "$warm_status" = '200' ] || fail "warm-up request returned HTTP $warm_status"
grep -Fq '"total_tax_atomic":"100"' "$work_dir/warm.json" \
  || fail "warm-up response did not contain the exact tax amount"
grep -Fq '"total_net_atomic":"1000"' "$work_dir/warm.json" \
  || fail "warm-up response did not contain the exact net amount"

set +e
seq "$request_count" | xargs -P "$concurrency" -n 1 sh -c '
  curl --silent --show-error --output "$4/response-$6.json" \
    --write-out "%{http_code} %{time_total}\n" --request POST \
    --header "content-type: application/json" --header "authorization: Bearer $2" \
    --max-time "$5" --data-binary "@$3" "$1/v1/calculate"
' _ "$base_url" "$auth_token" "$work_dir/calculate.json" "$response_dir" "$request_timeout" \
  > "$work_dir/results.txt"
xargs_status=$?
set -e
[ "$xargs_status" -eq 0 ] || fail "one or more measured requests failed at transport level"

sample_count=$(wc -l < "$work_dir/results.txt" | tr -d ' ')
[ "$sample_count" -eq "$request_count" ] \
  || fail "expected $request_count response samples but recorded $sample_count"
bad_status_count=$(awk '$1 != 200 { count += 1 } END { print count + 0 }' "$work_dir/results.txt")
[ "$bad_status_count" -eq 0 ] \
  || fail "$bad_status_count measured requests did not return HTTP 200"
response_count=$(find "$response_dir" -type f -name 'response-*.json' | wc -l | tr -d ' ')
[ "$response_count" -eq "$request_count" ] \
  || fail "expected $request_count response bodies but retained $response_count"
for response_file in "$response_dir"/response-*.json; do
  grep -Fq '"total_tax_atomic":"100"' "$response_file" \
    || fail "a measured response did not contain the exact tax amount"
  grep -Fq '"total_net_atomic":"1000"' "$response_file" \
    || fail "a measured response did not contain the exact net amount"
done

sort -k2,2n "$work_dir/results.txt" | awk '{ print $2 * 1000 }' > "$work_dir/durations-ms.txt"
percentile() {
  percentile_value=$1
  awk -v percentile="$percentile_value" '
    { values[NR] = $1 }
    END {
      rank = int((NR * percentile) + 0.999999)
      if (rank < 1) rank = 1
      if (rank > NR) rank = NR
      printf "%.3f", values[rank]
    }
  ' "$work_dir/durations-ms.txt"
}
p50_ms=$(percentile 0.50)
p95_ms=$(percentile 0.95)
p99_ms=$(percentile 0.99)
image_id=$(docker inspect --format '{{.Image}}' "$container_id" 2>/dev/null || true)
host_arch=$(uname -m)
measured_at_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)
result=$(printf '{"scope":"local_synthetic","service":"tax_engine","measured_at_utc":"%s","host_arch":"%s","image_id":"%s","requests":%s,"concurrency":%s,"request_timeout_seconds":%s,"cache_capacity":%s,"max_concurrency":%s,"http_status_200":%s,"p50_ms":%s,"p95_ms":%s,"p99_ms":%s}' \
  "$measured_at_utc" "$host_arch" "$image_id" "$request_count" "$concurrency" \
  "$request_timeout" "$cache_capacity" "$max_concurrency" "$sample_count" \
  "$p50_ms" "$p95_ms" "$p99_ms")
printf '%s\n' "$result"

if [ -n "${TAX_ENGINE_BENCHMARK_OUTPUT:-}" ]; then
  mkdir -p "$(dirname -- "$TAX_ENGINE_BENCHMARK_OUTPUT")"
  printf '%s\n' "$result" > "$TAX_ENGINE_BENCHMARK_OUTPUT"
fi
