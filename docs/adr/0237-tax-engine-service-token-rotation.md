# ADR-0237: Atomic Tax Engine service-token rotation

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Security, SRE
- Scope: Tax Engine application authentication
- Bounded contexts: Tier-1 Tax Engine, Service Runtime, Workload Identity

## Context

The Tax Engine requires a bearer token on protected application routes as
defense in depth after mTLS. A process-lifetime token snapshot forces a
restart for every credential rotation, creates avoidable connection churn, and
extends the exposure window of a replaced secret. Rotation must not add a
database, secret-manager, or mutex operation to every calculation request.

## Decision

Support either an inline `TAX_ENGINE_AUTH_TOKEN` or a file-backed
`TAX_ENGINE_AUTH_TOKEN_FILE`, but never both. The file source is bounded to 4
KiB, read as UTF-8, trimmed using the same token validation rules as the inline
source, and validated before listener bind. The file path is retained without
logging its contents.

On Unix, `SIGHUP` performs one sequential runtime reload. The service reads
and validates the complete replacement token and, when configured, the full
Rustls/mTLS configuration in one blocking task. Only after all replacements
validate does it atomically publish the immutable token snapshot and Rustls
snapshot. Invalid replacement material retains every last-known-good snapshot;
the service never falls back to an inline or local token. Existing connections
remain valid until naturally closed, while new requests use the replacement
token.

The request path performs one lock-free immutable snapshot load and constant-
time bounded token comparison. No raw token, token digest, or secret value is
logged, emitted as a metric label, or included in evidence.

Each reload attempt records only the fixed `runtime_reload` operation in the
existing `tax_engine` metrics registry. Success and rejection durations are
bounded telemetry; rejection increments the fixed error counter and triggers
the platform alert without exposing replacement contents.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Restart for every rotation | Rejected as the only mechanism | Causes connection churn and enlarges emergency blast radius. |
| Secret-manager API call per request | Rejected | Adds latency and an external availability dependency to the calculation path. |
| Mutable mutex-protected token | Rejected | Adds request-path contention and makes snapshot semantics less explicit. |
| **Bounded file source plus atomic SIGHUP snapshot replacement** | **Selected** | Keeps the hot path lock-free, supports auditable rotation, and retains last-known-good state on failure. |

## Invariants and failure behavior

- Inline and file-backed token configuration are mutually exclusive.
- Non-local startup still rejects the local development token.
- File reads are bounded and occur outside the Tokio request executor.
- A malformed, empty, oversized, or unreadable replacement cannot change the
  active token or TLS configuration.
- A failed token reload does not terminate a healthy server, but the failure
  is observable through a bounded warning and deployment evidence.
- Token values never appear in logs, metrics, traces, error bodies, or tests'
  retained artifacts.

## Verification and release impact

- Unit tests cover source exclusivity, bounded file reads, valid rotation,
  invalid-replacement retention, and old/new authorization behavior.
- The pinned Docker runtime contract mounts a synthetic token file, rotates it
  atomically with `SIGHUP`, and proves old-token rejection and new-token
  acceptance.
- The required Ubuntu CI workflow runs this real-container contract as a
  dedicated service-token rotation job; release-candidate evidence must retain
  the job result with the other runtime contracts.
- Prometheus alerts on any rejected `runtime_reload` attempt and points
  operators to this runbook; metrics never contain token or certificate data.
- Secret-manager atomic delivery, file ownership/permissions, rotation
  overlap, compromise response, regional reload latency, and alert/on-call
  evidence remain production deployment gates.
