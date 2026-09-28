# ADR-0128: Tenant-scoped vault-reconciliation operator read model

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Finance/Tax, Security, and UX
- Scope: authenticated read-only API and operator workspace for ADR-0127

## Context

ADR-0127 provides scheduled, append-only vault reconciliation, but a durable
control plane is incomplete if operators cannot inspect target identity,
opening-balance provenance, latest finalized observation, and run history. The
browser must not become an authority for opening balances, expected amounts,
merchant scope, or chain finality.

Target provisioning requires a separately authorized route-verification and
opening-balance ceremony. It must not be exposed as a browser form that accepts
an amount and calls it verified.

## Decision

Add two authenticated, tenant-scoped read endpoints:

- `GET /v1/vault/reconciliation/targets?limit=...`
- `GET /v1/vault/reconciliation/targets/{target_id}/runs?limit=...`

Both derive `merchant_id` from the authenticated principal, use bounded limits
of 1 through 64, and require `tax:vault:read` plus an existing governed vault
read role. A caller-supplied merchant identifier is not accepted. The API
returns server-owned route/account identity, opening-balance reference,
projection aggregates, latest run status, exact decimal-string atomic values,
observation slot, and bounded append-only history.

The React workspace provides loading, empty, stale, matched, mismatch, error,
timeout, unauthorized, and retry states. It renders mismatch/error as
operational stop signals and links operators to the incident runbook. It does
not offer target creation, balance editing, release execution, or a fake
“healthy” fallback when the API is unavailable. Access tokens remain
memory-only and every response is runtime-validated before rendering.

## Alternatives considered

### Browser target-creation form

Rejected. A browser-supplied opening balance cannot prove a finalized chain
observation and would create an unsafe authority boundary.

### Global, system-wide reconciliation dashboard

Rejected for this API boundary. It risks cross-tenant disclosure and would
require a separately governed system-operator projection.

### Logs only

Rejected. Operators need bounded, authenticated, replayable evidence and a
support reference in the product surface.

### **Tenant-scoped read model with immutable server-owned values**

Selected. It provides operational visibility without adding a settlement or
provisioning authority to the browser and preserves low-latency service
boundaries through bounded indexed queries.

## Invariants and failure behavior

- Every target and run is returned only under the authenticated merchant
  boundary.
- Atomic amounts and Solana slots are decimal strings in JSON.
- Missing, malformed, stale, or contradictory API data is rejected by the
  browser parser rather than displayed as a plausible status.
- A latest mismatch/error is never rendered as matched, and an empty target
  list is not rendered as “all clear.”
- API timeout, 401/403, 5xx, and malformed payloads retain explicit recovery
  states and never retry a mutating operation because these endpoints are
  read-only.
- The UI cannot mutate targets, runs, claims, checkpoints, or funds.

## Evidence and remaining gates

Rust API authorization and storage tests, TypeScript parser/component tests,
accessibility assertions, and Docker integration coverage are required.
Production identity governance, scrape/alert delivery, target-provisioning
ceremony, and a separate system-wide finance/auditor view remain distinct
release decisions.
