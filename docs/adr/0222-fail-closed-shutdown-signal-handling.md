# ADR-0222: Fail-closed shutdown-signal handling

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, SRE, Runtime Operations
- Bounded contexts: Service Runtime, Worker Operations, Indexer Operations, Route Verification

## Context

The long-running API, worker, indexer, and settlement-route-verifier services
use owned server/polling futures. The worker, indexer, and route-verifier
shutdown futures previously converted `tokio::signal::ctrl_c()` errors to
`None` with `.ok()`. The API logged the same error inside Axum's graceful
shutdown future and then returned success. If the operating-system signal
listener could not be installed or failed while waiting, a service could
continue or report a clean exit without a reliable shutdown control.

## Decision

Each service retains the signal outcome in its structured lifecycle and handles
both outcomes explicitly:

- `Ok(())` requests an orderly loop exit and telemetry shutdown;
- `Err(error)` immediately returns a process error, causing the container
  supervisor to restart or alert according to deployment policy.
- the API also fails if Axum's server future ends before the owned shutdown
  signal future reports a valid result.

The signal listener remains owned by the service loop. No detached task,
retry, or silent fallback is introduced.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Convert signal errors to `None` | Rejected | Masks a runtime control-plane failure and can leave a process running without reliable shutdown. |
| Retry signal registration forever | Rejected | Unbounded retry and ambiguous lifecycle; orchestration owns bounded restart policy. |
| Spawn a separate signal listener task | Rejected | Detaches lifecycle ownership and complicates cancellation/error propagation. |
| **Handle the result inside the existing structured loop or Axum shutdown boundary** | **Selected** | Minimal, bounded, fail-closed, and consistent across service entrypoints. |

## Invariants and failure policy

- A signal-listener error is never treated as a graceful shutdown.
- The API never reports clean shutdown when its server ends before a signal
  result is recorded.
- No service continues polling after its shutdown listener fails.
- Normal signal handling still drains the loop and flushes telemetry through
  the existing bounded shutdown path.
- Service restart and alert policy remain deployment concerns; this ADR does
  not claim production orchestration evidence.

## Verification and release impact

- Source-level runtime checks and workspace Clippy ensure no `.ok()` conversion
  remains on the service shutdown futures.
- The existing service-loop tests and Docker health/lifecycle drills remain
  required; production signal delivery and supervisor-restart evidence remain
  deployment gates.
