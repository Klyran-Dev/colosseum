# ADR-0299: Bind release evidence to the packaged source revision

## Status

Accepted — implemented in the release-bundle creator and integrity contract.

## Context

The release candidate bundle already verifies the pinned Linux validator
evidence, SBF artifact hash, resolved Compose configuration, SBOM, and security
scan. Structural evidence alone is insufficient if a valid conformance bundle
from a different commit is accidentally combined with the current binaries.
Release reviewers need one unambiguous source revision for the package and its
authority-sensitive validator evidence.

## Decision

`create_release_bundle.sh` must compare the full `git_revision` recorded in
`validator-conformance/metadata.txt` with the exact source revision used to
build the release bundle. A mismatch fails before copying or reporting a
complete bundle. The existing validator evidence verifier remains responsible
for Linux/amd64, pinned Agave, container identity, test-case, and SBF hash
requirements; this ADR adds the cross-artifact revision binding.

The integrity contract includes a negative test that mutates the evidence
revision and proves bundle creation rejects it. The release manifest continues
to identify the packaged source revision and does not authorize deployment.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Trust the evidence directory name or workflow run | Rejected | Names and runs do not cryptographically bind evidence to the packaged source |
| Bind only the SBF hash | Rejected | Equivalent program bytes do not bind API, indexer, UI, migrations, or configuration |
| Require a maintainer to compare revisions manually | Rejected | Human review remains necessary but a deterministic safety check should fail closed first |
| **Compare validator metadata revision with the bundle source revision** | **Selected** | Bounded, deterministic, auditable, and compatible with the existing evidence schema |

## Invariants and failure policy

- The evidence revision is a full lowercase hexadecimal Git revision.
- The evidence revision must equal the source revision captured in the bundle.
- A mismatch fails before release artifacts are copied or reported complete.
- A successful local integrity contract is synthetic evidence and never closes
  the Linux, staging, production, legal, custody, or audit gates.

## Verification

- `scripts/release_bundle_integrity_contract.sh` proves both matching and
  mismatched-revision behavior.
- The release-candidate workflow runs the contract before creating the final
  bundle, and the static workflow contract protects that ordering.
- `make public-docs-check` requires this ADR and the release contract.
