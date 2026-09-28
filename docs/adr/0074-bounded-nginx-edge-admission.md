# ADR-0074: Bounded NGINX edge admission control

- Status: Accepted
- Date: 2026-09-26
- Owners: Platform / Security / SRE
- Scope: Public HTTP edge before the Rust API

## Context

The API already rejects oversized JSON bodies and performs authenticated,
tenant-scoped authorization. Those controls do not prevent a single source
from consuming all NGINX worker connections or creating a request burst that
exhausts API, database, or RPC budgets. Edge admission must therefore bound
resource consumption before a request reaches the application.

The edge is not an authority for tax, settlement, identity, or idempotency.
Its limits are defensive capacity controls and must fail closed with an
explicit retryable response. Tenant quotas and abuse decisions remain in the
authenticated control plane, where merchant identity and product policy are
available.

## Decision

Configure the NGINX edge with:

- a per-source-IP request-rate zone with a bounded burst and `429` responses;
- a per-source-IP concurrent-connection zone with an explicit maximum and
  `429` responses; and
- config validation during the immutable web-image build.

The initial local baseline is `20` requests/second with a burst of `40` and
`100` concurrent connections per source IP. These are deployment defaults,
not universal production quotas: production values must be calibrated from
load, NAT concentration, IPv6 behavior, bot traffic, and API/database budgets.
The edge forwards the request ID and does not retry POST requests.

## Alternatives considered

- **API-only rate limiting**: rejected as the sole control because abusive
  traffic still consumes NGINX sockets, TLS/proxy resources, and application
  parsing capacity before tenant identity is available.
- **Redis-backed distributed edge quotas**: rejected for the first boundary
  because it adds a network dependency and tail latency to every request and
  makes local failure behavior harder to reason about. It may be added for
  tenant quotas behind the authenticated API boundary.
- **Cloud-provider WAF/CDN-only controls**: rejected as the only control
  because the local and self-hosted NGINX deployment must remain safe and
  reproducible without a vendor-specific perimeter.
- **Unbounded queueing**: rejected because queueing converts overload into
  latency and can exhaust downstream pools; excess work must be rejected.

## Invariants and failure behavior

1. The limit applies before proxying to the API.
2. A rejected request cannot create a quote, settlement intent, outbox event,
   tax calculation, or chain transaction.
3. A `429` response is observable through NGINX access logs and remains safe to
   retry only when the caller's operation is idempotent.
4. Edge limits never authorize a request and never establish settlement
   finality.
5. The NGINX configuration must pass `nginx -t` in the image build.

The Docker integration harness also sends a concurrent synthetic burst to the
edge and requires at least one `429`, proving that admission occurs before the
API path is allowed to process the entire burst. This is a deterministic
local guard, not a production capacity or fairness result.

## Consequences

This reduces overload amplification and makes local behavior closer to a
production edge. Per-IP limits can penalize large NATs or mobile networks, so
production rollout requires measured tuning and a tenant-aware quota layer.
The production readiness gate remains open until rate-limit behavior is load
tested, monitored, and reviewed with the deployment's source-IP model.
