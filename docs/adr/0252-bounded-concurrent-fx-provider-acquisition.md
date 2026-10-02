# ADR-0252: Bounded concurrent FX provider acquisition

- Status: Implemented as a vendor-neutral library boundary; production adapter pending
- Date: 2026-10-02
- Scope: FX provider adapter orchestration before deterministic admission

## Context

ADR-0246 defines a vendor-neutral provider source port and ADR-0250 defines
exact agreement semantics. The repository still needs an application boundary
that can query the approved provider set concurrently without allowing provider
latency, task creation, response identity, or cancellation to become an
unbounded part of the quote path.

Provider acquisition is outside the pure tax-engine calculation and signing
paths. It must return enough bounded outcome information for operations to
observe timeouts and source rejection while leaving `FxProviderPolicy` as the
only authority for candidate acceptance.

## Decision

Add a Tokio-backed bounded acquisition coordinator to the `fx-provider` crate.

1. Add an object-safe `AsyncFxQuoteSource` port with an immutable provider
   identity. The returned candidate must carry exactly that identity; a
   mismatch is a failed attempt and is never admitted to policy evaluation.
2. Validate the request, source count, unique source identities, provider
   allowlist, concurrency, and timeout bounds before any provider call.
3. Cap sources and in-flight work at `MAX_ALLOWED_PROVIDERS` (16). The caller
   may choose a lower `max_concurrency`; all user/configuration-controlled
   collection sizes remain bounded.
4. Execute approved sources concurrently using a bounded Tokio task set and
   semaphore. Each attempt has a per-provider deadline and the whole operation
   has an aggregate deadline. Aggregate timeout cancels outstanding work and
   returns `AcquisitionDeadlineExceeded`.
5. Preserve successful candidates and bounded failure classifications in an
   `FxProviderAcquisition` result. Individual timeout or source rejection does
   not become a quote; the caller must still run `select_candidate`, which
   enforces minimum agreement and exact economic equality.
6. Never average, interpolate, reorder by response timing, retry indefinitely,
   log raw provider payloads, or let a provider error authorize issuance.
   Retries, circuit breaking, transport telemetry, and provider-specific
   authentication remain adapter responsibilities with explicit budgets.
7. The coordinator has no database, clock, signer, tax-engine, or Solana RPC
   dependency. The injected request and provider futures are its only inputs.

### Bounds

| Bound | Default | Maximum | Failure |
| --- | ---: | ---: | --- |
| Provider sources | 16 | 16 | Invalid configuration |
| In-flight calls | 16 | 16 | Invalid configuration |
| Per-provider timeout | 750 ms | 5 s | Invalid configuration / timeout classification |
| Aggregate timeout | 1 s | 10 s | `AcquisitionDeadlineExceeded` |

Defaults are engineering starting points, not production SLOs. Regional
production values require measured provider latency, legal freshness, and
error-budget evidence.

## Invariants and failure policy

- No provider call occurs for an invalid request, invalid source topology,
  unapproved source, duplicate source identity, or invalid timeout config.
- A candidate from a source with a different provider identity is rejected.
- Returned candidates are not authority; `select_candidate` must be called
  before `FxEvidenceIssuer::issue_candidates`.
- Partial provider failures are visible as bounded classifications and cannot
  silently count toward agreement.
- Cancellation propagates when the aggregate deadline expires; no detached
  task may continue provider work after the acquisition future is dropped.
- Provider order and response completion order do not affect admission or
  selection. Policy priority remains the deterministic selection order.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Sequential provider calls | Simple implementation | Adds provider tail latency linearly and makes failover slow | Rejected |
| Unbounded task per provider/request | Low apparent code complexity | Task and socket exhaustion, retry storms, weak cancellation | Rejected |
| First successful response | Lowest apparent latency | Nondeterministic authority and no agreement evidence | Rejected |
| Broker-backed acquisition | Durable asynchronous retries | Adds a queue/database dependency to quote issuance and complicates expiry | Rejected |
| **Bounded concurrent in-process acquisition with explicit deadlines** | Low latency, bounded resources, cancellation-safe, and testable | Requires adapter-level retry/circuit policy and outcome telemetry | **Selected** |

## Verification

The `fx-provider` crate now has coverage for invalid topology,
duplicate/unapproved source identities, source identity substitution, request
rejection before network, bounded concurrency, provider timeout, aggregate
cancellation, partial failure, and successful agreement handoff to the existing
selector. The concurrency test measures maximum in-flight calls and proves it
never exceeds the configured bound. `cargo test --locked -p fx-provider` and
`cargo clippy --locked -p fx-provider --all-targets -- -D warnings` pass.
Production provider SLO, regional failover, HSM/KMS, legal ownership, and live
transport evidence remain separate gates.
