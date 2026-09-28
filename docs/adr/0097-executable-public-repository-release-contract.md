# ADR-0097: Executable public-repository release contract

- Status: Accepted
- Date: 2026-09-27

## Context

ADR-0019 established public documentation as a product contract, but a manual
review alone can allow required files, links, ADR entries, or status disclaimers
to drift. This repository handles financial and tax-related behavior, so an
incorrect public claim is an operational and governance defect even when the
implementation itself is unchanged.

The repository also has an intentionally unresolved license decision. A check
must not silently treat the absence of a license as permission to reuse the
source, and it must not mark production readiness based on local or simulated
evidence.

## Decision

Add a dependency-free `scripts/verify_public_repository.sh` contract check and
run it in the standard `make check` and CI paths. The check verifies:

1. required public governance and engineering documents exist;
2. the root README links to the principal public documents;
3. every ADR file is discoverable from the ADR index;
4. the README and public-release contract retain explicit foundation/not-ready
   language; and
5. tracked files do not match common secret-bearing filenames.

The check is an integrity gate, not a license grant, legal review, security
audit, or production-readiness approval. License approval remains a human
release gate documented in `docs/public-repository-release.md`; if a license is
later selected, the repository owner must add the approved license file and
update the release evidence in the same change.

## Alternatives considered

| Alternative | Decision |
| --- | --- |
| Continue with manual review only | Rejected: link and file drift is easy to miss and cannot be enforced consistently in CI. |
| Depend on a hosted documentation or link-checking service | Rejected for the baseline: it adds credentials, network dependence, and supply-chain scope to a local integrity check. |
| Require a license file immediately | Rejected: the owner has not selected a license, and inventing one would create a legal claim outside the implementation mandate. |
| Use a full documentation generator | Rejected for this gate: generated sites do not prove repository governance, status truthfulness, or secret filename hygiene. |

## Invariants and evidence

- A clean checkout fails the integrity gate if any required public contract file
  is missing or an indexed ADR is undiscoverable.
- The check never reports a pending license as approved.
- The check never changes source files, downloads dependencies, or contacts an
  external service.
- CI runs the same command developers run locally.
- The production-readiness plan remains the authority for release status and
  must not be changed from unchecked to checked without current evidence.

Evidence: `make public-docs-check`, `.github/workflows/ci.yml`, and the
versioned release contract linked from the root README.

