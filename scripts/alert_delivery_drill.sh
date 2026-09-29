#!/bin/sh

set -eu

project_name="solana-tax-protocol-alert-delivery"
export POSTGRES_PORT="25444"
export PROMETHEUS_PORT="49091"
export ALERTMANAGER_PORT="49094"
export GRAFANA_PORT="43001"
export API_PORT="18081"
export EDGE_PORT="8089"
export RUST_LOG="warn"

evidence_dir="${ALERT_DELIVERY_EVIDENCE_DIR:-test-results/alert-delivery}"
mkdir -p "$evidence_dir"

cleanup() {
    exit_status=$?
    if [ "$exit_status" -ne 0 ]; then
        docker compose -p "$project_name" logs --no-color alert-receiver alertmanager prometheus >&2 || true
    fi
    docker compose -p "$project_name" down --volumes --remove-orphans >/dev/null 2>&1 || true
    exit "$exit_status"
}
trap cleanup EXIT INT TERM

wait_for_url() {
    url="$1"
    description="$2"
    for _attempt in $(seq 1 90); do
        if curl --fail --silent "$url" >/dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    echo "$description did not become reachable" >&2
    return 1
}

wait_for_receiver_event() {
    for _attempt in $(seq 1 180); do
        events=$(docker compose -p "$project_name" exec -T alert-receiver \
            wget -qO- http://127.0.0.1:8086/admin/events 2>/dev/null || true)
        if printf '%s' "$events" | grep -Fq 'STPServiceScrapeUnavailable' && \
            printf '%s' "$events" | grep -Fq '"alert_jobs":["worker"]' && \
            printf '%s' "$events" | grep -Fq '"status":"firing"'; then
            printf '%s' "$events" >"$evidence_dir/events.json"
            return 0
        fi
        sleep 1
    done
    echo "Alertmanager did not deliver the worker scrape alert to the receiver" >&2
    printf 'last receiver event snapshot: %s\n' "$events" >&2
    return 1
}

docker compose -p "$project_name" --profile observability up --build --detach \
    alert-receiver alertmanager prometheus
wait_for_url "http://127.0.0.1:${PROMETHEUS_PORT}/-/ready" "Prometheus"
wait_for_url "http://127.0.0.1:${ALERTMANAGER_PORT}/-/ready" "Alertmanager"

docker compose -p "$project_name" stop worker >/dev/null
wait_for_receiver_event

if ! curl --fail --silent "http://127.0.0.1:${PROMETHEUS_PORT}/api/v1/alerts" | \
    grep -q 'STPServiceScrapeUnavailable'; then
    echo "Prometheus did not evaluate the isolated worker outage alert" >&2
    exit 1
fi

if ! curl --fail --silent "http://127.0.0.1:${ALERTMANAGER_PORT}/api/v2/alerts" | \
    grep -q 'STPServiceScrapeUnavailable'; then
    echo "Alertmanager did not retain the evaluated scrape outage alert" >&2
    exit 1
fi

printf '{"alertname":"STPServiceScrapeUnavailable","source":"isolated_worker_stop","prometheus":"firing","alertmanager":"routed","receiver":"delivered","authority":"none"}\n' \
    >"$evidence_dir/result.json"
echo "alert delivery drill passed: Prometheus rule -> Alertmanager -> alert receiver"
