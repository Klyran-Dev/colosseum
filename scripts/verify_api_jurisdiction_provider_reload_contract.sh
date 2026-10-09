#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

project_name=${COMPOSE_PROJECT_NAME:-solana-tax-protocol-jurisdiction-reload}
host_port=${API_JURISDICTION_RELOAD_PORT:-18083}
export POSTGRES_PORT=${JURISDICTION_RELOAD_POSTGRES_PORT:-55433}
container_id=''
work_dir=$(mktemp -d)
issuer=https://api-jurisdiction-reload-identity.invalid
audience=tax-api
merchant_id=01992f1b-4a70-7e30-8ea2-b6e37a9d9d11
first_provider_key_id=$(printf '04%.0s' $(seq 1 32))
second_provider_key_id=$(printf '05%.0s' $(seq 1 32))
first_provider_seed=$(printf '09%.0s' $(seq 1 32))
second_provider_seed=$(printf '0a%.0s' $(seq 1 32))

fail() {
  echo "jurisdiction-provider reload contract failed: $1" >&2
  if [ -n "$container_id" ]; then
    docker logs "$container_id" 2>&1 | tail -80 >&2 || true
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

command -v docker >/dev/null 2>&1 || fail "Docker is required"
command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v jq >/dev/null 2>&1 || fail "jq is required"

b64url() {
  printf '%s' "$1" | base64 | tr '+/' '-_' | tr -d '=\n'
}

b64url_stdin() {
  base64 | tr '+/' '-_' | tr -d '=\n'
}

create_auth_key() {
  key_path=$1
  public_path=$2
  openssl genpkey -algorithm ED25519 -out "$key_path" >/dev/null 2>&1
  openssl pkey -in "$key_path" -pubout -outform DER -out "$public_path" >/dev/null 2>&1
  tail -c 32 "$public_path" | b64url_stdin
}

sign_token() {
  key_path=$1
  now_unix=$(date +%s)
  token_header=$(b64url '{"alg":"EdDSA","kid":"jurisdiction-reload-auth-key","typ":"JWT"}')
  token_claims=$(b64url "{\"iss\":\"$issuer\",\"sub\":\"jurisdiction-reload-user\",\"aud\":\"$audience\",\"exp\":$((now_unix + 300)),\"iat\":$((now_unix - 10)),\"nbf\":$((now_unix - 10)),\"merchant_id\":\"$merchant_id\",\"scope\":\"tax:quote\",\"roles\":[]}")
  signing_input="$token_header.$token_claims"
  token_signature=$(printf '%s' "$signing_input" \
    | openssl pkeyutl -sign -inkey "$key_path" -rawin -in /dev/stdin 2>/dev/null \
    | b64url_stdin)
  printf 'Bearer %s.%s' "$signing_input" "$token_signature"
}

provider_public_key_from_evidence() {
  evidence_path=$1
  binary_path=$2
  jq -r '.provider_public_key[]' "$evidence_path" \
    | while IFS= read -r byte; do
        printf '%b' "\\$(printf '%03o' "$byte")"
      done > "$binary_path"
  base64 < "$binary_path" | tr '+/' '-_' | tr -d '=\n'
}

write_registry() {
  key_id=$1
  public_key=$2
  destination=$3
  printf '{"keys":[{"key_id":"%s","public_key":"%s"}]}' \
    "$key_id" "$public_key" > "$destination"
}

request_without_evidence="$work_dir/request-without-evidence.json"
printf '%s' '{"facts":{"gross_atomic":"1100","net_atomic":"1000","atomic_decimals":2,"currency":"USD","jurisdiction":"US-CA"}}' > "$request_without_evidence"

docker build --file ops/docker/Dockerfile.attestation-fixture \
  --tag "${project_name}-attestation-fixture" . >/dev/null

first_evidence="$work_dir/first-evidence.json"
docker run --rm --read-only --network none --cap-drop ALL \
  --security-opt no-new-privileges:true \
  --env ATTESTATION_PROVIDER_KEY_ID_HEX="$first_provider_key_id" \
  --volume "$request_without_evidence:/fixtures/request.json:ro" \
  "${project_name}-attestation-fixture" jurisdiction-evidence \
  /fixtures/request.json "$merchant_id" "$first_provider_seed" > "$first_evidence"
first_provider_public_key=$(provider_public_key_from_evidence \
  "$first_evidence" "$work_dir/first-provider-public.der")
write_registry "$first_provider_key_id" "$first_provider_public_key" \
  "$work_dir/providers.json"

second_evidence="$work_dir/second-evidence.json"
docker run --rm --read-only --network none --cap-drop ALL \
  --security-opt no-new-privileges:true \
  --env ATTESTATION_PROVIDER_KEY_ID_HEX="$second_provider_key_id" \
  --volume "$request_without_evidence:/fixtures/request.json:ro" \
  "${project_name}-attestation-fixture" jurisdiction-evidence \
  /fixtures/request.json "$merchant_id" "$second_provider_seed" > "$second_evidence"
second_provider_public_key=$(provider_public_key_from_evidence \
  "$second_evidence" "$work_dir/second-provider-public.der")

auth_key="$work_dir/auth-key.pem"
auth_public_der="$work_dir/auth-public.der"
auth_public_key=$(create_auth_key "$auth_key" "$auth_public_der")

docker compose -p "$project_name" build api migrator >/dev/null
docker compose -p "$project_name" up -d postgres >/dev/null
docker compose -p "$project_name" run --rm migrator >/dev/null

docker compose -p "$project_name" run -T --detach --no-deps \
  --name "${project_name}-api" \
  --publish "127.0.0.1:${host_port}:8080" \
  --volume "$work_dir:/run/jurisdiction:ro" \
  --env API_ENV=local \
  --env API_SIMULATION_MODE=true \
  --env API_AUTH_ISSUER="$issuer" \
  --env API_AUTH_AUDIENCE="$audience" \
  --env API_AUTH_JWKS_JSON= \
  --env API_AUTH_JWKS_KID=jurisdiction-reload-auth-key \
  --env API_AUTH_JWKS_PUBLIC_KEY="$auth_public_key" \
  --env API_JURISDICTION_PROVIDER_KEYS_FILE=/run/jurisdiction/providers.json \
  --env API_JURISDICTION_PROVIDER_KEYS_JSON= \
  --env API_JURISDICTION_PROVIDER_KEY_ID= \
  --env API_JURISDICTION_PROVIDER_PUBLIC_KEY= \
  --env API_EVIDENCE_KEY_RING_JSON= \
  --env API_EVIDENCE_KEY_ID=local-development-evidence-key-v1 \
  --env API_EVIDENCE_KEY_B64=bG9jYWwtZGV2LWV2aWRlbmNlLWtleS0zMi1ieXRlcyE= \
  api > "$work_dir/container-id"
container_id=$(sed -n '1p' "$work_dir/container-id")

base_url="http://127.0.0.1:${host_port}"
ready=false
attempt=0
while [ "$attempt" -lt 60 ]; do
  health_status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    "$base_url/readyz" || true)
  if [ "$health_status" = '204' ]; then
    ready=true
    break
  fi
  state=$(docker inspect --format '{{.State.Status}}' "$container_id" 2>/dev/null || true)
  if [ "$state" = 'exited' ] || [ "$state" = 'dead' ]; then
    fail "API exited before readiness"
  fi
  attempt=$((attempt + 1))
  sleep 1
done
[ "$ready" = true ] || fail "API readiness did not return 204 within the bounded startup window"

first_evidence_json=$(cat "$first_evidence")
second_evidence_json=$(cat "$second_evidence")
first_request=$(jq -cn --argjson evidence "$first_evidence_json" \
  '{idempotency_key:"jurisdiction-reload-first",rule_set_id:"operator-simulator-flat-rate",rule_version:1,jurisdiction:"US-CA",regime:"sales-tax",facts:{gross_atomic:"1100",net_atomic:"1000",atomic_decimals:2,currency:"USD",jurisdiction:"US-CA"},jurisdiction_evidence:[$evidence]}')
second_request=$(jq -cn --argjson evidence "$second_evidence_json" \
  '{idempotency_key:"jurisdiction-reload-second",rule_set_id:"operator-simulator-flat-rate",rule_version:1,jurisdiction:"US-CA",regime:"sales-tax",facts:{gross_atomic:"1100",net_atomic:"1000",atomic_decimals:2,currency:"USD",jurisdiction:"US-CA"},jurisdiction_evidence:[$evidence]}')
first_token=$(sign_token "$auth_key")

first_response="$work_dir/first-response.json"
first_status=$(printf '%s' "$first_request" | curl --silent --show-error \
  --output "$first_response" --write-out '%{http_code}' \
  --request POST --header 'content-type: application/json' \
  --header "authorization: $first_token" --data-binary @- \
  "$base_url/v1/tax/quote" || true)
case "$first_status" in
  200|201) ;;
  *) fail "valid evidence signed by the initial key was rejected (HTTP $first_status)" ;;
esac
jq -e '.calculation_id and .status' "$first_response" >/dev/null \
  || fail "initial quote response was not a persisted calculation"

temporary_registry="$work_dir/providers.json.next"
write_registry "$second_provider_key_id" "$second_provider_public_key" "$temporary_registry"
mv "$temporary_registry" "$work_dir/providers.json"
docker kill --signal HUP "$container_id" >/dev/null
sleep 1

old_status=$(printf '%s' "$first_request" | jq -c '.idempotency_key = "jurisdiction-reload-old-after-rotation"' | \
  curl --silent --show-error --output "$work_dir/old-response.json" --write-out '%{http_code}' \
  --request POST --header 'content-type: application/json' \
  --header "authorization: $first_token" --data-binary @- \
  "$base_url/v1/tax/quote" || true)
[ "$old_status" = '422' ] || fail "removed provider key remained accepted after rotation (HTTP $old_status)"
jq -e '.error.code == "JURISDICTION_PROVIDER_KEY_UNKNOWN"' "$work_dir/old-response.json" >/dev/null \
  || fail "removed provider key did not return the typed unknown-key error"

second_response="$work_dir/second-response.json"
second_status=$(printf '%s' "$second_request" | curl --silent --show-error \
  --output "$second_response" --write-out '%{http_code}' \
  --request POST --header 'content-type: application/json' \
  --header "authorization: $first_token" --data-binary @- \
  "$base_url/v1/tax/quote" || true)
case "$second_status" in
  200|201) ;;
  *) fail "replacement provider key was rejected after rotation (HTTP $second_status)" ;;
esac
jq -e '.calculation_id and .status' "$second_response" >/dev/null \
  || fail "replacement quote response was not a persisted calculation"

printf '%s' '{not-json' > "$temporary_registry"
mv "$temporary_registry" "$work_dir/providers.json"
docker kill --signal HUP "$container_id" >/dev/null
sleep 1
retained_status=$(printf '%s' "$second_request" | jq -c '.idempotency_key = "jurisdiction-reload-retained-after-invalid"' | \
  curl --silent --show-error --output "$work_dir/retained-response.json" --write-out '%{http_code}' \
  --request POST --header 'content-type: application/json' \
  --header "authorization: $first_token" --data-binary @- \
  "$base_url/v1/tax/quote" || true)
case "$retained_status" in
  200|201) ;;
  *) fail "invalid replacement displaced the last known-good provider registry (HTTP $retained_status)" ;;
esac

echo "api jurisdiction-provider registry reload contract passed"
