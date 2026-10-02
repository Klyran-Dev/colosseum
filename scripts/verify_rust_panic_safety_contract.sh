#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

for crate_root in crates/*/src/lib.rs crates/*/src/main.rs; do
    [ -f "$crate_root" ] || continue
    grep -Fq '#![cfg_attr(' "$crate_root" || {
        echo "Rust panic-safety cfg_attr missing from $crate_root" >&2
        exit 1
    }
    grep -Fq 'not(test)' "$crate_root" || {
        echo "Rust panic-safety test boundary missing from $crate_root" >&2
        exit 1
    }
    grep -Fq 'deny(' "$crate_root" || {
        echo "Rust panic-safety deny attribute missing from $crate_root" >&2
        exit 1
    }
    grep -Fq 'clippy::expect_used' "$crate_root" || {
        echo "Rust panic-safety contract missing from $crate_root" >&2
        exit 1
    }
    grep -Fq 'clippy::panic' "$crate_root" || {
        echo "Rust panic-safety contract missing from $crate_root" >&2
        exit 1
    }
    grep -Fq 'clippy::unwrap_used' "$crate_root" || {
        echo "Rust panic-safety contract missing from $crate_root" >&2
        exit 1
    }
done

echo "Rust panic-safety crate-root contract passed"
