#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
makefile="$repo_root/Makefile"
reset_script="$repo_root/scripts/reset_local.sh"

require_text() {
    file=$1
    text=$2
    grep -Fq -- "$text" "$file" || {
        echo "reset-local contract missing '$text' in $file" >&2
        exit 1
    }
}

require_text "$makefile" 'reset-local:'
require_text "$makefile" 'reset-local-contract:'
require_text "$makefile" './scripts/reset_local.sh'
test -x "$reset_script" || {
    echo "reset-local script must be executable" >&2
    exit 1
}
require_text "$reset_script" 'docker compose -p "$project_name" down --volumes --remove-orphans'
require_text "$reset_script" 'COMPOSE_PROJECT_NAME'
require_text "$reset_script" 'This removes only synthetic local state'

if grep -Eq 'rm[[:space:]]+-rf|docker[[:space:]]+(volume|system)[[:space:]]+rm|docker[[:space:]]+system[[:space:]]+prune' "$reset_script"; then
    echo "reset-local contract forbids broad deletion commands" >&2
    exit 1
fi

echo "project-scoped local reset contract passed"
