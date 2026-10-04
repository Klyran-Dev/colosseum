#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
project_name=${COMPOSE_PROJECT_NAME:-solana-tax-protocol-local}

case "$project_name" in
    ''|*[!A-Za-z0-9_.-]*)
        echo "COMPOSE_PROJECT_NAME contains unsupported characters" >&2
        exit 2
        ;;
esac

echo "WARNING: This removes only synthetic local state for Compose project '$project_name'."
echo "PostgreSQL, NATS, and other named volumes owned by this project will be deleted."
echo "Source files, images, and other Compose projects will not be touched."

if [ "${RESET_LOCAL_CONFIRM:-}" = "YES" ]; then
    answer=YES
elif [ -t 0 ]; then
    printf "Continue? Type YES to continue: "
    read -r answer
else
    echo "Refusing non-interactive reset; set RESET_LOCAL_CONFIRM=YES explicitly." >&2
    exit 2
fi

if [ "$answer" != "YES" ]; then
    echo "Local reset cancelled."
    exit 0
fi

cd "$repo_root"
docker compose -p "$project_name" down --volumes --remove-orphans
echo "Local Compose state reset for project '$project_name'."
