# ADR-0121: Structured service health-server lifecycle

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture and SRE
- Scope: worker, finalized indexer, and settlement route-verifier process lifecycle

## Context

The long-running Rust services run a health/ready/metrics HTTP server beside
their bounded work loop. Detached `tokio::spawn` tasks make the server lifetime
independent from the service lifecycle: a server failure can be lost, and a
shutdown signal can return from the work loop without awaiting cancellation of
the health task.

## Decision

Run each health server as a pinned future in the service's structured
`tokio::select!` loop. The service exits with an error if the health server
stops unexpectedly. A shutdown signal exits the loop and drops the health
future, propagating cancellation without a detached task or unbounded join
set.

The work loop remains bounded by its existing poll interval, lease, batch, and
RPC deadlines. No request or settlement path is moved onto the health server.

## Alternatives considered

### Detached health-server task

Rejected. It loses failure propagation and makes lifecycle ownership implicit.

### Unbounded task registry or join set

Rejected. There is exactly one health server per process; a registry adds
complexity and an unnecessary task-management surface.

### Separate supervisor process

Rejected for this boundary. Container/process supervision remains useful for
restart policy, but it cannot replace in-process health-server failure
propagation or graceful cancellation.

### Pinned health future selected with the work loop

Selected. It preserves low allocation overhead, bounded concurrency, explicit
failure ownership, and cancellation propagation with no detached work.

## Invariants and verification

- There is at most one health-server future per service process.
- Unexpected health-server termination fails the process rather than silently
  leaving readiness semantics undefined.
- Shutdown does not leave a detached health task behind.
- `cargo test`, clippy, and service startup/health checks remain green.
