# ADR-0189: Automated indexer replay recovery contract

## Status

Accepted — implement as the operational verification of ADR-0043 and the
PostgreSQL replay adapter.

## Context

The indexer already records finalized slot gaps, claims them with short
PostgreSQL leases, replays bounded chunks, persists projection evidence, and
dead-letters exhausted work. Those properties are separately covered by pure
domain tests and storage tests, but a production-grade system also needs proof
that the running service automatically executes the recovery path. Without an
integration contract, service wiring, environment configuration, and
multi-replica lease behavior can regress while unit tests remain green.

## Decision

Add an isolated Docker replay-recovery drill that:

1. boots PostgreSQL 18, the pinned synthetic finalized-RPC fixture, and two
   indexer service replicas from the repository images;
2. inserts one bounded `operator_requested` replay gap for the configured
   stream, with the forward indexer start slot deliberately above the fixture's
   finalized watermark;
3. waits for the running replicas to claim and complete the gap through
   `replay_once`, using the real `SolanaRpcPageSource`, `PgStore`, lease-fenced
   claim, projection transaction, and completion CAS;
4. proves the gap completed after exactly one attempt, the forward checkpoint
   was not advanced by replay, and the service exported fixed-cardinality replay
   metrics; and
5. tears down only the named Compose project and its volumes.

The drill is a release regression contract, not a claim of live-chain
correctness. Linux validator conformance, provider failover, finalized event
decoding, production load/SLO evidence, and staging recovery remain separate
gates.

## Invariants and budgets

- The replay gap has one durable UUIDv7 identity and is completed at most once.
- At most one live replica owns the gap at a time; a losing replica cannot
  advance or complete the row with another replica's claim token.
- Replay never mutates `chain_checkpoints`; only the forward path may advance a
  checkpoint.
- The replay page range is bounded by `INDEXER_PAGE_SLOTS` and the provider
  timeout is bounded by the indexer configuration contract.
- The drill uses no private keys, settlement funds, or production evidence.
- The test harness has a finite startup/recovery deadline and emits service
  logs on failure; it does not retry indefinitely.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Pure replay-queue unit tests | Fast and deterministic | Miss service wiring, SQL leases, and Docker lifecycle failures | Rejected as the sole gate |
| Manual SQL completion | Minimal setup | Bypasses automatic worker behavior and replica fencing | Rejected |
| Broker-owned replay work | Durable delivery primitives | Adds an unnecessary authority and duplicate-delivery surface | Rejected |
| **Two real indexer replicas over PostgreSQL and a bounded RPC fixture** | Verifies automatic claim, projection persistence, completion CAS, checkpoint isolation, and metrics | Synthetic provider; cannot establish live-chain or regional SLO claims | **Selected** |

## Verification and rollout

- Add `scripts/indexer_replay_drill.sh` and `make indexer-replay-drill`.
- Run the drill in Ubuntu CI with retained failure logs and a bounded evidence
  artifact.
- Keep the `indexer_replay_gaps` row visible after completion for forensic SQL
  assertions; named-volume teardown remains mandatory after the run.
- Update production readiness only to mark automated local/CI recovery evidence
  complete; retain live provider, Linux validator, replay SLO, and operational
  alert-delivery gates as open.
