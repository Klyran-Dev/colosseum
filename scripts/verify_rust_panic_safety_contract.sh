#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

find crates programs -type f \( -path '*/src/lib.rs' -o -path '*/src/main.rs' -o -path '*/src/bin/*.rs' \) -print |
while IFS= read -r crate_root; do
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
