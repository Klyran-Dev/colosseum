#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

rust_install='rustup toolchain install 1.98.1 --profile minimal --component rustfmt --component clippy --no-self-update'

for workflow in .github/workflows/ci.yml .github/workflows/release-candidate.yml; do
    grep -Fq "$rust_install" "$workflow" || {
        echo "pinned Rust installation is missing from $workflow" >&2
        exit 1
    }
    grep -Fq 'echo "RUSTUP_TOOLCHAIN=1.98.1" >> "$GITHUB_ENV"' "$workflow" || {
        echo "RUSTUP_TOOLCHAIN export is missing from $workflow" >&2
        exit 1
    }
done

grep -Fq 'rustc --version' .github/workflows/ci.yml || {
    echo "CI must record the selected rustc version" >&2
    exit 1
}
grep -Fq 'rustc --version' .github/workflows/release-candidate.yml || {
    echo "release workflow must record the selected rustc version" >&2
    exit 1
}

for workflow in .github/workflows/ci.yml .github/workflows/release-candidate.yml; do
    grep -Fq 'node-version: 24.21.0' "$workflow" || {
        echo "pinned Node.js version is missing from $workflow" >&2
        exit 1
    }
    grep -Fq 'corepack prepare pnpm@12.3.4 --activate' "$workflow" || {
        echo "pinned pnpm activation is missing from $workflow" >&2
        exit 1
    }
    grep -Fq 'node --version' "$workflow" || {
        echo "Node.js version evidence is missing from $workflow" >&2
        exit 1
    }
    grep -Fq 'pnpm --version' "$workflow" || {
        echo "pnpm version evidence is missing from $workflow" >&2
        exit 1
    }
done

echo "CI toolchain contract passed"
