#!/bin/sh

# Repository-side preflight for staging and production deployment manifests.
# This script never prints secret values. Runtime binaries retain the final
# authority to validate their own configuration before binding or submitting.
set -eu

fail() {
  echo "deployment configuration contract failed: $1" >&2
  exit 1
}

read_env() {
  printenv "$1" 2>/dev/null || true
}

require_env() {
  name=$1
  value=$(read_env "$name")
  [ -n "$value" ] || fail "$name is required"
}

require_exact() {
  name=$1
  expected=$2
  actual=$(read_env "$name")
  [ "$actual" = "$expected" ] || fail "$name must be exactly $expected"
}

validate_database_url() {
  name=$1
  require_env "$name"
  value=$(read_env "$name")
  case "$value" in
    *local-development-only*|*localhost*|*127.0.0.1*|*postgres:5432*|*sslmode=disable*)
      fail "$name points to a local or insecure database"
      ;;
  esac
  case "$value" in
    *sslmode=verify-full*) ;;
    *) fail "$name must require PostgreSQL sslmode=verify-full" ;;
  esac
}

validate_https_endpoint_list() {
  name=$1
  require_env "$name"
  endpoints=$(read_env "$name")
  old_ifs=$IFS
  IFS=,
  set -f
  set -- $endpoints
  set +f
  IFS=$old_ifs
  [ "$#" -ge 1 ] || fail "$name must contain at least one endpoint"
  [ "$#" -le 3 ] || fail "$name must contain at most three endpoints"
  for endpoint in "$@"; do
    [ -n "$endpoint" ] || fail "$name contains an empty endpoint"
    case "$endpoint" in
      https://*) ;;
      *) fail "$name endpoints must use https://" ;;
    esac
    case "$endpoint" in
      *localhost*|*127.0.0.1*|*validator:*|*http://*)
        fail "$name contains a local or insecure endpoint"
        ;;
    esac
  done
}

deployment_env=$(read_env DEPLOYMENT_ENV)
case "$deployment_env" in
  staging|production) ;;
  *) fail "DEPLOYMENT_ENV must be staging or production" ;;
esac

require_exact API_ENV "$deployment_env"
require_exact API_SIMULATION_MODE false
require_exact API_ALLOW_INSECURE_LOCAL_RPC false
require_exact INDEXER_ENV "$deployment_env"
require_exact INDEXER_ALLOW_INSECURE_LOCAL_RPC false
require_exact ROUTE_VERIFIER_ENV "$deployment_env"
require_exact ROUTE_VERIFIER_ALLOW_INSECURE_LOCAL_RPC false

for database_name in \
  DATABASE_URL \
  API_DATABASE_URL \
  WORKER_DATABASE_URL \
  INDEXER_DATABASE_URL \
  ROUTE_VERIFIER_DATABASE_URL
do
  validate_database_url "$database_name"
done

for endpoint_name in \
  API_SETTLEMENT_RPC_ENDPOINTS \
  API_INDEXER_RPC_ENDPOINTS \
  INDEXER_RPC_ENDPOINTS \
  ROUTE_VERIFIER_RPC_ENDPOINTS
do
  validate_https_endpoint_list "$endpoint_name"
done

require_env WORKER_NATS_URL
case "$(read_env WORKER_NATS_URL)" in
  *localhost*|*127.0.0.1*|*nats:4222*) fail "WORKER_NATS_URL points to a local broker" ;;
esac

require_env API_AUTH_ISSUER
case "$(read_env API_AUTH_ISSUER)" in
  https://*) ;;
  *) fail "API_AUTH_ISSUER must use https://" ;;
esac
case "$(read_env API_AUTH_ISSUER)" in
  *local-identity.invalid*|*localhost*|*127.0.0.1*) fail "API_AUTH_ISSUER points to local identity" ;;
esac
require_env API_AUTH_AUDIENCE

auth_jwks_json=$(read_env API_AUTH_JWKS_JSON)
if [ -n "$auth_jwks_json" ]; then
  case "$auth_jwks_json" in
    \{*\}) ;;
    *) fail "API_AUTH_JWKS_JSON must be a non-empty JSON object" ;;
  esac
  case "$auth_jwks_json" in
    *local-development*|*fBTjJ2wbKth0799nmehvmjl3jO_Qjjo18jfFqILUQWE*)
      fail "API_AUTH_JWKS_JSON contains local development key material"
      ;;
  esac
  [ -z "$(read_env API_AUTH_JWKS_KID)" ] || fail "API_AUTH_JWKS_KID cannot accompany API_AUTH_JWKS_JSON"
  [ -z "$(read_env API_AUTH_JWKS_PUBLIC_KEY)" ] || fail "API_AUTH_JWKS_PUBLIC_KEY cannot accompany API_AUTH_JWKS_JSON"
else
  for auth_name in API_AUTH_JWKS_KID API_AUTH_JWKS_PUBLIC_KEY; do
    require_env "$auth_name"
  done
fi
if [ -z "$auth_jwks_json" ]; then
  case "$(read_env API_AUTH_JWKS_KID)" in
    *local-development*) fail "API_AUTH_JWKS_KID uses a local development key" ;;
  esac
  case "$(read_env API_AUTH_JWKS_PUBLIC_KEY)" in
    fBTjJ2wbKth0799nmehvmjl3jO_Qjjo18jfFqILUQWE) fail "API_AUTH_JWKS_PUBLIC_KEY uses the local development key" ;;
  esac
fi

evidence_ring=$(read_env API_EVIDENCE_KEY_RING_JSON)
if [ -n "$evidence_ring" ]; then
  case "$evidence_ring" in
    *local-development*|*0404040404040404040404040404040404040404040404040404040404040404*)
      fail "API_EVIDENCE_KEY_RING_JSON contains a local development value"
      ;;
  esac
  [ -z "$(read_env API_EVIDENCE_KEY_ID)" ] || fail "legacy evidence key id cannot accompany API_EVIDENCE_KEY_RING_JSON"
  [ -z "$(read_env API_EVIDENCE_KEY_B64)" ] || fail "legacy evidence key cannot accompany API_EVIDENCE_KEY_RING_JSON"
else
  require_env API_EVIDENCE_KEY_ID
  require_env API_EVIDENCE_KEY_B64
fi
case "$(read_env API_EVIDENCE_KEY_ID)" in
  *local-development*) fail "API_EVIDENCE_KEY_ID uses a local development key" ;;
esac

for jurisdiction_name in API_JURISDICTION_PROVIDER_KEY_ID API_JURISDICTION_PROVIDER_PUBLIC_KEY; do
  require_env "$jurisdiction_name"
done
case "$(read_env API_JURISDICTION_PROVIDER_KEY_ID)" in
  0404040404040404040404040404040404040404040404040404040404040404)
    fail "API_JURISDICTION_PROVIDER_KEY_ID uses the local development key"
    ;;
esac
case "$(read_env API_JURISDICTION_PROVIDER_PUBLIC_KEY)" in
  _RckOFqgx1tk-3jNYC-h2ZH96_drE8WO1wLqyDXp9hg)
    fail "API_JURISDICTION_PROVIDER_PUBLIC_KEY uses the local development key"
    ;;
esac

if [ "${SEED_DIR+x}" = x ]; then
  fail "SEED_DIR must be unset outside local development"
fi

echo "deployment configuration contract passed for $deployment_env"
