# ADR-0303: Exact-commit browser evidence in the release candidate

## Status

Accepted — workflow and release-bundle binding implemented; retained Linux
release evidence remains an external gate.

## Context

The pinned Playwright/NGINX browser journey is already a required pull-request
gate, but a release candidate can be built from a later commit than the last
browser run. A production SaaS release must bind its responsive, accessibility,
theme, navigation, and failure-state evidence to the exact source revision that
produced the release artifacts. Otherwise a green browser job is only evidence
about a nearby revision.

The browser suite uses synthetic API fixtures and therefore cannot prove live
wallet behavior, chain finality, legal/tax correctness, or assistive-technology
sign-off. The decision is only about provenance and retention of the automated
UI evidence.

## Decision

The release-candidate workflow runs `make web-browser-check` on the checked-out
release revision. On success it copies the bounded `test-results/web-e2e`
directory into `release-bundle/web-e2e` and writes a single-line
`git_revision=<full commit>` binding. The release-bundle creator requires that
binding to equal the packaged source revision, copies the browser evidence into
the checksummed `artifacts/evidence/web-e2e` area, and records its result path in
the machine-readable manifest.

Browser failure evidence is uploaded separately when the journey fails. A
failed or missing browser result cannot produce a release bundle. The release
workflow keeps the browser test after the normal repository checks and before
release integrity and bundle creation, so later evidence cannot make a failed
UI gate appear successful.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Rely only on the pull-request browser job | Avoids a second browser run | Does not bind evidence to the tagged source revision | Rejected |
| Run browser journeys in the release job but retain no result | Simple workflow | Reviewers cannot inspect the exact run after the job completes | Rejected |
| Retain the raw browser result without revision binding | Inspectable artifacts | Evidence can be copied from a different checkout or run | Rejected |
| **Run, bind, checksum, and retain exact-commit browser evidence** | Provenance, reviewability, failure visibility, deterministic release gate | Adds bounded release-job time and artifact size | **Selected** |

## Invariants and failure policy

- The browser job uses the same pinned Docker/NGINX journey contract as CI.
- The source revision in `web-e2e/source-revision.txt` is a full Git revision
  and equals the revision used by `create_release_bundle.sh`.
- `results.json` and the revision binding are required inputs; missing or
  mismatched evidence fails closed.
- Browser evidence is synthetic UI evidence and must not be described as live
  wallet, finalized-chain, legal, or production accessibility evidence.
- Browser output is copied into the release artifact checksum set before the
  manifest is generated.

## Verification

- The workflow contract fails when the exact-commit browser step, copy, upload,
  or ordering is removed.
- The release-bundle integrity contract proves successful packaging and rejects
  a mismatched browser source revision.
- `make web-browser-check` remains the executable local reproduction command.
- The retained Linux release-candidate artifact and independent review remain
  required before production enablement.
