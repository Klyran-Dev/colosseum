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

# The token declarations are the only place scheme-specific color values may
# exist. Scan every component rule as well; otherwise a legacy selector can
# silently reintroduce a palette that the final precedence layer happens to
# mask today.
token_boundary=$(grep -n '^\* {' "$styles" | head -n 1 | cut -d: -f1)
if [ -z "$token_boundary" ]; then
  echo "missing stylesheet token boundary" >&2
  exit 1
fi
component_styles=$(tail -n +"$token_boundary" "$styles")
if printf '%s\n' "$component_styles" | grep -Eq '#[0-9a-fA-F]{3,8}|rgba?\('; then
  echo "raw color literal found outside the Material token declaration blocks" >&2
  exit 1
fi

# Material 3 Expressive uses tonal surfaces, semantic roles, shape, and
# purposeful motion. Decorative gradients in an operational stateful UI make
# elevation and status harder to scan and create a second, unaudited palette.
# Keep all component surfaces flat and role-based; expressive contrast belongs
# in the token scheme and bounded shape hierarchy.
if printf '%s\n' "$component_styles" | grep -Eiq 'gradient\('; then
  echo "decorative gradient found outside the Material token declaration blocks" >&2
  exit 1
fi

for role in --app-primary --app-on-surface --app-on-variant --app-surface-container --app-outline-variant --app-error --app-warning; do
  if ! printf '%s\n' "$application_layer" | grep -Fq "var($role)"; then
    echo "Material 3 final precedence layer does not consume required semantic role: $role" >&2
    exit 1
  fi
done

# The product shell consolidates deployment, network, service, and session
# diagnostics into native disclosures. Keep the retired persistent-badge
# selectors out of the stylesheet so the old noisy chrome cannot return through
# an accidental class reuse.
for retired_selector in '.environment-badge' '.network-badge' '.context-unconfigured' '.context-warning' '.shell-service' '.service-status'; do
  if grep -Fq "$retired_selector" "$styles"; then
    echo "retired persistent shell-status selector remains: $retired_selector" >&2
    exit 1
  fi
done

echo "web Material 3 color contract passed"
