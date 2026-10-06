# ADR-0334: Capability snapshot expiry boundary

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal Low-Latency Architecture, Security, and Product UI
- Scope: React capability-snapshot lifecycle and protected workspace affordances

## Context

The API returns a short-lived, server-evaluated capability snapshot. Parsing
the expiry protects the initial response, but a React tree can remain mounted
after that timestamp. Rendering an earlier `allow` after expiry would expose a
stale affordance and contradict the fail-closed authorization contract.

## Decision

The browser applies the expiry boundary in two places:

1. `evaluateUiAccess` treats an expired or missing expiry as indeterminate at
   render time, so stale state cannot present an actionable allow.
2. The shell schedules one bounded refresh at the server-provided expiry. The
   current snapshot is replaced with an evaluation-required state before a new
   capability request is issued, and the request reuses the existing in-memory
   credential. No protected operation is authorized by this timer; every API
   handler continues to re-authorize independently.

The refresh delay is derived by a pure helper and is clamped at zero for an
already-expired snapshot. Invalid clock or expiry data causes a fail-closed
refresh path rather than an optimistic affordance.

## Alternatives considered

| Option | Stale-affordance risk | Request-path cost | Recovery behavior | Decision |
| --- | --- | --- | --- | --- |
| No client expiry enforcement | High | None | None | Rejected |
| Check expiry only while rendering | Low | None | Requires a later render to refresh | Rejected |
| Poll on a fixed interval | Partial | Unbounded repeated requests | Eventually refreshes, independent of server TTL | Rejected |
| **Server TTL boundary with one bounded refresh and render-time check** | **Low** | **One request per snapshot** | **Immediate indeterminate state, then refresh** | **Selected** |

## Invariants and failure policy

- An expired capability snapshot never produces an `allow` UI decision.
- A server-allowed snapshot without a finite expiry never produces an
  actionable affordance.
- The browser never derives a replacement decision from roles, scopes,
  tenant identifiers, or attributes.
- The refresh timer is bounded to the snapshot lifetime and is cancelled when
  the token or snapshot changes or the shell unmounts.
- API authorization, tenant scoping, and resource checks remain authoritative.
- Timer failure, clock anomalies, malformed responses, and refresh errors leave
  protected actions indeterminate or denied; they do not authorize settlement.

## Latency and operations

The render-time check and delay calculation are constant-time and perform no
network, database, RPC, or broker work. Refresh is outside the wallet-signing
and settlement hot path. Capability TTL remains bounded by the API contract;
staging evidence must measure refresh latency, clock skew, revocation behavior,
and user-visible recovery under regional network failure.

## Verification

- Vitest covers render-time expiry, missing expiry, valid TTL delay, the expiry
  boundary, and non-server-allowed snapshots.
- `make web-check` and `make web-browser-check` remain required.
- Staging must retain identity-provider expiry/revocation, clock-skew,
  multi-tenant, screen-reader, and refresh-failure evidence before a
  production authorization claim.
