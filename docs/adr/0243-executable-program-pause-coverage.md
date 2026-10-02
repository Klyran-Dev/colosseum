# ADR-0243: Executable program pause-coverage contract

## Status

Accepted for implementation. The repository will fail closed when an
authority-bearing token-movement instruction lacks the protocol pause guard or
its required pause account boundary.

## Context

ADR-0052 and ADR-0242 require the protocol pause to stop every Solana token
movement path. Unit tests and validator conformance prove the current
instructions, but a future instruction can accidentally introduce a new CPI
without updating those tests. This is a high-impact emergency-control
regression that should be detected before deployment review.

## Decision

Add `scripts/verify_program_pause_coverage.sh` with an explicit, reviewable
instruction allowlist for every current token-moving entry point:

- `settle`;
- `settle_document`;
- `settle_split`;
- `execute_adjustment_refund`; and
- `execute_vault_release`.

The contract extracts each function body and requires the ordinary movement
instructions to call `validate_protocol_not_paused`. The vault-release path
must carry `protocol_config` through `ExecuteVaultRelease` and bind its pause
state into release validation. The contract also requires the pause helper and
the named entry points to exist, and runs from `make check` and the program CI
job.

The allowlist is intentionally explicit: adding a new token-moving instruction
requires an ADR/contract update and a test before it can pass CI. Static
coverage complements, but does not replace, Rust tests or Linux validator
execution.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Reviewer grep/checklist | No tooling | Future movement paths can bypass emergency pause unnoticed | Rejected |
| Unit tests only | Precise behavior tests | Does not ensure every entry point is covered after code growth | Rejected |
| Runtime reflection or macro-generated registry | Centralized metadata | Adds complexity and can alter the on-chain ABI/build path | Rejected |
| **Explicit static entry-point coverage contract** | Deterministic, credential-free, low maintenance, and fails before deployment | Requires an intentional allowlist update for new movement paths | **Selected** |

## Invariants and failure policy

1. Every allowlisted token-moving entry point has a mandatory pause boundary.
2. `execute_vault_release` cannot rely on API or UI pause state; its PDA-bound
   on-chain account is required.
3. Missing, renamed, or structurally weakened coverage fails the contract.
4. Passing the static contract does not claim runtime or validator evidence.

## Verification

- `sh -n scripts/verify_program_pause_coverage.sh`;
- `make program-pause-coverage-contract`;
- `make check`; and
- Rust plus validator-backed conformance tests remain required.

## Consequences

Emergency-stop coverage becomes a continuously checked repository invariant.
The explicit allowlist creates a small review obligation when adding a new
movement instruction, which is desirable because such an instruction changes
the authority and incident-response surface.
