# ADR-0217: Retained Docker integration evidence

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Platform Engineering, SRE
- Bounded contexts: Delivery Evidence, Settlement Integration, Public Repository Operations

## Context

The repository's authenticated Docker integration gate exercises multiple
independently important boundaries: PostgreSQL 18 migrations, tenant-scoped
authentication, two API replicas, route verification fixtures, outbox
publication, settlement submission fencing, crash recovery, rule rollback,
and wallet-bound adjustment finality. The command already fails closed, but
continuous integration previously retained only the pass/fail conclusion.

Without the output artifact, reviewers cannot inspect the exact workload
results, response cardinality, provider-call count, or recovery evidence for a
specific commit. Re-running the whole stack is also a poor incident-debugging
interface and can produce a different UUID/signature fixture result.

The retained record must remain public-safe: integration identities, keys,
facts, and provider state are synthetic and the artifact must not become a
settlement authority, credential store, or production performance claim.

## Decision

The CI `integration` job runs `scripts/verify_local_stack.sh` with
`set -o pipefail`, captures stdout and stderr into
`test-results/local-stack-integration/evidence.txt`, and uploads that directory
with a bounded 14-day retention period even when the job fails. The script
continues to own lifecycle cleanup and failure semantics; CI capture is an
observer and cannot turn a failed command green.

The evidence is plain text containing the existing structured scenario lines
and bounded diagnostics. It is intentionally not a second JSON protocol and
does not persist Docker volumes, credentials, private tax evidence, or raw
service logs by default. Failure diagnostics remain available in the captured
stream, while the local `VERIFY_LOCAL_STACK_KEEP_ON_FAILURE=true` workflow
continues to support interactive debugging.

The public-repository contract requires the ADR and CI wiring. Release claims
must cite the retained artifact for the exact commit and must still distinguish
synthetic Docker evidence from Linux validator, live-provider, production SLO,
identity-provider, and legal/tax evidence.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Retain only the CI exit status | Rejected | Proves neither scenario cardinality nor recovery behavior and is not useful for incident review. |
| Upload complete Docker logs and volumes | Rejected | Excessive noise, possible sensitive-data leakage, and poor public-safety boundary. |
| Add a second bespoke JSON reporter | Rejected | Duplicates the authoritative shell gate and creates schema drift between pass/fail and evidence. |
| Keep artifacts only on failure | Rejected | Successful evidence is needed to establish the behavior of a reviewed commit and detect drift. |
| **Pipe the authoritative bounded output to a retained CI artifact** | **Selected** | Minimal change, preserves exact command semantics, supports success and failure review, and keeps evidence public-safe. |

## Invariants and failure policy

- `pipefail` makes a failing integration command fail the CI step even when
  artifact capture succeeds.
- Artifact retention is bounded to 14 days and scoped to the integration job.
- The artifact contains no credentials, private keys, Docker volumes, or
  production tax evidence.
- Synthetic signatures, UUIDs, and provider responses are test evidence only.
- A retained artifact cannot authorize settlement, route activation, or a
  release by itself.
- Missing or malformed evidence is a delivery/observability failure, not a
  successful integration result.

## Verification and release impact

- The CI workflow contains the capture and `if: always()` upload steps.
- `make public-docs-check` requires this ADR and verifies the public repository
  contract remains complete.
- `make integration-check` remains the executable local gate; its exit status
  and output are unchanged.
- A Linux CI run must retain the artifact for the exact commit before the
  integration evidence is used in a release review.
- This ADR does not close validator-backed Solana conformance, live-wallet or
  live-RPC evidence, production SLO acceptance, or legal/tax approval.
