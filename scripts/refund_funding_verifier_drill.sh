#!/bin/sh

set -eu

# This is a Docker-only, synthetic evidence drill. It exercises the real
# route-verifier binary and PostgreSQL lease fence; it never talks to Solana
# mainnet/testnet and never uses a production key.
project_name="solana-tax-protocol-refund-funding-drill"
verifier_a=""
verifier_b=""
evidence_dir="${REFUND_FUNDING_VERIFIER_EVIDENCE_DIR:-}"
result_file=""
if [ -n "$evidence_dir" ]; then
    mkdir -p "$evidence_dir"
    result_file="$evidence_dir/result.env"
fi

merchant_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d21"
configuration_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d22"
second_configuration_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d23"
challenge_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d24"
binding_id="01992f1b-4a70-7e30-8ea2-b6e37a9d9d25"

# These are public synthetic addresses from the checked-in settlement vector.
# CsNj... is the PDA derived from the merchant/mint pair below. The fixture
# validates the same topology rules as the production verifier.
merchant_account="8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR"
mint="GgBaCs3NCBuZN12kCJgAW63ydqohFkHEdfdEXBPzLHq"
tax_vault="LbUiWL3xVV8hTFYBVdbTNrpDo41NKS6o3LHHuDzjfcY"
vault_authority="CsNjEE9EUZUJeAVqSWmbZxT9WB9h4xYvuLNRRPymrGcs"
refund_funder="4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi"
net_source="QWmroo4YnnMqYW3cnxWkFdaTxGD3P7vMSzwMHGbUzwF"
token_program="TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"

write_evidence() {
    status=$1
    if [ -z "$result_file" ]; then
        return 0
    fi
    {
        printf 'status=%s\n' "$status"
        printf 'two_replica_activation_events=%s\n' "${activation_count:-}"
        printf 'two_replica_finalized_reads=%s\n' "${two_replica_read_count:-}"
        printf 'stale_lease_reclaims=%s\n' "${reclaim_count:-}"
        printf 'stale_lease_activation_events=%s\n' "${second_activation_count:-}"
    } > "$result_file"
}

cleanup() {
    exit_status=$?
    write_evidence "$exit_status"
    if [ -n "$verifier_a" ]; then
        docker rm --force "$verifier_a" >/dev/null 2>&1 || true
    fi
    if [ -n "$verifier_b" ]; then
        docker rm --force "$verifier_b" >/dev/null 2>&1 || true
    fi
    docker compose -p "$project_name" down --volumes --remove-orphans >/dev/null 2>&1 || true
    exit "$exit_status"
}
trap cleanup EXIT INT TERM

export ROUTE_VERIFIER_ENV=local
export ROUTE_VERIFIER_RPC_ENDPOINTS=http://rpc-fixture:8899
export ROUTE_VERIFIER_ALLOW_INSECURE_LOCAL_RPC=true
export ROUTE_VERIFIER_BATCH_SIZE=1
export ROUTE_VERIFIER_POLL_MS=100
export ROUTE_VERIFIER_RPC_TIMEOUT_MS=1000
export POSTGRES_PORT=35432
export RPC_FIXTURE_PORT=38899
export RPC_FIXTURE_REFUND_FUNDING_READ_DELAY_MS=750
export RPC_FIXTURE_REFUND_FUNDING_MERCHANT="$merchant_account"
export RPC_FIXTURE_REFUND_FUNDING_MINT="$mint"
export RPC_FIXTURE_REFUND_FUNDING_TAX_VAULT="$tax_vault"
export RPC_FIXTURE_REFUND_FUNDING_VAULT_AUTHORITY="$vault_authority"
export RPC_FIXTURE_REFUND_FUNDING_REFUND_FUNDER="$refund_funder"
export RPC_FIXTURE_REFUND_FUNDING_NET_SOURCE="$net_source"
export RPC_FIXTURE_REFUND_FUNDING_SLOT=888
export RPC_FIXTURE_REFUND_FUNDING_MINT_DECIMALS=6

docker compose -p "$project_name" --profile settlement-submit-drill \
    up --build --detach postgres migrator rpc-fixture >/dev/null
docker compose -p "$project_name" --profile route-verifier build route-verifier >/dev/null

for _attempt in $(seq 1 60); do
    if docker compose -p "$project_name" exec -T postgres \
        pg_isready -U tax_protocol -d tax_protocol >/dev/null 2>&1; then
        break
    fi
    sleep 1
done

for _attempt in $(seq 1 60); do
    if curl --fail --silent "http://127.0.0.1:${RPC_FIXTURE_PORT}/healthz" >/dev/null 2>&1; then
        break
    fi
    sleep 1
done

seed_configuration() {
    configuration=$1
    configuration_digest=$2
    docker compose -p "$project_name" exec -T postgres \
        psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol <<SQL
INSERT INTO refund_funding_configurations
    (id, merchant_id, idempotency_key, request_digest, configuration_digest,
     configuration_key, version, status, merchant_account, mint, tax_vault,
     vault_authority, refund_funder, refund_funder_subject, net_source,
     token_program, proposer_subject, approver_subject, effective_from)
VALUES
    ('$configuration', '$merchant_id', 'drill-$configuration',
     decode(repeat('11', 32), 'hex'), decode('$configuration_digest', 'hex'),
     'drill-$configuration', 1, 'approved', '$merchant_account', '$mint',
     '$tax_vault', '$vault_authority', '$refund_funder',
     'drill-refund-funder', '$net_source', '$token_program',
     'drill-proposer', 'drill-approver', now());
SQL
}

seed_binding() {
    docker compose -p "$project_name" exec -T postgres \
        psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol <<SQL
INSERT INTO settlement_adjustment_authority_challenges
    (id, merchant_id, subject, role, message_digest, expires_at)
VALUES
    ('$challenge_id', '$merchant_id', 'drill-refund-funder', 'refund_funder',
     decode(repeat('22', 32), 'hex'), now() + interval '1 hour');

INSERT INTO settlement_adjustment_authority_bindings
    (id, merchant_id, subject, role, public_key, challenge_id,
     proof_signature, status)
VALUES
    ('$binding_id', '$merchant_id', 'drill-refund-funder', 'refund_funder',
     decode(repeat('01', 32), 'hex'), '$challenge_id',
     decode(repeat('33', 64), 'hex'), 'active');
SQL
}

seed_configuration "$configuration_id" \
    "2222222222222222222222222222222222222222222222222222222222222222"
seed_binding

start_verifier() {
    docker compose -p "$project_name" --profile route-verifier \
        run --detach --no-deps route-verifier
}

wait_for_status() {
    configuration=$1
    expected_status=$2
    for _attempt in $(seq 1 80); do
        status=$(docker compose -p "$project_name" exec -T postgres \
            psql -U tax_protocol -d tax_protocol -Atc \
            "select status from refund_funding_configurations where id = '$configuration';")
        if [ "$status" = "$expected_status" ]; then
            return 0
        fi
        sleep 1
    done
    echo "configuration $configuration did not reach $expected_status (last=$status)" >&2
    exit 1
}

wait_for_pending_lease() {
    configuration=$1
    for _attempt in $(seq 1 40); do
        lease_count=$(docker compose -p "$project_name" exec -T postgres \
            psql -U tax_protocol -d tax_protocol -Atc \
            "select count(*) from refund_funding_configurations where id = '$configuration' and status = 'pending_verification' and verification_lease_token is not null;")
        if [ "$lease_count" = "1" ]; then
            return 0
        fi
        sleep 0.1
    done
    echo "configuration $configuration never acquired a verification lease" >&2
    exit 1
}

verifier_a=$(start_verifier)
wait_for_pending_lease "$configuration_id"
verifier_b=$(start_verifier)
wait_for_status "$configuration_id" active

activation_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from refund_funding_configuration_events where configuration_id = '$configuration_id' and action = 'activate';")
read_count=$(curl --fail --silent "http://127.0.0.1:${RPC_FIXTURE_PORT}/admin/refund-funding-read-count" \
    | sed -E 's/[^0-9]*([0-9]+).*/\1/')
if [ "$activation_count" != "1" ] || [ "$read_count" != "1" ]; then
    echo "two-replica lease fence failed: activations=$activation_count finalized_reads=$read_count" >&2
    exit 1
fi
two_replica_read_count=$read_count

docker rm --force "$verifier_a" >/dev/null
verifier_a=""
docker rm --force "$verifier_b" >/dev/null
verifier_b=""

seed_configuration "$second_configuration_id" \
    "4444444444444444444444444444444444444444444444444444444444444444"

verifier_a=$(start_verifier)
wait_for_pending_lease "$second_configuration_id"
for _attempt in $(seq 1 40); do
    read_count=$(curl --fail --silent "http://127.0.0.1:${RPC_FIXTURE_PORT}/admin/refund-funding-read-count" \
        | sed -E 's/[^0-9]*([0-9]+).*/\1/')
    if [ "$read_count" -ge 2 ]; then
        break
    fi
    sleep 0.1
done
docker rm --force "$verifier_a" >/dev/null
verifier_a=""

docker compose -p "$project_name" exec -T postgres \
    psql -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol -c \
    "UPDATE refund_funding_configurations SET verification_lease_claimed_at = now() - interval '61 seconds' WHERE id = '$second_configuration_id' AND status = 'pending_verification';" \
    >/dev/null

verifier_b=$(start_verifier)
wait_for_status "$second_configuration_id" active

reclaim_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from refund_funding_configuration_events where configuration_id = '$second_configuration_id' and action = 'reclaim_verification';")
second_activation_count=$(docker compose -p "$project_name" exec -T postgres \
    psql -U tax_protocol -d tax_protocol -Atc \
    "select count(*) from refund_funding_configuration_events where configuration_id = '$second_configuration_id' and action = 'activate';")
if [ "$reclaim_count" != "1" ] || [ "$second_activation_count" != "1" ]; then
    echo "stale lease recovery failed: reclaims=$reclaim_count activations=$second_activation_count" >&2
    exit 1
fi

printf '%s\n' \
    "refund-funding-verifier-drill" \
    "two_replica_activation_events=$activation_count" \
    "two_replica_finalized_reads=1" \
    "stale_lease_reclaims=$reclaim_count" \
    "stale_lease_activation_events=$second_activation_count"
