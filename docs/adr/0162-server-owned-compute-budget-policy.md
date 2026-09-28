# ADR-0162: Server-owned compute-budget and priority-fee policy

- Status: Accepted; implementation boundary complete; target-cluster evidence open
- Date: 2026-09-28
- Deciders: Principal Low-Latency Architecture, Settlement, and SRE
- Scope: Solana settlement transaction compilation, wallet signing, and API admission

## Context

The settlement instruction is deterministic, but the current signed message
does not explicitly bind a compute-unit limit or a priority-fee bid. That
leaves behavior to wallet, RPC, and cluster defaults. Defaults vary by
provider and can produce avoidable compute exhaustion, fee surprises, and
non-replayable admission evidence. A client-supplied budget is also unsafe:
the client could change the transaction after compilation without changing the
compiled settlement artifact.

The budget must remain outside the tax engine and must not become an on-chain
tax authority. It is a transaction-execution policy applied to the already
compiled settlement instruction.

## Decision

Introduce a versioned, server-owned `ComputeBudgetPolicy` in the settlement
compile artifact. The policy contains:

- `policy_version`, a positive bounded integer;
- `compute_unit_limit`, a positive bounded `u32`; and
- `micro_lamports_per_compute_unit`, a bounded `u64` priority-fee bid.

For the initial settlement profile, the server emits exactly two instructions
before the settlement instruction, in this order:

1. `ComputeBudget111111111111111111111111111111`,
   `SetComputeUnitLimit(compute_unit_limit)`;
2. the same program, `SetComputeUnitPrice(micro_lamports_per_compute_unit)`;
3. the canonical settlement instruction.

The API compiler selects the policy from an immutable server-side profile
(the current v1 profile is a bounded code-level default; deployment-specific
profiles require a separately reviewed configuration change) and includes it
in the intent and instruction digest. The browser may serialize the exact
returned values but may not choose, omit, reorder, duplicate, or mutate them.
The API verifier checks program IDs, canonical instruction data, order, exact
policy values, settlement instruction, signer, account metas, lookup-table
expansion, and wire budget before submission. Split settlement uses the same
policy contract.

The policy is bounded and fail-closed. The initial numerical bounds and
production defaults are configuration and evidence, not legal or cluster
truth; validator-backed calibration must prove the selected limit, fee ceiling,
and safety margin on every supported target profile before the readiness gate
is closed. A missing, stale, malformed, or unrecognized policy rejects the
transaction.

## Alternatives considered

See the compute-unit and priority-fee section in
`docs/architecture/alternative-analysis.md`.

## Invariants and budgets

- The compile response contains one immutable policy version and both values.
- The signed message contains exactly two compute-budget instructions followed
  by exactly one governed settlement instruction.
- No compute-budget instruction may contain an unknown opcode, duplicate value,
  extra data, or client-selected value.
- The priority fee is bounded by the server policy and is never inferred from
  a client digest or RPC response.
- Policy changes are versioned configuration changes with an ADR or approved
  operational change, new golden vectors, and rollback evidence.
- Compute-unit, fee, packet-size, and latency evidence are measured
  separately; a passing unit test does not close the validator-backed release
  gate.

## Verification and rollout

- Add Rust golden vectors for both compute-budget instruction encodings and
  rejection tests for missing, reordered, duplicated, wrong, and unknown
  instructions.
- Add browser contract tests proving the wallet receives the exact server
  policy and cannot silently replace it.
- Add API integration coverage for legacy and v0+ALT messages with the policy.
- Run Linux validator-backed conformance and representative load tests to
  calibrate p50/p95/p99 compute use, packet bytes, priority-fee acceptance, and
  finalized latency. Until that evidence exists, the production readiness
  checkbox remains open.
