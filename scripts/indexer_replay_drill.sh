#!/bin/sh

set -eu

project_name="solana-tax-protocol-indexer-replay"
stream_key="integration/indexer-replay"
export POSTGRES_PORT="25442"
export RPC_FIXTURE_PORT="28999"
export INDEXER_HEALTH_PORT="18092"
export INDEXER_ENV="local"
export INDEXER_RPC_ENDPOINTS="http://rpc-fixture:8899"
export INDEXER_ALLOW_INSECURE_LOCAL_RPC="true"
export INDEXER_STREAM_KEY="$stream_key"
export INDEXER_START_SLOT="100"
export INDEXER_START_FINALIZED_SLOT="100"
export INDEXER_PAGE_SLOTS="16"
export INDEXER_POLL_MS="5000"
export INDEXER_RPC_TIMEOUT_MS="500"
export INDEXER_REPLAY_LEASE_MS="30000"
export RUST_LOG="info"

evidence_dir="${INDEXER_REPLAY_EVIDENCE_DIR:-test-results/indexer-replay}"
mkdir -p "$evidence_dir"
replica_one=""
replica_two=""

cleanup() {
    exit_status=$?
    if [ -n "$replica_one" ]; then
        docker rm --force "$replica_one" >/dev/null 2>&1 || true
    fi
    if [ -n "$replica_two" ]; then
        docker rm --force "$replica_two" >/dev/null 2>&1 || true
    fi
    if [ "$exit_status" -ne 0 ]; then
        docker compose -p "$project_name" logs --no-color indexer rpc-fixture postgres >&2 || true
    fi
    docker compose -p "$project_name" down --volumes --remove-orphans >/dev/null 2>&1 || true
    exit "$exit_status"
}
trap cleanup EXIT INT TERM

wait_for_rpc() {
    for _attempt in $(seq 1 60); do
        if curl --fail --silent "http://127.0.0.1:${RPC_FIXTURE_PORT}/healthz" >/dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    echo "indexer replay RPC fixture did not become reachable" >&2
    return 1
}

wait_for_replica() {
    replica_id="$1"
    for _attempt in $(seq 1 60); do
        if docker exec "$replica_id" wget --spider --quiet http://127.0.0.1:8082/readyz >/dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    echo "indexer replay replica did not become ready: $replica_id" >&2
    return 1
}

docker compose -p "$project_name" --profile indexer --profile settlement-submit-drill \
    up --build --detach postgres rpc-fixture
wait_for_rpc
docker compose -p "$project_name" --profile indexer run --rm migrator >/dev/null
docker compose -p "$project_name" --profile indexer --profile settlement-submit-drill \
    build indexer >/dev/null

replica_one=$(docker compose -p "$project_name" --profile indexer --profile settlement-submit-drill \
    run --detach --no-deps indexer)
replica_two=$(docker compose -p "$project_name" --profile indexer --profile settlement-submit-drill \
    run --detach --no-deps indexer)
wait_for_replica "$replica_one"
wait_for_replica "$replica_two"

checkpoint_before=$(docker compose -p "$project_name" exec -T postgres \
    psql -qAt -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    -c "SELECT count(*) FROM chain_checkpoints WHERE stream_key = '$stream_key';")
gap_id=$(docker compose -p "$project_name" exec -T postgres \
    psql -qAt -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    -c "INSERT INTO indexer_replay_gaps
        (stream_key, start_slot, end_slot, observed_finalized_slot, reason, attempts, status)
       VALUES ('$stream_key', 20, 21, 42, 'operator_requested', 0, 'queued')
       RETURNING id;" | sed -n '1p')
if [ -z "$gap_id" ]; then
    echo "failed to create the bounded replay gap" >&2
    exit 1
fi

completed="false"
for _attempt in $(seq 1 60); do
    state=$(docker compose -p "$project_name" exec -T postgres \
        psql -qAt -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
        -c "SELECT status || ':' || attempts FROM indexer_replay_gaps WHERE id = '$gap_id';" | sed -n '1p')
    if [ "$state" = "completed:1" ]; then
        completed="true"
        break
    fi
    sleep 1
done
if [ "$completed" != "true" ]; then
    echo "replay gap did not complete once within the bounded recovery window (state: ${state:-missing})" >&2
    exit 1
fi

checkpoint_after=$(docker compose -p "$project_name" exec -T postgres \
    psql -qAt -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    -c "SELECT count(*) FROM chain_checkpoints WHERE stream_key = '$stream_key';")
gap_shape=$(docker compose -p "$project_name" exec -T postgres \
    psql -qAt -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol \
    -c "SELECT status || ':' || attempts || ':' || (claim_token IS NULL)::text || ':' || (completed_at IS NOT NULL)::text
        FROM indexer_replay_gaps WHERE id = '$gap_id';")
if [ "$checkpoint_before" != "$checkpoint_after" ] || [ "$gap_shape" != "completed:1:true:true" ]; then
    echo "replay invariants failed: checkpoint $checkpoint_before->$checkpoint_after, gap=$gap_shape" >&2
    exit 1
fi

metrics_file="$evidence_dir/metrics.txt"
docker exec "$replica_one" wget -qO- http://127.0.0.1:8082/metrics >"$metrics_file"
if ! grep -q 'stp_indexer_replay_gaps{service="indexer",status="queued"}' "$metrics_file" || \
    ! grep -q 'stp_indexer_replay_gaps{service="indexer",status="leased"}' "$metrics_file" || \
    ! grep -q 'stp_indexer_replay_gaps{service="indexer",status="dead_lettered"}' "$metrics_file"; then
    echo "fixed-cardinality replay metrics were not exported" >&2
    exit 1
fi

printf '{"stream_key":"%s","gap_id":"%s","status":"completed","attempts":1,"checkpoint_rows_before":%s,"checkpoint_rows_after":%s,"replicas":2}\n' \
    "$stream_key" "$gap_id" "$checkpoint_before" "$checkpoint_after" >"$evidence_dir/result.json"
echo "indexer replay drill passed: gap=$gap_id replicas=2 attempts=1 checkpoint_rows=$checkpoint_after"
