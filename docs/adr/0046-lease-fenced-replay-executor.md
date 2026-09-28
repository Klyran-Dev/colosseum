# ADR-0046: Lease-fenced bounded replay executor

- Status: Accepted for the first executable replay path
- Date: 2026-09-26
- Owners: Chain Data / SRE
- Scope: Historical indexer projection repair

## Context

ADR-0044 persisted replay gaps and leases, but a lease table without an
executor leaves recovery dependent on manual SQL. Replay must repair receipt
and raw-event projections without moving the forward checkpoint, must tolerate
process loss, and must not allow a stale worker to acknowledge newer work.

## Decision

The Rust `indexer-service` executes at most one replay claim per polling tick.
It bounds every claimed range into `INDEXER_PAGE_SLOTS` chunks, fetches each
chunk through the same finalized-only provider/coordinator path as forward
indexing, persists receipts and raw evidence in a serializable idempotent
transaction, and advances the leased row with its claim token.

The replay persistence transaction never writes `chain_checkpoints`. A crash
after projection commit and before lease advancement is safe: the lease can
be reclaimed and the same page merges idempotently. A provider or projection
failure releases the lease through the existing bounded retry/dead-letter
state machine. Successful partial chunks re-queue only the remaining suffix
and reset the failure counter because the completed chunk succeeded.

The executor processes one replay chunk before the normal forward-indexing
iteration. This gives repair work a bounded resource budget and prevents a
large historical gap from monopolizing the service or causing unbounded
concurrency.

## Alternatives considered

- Advance the canonical checkpoint while replaying: rejected because a
  historical repair is not forward progress and could skip authoritative
  evidence.
- Mark a whole gap complete after the first bounded page: rejected because it
  would silently discard the unprocessed suffix.
- Use an in-memory replay queue only: rejected because process loss would lose
  recovery work and operator visibility.
- Run replay in an unbounded worker pool: rejected because RPC, database, and
  provider budgets must remain explicit and lease contention would obscure
  ordering.

## Consequences

Positive:

- Replay is executable, crash-recoverable, idempotent, and lease-fenced.
- Forward checkpoint advancement remains isolated from historical repair.
- Large gaps are split into bounded chunks without dropping suffixes.

Costs and follow-on gates:

- Live localnet and multi-provider replay drills are still required before
  mainnet enablement.
- Replay metrics, dead-letter alert routing, operator acknowledgement UI, and
  clean-database rebuild evidence remain production-readiness gates.
