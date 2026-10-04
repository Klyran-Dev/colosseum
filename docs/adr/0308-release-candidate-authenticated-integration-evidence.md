# ADR-0308: Exact-commit authenticated integration evidence in the release candidate

## Status

Accepted — workflow, bundle provenance, and integrity contract implemented;
retained Linux release evidence remains an external gate.

## Context

The repository has a Docker integration gate that exercises the authenticated
API against PostgreSQL, NATS, the settlement RPC fixture, a worker, an edge
proxy, and a second API replica. It proves tenant-derived authorization,
duplicate settlement fencing, ambiguous RPC reconciliation, process-crash
recovery, rule rollback, finalized adjustment proof, and bounded operational
evidence.

The release-candidate workflow previously depended on the ordinary CI
integration job. That job is useful pull-request evidence, but it does not
prove that the exact tagged source revision packaged into the release passed
the authenticated integration topology.

## Decision

The release-candidate workflow runs `make integration-check` after the normal
repository checks and before later release evidence and bundle creation. The
step uses `pipefail`, captures the bounded integration stream in
`release-bundle/integration/evidence.txt`, and writes
`release-bundle/integration/source-revision.txt` with the full checked-out Git
revision. Failed runs upload the integration directory as failure evidence;
missing evidence is an upload error.

The release-bundle creator requires both files, rejects a source-revision
mismatch, copies the evidence into the checksummed artifact tree, and records
the path in the machine-readable release manifest. The integration stream is
synthetic and public-safe: it proves application and persistence semantics in
the declared Docker topology, not live wallet behavior, mainnet finality,
provider availability, legal compliance, or regional production SLOs.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Rely only on the separate CI integration job | No additional release time | Evidence may belong to another revision or dependency state | Rejected |
| Repeat only selected integration drills | Lower runtime | Omits the authenticated cross-service contract | Rejected |
| Run integration but retain no output | Simple workflow | Reviewers cannot verify the exact release evidence | Rejected |
| **Run, bind, checksum, and retain the complete authenticated integration stream** | Exact provenance, reviewability, and failure visibility | Adds bounded Docker runtime and artifact storage | **Selected** |

## Invariants and failure policy

- The integration command executes on the checked-out release revision.
- `source-revision.txt` contains one full hexadecimal Git revision equal to the
  revision packaged by `create_release_bundle.sh`.
- `evidence.txt` is required, copied into the artifact checksum set, and
  listed in the release manifest.
- The workflow uses `set -o pipefail`; a failed integration command cannot be
  hidden by a successful `tee`.
- Missing or mismatched evidence fails bundle creation before completion.
- Failure evidence is retained separately from a successful release artifact.
- Synthetic integration evidence must not be described as mainnet, legal,
  live-provider, live-wallet, or production regional performance evidence.

## Verification

- The workflow contract fails when the exact integration step, capture,
  revision binding, failure upload, or ordering is removed.
- The release-bundle integrity contract proves successful packaging and rejects
  mismatched integration evidence.
- `make integration-check` remains the executable local reproduction command.
- The retained Linux release-candidate artifact and independent review remain
  required before production enablement.
