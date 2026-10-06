#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
styles="$repo_root/apps/web/src/styles.css"
marker='@media (forced-colors: active)'

if ! grep -Fq "$marker" "$styles"; then
  echo "missing forced-colors accessibility layer" >&2
  exit 1
fi

marker_line=$(grep -nF "$marker" "$styles" | tail -n 1 | cut -d: -f1)
forced_colors_layer=$(sed -n "${marker_line},\$p" "$styles")

for system_color in Canvas CanvasText ButtonFace ButtonText Highlight HighlightText GrayText Mark; do
  if ! printf '%s\n' "$forced_colors_layer" | grep -Fq "$system_color"; then
    echo "forced-colors layer is missing system color: $system_color" >&2
    exit 1
  fi
done

if ! printf '%s\n' "$forced_colors_layer" | grep -Fq 'forced-color-adjust: auto'; then
  echo "forced-colors layer must preserve user-agent system color mapping" >&2
  exit 1
fi

if printf '%s\n' "$forced_colors_layer" | grep -Eq '#[0-9a-fA-F]{3,8}|rgba?\('; then
  echo "forced-colors layer must not introduce palette literals" >&2
  exit 1
fi

echo "forced-colors accessibility contract passed"
