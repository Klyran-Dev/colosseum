# ADR-0086: Authenticated, append-only replay-gap operations

- Status: Accepted
- Date: 2026-09-26
- Owners: Chain Data / SRE
- Scope: Indexer replay-gap inspection, dead-letter acknowledgement, and requeue

## Context

The indexer already persists bounded replay work with lease fencing and moves
retry-exhausted rows to `dead_lettered`. The storage adapter also had an
internal lease acknowledgement helper, but operators had no authenticated,
auditable control-plane boundary. Manual SQL edits would bypass authorization,
incident references, and immutable action evidence.

Replay gaps are system-level recovery work, not merchant-owned tax policy. A
dedicated operator role is therefore required; a merchant identifier in an
operator JWT is not used as an authority boundary for these routes.

## Decision

Add the Indexer Operations bounded context at the API/storage boundary:

- `GET /v1/indexer/replay-gaps` requires `tax:indexer:read` and the exact
  `indexer_operator` role. It accepts a validated stream key, bounded status
  filter, and limit of at most 100 rows.
- `POST /v1/indexer/replay-gaps/{gap_id}/acknowledge` requires
  `tax:indexer:operate` and `indexer_operator`. It records a mandatory,
  bounded operator subject and note on a dead-lettered gap without making the
  gap look repaired.
- `POST /v1/indexer/replay-gaps/{gap_id}/requeue` requires the same scope and
  role. It succeeds only after acknowledgement, resets the bounded retry
  state, and clears the current acknowledgement so the next dead letter needs
  fresh review.

The database adds bounded acknowledgement fields to the gap row and an
append-only `replay_gap_operator_actions` ledger with UUIDv7 IDs. A trigger
rejects updates/deletes to action evidence. Requeue and its action evidence
commit in one transaction. The chain remains authoritative; these operations
only control projection repair work.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Manual SQL updates | Rejected | Bypasses identity, bounded notes, audit evidence, and review policy. |
| Merchant-scoped routes | Rejected | Replay stream ownership is operational/system-level and not safely inferred from merchant JWT claims. |
| Mark dead-lettered work completed on acknowledgement | Rejected | Confuses operator review with finalized chain repair. |
| **Role/scope-protected API plus append-only PostgreSQL action ledger** | **Selected** | Explicit authority, replayable evidence, bounded input, and atomic requeue semantics. |

## Invariants and failure behavior

1. Unauthenticated, wrong-scope, and wrong-role callers cannot inspect or
   mutate replay gaps.
2. Acknowledgement never advances a checkpoint or changes a dead-lettered row
   to completed.
3. Requeue requires an existing acknowledgement and is fenced by the row's
   current dead-lettered state.
4. Requeue clears the current acknowledgement, forcing a new review after a
   later retry exhaustion.
5. Operator notes and subjects are bounded, redacted from metrics labels, and
   preserved in the append-only action ledger.
6. Duplicate requests are safe: an identical acknowledgement is idempotent;
   conflicting acknowledgement or requeue state returns a conflict.
7. No route accepts a caller-supplied merchant identifier as an authority
   boundary.

## Evidence

- `db/migrations/0017_replay_gap_operator_actions.sql`
- `crates/storage/src/lib.rs`
- `crates/api/src/lib.rs`
- `scripts/verify_local_stack.sh`
