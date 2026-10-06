# ADR-0327: Fail-closed public release claims and authority boundary

- Status: Accepted
- Date: 2026-10-04
- Deciders: Principal Architecture, Security, SRE, and Repository Governance
- Scope: public documentation, release-candidate workflow, and release bundle metadata

## Context

The repository contains substantial executable foundation work, but production
settlement, public licensing, legal approval, independent audit, and retained
Linux validator evidence are separate decisions. A future documentation or
workflow edit must not turn a successful local gate into a production claim or
grant deployment authority to an evidence workflow.

The existing public-repository contract documents this boundary. It needs an
executable guard that runs in the normal local gate and tests the negative
cases, including an accidental `production-ready` README claim and an
unapproved `LICENSE` file.

## Decision

1. Add `verify_release_claim_contract.sh` to the normal `make check` gate.
2. Require the root README, public-release contract, production-readiness plan,
   decision register, release workflow, and release-bundle generator to retain
   explicit status and authority-boundary language.
3. Require the public-source license decision to remain `OPEN` while no
   approved license file exists. If `LICENSE` or `LICENSE.md` appears, the
   decision register must be `FROZEN` or `APPROVED`; the contract does not
   choose a license.
4. Reject positive `production-ready`, `mainnet-ready`, or general-license
   claims in the root README while the foundation status remains open.
5. Reject deployment or image-publication commands in the release-candidate
   workflow. Release bundles may build, hash, and retain evidence, but their
   manifest must continue to declare that artifacts are not deployments.
6. Run mutation tests that prove the contract rejects both a false production
   claim and an unapproved license file.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Rely on reviewer vigilance and prose | No code or maintenance | A small README/workflow edit can create a materially false release claim | Rejected |
| Block all release-candidate evidence | Maximum conservatism | Prevents useful evidence collection and does not distinguish evidence from deployment | Rejected |
| Automatically choose a permissive license | Fast public-source path | Creates an unauthorized legal grant | Rejected |
| **Executable negative contract with mutation tests** | Preserves evidence workflows, catches authority drift, does not make legal decisions | Requires maintaining explicit status markers | **Selected** |

## Invariants

- The contract never claims production readiness, legal approval, audit
  completion, or licensing.
- Local `make check` remains useful while external release gates are open.
- Release-candidate workflows cannot deploy images, programs, or infrastructure.
- Release bundles preserve exact-source and non-deployment metadata.
- A license file cannot silently change repository reuse rights.
- The check does not replace legal, security, validator, HSM/KMS, staging, or
  production evidence.

## Verification

- `make release-claim-contract` passes the baseline and both negative mutation
  cases.
- `make check` includes the contract.
- `make public-docs-check` requires the contract scripts and Makefile target.
- Release-candidate workflow and release-bundle integrity checks remain
  authoritative for exact-source evidence and artifact non-deployment claims.

## References

- [Public repository release contract](../public-repository-release.md)
- [Production-readiness plan](../plans/production-readiness.md)
- [Decision register](../operations/decision-register.md)
- [ADR-0129: Release artifact provenance and dependency automation](0129-release-artifact-provenance-and-dependency-automation.md)
