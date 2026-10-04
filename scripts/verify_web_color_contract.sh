#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
styles="$repo_root/apps/web/src/styles.css"
marker='/* Material 3 Expressive final precedence layer.'

if ! grep -Fq "$marker" "$styles"; then
  echo "missing Material 3 Expressive final precedence layer marker" >&2
  exit 1
fi

marker_line=$(grep -nF "$marker" "$styles" | tail -n 1 | cut -d: -f1)
application_layer=$(sed -n "${marker_line},\$p" "$styles")
if printf '%s\n' "$application_layer" | grep -Eq '#[0-9a-fA-F]{3,8}'; then
  echo "raw color literal found in the Material 3 final precedence layer" >&2
  exit 1
fi

for role in --app-primary --app-on-surface --app-on-variant --app-surface-container --app-outline-variant --app-error --app-warning; do
  if ! printf '%s\n' "$application_layer" | grep -Fq "var($role)"; then
    echo "Material 3 final precedence layer does not consume required semantic role: $role" >&2
    exit 1
  fi
done

echo "web Material 3 color contract passed"
