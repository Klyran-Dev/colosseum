#!/bin/sh

set -eu

repo_root=${RELEASE_CLAIM_REPO_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)}

fail() {
  echo "release claim contract failed: $1" >&2
  exit 1
}

require_file() {
  [ -f "$repo_root/$1" ] || fail "missing required file: $1"
}

require_text() {
  file=$1
  text=$2
  grep -Fq "$text" "$repo_root/$file" || fail "missing required text in $file: $text"
}

require_file README.md
require_file docs/public-repository-release.md
require_file docs/plans/production-readiness.md
require_file docs/operations/decision-register.md
require_file .github/workflows/release-candidate.yml
require_file scripts/create_release_bundle.sh

# The public entry point must continue to distinguish an implementation
# foundation from a production/mainnet release. A green local gate is not a
# license, legal approval, validator run, audit, or deployment authorization.
require_text README.md 'It does not claim production deployment'
require_text README.md 'Do not infer production support from architecture documents'
require_text docs/public-repository-release.md 'must not be represented as licensed for general reuse'
require_text docs/public-repository-release.md 'must not be described as production-ready'
require_text docs/plans/production-readiness.md 'License choice is explicit before public source release.'
require_text docs/plans/production-readiness.md 'Program audit complete with no unresolved critical/high findings.'
require_text docs/operations/decision-register.md '| `DEC-0251-10` | Public-source license | `FROZEN` |'

# Reject an accidental positive release claim in the root public entry point.
if grep -Eiq '(^|[^[:alnum:]])production-ready([^[:alnum:]]|$)|(^|[^[:alnum:]])mainnet-ready([^[:alnum:]]|$)|licensed for general reuse' "$repo_root/README.md"; then
  fail 'README contains an unsupported production, mainnet, or license claim'
fi

# A license file is a legal grant. It may appear only after the decision
# register is moved to an approved state and the public release contract is
# updated by the repository owner.
if [ -e "$repo_root/LICENSE" ] || [ -e "$repo_root/LICENSE.md" ]; then
  if ! grep -Eq '\| `DEC-0251-10` \| Public-source license \| `(FROZEN|APPROVED)` \|' "$repo_root/docs/operations/decision-register.md"; then
    fail 'LICENSE exists while the public-source license decision is not approved'
  fi
fi

# Release-candidate evidence may build and retain artifacts, but must not gain
# deployment authority through a casual workflow edit.
if grep -Eiq 'docker[[:space:]]+push|kubectl[[:space:]]+(apply|rollout)|helm[[:space:]]+(install|upgrade)|solana[[:space:]]+program[[:space:]]+deploy' "$repo_root/.github/workflows/release-candidate.yml"; then
  fail 'release-candidate workflow contains deployment or image-publication authority'
fi
require_text scripts/create_release_bundle.sh 'artifacts_are_not_deployments'

echo "release claim contract passed: public status, license boundary, and release authority remain explicit"
