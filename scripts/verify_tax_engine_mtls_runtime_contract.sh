#!/bin/sh

set -eu

# Synthetic transport/image evidence only. No production certificate or key is
# ever read by this contract.
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

project_name=${COMPOSE_PROJECT_NAME:-solana-tax-protocol-tax-engine-mtls-contract}
host_port=${TAX_ENGINE_MTLS_CONTRACT_PORT:-18443}
auth_token=tax-engine-mtls-runtime-contract-token
container_id=''
work_dir=$(mktemp -d)
tls_dir="$work_dir/tls"
mkdir -p "$tls_dir"

fail() {
  echo "tax-engine mTLS runtime contract failed: $1" >&2
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

case "$host_port" in
  ''|*[!0-9]*|0)
    echo "TAX_ENGINE_MTLS_CONTRACT_PORT must be a positive integer" >&2
    exit 2
    ;;
esac
[ "$host_port" -le 65535 ] || { echo "TAX_ENGINE_MTLS_CONTRACT_PORT is too large" >&2; exit 2; }

command -v docker >/dev/null 2>&1 || { echo "Docker is required" >&2; exit 1; }
command -v curl >/dev/null 2>&1 || { echo "curl is required" >&2; exit 1; }
command -v openssl >/dev/null 2>&1 || { echo "OpenSSL is required" >&2; exit 1; }

openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout "$tls_dir/ca.key" -out "$tls_dir/ca.crt" -days 1 \
  -subj '/CN=solana-tax-protocol synthetic mTLS CA' \
  -addext 'basicConstraints=critical,CA:TRUE,pathlen:1' \
  -addext 'keyUsage=critical,keyCertSign,cRLSign' >/dev/null 2>&1

issue_certificate() {
  name=$1
  common_name=$2
  usage=$3
  san=$4
  openssl req -newkey rsa:2048 -nodes \
    -keyout "$tls_dir/$name.key" -out "$tls_dir/$name.csr" \
    -subj "/CN=$common_name" >/dev/null 2>&1
  printf '%s\n' \
    'basicConstraints=critical,CA:FALSE' \
    'keyUsage=critical,digitalSignature,keyEncipherment' \
    "extendedKeyUsage=$usage" \
    "subjectAltName=$san" > "$tls_dir/$name.ext"
  openssl x509 -req -in "$tls_dir/$name.csr" \
    -CA "$tls_dir/ca.crt" -CAkey "$tls_dir/ca.key" \
    -CAcreateserial -out "$tls_dir/$name.crt" -days 1 -sha256 \
    -extfile "$tls_dir/$name.ext" >/dev/null 2>&1
}

issue_certificate server localhost serverAuth 'DNS:localhost,IP:127.0.0.1'
issue_certificate client tax-engine-client clientAuth 'DNS:tax-engine-client,URI:spiffe://solana-tax-protocol/tax-engine-client'
issue_certificate wrong-client tax-engine-wrong-client clientAuth 'DNS:tax-engine-wrong-client,URI:spiffe://solana-tax-protocol/tax-engine-wrong-client'

openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout "$tls_dir/untrusted-ca.key" -out "$tls_dir/untrusted-ca.crt" -days 1 \
  -subj '/CN=untrusted synthetic CA' \
  -addext 'basicConstraints=critical,CA:TRUE,pathlen:1' \
  -addext 'keyUsage=critical,keyCertSign,cRLSign' >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes \
  -keyout "$tls_dir/untrusted-client.key" -out "$tls_dir/untrusted-client.csr" \
  -subj '/CN=untrusted-tax-engine-client' >/dev/null 2>&1
printf '%s\n' \
  'basicConstraints=critical,CA:FALSE' \
  'keyUsage=critical,digitalSignature,keyEncipherment' \
  'extendedKeyUsage=clientAuth' \
  'subjectAltName=DNS:untrusted-tax-engine-client' > "$tls_dir/untrusted-client.ext"
openssl x509 -req -in "$tls_dir/untrusted-client.csr" \
  -CA "$tls_dir/untrusted-ca.crt" -CAkey "$tls_dir/untrusted-ca.key" \
  -CAcreateserial -out "$tls_dir/untrusted-client.crt" -days 1 -sha256 \
  -extfile "$tls_dir/untrusted-client.ext" >/dev/null 2>&1

cp "$tls_dir/ca.crt" "$work_dir/original-ca.crt"
# The image runs as a non-root user. These are ephemeral synthetic fixtures.
chmod 0444 "$tls_dir"/*.crt "$tls_dir"/*.key

docker compose -p "$project_name" build tax-engine
container_id=$(docker compose -p "$project_name" run \
  --detach --rm --no-deps \
  --name "${project_name}-tax-engine" \
  --publish "127.0.0.1:${host_port}:8084" \
  --volume "$tls_dir:/run/tax-engine:ro" \
  --env TAX_ENGINE_ENV=staging \
  --env TAX_ENGINE_AUTH_TOKEN="$auth_token" \
  --env TAX_ENGINE_TLS_CERT_FILE=/run/tax-engine/server.crt \
  --env TAX_ENGINE_TLS_KEY_FILE=/run/tax-engine/server.key \
  --env TAX_ENGINE_MTLS_CA_FILE=/run/tax-engine/ca.crt \
  --env 'TAX_ENGINE_MTLS_ALLOWED_CLIENT_URIS=["spiffe://solana-tax-protocol/tax-engine-client"]' \
  --env OTEL_ENVIRONMENT=local \
  --env OTEL_EXPORTER_OTLP_TRACES_ENDPOINT= \
  tax-engine)

base_url="https://127.0.0.1:${host_port}"
healthy=false
attempt=0
while [ "$attempt" -lt 60 ]; do
  trusted_health_status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --cacert "$tls_dir/ca.crt" --cert "$tls_dir/client.crt" --key "$tls_dir/client.key" \
    --max-time 5 "$base_url/healthz" || true)
  if [ "$trusted_health_status" = '204' ]; then
    healthy=true
    break
  fi
  state=$(docker inspect --format '{{.State.Status}}' "$container_id" 2>/dev/null || true)
  if [ "$state" = 'exited' ] || [ "$state" = 'dead' ]; then
    fail "container exited before trusted mTLS health became ready"
  fi
  attempt=$((attempt + 1))
  sleep 1
done
[ "$healthy" = true ] || fail "trusted mTLS health did not return 204 within the bounded startup window"

no_client_status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
  --cacert "$tls_dir/ca.crt" --max-time 5 "$base_url/healthz" || true)
[ "$no_client_status" != '204' ] || fail "health endpoint accepted a client without a certificate"

untrusted_status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
  --cacert "$tls_dir/ca.crt" --cert "$tls_dir/untrusted-client.crt" \
  --key "$tls_dir/untrusted-client.key" --max-time 5 "$base_url/healthz" || true)
[ "$untrusted_status" != '204' ] || fail "health endpoint accepted an untrusted client certificate"

wrong_identity_status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
  --cacert "$tls_dir/ca.crt" --cert "$tls_dir/wrong-client.crt" \
  --key "$tls_dir/wrong-client.key" --max-time 5 "$base_url/healthz" || true)
[ "$wrong_identity_status" != '204' ] || fail "health endpoint accepted a trusted but unauthorized client identity"

ready_without_token=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
  --cacert "$tls_dir/ca.crt" --cert "$tls_dir/client.crt" --key "$tls_dir/client.key" \
  --max-time 5 "$base_url/readyz" || true)
[ "$ready_without_token" = '401' ] || fail "trusted mTLS readiness without bearer token returned HTTP $ready_without_token"

ready_with_token=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
  --cacert "$tls_dir/ca.crt" --cert "$tls_dir/client.crt" --key "$tls_dir/client.key" \
  --header "authorization: Bearer $auth_token" --max-time 5 "$base_url/readyz" || true)
[ "$ready_with_token" = '204' ] || fail "trusted mTLS readiness returned HTTP $ready_with_token"

printf '%s' '{"rule_set":{"schema":"solana-tax-protocol/tax-rule/v1","rule_set":{"id":"mtls-runtime-flat-rate","version":1,"components":[{"id":"sales-tax","label":"Sales tax","base":{"sources":["Net"]},"rate":{"Flat":{"numerator":"10","denominator":"100"}},"price_mode":"Exclusive","credit_atomic":"0","floor_atomic":null,"cap_atomic":null,"rounding":{"mode":"HalfUp"},"threshold":null,"exemption_codes":[]}] }},"facts":{"gross_atomic":"1100","net_atomic":"1000","currency":"USD","jurisdiction":"US-CA","exemption_codes":[],"cumulative_period":null}}' > "$work_dir/calculate.json"
calculation_status=$(curl --silent --show-error --output "$work_dir/calculation.json" --write-out '%{http_code}' \
  --cacert "$tls_dir/ca.crt" --cert "$tls_dir/client.crt" --key "$tls_dir/client.key" \
  --header 'content-type: application/json' --header "authorization: Bearer $auth_token" \
  --max-time 5 --request POST --data-binary "@$work_dir/calculate.json" "$base_url/v1/calculate" || true)
[ "$calculation_status" = '200' ] || fail "trusted mTLS calculation returned HTTP $calculation_status"
grep -Fq '"total_tax_atomic":"100"' "$work_dir/calculation.json" \
  || fail "mTLS calculation did not return the exact tax amount"
grep -Fq '"total_net_atomic":"1000"' "$work_dir/calculation.json" \
  || fail "mTLS calculation did not return the exact net amount"

alpn_output=$(openssl s_client -connect "127.0.0.1:${host_port}" -servername localhost \
  -alpn h2 -cert "$tls_dir/client.crt" -key "$tls_dir/client.key" \
  -CAfile "$tls_dir/ca.crt" < /dev/null 2>&1 || true)
printf '%s\n' "$alpn_output" | grep -Fq 'ALPN protocol: h2' \
  || fail "trusted mTLS client did not negotiate the configured h2 ALPN protocol"

# An invalid CA replacement must not displace the active verifier.
printf '%s\n' 'not a certificate' > "$tls_dir/replacement-ca.crt"
mv "$tls_dir/replacement-ca.crt" "$tls_dir/ca.crt"
docker kill --signal HUP "$container_id" >/dev/null
sleep 1
retained_health_status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
  --cacert "$work_dir/original-ca.crt" --cert "$tls_dir/client.crt" --key "$tls_dir/client.key" \
  --max-time 5 "$base_url/healthz" || true)
[ "$retained_health_status" = '204' ] || fail "invalid mTLS reload displaced the known-good configuration"

# Restore a valid CA file atomically and prove the signal path remains usable.
cp "$work_dir/original-ca.crt" "$tls_dir/replacement-ca.crt"
mv "$tls_dir/replacement-ca.crt" "$tls_dir/ca.crt"
docker kill --signal HUP "$container_id" >/dev/null
sleep 1
reloaded_health_status=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
  --cacert "$tls_dir/ca.crt" --cert "$tls_dir/client.crt" --key "$tls_dir/client.key" \
  --max-time 5 "$base_url/healthz" || true)
[ "$reloaded_health_status" = '204' ] || fail "valid mTLS reload did not preserve trusted health"

echo "tax-engine mTLS runtime contract passed"
