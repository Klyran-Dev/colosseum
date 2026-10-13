#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

synthetic_database='postgres://staging_user:synthetic@db.example.invalid:5432/tax_protocol?sslmode=verify-full'
synthetic_rpc='https://rpc-a.example.invalid,https://rpc-b.example.invalid'

run_checker() {
  simulation_mode=${1:-false}
  seed_dir=${2:-}
  insecure_rpc=${3:-false}
  auth_mode=${4:-pair}
  jurisdiction_mode=${5:-pair}
  tax_engine_env=${6:-staging}
  tax_engine_token=${7-synthetic-nonlocal-service-token}
  tax_engine_tls_cert=${8-/run/secrets/tax-engine/tls.crt}
  tax_engine_tls_key=${9-/run/secrets/tax-engine/tls.key}
  tax_engine_mtls_ca=${10-/run/secrets/tax-engine/client-ca.pem}
  tax_engine_client_uris=${11-'["spiffe://solana-tax-protocol/api"]'}
  tax_engine_token_file=${12-}
  auth_environment=${13-staging}
  auth_self_signup=${14-false}
  auth_email_mode=${15-provider}
  browser_auth_method=${16-email-otp}
  browser_auth_service_url=${17-https://auth.example.invalid}
  solana_network=${18-devnet}
  browser_rpc_url=${19-https://rpc.example.invalid}
  set -- env -i \
    PATH="$PATH" \
    DEPLOYMENT_ENV=staging \
    AUTH_ENV="$auth_environment" \
    AUTH_SELF_SIGNUP="$auth_self_signup" \
    AUTH_LOCAL_BOOTSTRAP_ADMIN_EMAIL= \
    AUTH_DATABASE_URL="$synthetic_database" \
    AUTH_ISSUER=https://identity.example.invalid \
    AUTH_AUDIENCE=tax-api-staging \
    AUTH_SIGNING_KEY_ID=staging-auth-signing-key-v1 \
    AUTH_SIGNING_KEY_HEX=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa \
    AUTH_EMAIL_MODE="$auth_email_mode" \
    AUTH_EMAIL_PROVIDER_ENDPOINT=https://api.resend.com/emails \
    AUTH_EMAIL_PROVIDER_API_KEY=synthetic-provider-key \
    AUTH_EMAIL_SMTP_HOST=smtp.gmail.com \
    AUTH_EMAIL_SMTP_PORT=587 \
    AUTH_EMAIL_SMTP_SECURITY=starttls \
    AUTH_EMAIL_SMTP_AUTH=app-password \
    AUTH_EMAIL_SMTP_TIMEOUT_MILLIS=10000 \
    AUTH_EMAIL_SMTP_USERNAME=noreply@example.invalid \
    AUTH_EMAIL_SMTP_PASSWORD=synthetic-smtp-password \
    AUTH_EMAIL_FROM='Solana Tax Protocol <auth@example.invalid>' \
    AUTH_APP_BASE_URL=https://app.example.invalid \
    AUTH_INITIAL_ADMIN_EMAIL= \
    VITE_DEPLOYMENT_ENVIRONMENT=staging \
    VITE_SOLANA_NETWORK="$solana_network" \
    VITE_SOLANA_RPC_URL="$browser_rpc_url" \
    VITE_AUTH_METHOD="$browser_auth_method" \
    VITE_AUTH_SERVICE_BASE_URL="$browser_auth_service_url" \
    API_ENV=staging \
    API_SIMULATION_MODE="$simulation_mode" \
    API_ALLOW_INSECURE_LOCAL_RPC="$insecure_rpc" \
    INDEXER_ENV=staging \
    INDEXER_ALLOW_INSECURE_LOCAL_RPC=false \
    ROUTE_VERIFIER_ENV=staging \
    ROUTE_VERIFIER_ALLOW_INSECURE_LOCAL_RPC=false \
    TAX_ENGINE_ENV="$tax_engine_env" \
    TAX_ENGINE_AUTH_TOKEN="$tax_engine_token" \
    TAX_ENGINE_AUTH_TOKEN_FILE="$tax_engine_token_file" \
    TAX_ENGINE_TLS_CERT_FILE="$tax_engine_tls_cert" \
    TAX_ENGINE_TLS_KEY_FILE="$tax_engine_tls_key" \
    TAX_ENGINE_MTLS_CA_FILE="$tax_engine_mtls_ca" \
    TAX_ENGINE_MTLS_ALLOWED_CLIENT_URIS="$tax_engine_client_uris" \
    DATABASE_URL="$synthetic_database" \
    API_DATABASE_URL="$synthetic_database" \
    WORKER_DATABASE_URL="$synthetic_database" \
    INDEXER_DATABASE_URL="$synthetic_database" \
    ROUTE_VERIFIER_DATABASE_URL="$synthetic_database" \
    WORKER_NATS_URL=nats://messaging.example.invalid:4222 \
    API_SETTLEMENT_RPC_ENDPOINTS="$synthetic_rpc" \
    API_INDEXER_RPC_ENDPOINTS="$synthetic_rpc" \
    INDEXER_RPC_ENDPOINTS="$synthetic_rpc" \
    ROUTE_VERIFIER_RPC_ENDPOINTS="$synthetic_rpc" \
    API_AUTH_ISSUER=https://identity.example.invalid \
    API_AUTH_AUDIENCE=tax-api-staging \
    API_EVIDENCE_KEY_ID=staging-evidence-key-v1 \
    API_EVIDENCE_KEY_B64=synthetic-evidence-key
  if [ -n "$seed_dir" ]; then
    set -- "$@" SEED_DIR="$seed_dir"
  fi
  if [ "$auth_mode" = json ]; then
    set -- "$@" 'API_AUTH_JWKS_JSON={"keys":[{"kid":"staging-auth-key-v1","kty":"OKP","crv":"Ed25519","x":"synthetic-public-key"}]}'
  elif [ "$auth_mode" = file ]; then
    set -- "$@" API_AUTH_JWKS_FILE=/run/secrets/identity/jwks.json
  else
    set -- "$@" API_AUTH_JWKS_KID=staging-auth-key-v1 API_AUTH_JWKS_PUBLIC_KEY=synthetic-public-key
  fi
  if [ "$jurisdiction_mode" = json ]; then
    set -- "$@" 'API_JURISDICTION_PROVIDER_KEYS_JSON={"keys":[{"key_id":"staging-provider-key-v1","public_key":"synthetic-provider-key"}]}'
  elif [ "$jurisdiction_mode" = file ]; then
    set -- "$@" API_JURISDICTION_PROVIDER_KEYS_FILE=/run/secrets/jurisdiction/providers.json
  elif [ "$jurisdiction_mode" = mixed ]; then
    set -- "$@" API_JURISDICTION_PROVIDER_KEY_ID=staging-provider-key-v1 API_JURISDICTION_PROVIDER_PUBLIC_KEY=synthetic-provider-key 'API_JURISDICTION_PROVIDER_KEYS_JSON={"keys":[{"key_id":"staging-provider-key-v1","public_key":"synthetic-provider-key"}]}'
  else
    set -- "$@" API_JURISDICTION_PROVIDER_KEY_ID=staging-provider-key-v1 API_JURISDICTION_PROVIDER_PUBLIC_KEY=synthetic-provider-key
  fi
  "$@" ./scripts/verify_deployment_config.sh
}

run_checker
run_checker false '' false pair pair staging synthetic-nonlocal-service-token \
  /run/secrets/tax-engine/tls.crt /run/secrets/tax-engine/tls.key \
  /run/secrets/tax-engine/client-ca.pem \
  '["spiffe://solana-tax-protocol/api"]' '' staging false google-smtp
run_checker false '' false json
run_checker false '' false file
run_checker false '' false pair file

if run_checker false '' false pair mixed >/dev/null 2>&1; then
  echo "mixed jurisdiction-provider key forms did not fail" >&2
  exit 1
fi

run_checker false '' false pair json

if run_checker true >/dev/null 2>&1; then
  echo "simulation-mode negative contract did not fail" >&2
  exit 1
fi

if run_checker false '' true >/dev/null 2>&1; then
  echo "insecure-rpc negative contract did not fail" >&2
  exit 1
fi

if run_checker false /seeds >/dev/null 2>&1; then
  echo "seed-directory negative contract did not fail" >&2
  exit 1
fi

if run_checker false '' false pair pair production synthetic-nonlocal-service-token >/dev/null 2>&1; then
  echo "tax engine environment mismatch did not fail" >&2
  exit 1
fi

if run_checker false '' false pair pair staging local-development-only >/dev/null 2>&1; then
  echo "local tax engine token did not fail" >&2
  exit 1
fi

run_checker false '' false pair pair staging '' \
  /run/secrets/tax-engine/tls.crt /run/secrets/tax-engine/tls.key \
  /run/secrets/tax-engine/client-ca.pem \
  '["spiffe://solana-tax-protocol/api"]' \
  /run/secrets/tax-engine/auth-token

if run_checker false '' false pair pair staging synthetic-nonlocal-service-token \
  /run/secrets/tax-engine/tls.crt /run/secrets/tax-engine/tls.key \
  /run/secrets/tax-engine/client-ca.pem \
  '["spiffe://solana-tax-protocol/api"]' \
  /run/secrets/tax-engine/auth-token >/dev/null 2>&1; then
  echo "simultaneous Tax Engine token sources did not fail" >&2
  exit 1
fi

if run_checker false '' false pair pair staging synthetic-nonlocal-service-token \
  /run/secrets/tax-engine/tls.crt /run/secrets/tax-engine/tls.key \
  /run/secrets/tax-engine/client-ca.pem \
  '["spiffe://solana-tax-protocol/api"]' '' production false provider email-otp \
  >/dev/null 2>&1; then
  echo "auth environment mismatch did not fail" >&2
  exit 1
fi

if run_checker false '' false pair pair staging synthetic-nonlocal-service-token \
  /run/secrets/tax-engine/tls.crt /run/secrets/tax-engine/tls.key \
  /run/secrets/tax-engine/client-ca.pem \
  '["spiffe://solana-tax-protocol/api"]' '' staging true provider email-otp \
  >/dev/null 2>&1; then
  echo "auth self-signup negative contract did not fail" >&2
  exit 1
fi

if run_checker false '' false pair pair staging synthetic-nonlocal-service-token \
  /run/secrets/tax-engine/tls.crt /run/secrets/tax-engine/tls.key \
  /run/secrets/tax-engine/client-ca.pem \
  '["spiffe://solana-tax-protocol/api"]' '' staging false local-file email-otp \
  >/dev/null 2>&1; then
  echo "local email mode outside local did not fail" >&2
  exit 1
fi

if run_checker false '' false pair pair staging synthetic-nonlocal-service-token \
  /run/secrets/tax-engine/tls.crt /run/secrets/tax-engine/tls.key \
  /run/secrets/tax-engine/client-ca.pem \
  '["spiffe://solana-tax-protocol/api"]' '' staging false provider unconfigured \
  >/dev/null 2>&1; then
  echo "unconfigured browser auth method did not fail" >&2
  exit 1
fi

if run_checker false '' false pair pair staging synthetic-nonlocal-service-token '' \
  /run/secrets/tax-engine/tls.key /run/secrets/tax-engine/client-ca.pem >/dev/null 2>&1; then
  echo "partial tax engine TLS configuration did not fail" >&2
  exit 1
fi

if run_checker false '' false pair pair staging synthetic-nonlocal-service-token \
  /run/secrets/tax-engine/tls.crt /run/secrets/tax-engine/tls.key \
  /run/secrets/tax-engine/client-ca.pem \
  '["spiffe://solana-tax-protocol/api"]' '' staging false provider email-otp \
  https://auth.example.invalid invalid-network https://rpc.example.invalid \
  >/dev/null 2>&1; then
  echo "invalid Solana network did not fail" >&2
  exit 1
fi

if run_checker false '' false pair pair staging synthetic-nonlocal-service-token \
  /run/secrets/tax-engine/tls.crt /run/secrets/tax-engine/tls.key \
  /run/secrets/tax-engine/client-ca.pem \
  '["spiffe://solana-tax-protocol/api"]' '' staging false provider email-otp \
  https://auth.example.invalid devnet http://localhost:8899 \
  >/dev/null 2>&1; then
  echo "insecure browser Solana RPC did not fail" >&2
  exit 1
fi

echo "deployment configuration contract passed"
