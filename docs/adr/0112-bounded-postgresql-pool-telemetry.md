# ADR-0112: Bounded PostgreSQL pool telemetry

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Storage, and SRE
- Scope: Rust storage adapter, API metrics, saturation evidence

## Context

The API uses a bounded PostgreSQL connection pool with FIFO acquisition and a
250 ms wait budget. Pool exhaustion is a latency and availability failure
mode, but the current metrics surface exposes only request timing and rule
cache state. Operators cannot distinguish database contention from CPU, RPC,
or application errors, and a local load result cannot prove the configured
pool bound was actually exercised.

The telemetry must be read-only, cheap, bounded in cardinality, and safe to
sample while overloaded. It must not add a database round trip or put a
per-request label such as merchant, request ID, or SQL text into Prometheus.

## Decision

Expose the deadpool status snapshot through the storage port as
`DatabasePoolStats` containing only `max_size`, `size`, `available`, and
`waiting`. The API renders these as fixed-label Prometheus gauges:

- `stp_database_pool_max_size`
- `stp_database_pool_size`
- `stp_database_pool_available`
- `stp_database_pool_waiting`

The snapshot is explicitly eventually consistent, as documented by deadpool;
it is an operational signal rather than a correctness decision. The metrics
endpoint reads it without acquiring a connection, so scraping remains useful
when all database connections are busy. The existing configured wait timeout
continues to fail requests rather than allowing unbounded queues.

The saturation drill will run a small, explicitly configured pool under
bounded concurrent quote/idempotency load, sample the gauges during the load,
and retain the observed pool bound and peak waiting count. This is local
capacity evidence, not a production SLO or a substitute for production-like
database sizing.

## Alternatives considered

### Query PostgreSQL pool state from the metrics handler

Rejected. It consumes a scarce connection, adds latency to an overload path,
and cannot observe the pool when acquisition is already failing.

### Export per-request or tenant pool labels

Rejected. It creates unbounded Prometheus cardinality and risks leaking
operational identifiers.

### Rely only on request latency and error rate

Rejected. Those signals do not identify pool exhaustion or distinguish waiting
from query execution.

### Unbounded pool growth under pressure

Rejected. It moves overload into PostgreSQL, removes predictable resource
bounds, and creates connection storms across replicas.

## Consequences

- Operators can alert on pool waiters and unavailable connections without a
  database round trip.
- The local drill can prove the configured pool bound and observed queueing.
- Status values are snapshots and may be transient or eventually consistent;
  they must not be used as settlement authority.
- Production pool sizing, PostgreSQL `max_connections`, replica budgets,
  account-lock contention, and regional load evidence remain deployment gates.

## Verification

- Storage exposes the bounded typed status and API metrics render fixed labels.
- Unit tests assert metric names and reject dynamic labels.
- Docker load evidence samples the gauges while a constrained pool handles
  concurrent quote and same-key idempotency traffic.

The local drill completed on 2026-09-27 with a one-connection pool, 256 normal
requests at concurrency 64, 256 same-key contention requests, zero errors,
and a peak observed waiter count of 3. It converged to one contention
idempotency record, calculation, and outbox event. This is a bounded local
capacity result, not a production capacity claim.
