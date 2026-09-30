# ADR-0211: Explicit Docker host-port overrides

- Status: Accepted
- Date: 2026-09-30
- Owners: Platform / SRE
- Scope: Local Compose developer topology and reproducible test isolation

## Context

The full-topology bootstrap uses a distinct Compose project name so that
containers, networks, and named volumes are isolated. Compose project
isolation does not isolate host sockets: a second checkout or an existing
local stack can still own the default Redis, API, observability, or validator
ports. The previous fixed validator bindings also made the collision policy
inconsistent with the other services, whose host ports were already
configurable.

The failure must remain fail-closed, but developers need a deterministic way
to run two isolated topologies without stopping an unrelated stack.

## Decision

Expose every host-facing port in `compose.yaml` through an explicit
environment variable. Internal container ports remain fixed and service-to-
service traffic continues to use Compose DNS and the internal port. The
validator uses `SOLANA_RPC_PORT` and `SOLANA_FAUCET_PORT` in addition to the
existing service-specific variables such as `API_PORT`, `REDIS_PORT`, and
`EDGE_PORT`.

The bootstrap command does not auto-select random ports. It prints the
resolved UI, API, and validator endpoints, while Compose remains the
authoritative bind/readiness mechanism. A port collision is still a non-zero
failure and leaves the named project available for inspection; operators may
retry with an explicit, recorded port map.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Keep fixed ports and require stopping other stacks | Rejected | Disrupts unrelated work and makes project-name isolation misleading. |
| Auto-select random host ports | Rejected | Weakens reproducibility, makes wallet/browser configuration unstable, and complicates scripts and support. |
| Bind all services to host networking | Rejected | Removes network isolation and increases collision/security risk. |
| **Explicit environment-backed host-port overrides** | **Selected** | Preserves stable defaults, supports parallel isolated topologies, and keeps internal service contracts unchanged. |

## Invariants and failure policy

- Host overrides affect only published host bindings; internal ports and
  service DNS names do not change.
- The default port map remains stable and documented.
- A requested host port must be an explicit operator choice; the bootstrap
  must never silently bind a different port.
- Compose health checks are still required before bootstrap reports success.
- Validator RPC and faucet ports are independently configurable because the
  faucet is an externally exposed localnet authority.
- Port overrides are local-development controls and do not replace network
  policy, TLS, authentication, or production ingress controls.

## Consequences

Parallel local stacks can coexist when their complete port maps are distinct.
The developer must record the chosen environment variables when using a
non-default map, and browser/API clients must use the printed endpoints.
This is a local operability improvement; it does not close Linux/amd64 Agave
capability, conformance, security, or production-provider gates.
