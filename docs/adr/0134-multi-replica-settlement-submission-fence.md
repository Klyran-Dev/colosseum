# ADR-0134: Multi-replica settlement submission-fence evidence

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Security, SRE
- Scope: local settlement submission-fence drill

## Context

ADR-0118 makes the durable `signed -> submitting` transition the authority
that fences wallet-signed settlement submission. A single API process can
still hide a routing defect: two requests may reach different replicas and
both call an RPC provider if correctness depends on process-local state.
Because settlement moves money, this requires explicit cross-replica evidence.

## Decision

Extend the Docker local-stack settlement drill to run an independent second
API container against the same PostgreSQL, NATS, and validating RPC fixture.
The concurrent submission scenario compiles and signs one immutable artifact
once, then sends the two identical signed requests to different API replicas.
The drill requires:

- exactly one HTTP `200` with durable `submitted` state;
- exactly one HTTP `409` fencing response;
- exactly one validating `sendTransaction` call;
- one durable intent and native signature readable from either replica.

The second process has no shared memory or process-local lock with the first.
The database compare-and-set remains the sole submission fence. The drill
does not authorize production RPC or claim regional failover readiness.

## Alternatives considered

### Two requests to one API process

Rejected as the only evidence. It does not exercise replica boundaries.

### Sticky routing or an in-process single-flight lock

Rejected. Both can conceal failover routing and are not durable settlement
authority.

### Let Solana/RPC deduplicate duplicate signatures

Rejected. Duplicate provider work is unsafe operational behavior even when the
ledger rejects or deduplicates the second transaction.

### Selected: two API processes with PostgreSQL compare-and-set fencing

Selected because it exercises the actual authority boundary with bounded,
reproducible Docker evidence while leaving RPC outside the database
transaction.

## Invariants and evidence

- Both submissions contain the exact same verified wallet-signed bytes.
- At most one replica calls `sendTransaction`.
- A losing replica cannot report successful submission or mutate the intent.
- The durable signature is readable and identical across replicas.
- The second container is removed by the drill cleanup path.
- Validator-backed, multi-region, provider-failover, and production capacity
  evidence remain separate release gates.
