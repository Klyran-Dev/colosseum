# ADR-0328: Durable route-verification run evidence

- Status: Accepted
- Date: 2026-10-04
- Deciders: Principal Low-Latency Blockchain Architecture, Settlement Domain, SRE
- Scope: settlement route verifier and PostgreSQL operational evidence

## Context

ADR-0325 suspends active routes that do not have a complete typed
`AssetConfig` projection and requires explicit finalized re-verification.
The verifier already processes bounded pending/suspended batches, but its
examined, activated, already-active, and failed counts currently exist only in
logs and non-authoritative metrics. A crash or operator review therefore
cannot prove which backfill work completed or which routes still need action.

The evidence must be durable, bounded, append-only after completion, and
strictly operational. It must not become route or Solana authority and must
not add work to quote, compile, wallet-signing, or submit paths.

## Decision

Add `settlement_route_verification_runs`, one row per non-empty settlement
route verification pass. Each row contains a UUIDv7 run identity, trigger
kind, bounded batch size, lifecycle status, exact counters, start/completion
timestamps, and a bounded failure code. A pass is `running` until its summary
is committed; a clean pass is `completed`; a pass with per-route failures is
`completed_with_failures`; a storage or process-level failure is `failed`.

The verifier creates a run only after it has read a non-empty candidate batch,
so an idle polling loop does not create unbounded empty evidence. Counts are
updated once at pass completion. A process crash leaves a bounded `running`
row that is evidence of interruption, not a false success. The table is
append-only after terminal state, is subject to a documented bounded retention
policy, and stores no RPC credentials, raw account data, or PII.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Logs and metrics only | No schema change | Not durable, difficult to reconcile after restart, mutable aggregation | Rejected |
| One mutable singleton status row | Simple latest view | Loses historical backfill evidence and hides concurrent/replayed passes | Rejected |
| Per-route audit row for every attempt | Maximum detail | Duplicates immutable chain evidence and adds high write amplification | Rejected |
| **Bounded append-only run ledger plus existing route evidence** | Durable counts, restart visibility, low write cost, no hot-path dependency | Requires retention and operator read tooling | **Selected** |

## Invariants

- Every persisted run ID is UUIDv7 and every count is non-negative and bounded.
- A terminal run has `completed_at`; a running run does not.
- Terminal run counters are immutable.
- `completed_with_failures` is used when per-route failures are non-zero.
- A run ledger never activates a route and never authorizes token movement.
- A route verification pass remains bounded by the existing batch, provider,
  deadline, and database timeout limits.
- A crash cannot be represented as a successful completed run.

## Verification

- Add migration and direct-SQL negative tests for lifecycle, bounds, UUIDv7,
  terminal immutability, and failure classification.
- Add storage methods that reject invalid counters and stale lifecycle updates.
- Add route-verifier tests proving empty batches do not create runs and that
  completed summaries preserve exact counters.
- Retain Linux/amd64 and representative production snapshot evidence before
  enabling paid-state movement.

## References

- [ADR-0325: Typed asset-admission projection](0325-typed-asset-admission-projection.md)
- [ADR-0329: Approval-bound route-verification backfill mode](0329-approved-route-verification-backfill-mode.md)
- [ADR-0321: Finalized asset-configuration evidence](0321-finalized-asset-config-route-evidence.md)
- [Route verification runbook](../runbooks/settlement-route-verification.md)
