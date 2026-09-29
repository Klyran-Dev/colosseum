# ADR-0190: Indexer replay retry-exhaustion contract

## Status

Accepted — implement as the failure-path verification of ADR-0043, ADR-0123,
and ADR-0189.

## Context

Replay recovery must be automatic, but automatic retry without a hard boundary
can amplify an RPC outage and hide an accounting omission. The indexer domain
and storage adapter define a bounded eight-attempt state machine with explicit
dead-letter state and fixed-cardinality metrics. Existing unit tests and the
authenticated operator API do not prove that the running two-replica service
reaches that state when the provider continuously fails.

## Decision

Add an isolated Docker failure drill that boots PostgreSQL 18, the pinned RPC
fixture configured to fail `getBlocks`, and two real indexer service replicas.
The drill inserts one bounded replay gap and waits for the service to:

1. claim it through the PostgreSQL lease;
2. call the finalized page source and receive the deterministic provider
   failure;
3. release the exact claim token back to the retry state machine;
4. repeat only through the configured maximum of eight attempts; and
5. persist `dead_lettered` with a bounded `last_error_code`, no live claim
   token, and a refreshed fixed-cardinality metric.

The forward checkpoint is asserted unchanged. The drill is local/CI failure
evidence; it does not claim live Alertmanager delivery, provider outage SLOs,
or that dead-lettered work authorizes settlement.

## Invariants and budgets

- The target gap reaches `dead_lettered` at exactly `MAX_REPLAY_ATTEMPTS`.
- A dead-lettered gap has no active claim token or claim timestamp.
- The last provider failure is retained as a bounded error code.
- No retry creates or advances a `chain_checkpoints` row.
- Two replicas share durable state; process-local counters cannot determine
  exhaustion.
- The harness has a finite recovery deadline and tears down only its named
  Compose project and volumes.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Pure replay state-machine tests | Fast and deterministic | Misses service polling, SQL lease release, and metric refresh wiring | Rejected as the sole gate |
| Seed an already-dead-lettered row | Exercises read and alert projection | Bypasses automatic retry and lease fencing | Rejected |
| Unbounded retry with operator cancellation | May eventually recover transient provider faults | Retry storms and no finite incident boundary | Rejected |
| **Two indexer replicas over PostgreSQL with a failing finalized RPC fixture** | Exercises automatic bounded attempts, token release, dead-letter persistence, metrics, and checkpoint isolation | Synthetic provider; notification delivery remains external | **Selected** |

## Verification and rollout

- Add `scripts/indexer_replay_dead_letter_drill.sh` and
  `make indexer-replay-dead-letter-drill`.
- Run it in Ubuntu CI and retain the JSON/metrics evidence in the release
  candidate bundle.
- Mark automatic retry/dead-letter execution as locally verified while leaving
  production alert routing, on-call response, and live-provider recovery SLOs
  as deployment gates.
