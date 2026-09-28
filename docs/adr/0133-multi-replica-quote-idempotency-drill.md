# ADR-0133: Multi-replica quote idempotency evidence

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, SRE, Security
- Scope: bounded quote-service load and idempotency evidence

## Context

The quote endpoint uses PostgreSQL uniqueness and transactional evidence to
fence idempotency, but a single-process load test cannot prove that two API
replicas observe the same authority boundary. An in-process lock would make a
single replica appear correct while allowing duplicate calculations when a
load balancer routes retries to another replica.

## Decision

Extend the Docker quote-service evidence harness to run two independent API
containers against the same PostgreSQL and NATS services:

- the Compose-managed API remains the primary replica;
- a second API container is launched from the same pinned image with an
  independent host port and no private state sharing;
- unique-key workload requests alternate between replicas;
- bounded same-key contention requests also alternate between replicas;
- the harness requires one durable idempotency record, one calculation, and
  one transactional outbox event for the shared key, with no request errors.

The database remains the authoritative fence. No in-process mutex, sticky
session, cache, or broker acknowledgement is used as correctness evidence.
The drill is local capacity/concurrency evidence, not a production SLO or a
claim that a particular load balancer, region, or database topology has been
validated.

## Alternatives considered

### Single API process with high concurrency

Rejected as the only evidence. It exercises row contention but cannot expose
replica-local state or routing differences.

### In-process lock or distributed cache lock

Rejected. It adds a second authority and can diverge from the durable
transaction; Redis is explicitly non-authoritative in this system.

### Sticky load-balancer sessions

Rejected. Stickiness hides failover and retry routing behavior and is not a
correctness guarantee.

### Selected: two real API replicas with PostgreSQL fencing

Selected because it is bounded, reproducible in Docker, exercises the actual
service binary and persistence boundary, and fails if correctness depends on
process-local state.

## Invariants and evidence

- Both replicas use the same authenticated tenant and request document.
- The shared idempotency key produces exactly one durable record, calculation,
  and outbox event.
- All bounded workload requests return the declared success/conflict status;
  no duplicate calculation or event is accepted.
- Cleanup removes the second container and only the drill's named Compose
  resources.
- CI retains the workload output as evidence; it does not mark production
  readiness without regional, resource-profile, and live-provider evidence.
