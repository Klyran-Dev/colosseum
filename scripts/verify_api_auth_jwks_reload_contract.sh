#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

project_name=${COMPOSE_PROJECT_NAME:-solana-tax-protocol-api-jwks-reload}
host_port=${API_AUTH_JWKS_RELOAD_PORT:-18082}
container_id=''
work_dir=$(mktemp -d)
issuer=https://api-jwks-reload-identity.invalid
audience=tax-api
merchant_id=01992f1b-4a70-7e30-8ea2-b6e37a9d9d11

fail() {
  echo "api JWKS reload contract failed: $1" >&2
  if [ -n "$container_id" ]; then
    docker logs "$container_id" 2>&1 | tail -60 >&2 || true
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
command -v openssl >/dev/null 2>&1 || fail "OpenSSL is required"

b64url() {
  printf '%s' "$1" | base64 | tr '+/' '-_' | tr -d '=\n'
}

b64url_stdin() {
  base64 | tr '+/' '-_' | tr -d '=\n'
}

create_signing_key() {
  key_path=$1
  public_path=$2
  openssl genpkey -algorithm ED25519 -out "$key_path" >/dev/null 2>&1
  openssl pkey -in "$key_path" -pubout -outform DER -out "$public_path" >/dev/null 2>&1
  tail -c 32 "$public_path" | b64url_stdin
}

write_jwks() {
  kid=$1
  public_key=$2
  destination=$3
  printf '{"keys":[{"kid":"%s","kty":"OKP","crv":"Ed25519","alg":"EdDSA","x":"%s"}]}' \
    "$kid" "$public_key" > "$destination"
}

sign_token() {
  kid=$1
  key_path=$2
  now_unix=$(date +%s)
  token_header=$(b64url '{"alg":"EdDSA","kid":"'"$kid"'","typ":"JWT"}')
  token_claims=$(b64url "{\"iss\":\"$issuer\",\"sub\":\"jwks-reload-user\",\"aud\":\"$audience\",\"exp\":$((now_unix + 300)),\"iat\":$((now_unix - 10)),\"nbf\":$((now_unix - 10)),\"merchant_id\":\"$merchant_id\",\"scope\":\"tax:quote\",\"roles\":[]}")
  signing_input="$token_header.$token_claims"
  token_signature=$(printf '%s' "$signing_input" \
    | openssl pkeyutl -sign -inkey "$key_path" -rawin -in /dev/stdin 2>/dev/null \
    | b64url_stdin)
  printf 'Bearer %s.%s' "$signing_input" "$token_signature"
}

first_key="$work_dir/first-key.pem"
first_public_der="$work_dir/first-public.der"
second_key="$work_dir/second-key.pem"
second_public_der="$work_dir/second-public.der"
first_public_key=$(create_signing_key "$first_key" "$first_public_der")
second_public_key=$(create_signing_key "$second_key" "$second_public_der")
jwks_file="$work_dir/jwks.json"
write_jwks first-key "$first_public_key" "$jwks_file"

docker compose -p "$project_name" build api
container_id=$(docker compose -p "$project_name" run \
  --detach \
  --no-deps \
  --name "${project_name}-api" \
  --publish "127.0.0.1:${host_port}:8080" \
  --volume "$work_dir:/run/api-auth:ro" \
  --env API_ENV=local \
  --env API_SIMULATION_MODE=true \
  --env API_DATABASE_URL= \
  --env API_AUTH_ISSUER="$issuer" \
  --env API_AUTH_AUDIENCE="$audience" \
  --env API_AUTH_JWKS_FILE=/run/api-auth/jwks.json \
  --env API_AUTH_JWKS_JSON= \
  --env API_AUTH_JWKS_KID= \
  --env API_AUTH_JWKS_PUBLIC_KEY= \
  api)

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
    fail "API exited before health became ready"
  fi
  attempt=$((attempt + 1))
  sleep 1
done
[ "$healthy" = true ] || fail "API health did not return 204 within the bounded startup window"

quote_body='{"idempotency_key":"jwks-reload-contract","rule_set_id":"not-used","rule_version":1,"jurisdiction":"US-CA","regime":"sales-tax","facts":{"gross_atomic":"1100","net_atomic":"1000","atomic_decimals":2,"currency":"USD","jurisdiction":"US-CA"},"jurisdiction_evidence":[]}'
first_token=$(sign_token first-key "$first_key")
second_token=$(sign_token second-key "$second_key")

status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
  --request POST --header 'content-type: application/json' \
  --header "authorization: $first_token" --data-binary "$quote_body" \
  "$base_url/v1/tax/quote" || true)
[ "$status" = '503' ] || fail "initial valid key did not pass authentication (HTTP $status)"

temporary_jwks="$work_dir/jwks.json.next"
write_jwks second-key "$second_public_key" "$temporary_jwks"
mv "$temporary_jwks" "$jwks_file"
docker kill --signal HUP "$container_id" >/dev/null
sleep 1

status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
  --request POST --header 'content-type: application/json' \
  --header "authorization: $first_token" --data-binary "$quote_body" \
  "$base_url/v1/tax/quote" || true)
[ "$status" = '401' ] || fail "removed key was still accepted after valid reload (HTTP $status)"

status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
  --request POST --header 'content-type: application/json' \
  --header "authorization: $second_token" --data-binary "$quote_body" \
  "$base_url/v1/tax/quote" || true)
[ "$status" = '503' ] || fail "replacement key was not accepted after valid reload (HTTP $status)"

printf '%s' '{not-json' > "$temporary_jwks"
mv "$temporary_jwks" "$jwks_file"
docker kill --signal HUP "$container_id" >/dev/null
sleep 1

status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
  --request POST --header 'content-type: application/json' \
  --header "authorization: $second_token" --data-binary "$quote_body" \
  "$base_url/v1/tax/quote" || true)
[ "$status" = '503' ] || fail "invalid replacement displaced the last known-good key (HTTP $status)"

echo "api JWKS reload contract passed"
