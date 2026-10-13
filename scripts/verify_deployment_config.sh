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

validate_https_url() {
  name=$1
  require_env "$name"
  value=$(read_env "$name")
  case "$value" in
    https://*) ;;
    *) fail "$name must use https://" ;;
  esac
  case "$value" in
    *localhost*|*127.0.0.1*|*\[::1\]*|*auth-service*|*local-development*)
      fail "$name points to a local or development endpoint"
      ;;
  esac
}

deployment_env=$(read_env DEPLOYMENT_ENV)
case "$deployment_env" in
  staging|production) ;;
  *) fail "DEPLOYMENT_ENV must be staging or production" ;;
esac

require_exact AUTH_ENV "$deployment_env"
require_exact AUTH_SELF_SIGNUP false
[ -z "$(read_env AUTH_LOCAL_BOOTSTRAP_ADMIN_EMAIL)" ] || fail "AUTH_LOCAL_BOOTSTRAP_ADMIN_EMAIL must be empty outside local development"
validate_database_url AUTH_DATABASE_URL
validate_https_url AUTH_ISSUER
require_env AUTH_AUDIENCE
require_env AUTH_SIGNING_KEY_ID
auth_signing_key_id=$(read_env AUTH_SIGNING_KEY_ID)

auth_signing_key_file=$(read_env AUTH_SIGNING_KEY_FILE)
auth_signing_key_hex=$(read_env AUTH_SIGNING_KEY_HEX)
if [ -n "$auth_signing_key_file" ] && [ -n "$auth_signing_key_hex" ]; then
  fail "AUTH_SIGNING_KEY_FILE and AUTH_SIGNING_KEY_HEX are mutually exclusive"
fi
if [ -z "$auth_signing_key_file" ] && [ -z "$auth_signing_key_hex" ]; then
  fail "AUTH_SIGNING_KEY_FILE or AUTH_SIGNING_KEY_HEX is required"
fi
if [ -n "$auth_signing_key_file" ]; then
  case "$auth_signing_key_file" in
    /*) ;;
    *) fail "AUTH_SIGNING_KEY_FILE must be an absolute deployment path" ;;
  esac
fi
case "$auth_signing_key_id:$auth_signing_key_hex" in
  *local-development*) fail "auth signing configuration uses local development material" ;;
esac

email_mode=$(read_env AUTH_EMAIL_MODE)
case "$email_mode" in
  provider)
    validate_https_url AUTH_EMAIL_PROVIDER_ENDPOINT
    require_env AUTH_EMAIL_PROVIDER_API_KEY
    ;;
  google-smtp)
    require_env AUTH_EMAIL_SMTP_HOST
    require_env AUTH_EMAIL_SMTP_PORT
    require_env AUTH_EMAIL_SMTP_SECURITY
    require_env AUTH_EMAIL_SMTP_AUTH
    smtp_host=$(read_env AUTH_EMAIL_SMTP_HOST)
    case "$smtp_host" in
      *localhost*|*127.0.0.1*|*local-development*) fail "AUTH_EMAIL_SMTP_HOST points to a local or development endpoint" ;;
    esac
    smtp_port=$(read_env AUTH_EMAIL_SMTP_PORT)
    case "$smtp_port" in
      587|465) ;;
      *) fail "AUTH_EMAIL_SMTP_PORT must be 587 or 465 outside local development" ;;
    esac
    smtp_security=$(read_env AUTH_EMAIL_SMTP_SECURITY)
    case "$smtp_security:$smtp_port" in
      starttls:587|tls:465) ;;
      *) fail "AUTH_EMAIL_SMTP_SECURITY and AUTH_EMAIL_SMTP_PORT are incompatible" ;;
    esac
    smtp_auth=$(read_env AUTH_EMAIL_SMTP_AUTH)
    case "$smtp_auth" in
      app-password)
        require_env AUTH_EMAIL_SMTP_USERNAME
        require_env AUTH_EMAIL_SMTP_PASSWORD
        case "$(read_env AUTH_EMAIL_SMTP_PASSWORD)" in
          *local-development*) fail "AUTH_EMAIL_SMTP_PASSWORD uses a local development value" ;;
        esac
        ;;
      ip-allowlist)
        [ -z "$(read_env AUTH_EMAIL_SMTP_USERNAME)" ] || fail "SMTP IP-allowlist mode must not receive a username"
        [ -z "$(read_env AUTH_EMAIL_SMTP_PASSWORD)" ] || fail "SMTP IP-allowlist mode must not receive a password"
        ;;
      *) fail "AUTH_EMAIL_SMTP_AUTH must be app-password or ip-allowlist" ;;
    esac
    ;;
  *) fail "AUTH_EMAIL_MODE must be provider or google-smtp" ;;
esac
require_env AUTH_EMAIL_FROM
validate_https_url AUTH_APP_BASE_URL
initial_admin_email=$(read_env AUTH_INITIAL_ADMIN_EMAIL)
if [ -n "$initial_admin_email" ]; then
  case "$initial_admin_email" in
    *@*.*) ;;
    *) fail "AUTH_INITIAL_ADMIN_EMAIL must be a valid email address" ;;
  esac
  case "$initial_admin_email" in
    *[!A-Za-z0-9.@_%+-]*) fail "AUTH_INITIAL_ADMIN_EMAIL contains unsupported characters" ;;
  esac
fi

require_exact VITE_DEPLOYMENT_ENVIRONMENT "$deployment_env"
require_env VITE_SOLANA_NETWORK
solana_network=$(read_env VITE_SOLANA_NETWORK)
case "$deployment_env:$solana_network" in
  staging:devnet|production:mainnet-beta) ;;
  *) fail "VITE_SOLANA_NETWORK must be devnet for staging or mainnet-beta for production" ;;
esac
validate_https_url VITE_SOLANA_RPC_URL
browser_auth_method=$(read_env VITE_AUTH_METHOD)
case "$browser_auth_method" in
  email-otp)
    validate_https_url VITE_AUTH_SERVICE_BASE_URL
    ;;
  oidc)
    validate_https_url VITE_OIDC_ISSUER
    require_env VITE_OIDC_CLIENT_ID
    validate_https_url VITE_OIDC_REDIRECT_URI
    require_env VITE_OIDC_SCOPE
    case " $(read_env VITE_OIDC_SCOPE) " in
      *" openid "*) ;;
      *) fail "VITE_OIDC_SCOPE must include openid" ;;
    esac
    ;;
  *) fail "VITE_AUTH_METHOD must be email-otp or oidc" ;;
esac

require_exact API_ENV "$deployment_env"
require_exact API_SIMULATION_MODE false
require_exact API_ALLOW_INSECURE_LOCAL_RPC false
require_exact INDEXER_ENV "$deployment_env"
require_exact INDEXER_ALLOW_INSECURE_LOCAL_RPC false
require_exact ROUTE_VERIFIER_ENV "$deployment_env"
require_exact ROUTE_VERIFIER_ALLOW_INSECURE_LOCAL_RPC false
require_exact TAX_ENGINE_ENV "$deployment_env"
tax_engine_auth_token=$(read_env TAX_ENGINE_AUTH_TOKEN)
tax_engine_auth_token_file=$(read_env TAX_ENGINE_AUTH_TOKEN_FILE)
if [ -n "$tax_engine_auth_token" ] && [ -n "$tax_engine_auth_token_file" ]; then
  fail "TAX_ENGINE_AUTH_TOKEN and TAX_ENGINE_AUTH_TOKEN_FILE are mutually exclusive"
fi
if [ -z "$tax_engine_auth_token" ] && [ -z "$tax_engine_auth_token_file" ]; then
  fail "TAX_ENGINE_AUTH_TOKEN or TAX_ENGINE_AUTH_TOKEN_FILE is required"
fi
case "$tax_engine_auth_token" in
  local-development-only) fail "TAX_ENGINE_AUTH_TOKEN uses the local development token" ;;
esac
for tax_engine_tls_name in \
  TAX_ENGINE_TLS_CERT_FILE \
  TAX_ENGINE_TLS_KEY_FILE \
  TAX_ENGINE_MTLS_CA_FILE
do
  require_env "$tax_engine_tls_name"
done
require_env TAX_ENGINE_MTLS_ALLOWED_CLIENT_URIS
case "$(read_env TAX_ENGINE_MTLS_ALLOWED_CLIENT_URIS)" in
  *spiffe://*) ;;
  *) fail "TAX_ENGINE_MTLS_ALLOWED_CLIENT_URIS must contain a spiffe:// identity" ;;
esac

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
auth_jwks_file=$(read_env API_AUTH_JWKS_FILE)
if [ -n "$auth_jwks_file" ]; then
  case "$auth_jwks_file" in
    /*) ;;
    *) fail "API_AUTH_JWKS_FILE must be an absolute deployment path" ;;
  esac
  [ -z "$auth_jwks_json" ] || fail "API_AUTH_JWKS_FILE cannot accompany API_AUTH_JWKS_JSON"
  [ -z "$(read_env API_AUTH_JWKS_KID)" ] || fail "API_AUTH_JWKS_FILE cannot accompany API_AUTH_JWKS_KID"
  [ -z "$(read_env API_AUTH_JWKS_PUBLIC_KEY)" ] || fail "API_AUTH_JWKS_FILE cannot accompany API_AUTH_JWKS_PUBLIC_KEY"
elif [ -n "$auth_jwks_json" ]; then
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
if [ -z "$auth_jwks_json" ] && [ -z "$auth_jwks_file" ]; then
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

jurisdiction_provider_keys_json=$(read_env API_JURISDICTION_PROVIDER_KEYS_JSON)
jurisdiction_provider_keys_file=$(read_env API_JURISDICTION_PROVIDER_KEYS_FILE)
if [ -n "$jurisdiction_provider_keys_file" ]; then
  case "$jurisdiction_provider_keys_file" in
    /*) ;;
    *) fail "API_JURISDICTION_PROVIDER_KEYS_FILE must be an absolute deployment path" ;;
  esac
  [ -z "$jurisdiction_provider_keys_json" ] || fail "API_JURISDICTION_PROVIDER_KEYS_FILE cannot accompany API_JURISDICTION_PROVIDER_KEYS_JSON"
  [ -z "$(read_env API_JURISDICTION_PROVIDER_KEY_ID)" ] || fail "API_JURISDICTION_PROVIDER_KEYS_FILE cannot accompany API_JURISDICTION_PROVIDER_KEY_ID"
  [ -z "$(read_env API_JURISDICTION_PROVIDER_PUBLIC_KEY)" ] || fail "API_JURISDICTION_PROVIDER_KEYS_FILE cannot accompany API_JURISDICTION_PROVIDER_PUBLIC_KEY"
elif [ -n "$jurisdiction_provider_keys_json" ]; then
  case "$jurisdiction_provider_keys_json" in
    \{*\}) ;;
    *) fail "API_JURISDICTION_PROVIDER_KEYS_JSON must be a non-empty JSON object" ;;
  esac
  case "$jurisdiction_provider_keys_json" in
    *local-development*|*0404040404040404040404040404040404040404040404040404040404040404*|*_RckOFqgx1tk-3jNYC-h2ZH96_drE8WO1wLqyDXp9hg*)
      fail "API_JURISDICTION_PROVIDER_KEYS_JSON contains a local development value"
      ;;
  esac
  [ -z "$(read_env API_JURISDICTION_PROVIDER_KEY_ID)" ] || fail "API_JURISDICTION_PROVIDER_KEY_ID cannot accompany API_JURISDICTION_PROVIDER_KEYS_JSON"
  [ -z "$(read_env API_JURISDICTION_PROVIDER_PUBLIC_KEY)" ] || fail "API_JURISDICTION_PROVIDER_PUBLIC_KEY cannot accompany API_JURISDICTION_PROVIDER_KEYS_JSON"
else
  for jurisdiction_name in API_JURISDICTION_PROVIDER_KEY_ID API_JURISDICTION_PROVIDER_PUBLIC_KEY; do
    require_env "$jurisdiction_name"
  done
fi
if [ -z "$jurisdiction_provider_keys_json" ] && [ -z "$jurisdiction_provider_keys_file" ]; then
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
fi

if [ "${SEED_DIR+x}" = x ]; then
  fail "SEED_DIR must be unset outside local development"
fi

echo "deployment configuration contract passed for $deployment_env"
