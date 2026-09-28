# ADR-0127: Finalized vault-balance reconciliation

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Indexing, Finance/Tax, and SRE
- Scope: scheduled read-only reconciliation of governed tax-vault token accounts

## Context

The Solana token account is authoritative for funds, but production operators
also need to know whether a finalized balance is explainable by finalized
receipt and governed-release evidence. A raw balance scrape cannot distinguish
legitimate tax inflows from an unrecorded transfer. A PostgreSQL projection
cannot become the authority for funds.

The existing `indexed_receipts` projection contains validated tax amounts per
indexer stream. The governed vault-release projection contains immutable claim
lifecycles and finalized execution evidence. The generic reconciliation row is
insufficient for route identity, finalized observation slot, opening-balance
provenance, and operator review state.

## Decision

Add a dedicated, bounded vault-reconciliation context:

1. An immutable reconciliation target binds one active settlement route, one
   indexer stream, one mint, one token program, one tax-vault address, and an
   operator-approved opening balance. A changed chain account requires a new
   target version.
2. The opening balance is never inferred. It is captured from a separately
   verified finalized chain observation and requires an operator reference;
   pre-existing or unexplained funds therefore cannot be silently classified
   as tax revenue.
3. The expected current balance is:

   `opening_balance + sum(finalized indexed receipt tax) - sum(finalized executed release claims)`.

   All terms use checked `u64` atomic arithmetic. Negative or overflowing
   expectations are invalid projection state and fail closed.
4. Each scheduled run reads bounded projection aggregates, fetches the token
   account through a finalized `getMultipleAccounts` RPC request, validates
   owner, mint, token program, legacy token-account layout, and amount offset,
   and records the observation slot and result in an append-only run ledger.
5. Token-2022 targets are an explicit compatibility boundary. Unsupported
   extensions or layouts are rejected and alerted until a separate ADR adds
   extension-specific semantics. The reconciler never guesses through transfer
   fees, hooks, confidential balances, delegates, or withheld amounts.
6. Reconciliation is read-only. It cannot submit, pause, release, mutate a
   route, advance an indexer checkpoint, or mark a claim executed. A mismatch
   raises fixed-cardinality telemetry and a critical alert; operators follow
   the vault-release incident runbook.
7. The loop is bounded: at most 64 targets per cycle, one RPC account read per
   target, provider failover bounded to three providers, explicit deadlines,
   and no task spawned per target. Under backpressure it retains the last
   metric snapshot and does not affect settlement.

## Alternatives considered

### Compare balances only with cumulative receipts

Rejected. It ignores governed vault outflows and makes legitimate releases
look like theft.

### Reconcile only from PostgreSQL projections

Rejected. PostgreSQL is rebuildable evidence/projection state and cannot prove
the authoritative token balance.

### Maintain an on-chain cumulative vault counter

Rejected. It adds hot writable state and contention to settlement and still
requires an independent token-account check.

### Use `getTokenAccountBalance` without account-layout verification

Rejected. It does not provide the full owner/mint/program binding needed to
fail closed against account substitution or unsupported extensions.

### Explicit opening balance plus finalized receipt/release delta

Selected. It gives a conservative explainability invariant, preserves Solana
authority, makes the initial condition auditable, and keeps reconciliation off
the settlement hot path.

## Invariants and failure behavior

- Target route, stream, mint, token program, and vault are immutable and must
  match the verified route contract.
- The observed account is read at `finalized` commitment and its context slot
  is persisted with the result.
- No floating point, UI amount, caller-supplied expected value, or database
  clamp participates in the calculation.
- Missing targets, stale routes, provider disagreement, unsupported
  extensions, malformed responses, aggregate overflow/underflow, and
  non-finalized responses are operational failures, never matches.
- A mismatch produces durable evidence and bounded alerting without mutating
  funds or projections.
- Repeated observations are new append-only evidence, not updates to an old
  run.

## Budgets and evidence

- Maximum active targets per cycle: 64.
- Maximum target RPC response: the existing bounded route RPC limit.
- Default period: 30 seconds, bounded to 5 seconds through 15 minutes.
- RPC deadline: the existing bounded 50 ms through 10 second provider limit.

Unit, property, malformed-RPC, overflow/underflow, account-substitution,
Token-2022 rejection, storage, and Docker integration tests are required. Live
multi-provider evidence, Alertmanager delivery, opening-balance ceremony,
production-scale retention, and supported Token-2022 extension semantics
remain release gates.
