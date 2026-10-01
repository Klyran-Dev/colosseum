#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

project_name=${COMPOSE_PROJECT_NAME:-solana-tax-protocol-tax-engine-contract}
host_port=${TAX_ENGINE_CONTRACT_PORT:-18084}
auth_token=tax-engine-runtime-contract-token
new_auth_token=tax-engine-runtime-contract-token-v2
container_id=''
work_dir=$(mktemp -d)

fail() {
  echo "tax-engine runtime contract failed: $1" >&2
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

trap cleanup EXIT

printf '%s' "$auth_token" > "$work_dir/auth-token"
chmod 755 "$work_dir"
chmod 644 "$work_dir/auth-token"

docker compose -p "$project_name" build tax-engine
container_id=$(docker compose -p "$project_name" run \
  --detach \
  --rm \
  --no-deps \
  --name "${project_name}-tax-engine" \
  --publish "127.0.0.1:${host_port}:8084" \
  --env TAX_ENGINE_ENV=local \
  --env TAX_ENGINE_AUTH_TOKEN= \
  --env TAX_ENGINE_AUTH_TOKEN_FILE=/run/tax-engine-contract/auth-token \
  --volume "$work_dir:/run/tax-engine-contract:ro" \
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

unauthenticated_ready_status=$(curl --silent --show-error --output "$work_dir/ready-unauthenticated" \
  --write-out '%{http_code}' "$base_url/readyz" || true)
[ "$unauthenticated_ready_status" = '401' ] || fail "unauthenticated readiness returned HTTP $unauthenticated_ready_status"

authenticated_ready_status=$(curl --silent --show-error --output /dev/null \
  --write-out '%{http_code}' --header "authorization: Bearer $auth_token" \
  "$base_url/readyz" || true)
[ "$authenticated_ready_status" = '204' ] || fail "authenticated readiness returned HTTP $authenticated_ready_status"

printf '%s' '{"rule_set":{"schema":"solana-tax-protocol/tax-rule/v1","rule_set":{"id":"runtime-flat-rate","version":1,"components":[{"id":"sales-tax","label":"Sales tax","base":{"sources":["Net"]},"rate":{"Flat":{"numerator":"10","denominator":"100"}},"price_mode":"Exclusive","credit_atomic":"0","floor_atomic":null,"cap_atomic":null,"rounding":{"mode":"HalfUp"},"threshold":null,"exemption_codes":[]}] }},"facts":{"gross_atomic":"1100","net_atomic":"1000","currency":"USD","jurisdiction":"US-CA","exemption_codes":[],"cumulative_period":null}}' > "$work_dir/calculate.json"

calculation_status=$(curl --silent --show-error --output "$work_dir/calculation.json" \
  --write-out '%{http_code}' --request POST \
  --header 'content-type: application/json' \
  --header "authorization: Bearer $auth_token" \
  --data-binary "@$work_dir/calculate.json" \
  "$base_url/v1/calculate" || true)
if [ "$calculation_status" != '200' ]; then
  sed -n '1,12p' "$work_dir/calculation.json" >&2 || true
  fail "valid calculation returned HTTP $calculation_status"
fi
grep -Fq '"total_tax_atomic":"100"' "$work_dir/calculation.json" \
  || fail "calculation did not return the expected exact tax amount"
grep -Fq '"total_net_atomic":"1000"' "$work_dir/calculation.json" \
  || fail "calculation did not return the expected exact net amount"

invalid_request_status=$(curl --silent --show-error --output /dev/null \
  --write-out '%{http_code}' --request POST \
  --header 'content-type: application/json' \
  --header "authorization: Bearer $auth_token" \
  --data '{"unsupported":true}' "$base_url/v1/calculate" || true)
[ "$invalid_request_status" = '400' ] || fail "invalid calculation returned HTTP $invalid_request_status"

wrong_token_status=$(curl --silent --show-error --output /dev/null \
  --write-out '%{http_code}' --request POST \
  --header 'content-type: application/json' \
  --header 'authorization: Bearer wrong-token' \
  --data-binary "@$work_dir/calculate.json" "$base_url/v1/calculate" || true)
[ "$wrong_token_status" = '401' ] || fail "wrong service token returned HTTP $wrong_token_status"

dd if=/dev/zero of="$work_dir/oversized.json" bs=1 count=131073 2>/dev/null
oversized_status=$(curl --silent --show-error --output /dev/null \
  --write-out '%{http_code}' --request POST \
  --header 'content-type: application/json' \
  --header "authorization: Bearer $auth_token" \
  --data-binary "@$work_dir/oversized.json" \
  "$base_url/v1/calculate" || true)
[ "$oversized_status" = '413' ] || fail "oversized request returned HTTP $oversized_status"

metrics_status=$(curl --silent --show-error --output "$work_dir/metrics.txt" \
  --write-out '%{http_code}' "$base_url/metrics" || true)
[ "$metrics_status" = '200' ] || fail "metrics returned HTTP $metrics_status"
grep -Fq 'service="tax_engine"' "$work_dir/metrics.txt" \
  || fail "metrics did not expose the canonical tax_engine identity"
grep -Fq 'operation="calculate"' "$work_dir/metrics.txt" \
  || fail "metrics did not expose the fixed calculate operation"
if grep -Fq "$auth_token" "$work_dir/metrics.txt"; then
  fail "service authentication token leaked into metrics"
fi

printf '%s' "$new_auth_token" > "$work_dir/auth-token.next"
chmod 644 "$work_dir/auth-token.next"
mv "$work_dir/auth-token.next" "$work_dir/auth-token"
docker kill --signal HUP "$container_id" >/dev/null
sleep 1
old_token_after_rotation_status=$(curl --silent --show-error --output /dev/null \
  --write-out '%{http_code}' --header "authorization: Bearer $auth_token" \
  "$base_url/readyz" || true)
[ "$old_token_after_rotation_status" = '401' ] \
  || fail "old service token remained valid after rotation"
new_token_status=$(curl --silent --show-error --output /dev/null \
  --write-out '%{http_code}' --header "authorization: Bearer $new_auth_token" \
  "$base_url/readyz" || true)
[ "$new_token_status" = '204' ] \
  || fail "new service token was not accepted after rotation"

echo "tax-engine runtime contract passed"
