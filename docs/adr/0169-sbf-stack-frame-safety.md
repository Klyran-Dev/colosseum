# ADR-0169: SBF stack-frame safety boundary

## Status

Accepted.

## Context

The Solana BPF/SBF runtime has a bounded stack frame. The pinned
`cargo-build-sbf` toolchain can emit stack-overflow diagnostics while still
returning a successful process status. Treating that artifact as deployable
would make validator behavior undefined and would violate the fail-closed
settlement mandate.

The adjustment-refund instructions have the largest account-validation
contexts because they bind the registry, adjustment, original receipt, claim,
mint, and three token accounts in one atomic authority boundary.

## Decision

Use Anchor `Box<Account<...>>` and `Box<InterfaceAccount<...>>` for the large
state and token accounts in `CreateAdjustmentRefundClaim` and
`ExecuteAdjustmentRefund`. Boxing moves account deserialization ownership off
the instruction-validation stack without changing account order, PDA seeds,
wire serialization, authority checks, or the atomic transaction boundary.

Wrap both local and Dockerized SBF builds in a safety gate that fails if the
compiler output contains a stack-frame diagnostic, including frame overwrite,
maximum-frame overflow, or maximum-stack-offset messages. A successful build
is not accepted as evidence when those diagnostics are present.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Ignore compiler diagnostics because the process exits zero | No code change | Can ship undefined SBF behavior and hide a deployability defect | Rejected |
| Remove refund functionality from the program | Reduces stack pressure | Sacrifices a required bounded-context capability and hides the architectural issue | Rejected |
| Split authority validation across multiple instructions | Smaller frames | Breaks atomic authorization/evidence binding and creates an unsafe intermediate state | Rejected |
| **Box large Anchor accounts and fail closed on SBF stack diagnostics** | Preserves atomic semantics, keeps account order stable, and makes build safety executable | Adds pointer indirection and a compiler-output contract | **Selected** |

## Invariants and failure policy

- no released SBF artifact may contain a known stack-frame safety diagnostic;
- account order, PDA derivation, signer requirements, token-program checks,
  and refund amount conservation remain unchanged;
- a future account-context expansion that crosses the stack limit fails the
  build before validator startup or release bundling; and
- the guard is diagnostic-only and never downgrades a failed validator or RPC
  conformance run to a pass.

## Verification

- host settlement unit tests continue to cover refund authorization and
  conservation predicates;
- Dockerized SBF compilation succeeds after boxing the two contexts;
- the guarded build output contains no frame-overwrite or maximum-frame
  diagnostics; and
- `make solana-conformance` still fails closed when the validator cannot start
  on the current Docker Desktop `io_uring` environment.

## Consequences

The refund instructions remain one atomic on-chain boundary while gaining an
explicit deployability gate. Compiler output parsing is intentionally narrow
and version-reviewed; any toolchain diagnostic wording change requires a
compatibility review rather than silently weakening the safety gate.
