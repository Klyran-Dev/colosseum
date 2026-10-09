#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
evidence_dir=${SOLANA_CONFORMANCE_EVIDENCE_DIR:-$repo_root/test-results/solana-conformance}
mkdir -p "$evidence_dir"

host_os=$(uname -s 2>/dev/null || true)
host_architecture=$(uname -m 2>/dev/null || true)
docker_ostype=$(docker info --format '{{.OSType}}' 2>/dev/null || true)
docker_server_architecture=$(docker info --format '{{.Architecture}}' 2>/dev/null || true)
docker_server_version=$(docker version --format '{{.Server.Version}}' 2>/dev/null || true)

status=ready
failure_reason=none

if [ "$host_os" != Linux ]; then
    status=failed
    failure_reason=host_os_must_be_linux
elif [ "$host_architecture" != x86_64 ] && [ "$host_architecture" != amd64 ]; then
    status=failed
    failure_reason=host_architecture_must_be_amd64
elif [ "$docker_ostype" != linux ]; then
    status=failed
    failure_reason=docker_engine_must_be_linux
elif [ "$docker_server_architecture" != amd64 ] && [ "$docker_server_architecture" != x86_64 ]; then
    status=failed
    failure_reason=docker_engine_architecture_must_be_amd64
fi

{
    printf 'schema_version=solana-tax-protocol/conformance-host/v1\n'
    printf 'status=%s\n' "$status"
    printf 'failure_reason=%s\n' "$failure_reason"
    printf 'host_os=%s\n' "$host_os"
    printf 'host_architecture=%s\n' "$host_architecture"
    printf 'docker_ostype=%s\n' "$docker_ostype"
    printf 'docker_server_architecture=%s\n' "$docker_server_architecture"
    printf 'docker_server_version=%s\n' "$docker_server_version"
    printf 'checked_at_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
} > "$evidence_dir/preflight.txt"

if [ "$status" != ready ]; then
    echo "Solana conformance host preflight failed: $failure_reason" >&2
    echo "Authoritative evidence requires a native Linux/amd64 host and Linux/amd64 Docker engine; ARM Docker emulation is diagnostic only." >&2
    exit 1
fi

echo "Solana conformance host preflight passed: Linux/amd64 Docker engine"
