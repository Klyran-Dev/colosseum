# ADR-0120: Process-crash settlement recovery drill

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture and SRE
- Scope: local Compose recovery evidence for durable settlement intents

## Context

An accepted-but-ambiguous provider response proves the API's explicit unknown
path, but a process crash can happen after the durable `submitting` claim and
before the provider response is persisted. The recovery contract must prove
that restarting the API does not reopen submission or require the wallet to
sign again.

## Decision

Add a local-only RPC fixture hold/release control and a Compose drill that:

1. holds the provider response after validating and recording the exact signed
   transaction;
2. waits until PostgreSQL reports the intent as `submitting` with its native
   signature;
3. sends `SIGKILL` to only the project-scoped API container;
4. releases the fixture response and waits for the API container to become
   ready again;
5. reads the recovered `submitting` intent; and
6. reconciles the persisted signature to `confirmed` without calling
   `sendTransaction` a second time.

Readiness, durable readback, and reconciliation use bounded retries. A
container can report readiness while the first post-restart application socket
is still being replaced; a transient connection reset is therefore treated as
recoverable only inside the drill's finite recovery window, never as a reason
to extend retries indefinitely.

The fixture controls are test-only, bounded, and unreachable through the
production API. The drill uses the real container, PostgreSQL, API, coordinator,
and RPC transport boundaries. It does not mutate lifecycle rows directly.

## Alternatives considered

### Unit-test a panic or cancellation inside the submit handler

Rejected. It does not exercise container restart, durable storage recovery, or
the real HTTP/RPC transport boundary.

### Kill the API on a fixed timer

Rejected. Scheduler timing does not prove the process died after the durable
submission claim and before response persistence.

### Manually edit an intent to `submitting`

Rejected. It bypasses the actual claim and cannot prove the signature was bound
before the crash.

### Hold the exact provider response, kill after durable claim, reconcile after restart

Selected. It creates a deterministic crash window around the real critical
section and preserves the same native signature throughout recovery.

## Invariants and release limits

- The crash drill kills only the named local API container and removes its
  project-scoped resources during normal cleanup.
- The intent is observed as `submitting` before the process is killed.
- The recovered intent retains its native signature and does not contain signed
  transaction bytes.
- Reconciliation advances from chain evidence and the send count increases by
  exactly one across the complete drill.
- This is local single-replica evidence; multi-replica, real validator,
  provider failover, and production orchestration recovery remain separate
  release gates.

## Verification

`scripts/verify_local_stack.sh` runs the drill against the pinned Compose API,
PostgreSQL 18, and RPC fixture images. The output must include a recovered
`submitting` state, a `confirmed` reconciliation, and one total send call.
