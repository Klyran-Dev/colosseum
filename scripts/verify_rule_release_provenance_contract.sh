#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

failures=0

require_text() {
  file=$1
  text=$2
  if ! grep -Fq "$text" "$file"; then
    echo "rule release provenance contract: missing '$text' in $file" >&2
    failures=$((failures + 1))
  fi
}

require_text docs/adr/0316-versioned-rule-release-provenance.md 'tax-rule-provenance/v1'
require_text crates/storage/src/lib.rs 'InvalidRuleReleaseProvenance'
require_text crates/storage/src/lib.rs 'if command.to_status == "published"'
require_text crates/storage/src/lib.rs 'serde(deny_unknown_fields)'
require_text crates/storage/src/lib.rs 'rule_release_provenance_requires_a_bounded_versioned_envelope'
require_text crates/api/src/lib.rs 'INVALID_RULE_RELEASE_PROVENANCE'
require_text db/migrations/0065_rule_release_provenance_readiness.sql 'published_rule_provenance_readiness'
require_text db/migrations/0065_rule_release_provenance_readiness.sql 'jurisdiction_scope'
require_text scripts/verify_local_stack.sh 'database trigger unexpectedly allowed a published rule without release provenance'
require_text scripts/verify_local_stack.sh 'expected incomplete rule release provenance to return HTTP 422'
require_text apps/web/src/GovernancePanel.tsx 'Source SHA-256 digest'
require_text apps/web/src/GovernancePanel.tsx 'Legal/tax owner'
require_text apps/web/src/GovernancePanel.tsx 'tax-rule-provenance/v1'

if [ "$failures" -ne 0 ]; then
  echo "rule release provenance contract failed with $failures issue(s)" >&2
  exit 1
fi

echo "rule release provenance contract passed"
