#!/bin/sh

set -eu

project_name="solana-tax-protocol-otel-trace"
export POSTGRES_PORT="25470"
export API_PORT="28070"
export OTEL_GRPC_PORT="25471"
export OTEL_HTTP_PORT="25472"
export OTEL_ENVIRONMENT="local"
export OTEL_ALLOW_INSECURE_TELEMETRY="true"
export OTEL_EXPORTER_OTLP_TRACES_ENDPOINT="http://otel-collector:4317"
export OTEL_TRACE_SAMPLE_RATIO="1.0"
export RUST_LOG="info"

evidence_dir="${OTEL_TRACE_EVIDENCE_DIR:-test-results/otel-trace}"
mkdir -p "$evidence_dir"

cleanup() {
    exit_status=$?
    if [ "$exit_status" -ne 0 ]; then
        docker compose -p "$project_name" logs --no-color api otel-collector >&2 || true
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

docker compose -p "$project_name" --profile observability up --build --detach \
    otel-collector api
wait_for_url "http://127.0.0.1:${API_PORT}/healthz" "API"

curl --fail --silent --dump-header "$evidence_dir/response-headers.txt" \
    --output /dev/null "http://127.0.0.1:${API_PORT}/healthz"
request_id="$(awk 'BEGIN { IGNORECASE=1 } /^x-request-id:/ { gsub("\r", "", $2); print $2; exit }' "$evidence_dir/response-headers.txt")"
if [ -z "$request_id" ]; then
    echo "API did not return an x-request-id correlation header" >&2
    exit 1
fi

trace_log="$evidence_dir/collector.log"
for _attempt in $(seq 1 45); do
    docker compose -p "$project_name" logs --no-color otel-collector >"$trace_log" 2>&1 || true
    if grep -Fq "$request_id" "$trace_log" && grep -Fq 'http.request' "$trace_log"; then
        break
    fi
    sleep 1
done

if ! grep -Fq "$request_id" "$trace_log" || ! grep -Fq 'http.request' "$trace_log"; then
    echo "collector did not receive the API request span" >&2
    exit 1
fi

if grep -Eiq 'signed_transaction_base64|private[_ -]?key|seed phrase|authorization: bearer|tax facts' "$trace_log"; then
    echo "collector trace evidence contains a prohibited sensitive payload field" >&2
    exit 1
fi

printf '{"request_id":"%s","span":"http.request","collector":"otel-collector","sensitive_payload_fields":false}\n' \
    "$request_id" >"$evidence_dir/result.json"
echo "OTLP trace drill passed: API request correlation reached the collector without sensitive payload fields"
