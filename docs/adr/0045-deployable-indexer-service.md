# ADR-0045: Run finalized indexing as an independently scalable service

- Status: Accepted for the first executable indexer runtime
- Date: 2026-09-26
- Owners: Chain Data / SRE
- Scope: Deployable Solana indexer process and Docker topology

## Context

The indexer domain, finalized provider adapter, storage transaction, and replay
lease API are individually implemented, but a library without a process cannot
advance a checkpoint or recover a gap in production. Combining indexing with
the transactional-outbox publisher would couple unrelated failure domains and
make scaling, permissions, and SLOs opaque.

## Decision

Create a separate Rust/Tokio `indexer-service` binary. It constructs one
finalized-only HTTP JSON-RPC adapter per configured HTTPS endpoint, injects the
pinned `SettlementExecutedDecoder`, and composes them behind the bounded,
health-scored failover port. Each loop iteration loads the stream checkpoint,
plans one bounded page, fetches and validates it, and commits receipts/raw
evidence/checkpoint state through the serializable PostgreSQL adapter. Provider
or persistence failures create a deduplicated replay-gap row and leave the
checkpoint unchanged. The runtime uses bounded polling, cancellation-aware
shutdown, readiness health, and no wallet or signing authority.

The outbox worker remains a separate deployable. The indexer receives its own
database pool and resource budget, and the Compose topology enables it only
when an RPC endpoint and explicit starting cursor are configured. No local
fixture or simulated chain status is used as an indexing success path.

The local Compose profile is an explicit exception: `INDEXER_ENV=local` and
`INDEXER_ALLOW_INSECURE_LOCAL_RPC=true` permit the validator container's
network-only HTTP endpoint. The transport rejects that exception unless both
values are present; production deployments must use HTTPS RPC endpoints and
leave the flag disabled.

## Alternatives considered

- Run indexing inside the outbox worker: rejected because broker outages would
  affect chain evidence ingestion and the two workloads need independent scale.
- Use a cron job or shell loop: rejected because checkpoint fencing,
  cancellation, health, and retry semantics belong in typed application code.
- Use a managed indexer as the authority: rejected because provider output still
  needs finalized commitment, pinned ABI decoding, raw evidence, and replay
  validation under our control.
- Run one process per RPC provider: rejected for the first deployment because
  it duplicates checkpoint coordination; the bounded failover pool is the
  simpler initial topology.

## Consequences

Positive:

- The full finalized-RPC-to-PostgreSQL path is executable and independently
  observable.
- Indexer and outbox failure domains, permissions, and scaling are separated.
- No failure path advances a checkpoint or reports settlement success without
  finalized page validation and serializable persistence.

Costs and follow-on gates:

- Provider URLs and starting cursors require secret/configuration management and
  operational review; the service refuses incomplete configuration.
- Replay leases are persisted but still require a dedicated replay worker path
  and operator UI/runbook before mainnet enablement.
- Live Solana localnet transaction evidence, load tests, and fleet-level SLOs
  remain release gates.
