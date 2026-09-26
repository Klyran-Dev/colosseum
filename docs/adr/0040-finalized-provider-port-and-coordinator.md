# ADR-0040: Enforce finalized provider pages at the indexer coordinator

- Status: Accepted for the first provider/application boundary
- Date: 2026-09-26
- Owners: Chain Data / SRE
- Scope: Solana finalized RPC/Geyser adapters and bounded backfill orchestration

## Context

The persistence adapter cannot determine whether a page came from a finalized
Solana observation, whether the provider returned the requested slot range, or
whether a response was truncated. Treating a provider response as trusted
would allow a weaker commitment or malformed page to advance durable state.

## Decision

Introduce a finalized-only `FinalizedPageSource` port in the indexer bounded
context. The `IndexerCoordinator` plans bounded ranges from a checkpoint,
requests exactly that range, validates exact range identity, requires a
complete page and a finalized watermark at or beyond the range end, rejects
duplicate/conflicting receipt identities, and only then delegates checkpoint
advancement to the domain transition.

The first HTTP JSON-RPC adapter is implemented with Rustls, bounded response
body reads, request/connect deadlines, JSON-RPC envelope validation, HTTPS-only
remote endpoints, and finalized `getSlot`, `getBlocks`, and `getBlock` calls.
Receipt decoding remains an injected port because program/event layouts are
domain-specific. A bounded failover wrapper tries at most three providers with
an individual deadline and preserves the same finalized-page contract; it never
downgrades commitment. Geyser remains an adapter behind the same port. Provider
implementations must map errors into bounded transport errors. The storage
adapter remains responsible for the serializable commit described by ADR-0039.

## Alternatives considered

See `docs/architecture/alternative-analysis.md` under “Chain indexing and
reconciliation”. Letting storage trust raw provider payloads was rejected
because it mixes transport semantics with evidence authority. WebSocket-only
consumption was rejected because notification loss is not repairable without
bounded backfill.

## Consequences

Positive:

- Provider implementations are replaceable and testable without a live RPC.
- Commitment, range, completeness, and identity invariants are enforced before
  persistence.
- Provider failover cannot silently weaken finality semantics.

Remaining work:

- Add provider rate limits, health scoring, adaptive endpoint selection, and
  fleet-level cancellation budgets around the bounded failover wrapper.
- Add program-specific raw transaction/event decoding, signature pagination,
  gap records, and localnet transaction tests before enabling a live indexer
  deployment.
