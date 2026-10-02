# ADR-0241: Executable Linux conformance workflow contract

## Status

Accepted for implementation. The repository contract will fail closed when the
authoritative CI or release-candidate workflow drifts away from the Linux/amd64
validator evidence boundary.

## Context

ADR-0199 makes a successful Linux/amd64 Agave run a release requirement. The
runtime verifier proves the evidence bundle after the command runs, but a
workflow edit could still remove the Linux assertions, invoke a different
command, omit postcondition verification, or upload an incomplete bundle.
Those failures would make the release process depend on reviewer memory rather
than an executable invariant.

## Decision

Add `scripts/verify_solana_conformance_workflow_contract.sh` and run it from
the normal repository check. The contract inspects both authoritative
workflows and requires each job to retain:

- the pinned `ubuntu-24.04` runner;
- Linux Docker and amd64 assertions before validator execution;
- `SOLANA_USE_DOCKER_BUILDER: true` and `make solana-conformance`;
- the postcondition verifier after the command with `if: always()`; and
- bounded artifact retention of conformance evidence with
  `if-no-files-found: error`.

This is a static workflow contract. It does not replace the runner,
postcondition verifier, or a successful Linux execution, and it never treats a
local `io_uring` failure as evidence.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Reviewer checklist only | No code | Workflow drift can silently remove a release invariant | Rejected |
| Verify only the runtime evidence bundle | Proves the artifact after execution | Cannot detect a workflow that never asserts the required runner or retention policy | Rejected |
| Duplicate a second workflow for conformance | Independent execution path | More drift, cost, and ambiguous authority | Rejected |
| **Static contract over the two authoritative workflows** | Small, deterministic, credential-free, and runs in every normal check | Must be updated when the workflow contract intentionally changes | **Selected** |

## Invariants and failure policy

1. Both authoritative workflows must contain every required boundary.
2. Missing, malformed, or moved required text fails the contract; the check
   must not infer a weaker equivalent.
3. The contract has no authority to approve a release or alter workflow state.
4. A successful contract check proves only workflow shape. Linux validator,
   transaction, provider, identity, and operational evidence remain separate
   release gates.

## Verification

- `sh -n scripts/verify_solana_conformance_workflow_contract.sh`
- `make solana-conformance-workflow-contract`
- `make check`
- CI and the release-candidate job continue to run
  `verify_solana_conformance_evidence.sh` after the real validator command.

## Consequences

The repository catches accidental conformance workflow weakening during normal
development and before release review. Intentional changes require updating
the ADR, contract, and both workflow/evidence tests, adding a small maintenance
cost while preserving one explicit authority boundary.
