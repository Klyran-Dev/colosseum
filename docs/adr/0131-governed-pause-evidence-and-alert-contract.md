# ADR-0131: Governed pause evidence and alert contract

- Status: Accepted
- Date: 2026-09-27
- Owners: Protocol Security, Platform SRE
- Related: ADR-0052, ADR-0084, ADR-0123, ADR-0130

## Context

ADR-0052 established the singleton on-chain `ProtocolConfig.paused` control,
but a production containment control also needs finalized evidence, restart
recovery, fixed-cardinality alerting, and a rehearsable operational workflow.
An API or cache flag cannot stop direct wallet transactions and must not be
treated as an authority boundary. Conversely, treating an indexer projection as
the source of funds would create an unsafe split-brain authority model.

## Decision

The finalized Solana chain remains authoritative. The indexer recognizes the
versioned `ProtocolPauseChanged` Anchor event, validates its discriminator,
authority, canonical boolean encoding, signature, slot, and evidence digest,
and persists the event in the append-only `protocol_pause_events` table in the
same serializable projection transaction as the page's checkpoint/evidence.
Forward scans and bounded replay use the same idempotent merge contract. The
database migration also installs immutable update/delete guards, and the clean
Compose bootstrap contract verifies that every migration, including this one,
is mounted before services start.

The indexer exposes only fixed-cardinality operational signals:

- `stp_settlement_protocol_pause_known`;
- `stp_settlement_protocol_paused`; and
- `stp_settlement_protocol_pause_slot`.

Prometheus pages on an active pause immediately and on unknown pause state
after the bounded grace period. Unknown state is unsafe, not equivalent to
unpaused. Initialization emits the same canonical event with `paused=false`,
so a configured protocol becomes known without inventing state from event
absence. The alert routes to the governed emergency-pause runbook. No API,
web client, Redis value, or database row may authorize pause or unpause.

The localnet conformance harness must submit pause, prove a settlement attempt
fails without token or receipt mutation, submit unpause, and then prove the
normal settlement path. Staging and production additionally require approved
multisig/timelock authority, independent finalized-RPC verification, incident
evidence, and release-candidate rehearsal before the readiness gate closes.

## Alternatives considered

1. **Logs-only alerting.** Rejected: logs are not restart-safe, have no
   bounded query contract, and can be lost or duplicated during provider
   failover.
2. **API/NGINX kill switch.** Rejected as authority: it cannot stop direct
   wallet transactions or already signed transactions and can create a false
   impression of on-chain containment.
3. **Database-owned pause flag.** Rejected: PostgreSQL is a projection and
   must not become a second settlement authority.
4. **Unbounded event labels per merchant or authority.** Rejected: user- or
   operator-controlled labels create cardinality and cost risk.
5. **Finalized event projection with fixed-cardinality alert signals.**
   Selected: restart-safe, replayable, auditable, bounded, and explicitly
   subordinate to the Solana account and finalized transaction evidence.

## Invariants and failure behavior

- A malformed, non-finalized, conflicting, or digest-mismatched event fails
  closed and cannot advance a page.
- A duplicate event is idempotent only when every immutable field matches.
- A page and its pause projection are committed atomically; partial commits are
  not accepted.
- Pause status is not inferred from missing events. Missing history emits the
  unknown-state signal.
- Unpause is never automatic, retried by the service, or initiated by API
  credentials.

## Evidence and acceptance

Unit tests cover canonical event decoding, digest validation, duplicate and
conflicting evidence, and fixed-cardinality metrics. The validator-backed
localnet test covers atomic settlement blocking and recovery. The remaining
production gate is independent staging rehearsal with real governance
authority, alert delivery, and incident-record evidence.
