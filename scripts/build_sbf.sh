#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
output_dir="${repo_root}/target/deploy"
mkdir -p "$output_dir"

if command -v cargo-build-sbf >/dev/null 2>&1 \
    && [ "${SOLANA_USE_DOCKER_BUILDER:-false}" != "true" ]; then
    cargo-build-sbf --sbf-out-dir "$output_dir" --manifest-path "$repo_root/programs/settlement/Cargo.toml" -- --locked
    exit 0
fi

command -v docker >/dev/null 2>&1 || {
    echo "cargo-build-sbf or Docker is required to build the pinned SBF artifact" >&2
    exit 1
}

builder_image="solana-tax-protocol/sbf-builder:4.3.0"
docker build --platform linux/amd64 --tag "$builder_image" --file "$repo_root/ops/docker/Dockerfile.sbf-builder" "$repo_root"

platform_tools_version="v1.57"
platform_tools_archive_name="platform-tools-linux-x86_64.tar.bz2"
platform_tools_archive_sha256="b0f7af104adf726fff2a6a09ea2eb2f2d2965c92295f4d7388c08d140e0c2b00"
platform_tools_cache_root="${SOLANA_PLATFORM_TOOLS_CACHE_DIR:-$repo_root/target/solana-platform-tools-cache}"
platform_tools_dir="$platform_tools_cache_root/$platform_tools_version/platform-tools"
platform_tools_archive="$platform_tools_cache_root/$platform_tools_archive_name"

verify_platform_tools_archive() {
    expected_sha256="$1"
    archive_path="$2"
    if command -v sha256sum >/dev/null 2>&1; then
        actual_sha256=$(sha256sum "$archive_path" | awk '{print $1}')
        [ "$actual_sha256" = "$expected_sha256" ]
    elif command -v shasum >/dev/null 2>&1; then
        actual_sha256=$(shasum -a 256 "$archive_path" | awk '{print $1}')
        [ "$actual_sha256" = "$expected_sha256" ]
    else
        echo "sha256sum or shasum is required to verify platform-tools" >&2
        exit 1
    fi
}

mkdir -p "$platform_tools_cache_root"

if [ ! -f "$platform_tools_archive" ]; then
    if [ -n "${SOLANA_PLATFORM_TOOLS_ARCHIVE:-}" ]; then
        cp "$SOLANA_PLATFORM_TOOLS_ARCHIVE" "$platform_tools_archive"
    else
        command -v curl >/dev/null 2>&1 || {
            echo "curl is required to acquire the pinned platform-tools archive" >&2
            exit 1
        }
        curl --fail --location --retry 2 --retry-delay 2 --silent --show-error \
            "https://github.com/anza-xyz/platform-tools/releases/download/$platform_tools_version/$platform_tools_archive_name" \
            --output "$platform_tools_archive"
    fi
fi

verify_platform_tools_archive "$platform_tools_archive_sha256" "$platform_tools_archive" || {
    echo "platform-tools archive SHA-256 verification failed" >&2
    exit 1
}

if [ ! -x "$platform_tools_dir/rust/bin/rustc" ] || [ ! -x "$platform_tools_dir/rust/bin/cargo" ]; then
    rm -rf "$platform_tools_dir"
    mkdir -p "$platform_tools_dir"
    tar -xjf "$platform_tools_archive" -C "$platform_tools_dir"
fi

if [ ! -x "$platform_tools_dir/rust/bin/rustc" ] || [ ! -x "$platform_tools_dir/rust/bin/cargo" ]; then
    echo "verified platform-tools archive did not produce a complete toolchain" >&2
    exit 1
fi

docker run --rm --platform linux/amd64 \
    --env HOME=/home/sbf-builder \
    --env CARGO_HOME=/tmp/cargo-home \
    --volume "$platform_tools_cache_root:/home/sbf-builder/.cache/solana:ro" \
    --volume "$repo_root:/workspace" \
    --workdir /workspace \
    "$builder_image" \
    --skip-tools-install \
    --sbf-out-dir target/deploy \
    --manifest-path programs/settlement/Cargo.toml \
    -- --locked
