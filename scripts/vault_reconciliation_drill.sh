#!/bin/sh

set -eu

project_name="solana-tax-protocol-vault-reconciliation"
export POSTGRES_PORT="35432"
export RPC_FIXTURE_PORT="38899"
export RPC_FAILOVER_FIXTURE_PORT="38900"
export INDEXER_HEALTH_PORT="38082"
export INDEXER_ENV="local"
export INDEXER_RPC_ENDPOINTS="http://rpc-fixture:8899"
export INDEXER_ALLOW_INSECURE_LOCAL_RPC="true"
export INDEXER_STREAM_KEY="drill/vault-reconciliation"
export INDEXER_START_SLOT="0"
export INDEXER_START_FINALIZED_SLOT="0"
export INDEXER_PAGE_SLOTS="32"
export INDEXER_POLL_MS="250"
export INDEXER_RPC_TIMEOUT_MS="1000"
export INDEXER_VAULT_RECONCILIATION_POLL_MS="5000"
export INDEXER_VAULT_RECONCILIATION_TARGET_LIMIT="1"

provider_mode="${VAULT_RECONCILIATION_PROVIDER_MODE:-single}"
case "$provider_mode" in
    single|failover) ;;
    *)
        echo "VAULT_RECONCILIATION_PROVIDER_MODE must be single or failover" >&2
        exit 2
        ;;
esac

merchant_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d11"
route_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d12"
target_reference="adr-0136-vault-reconciliation-drill"
vault_address="Sysvar1nstructions1111111111111111111111111"
mint_address="So11111111111111111111111111111111111111112"
token_program="TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
fail_container_name="${project_name}-rpc-fail"
fail_container_id=""

cleanup() {
    exit_status=$?
    if [ "$exit_status" -ne 0 ]; then
        docker compose -p "$project_name" logs --no-color indexer rpc-fixture >&2 || true
    fi
    if [ -n "$fail_container_id" ]; then
        docker rm --force "$fail_container_id" >/dev/null 2>&1 || true
    fi
    docker compose -p "$project_name" down --volumes --remove-orphans >/dev/null 2>&1 || true
    exit "$exit_status"
}
trap cleanup EXIT INT TERM

compose_build_option="--build"
if [ "${VAULT_RECONCILIATION_DRILL_SKIP_BUILD:-false}" = "true" ]; then
    compose_build_option="--no-build"
fi

if [ "$provider_mode" = "failover" ]; then
    docker compose -p "$project_name" --profile indexer --profile settlement-submit-drill \
        up "$compose_build_option" --detach rpc-fixture postgres
    fail_container_id=$(docker compose -p "$project_name" --profile settlement-submit-drill \
        run --detach --no-deps --name "$fail_container_name" \
        --publish "${RPC_FAILOVER_FIXTURE_PORT}:8899" \
        --env RPC_FIXTURE_FAIL_METHODS=getMultipleAccounts rpc-fixture)
    export INDEXER_RPC_ENDPOINTS="http://${fail_container_name}:8899,http://rpc-fixture:8899"
else
    docker compose -p "$project_name" --profile indexer --profile settlement-submit-drill \
        up "$compose_build_option" --detach rpc-fixture postgres indexer
fi

for _attempt in $(seq 1 60); do
    if curl --fail --silent "http://127.0.0.1:${RPC_FIXTURE_PORT}/healthz" >/dev/null 2>&1; then
        break
    fi
    sleep 1
done
curl --fail --silent "http://127.0.0.1:${RPC_FIXTURE_PORT}/healthz" >/dev/null

if [ "$provider_mode" = "failover" ]; then
    for _attempt in $(seq 1 60); do
        if curl --fail --silent "http://127.0.0.1:${RPC_FAILOVER_FIXTURE_PORT}/healthz" >/dev/null 2>&1; then
            break
        fi
        sleep 1
    done
    curl --fail --silent "http://127.0.0.1:${RPC_FAILOVER_FIXTURE_PORT}/healthz" >/dev/null
    docker compose -p "$project_name" --profile indexer \
        up "$compose_build_option" --detach indexer
fi

for _attempt in $(seq 1 60); do
    if docker compose -p "$project_name" exec -T postgres \
        pg_isready -U tax_protocol -d tax_protocol >/dev/null 2>&1; then
        break
    fi
    sleep 1
done
docker compose -p "$project_name" exec -T postgres \
    pg_isready -U tax_protocol -d tax_protocol >/dev/null

for _attempt in $(seq 1 60); do
    if curl --fail --silent "http://127.0.0.1:${INDEXER_HEALTH_PORT}/readyz" >/dev/null 2>&1; then
        break
    fi
    sleep 1
done
curl --fail --silent "http://127.0.0.1:${INDEXER_HEALTH_PORT}/readyz" >/dev/null

docker compose -p "$project_name" exec -T postgres psql -v ON_ERROR_STOP=1 \
    -U tax_protocol -d tax_protocol \
    -v merchant_id="$merchant_id" \
    -v route_id="$route_id" \
    -v target_reference="$target_reference" \
    -v vault_address="$vault_address" \
    -v mint_address="$mint_address" \
    -v token_program="$token_program" <<'SQL'
INSERT INTO vault_reconciliation_targets
    (merchant_id, route_id, stream_key, mint, token_program, tax_vault,
     opening_balance_atomic, opening_balance_slot, opening_reference)
VALUES
    (:'merchant_id'::uuid, :'route_id'::uuid, 'drill/vault-reconciliation',
     :'mint_address', :'token_program', :'vault_address', 1000, 1,
     :'target_reference')
ON CONFLICT (merchant_id, route_id) DO UPDATE
SET status = 'active', updated_at = now();
SQL

matched="false"
for _attempt in $(seq 1 30); do
    matched_output=$(docker compose -p "$project_name" exec -T postgres psql -qAt \
        -U tax_protocol -d tax_protocol \
        -v target_reference="$target_reference" \
        -c "SELECT status || '|' || COALESCE(expected_balance_atomic::text, '') || '|' || COALESCE(observed_balance_atomic::text, '') || '|' || COALESCE(observed_slot::text, '') FROM vault_reconciliation_runs WHERE target_id = (SELECT id FROM vault_reconciliation_targets WHERE opening_reference = '$target_reference') ORDER BY created_at DESC, id DESC LIMIT 1" \
        ) || {
        echo "failed to read vault reconciliation result" >&2
        exit 1
    }
    matched_row=$(printf '%s\n' "$matched_output" | tr -d '\r' | sed -n '1p')
    if [ "$matched_row" = "matched|1000|1000|42" ]; then
        matched="true"
        break
    fi
    sleep 1
done
if [ "$matched" != "true" ]; then
    echo "scheduled vault reconciliation did not produce the expected matched run" >&2
    exit 1
fi

if [ "$provider_mode" = "failover" ]; then
    fail_reads=$(curl --fail --silent "http://127.0.0.1:${RPC_FAILOVER_FIXTURE_PORT}/admin/vault-read-count" \
        | sed -n 's/.*"vault_read_calls":\([0-9][0-9]*\).*/\1/p')
    healthy_reads=$(curl --fail --silent "http://127.0.0.1:${RPC_FIXTURE_PORT}/admin/vault-read-count" \
        | sed -n 's/.*"vault_read_calls":\([0-9][0-9]*\).*/\1/p')
    if [ -z "$fail_reads" ] || [ "$fail_reads" -lt 1 ] || [ -z "$healthy_reads" ] || [ "$healthy_reads" -lt 1 ]; then
        echo "bounded RPC failover did not attempt both providers" >&2
        exit 1
    fi
fi

curl --fail --silent --show-error -X POST \
    -H 'content-type: application/json' \
    --data '{"amount_atomic":999,"slot":43}' \
    "http://127.0.0.1:${RPC_FIXTURE_PORT}/admin/vault-amount" >/dev/null

mismatch="false"
for _attempt in $(seq 1 30); do
    mismatch_output=$(docker compose -p "$project_name" exec -T postgres psql -qAt \
        -U tax_protocol -d tax_protocol \
        -v target_reference="$target_reference" \
        -c "SELECT status || '|' || COALESCE(expected_balance_atomic::text, '') || '|' || COALESCE(observed_balance_atomic::text, '') || '|' || COALESCE(observed_slot::text, '') FROM vault_reconciliation_runs WHERE target_id = (SELECT id FROM vault_reconciliation_targets WHERE opening_reference = '$target_reference') ORDER BY created_at DESC, id DESC LIMIT 1" \
        ) || {
        echo "failed to read vault reconciliation mismatch result" >&2
        exit 1
    }
    mismatch_row=$(printf '%s\n' "$mismatch_output" | tr -d '\r' | sed -n '1p')
    if [ "$mismatch_row" = "mismatch|1000|999|43" ]; then
        mismatch="true"
        break
    fi
    sleep 1
done
if [ "$mismatch" != "true" ]; then
    echo "scheduled vault reconciliation did not produce the expected mismatch run" >&2
    exit 1
fi

run_count_output=$(docker compose -p "$project_name" exec -T postgres psql -qAt \
    -U tax_protocol -d tax_protocol \
    -v target_reference="$target_reference" \
    -c "SELECT COUNT(*) FROM vault_reconciliation_runs WHERE target_id = (SELECT id FROM vault_reconciliation_targets WHERE opening_reference = '$target_reference')" \
    ) || {
    echo "failed to count vault reconciliation runs" >&2
    exit 1
}
run_count=$(printf '%s\n' "$run_count_output" | tr -d '[:space:]')
if [ "$run_count" -lt 2 ]; then
    echo "vault reconciliation runs were not append-only" >&2
    exit 1
fi

metrics=$(curl --fail --silent "http://127.0.0.1:${INDEXER_HEALTH_PORT}/metrics")
printf '%s\n' "$metrics" | grep -Fq 'stp_indexer_vault_reconciliation_mismatch{service="indexer"} 1'
printf '%s\n' "$metrics" | grep -Fq 'stp_indexer_vault_reconciliation_error{service="indexer"} 0'

echo "vault-reconciliation-drill"
echo "target_reference=$target_reference"
echo "provider_mode=$provider_mode"
echo "matched_run=matched|1000|1000|42"
echo "mismatch_run=mismatch|1000|999|43"
echo "append_only_run_count=$run_count"
echo "mismatch_metric=1"
echo "error_metric=0"
if [ "$provider_mode" = "failover" ]; then
    echo "failing_provider_vault_reads=$fail_reads"
    echo "healthy_provider_vault_reads=$healthy_reads"
fi
