# ADR-0228: Atomic Tax Engine mTLS certificate reload

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Security, SRE
- Scope: Tax Engine service certificate lifecycle
- Bounded contexts: Tier-1 Tax Engine, Service Runtime, Platform Security

## Context

ADR-0227 requires direct Rustls mutual TLS for staging and production Tax
Engine listeners. A process restart is safe but creates avoidable connection
churn and a larger rotation blast radius. A reload path must not expose a
partially written certificate, install an invalid trust policy, block the
async runtime on file I/O, or silently fall back to plaintext.

## Decision

On Unix staging and production deployments, the Tax Engine accepts `SIGHUP` as
an explicit certificate-rotation signal. Each delivered signal performs one
bounded reload cycle:

1. Read and validate the complete server certificate chain, private key, client
   CA bundle, client verifier, and ALPN policy in a blocking task.
2. If validation succeeds, atomically replace the `RustlsConfig` snapshot used
   for new connections.
3. If validation fails, retain the previous live configuration, emit one
   structured warning, and continue serving with the last known-good identity.

Only one reload is evaluated at a time. The signal stream is bounded by the
OS signal semantics; the service does not create a task or queue per signal.
Shutdown owns and drops the reload future with the server future, preserving
structured concurrency. Signal-stream setup failure or termination fails the
service closed rather than leaving rotation observability ambiguous.

Certificate files must be provisioned by an external secret manager using
atomic replacement and appropriate filesystem permissions. This ADR does not
grant the service authority to issue, revoke, distribute, or escrow keys.
CA rotation requires a compatible overlap/rollout plan and remains a
deployment-controlled change.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Restart the process for every rotation | Simple lifecycle and clean memory state | Connection churn, larger blast radius, slower emergency response | Rejected as the only mechanism |
| Poll or watch certificate files | Automatic detection | Watcher semantics vary, partial writes are easy to observe, hidden reload authority | Rejected |
| Sidecar-only reload and proxy termination | Centralized platform operations | Adds a hop and a second transport authority to the internal hot path | Rejected as the default |
| **Explicit SIGHUP with validated atomic Rustls snapshot replacement** | Auditable operator action, no extra hop, bounded work, preserves last known-good state | Unix signal contract and deployment ceremony are required | **Selected** |

## Invariants and failure behavior

- No connection observes a partially constructed Rustls configuration.
- Invalid or incomplete replacement material never replaces a valid active
  configuration and never downgrades transport security.
- Certificate or private-key contents are never logged or included in tax,
  request, or audit evidence.
- Reload parsing and file I/O do not execute on the Tokio worker thread.
- Reload work is sequential, bounded, cancellation-safe, and does not create
  unbounded tasks or queues.
- New connections use the replacement snapshot after a successful swap;
  existing connections are not forcibly terminated by the reload.
- SIGHUP is not an authorization signal for tax calculations, settlement, or
  policy changes.

## Verification and release impact

- Unit tests prove a valid replacement changes the active configuration and a
  malformed replacement leaves the prior configuration usable.
- Existing trusted-client and untrusted-client handshake tests remain the
  transport boundary checks.
- Production evidence must still prove atomic secret-manager file delivery,
  certificate/key rotation, CA overlap and revocation response, signal
  permissions, regional reload latency, connection-drain behavior, and alert
  delivery for rejected reloads.
