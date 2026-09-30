#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
compose_project="${COMPOSE_PROJECT_NAME:-solana-tax-protocol-conformance}"
export COMPOSE_PROJECT_NAME="$compose_project"
cd "$repo_root"

evidence_dir="${SOLANA_CONFORMANCE_EVIDENCE_DIR:-$repo_root/test-results/solana-conformance}"
mkdir -p "$evidence_dir"
phase="initialization"
run_status=1
failure_reason="not_completed"

{
    printf 'schema_version=solana-tax-protocol/conformance-metadata/v1\n'
    printf 'started_at_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'git_revision='
    git rev-parse HEAD 2>/dev/null || true
    printf 'platform='
    uname -a 2>/dev/null || true
    printf 'docker_server_architecture='
    docker info --format '{{.Architecture}}' 2>/dev/null || true
    printf 'docker_version='
    docker version --format '{{.Server.Version}}' 2>/dev/null || true
    printf 'compose_version='
    docker compose version 2>/dev/null || true
    printf '\nconfigured_images:\n'
    docker compose --profile solana config --images 2>/dev/null || true
} > "$evidence_dir/metadata.txt"

cleanup() {
    cleanup_status="$run_status"
    printf 'schema_version=solana-tax-protocol/conformance-result/v1\nphase=%s\nstatus=%s\nfailure_reason=%s\n' "$phase" "$cleanup_status" "$failure_reason" > "$evidence_dir/result.env"
    if [ ! -s "$evidence_dir/localnet-tests.log" ]; then
        printf 'localnet tests were not started; phase=%s; failure_reason=%s\n' "$phase" "$failure_reason" > "$evidence_dir/localnet-tests.log"
    fi
    if [ -f "$repo_root/target/deploy/settlement.so" ]; then
        if command -v sha256sum >/dev/null 2>&1; then
            sha256sum "$repo_root/target/deploy/settlement.so" > "$evidence_dir/settlement.so.sha256"
        elif command -v shasum >/dev/null 2>&1; then
            shasum -a 256 "$repo_root/target/deploy/settlement.so" > "$evidence_dir/settlement.so.sha256"
        fi
    fi
    docker compose --profile solana config > "$evidence_dir/compose-config.yml" 2>&1 || true
    {
        printf 'container_id='
        validator_container_id="$(docker compose --profile solana ps -aq validator 2>/dev/null || true)"
        printf '%s\n' "$validator_container_id"
        if [ -n "$validator_container_id" ]; then
            printf 'image_id='
            validator_image_id="$(docker inspect --format '{{.Image}}' "$validator_container_id" 2>/dev/null || true)"
            printf '%s\n' "$validator_image_id"
            if [ -n "$validator_image_id" ]; then
                printf 'repo_digests='
                docker image inspect --format '{{json .RepoDigests}}' "$validator_image_id" 2>/dev/null || true
            fi
        fi
    } > "$evidence_dir/container-digests.txt"
    docker compose --profile solana logs --no-color validator > "$evidence_dir/validator.log" 2>&1 || true
    docker compose --profile solana down --volumes --remove-orphans >/dev/null 2>&1 || true
    exit "$cleanup_status"
}
trap cleanup EXIT INT TERM

phase="sbf_build"
if ! "$repo_root/scripts/build_sbf.sh" > "$evidence_dir/sbf-build.log" 2>&1; then
    failure_reason="sbf_build_failed"
    cat "$evidence_dir/sbf-build.log"
    exit 1
fi
cat "$evidence_dir/sbf-build.log"

if command -v sha256sum >/dev/null 2>&1; then
    sbf_artifact_sha256="$(sha256sum "$repo_root/target/deploy/settlement.so" | awk '{print $1}')"
else
    sbf_artifact_sha256="$(shasum -a 256 "$repo_root/target/deploy/settlement.so" | awk '{print $1}')"
fi
printf 'sbf_artifact=target/deploy/settlement.so\nsbf_artifact_sha256=%s\n' "$sbf_artifact_sha256" | tee "$evidence_dir/artifact.txt"

phase="validator_startup"
docker compose --profile solana up --build --detach validator

phase="validator_health"
ready=false
for _attempt in $(seq 1 60); do
    if curl --fail --silent --connect-timeout 1 --max-time 2 \
        http://127.0.0.1:8899/health >/dev/null 2>&1; then
        ready=true
        break
    fi
    if [ -n "$(docker compose --profile solana ps -q --status exited validator 2>/dev/null || true)" ]; then
        validator_startup_log="$(docker compose --profile solana logs --no-color --tail=160 validator 2>/dev/null || true)"
        if printf '%s\n' "$validator_startup_log" | grep -Fq 'io_uring NOT supported'; then
            failure_reason="validator_io_uring_unsupported"
            echo "Solana validator requires a Linux kernel with io_uring support" >&2
        else
            failure_reason="validator_exited_before_health"
        fi
        echo "Solana validator exited before becoming healthy" >&2
        printf '%s\n' "$validator_startup_log" >&2
        exit 1
    fi
    sleep 1
done

if [ "$ready" != "true" ]; then
    failure_reason="validator_health_timeout"
    echo "Solana validator did not become healthy" >&2
    exit 1
fi

phase="localnet_tests"
if docker compose --profile solana run --build --rm --no-deps solana-conformance \
    > "$evidence_dir/localnet-tests.log" 2>&1; then
    run_status=0
    failure_reason="none"
else
    run_status=$?
    failure_reason="conformance_tests_failed"
fi
cat "$evidence_dir/localnet-tests.log"
phase="completed"
exit "$run_status"
