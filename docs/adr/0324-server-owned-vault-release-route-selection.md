# ADR-0324: Server-owned vault-release route selection

- Status: Accepted
- Date: 2026-10-04
- Scope: Vault-release bounded context, API, storage, and operations UI

## Context

Vault-release proposals previously accepted the mint, token-program, and tax
vault addresses directly from the browser. That made the UI responsible for
selecting merchant-side settlement topology and allowed a caller to submit an
address set that was unrelated to the tenant's finalized route registry. The
browser may choose a beneficiary token account, but it must not choose the
merchant-owned accounts that define the release authority boundary.

## Decision

Add `GET /v1/vault/release-routes`, scoped to the authenticated merchant and
the exact vault-proposer scope/role. It returns only bounded active routes
whose verification schema is `settlement-route-verification/v2` and whose
effective window contains the database clock.

Change `POST /v1/vault/release-claims` to accept a UUIDv7 `route_id` and the
beneficiary destination, but no mint, token-program, or tax-vault fields. The
storage adapter resolves the route inside the serializable claim transaction,
checks active/effective status, supported token program, and decimal bounds,
rejects a destination equal to the resolved tax vault, and persists the
resolved account snapshot. The request digest is versioned to v2 and binds
merchant, route ID, resolved merchant-side accounts, beneficiary, financial
inputs, and evidence. Creation evidence also records the route ID.

The transaction locks the route row while resolving it. If the route is no
longer active or effective, the service rejects a new idempotency key but first
checks for an existing claim under that key. An exact replay returns the
persisted claim without creating lifecycle events or outbox records; a changed
request remains an idempotency conflict. This preserves recovery from a
successful response lost during a later route suspension.

The UI presents a bounded route selector and read-only route evidence. It has
no free-form merchant-side account inputs and cannot create a request while
route loading fails or no verified route is available. All account validation,
tenant authorization, route activation, and final chain verification remain
server/on-chain responsibilities.

## Alternatives considered

| Option | Decision | Reason |
| --- | --- | --- |
| Continue accepting account addresses from the browser | Rejected | Client-controlled topology is an authority-boundary and confused-deputy risk. |
| Accept a route ID but resolve it before the transaction | Rejected | A route can change between lookup and persistence; it weakens the atomic binding. |
| Accept route ID and resolve it inside the serializable transaction | Selected | Tenant scope, lifecycle, account snapshot, and digest binding share one write boundary. |
| Add a new vault-specific route registry | Rejected | Duplicates finalized route verification and creates conflicting authorities. |

## Invariants

1. A claim request contains a UUIDv7 route ID and never accepts merchant-side
   account topology from the browser.
2. Route resolution is tenant-scoped and requires finalized route evidence,
   active lifecycle state, and an effective window.
3. The beneficiary is distinct from the resolved tax vault.
4. The stored request digest binds the route ID and resolved account snapshot.
5. No route lookup failure, stale route, malformed route, or missing route can
   create a new claim or produce a client-side success state.
6. An exact replay of an existing claim remains idempotent even if its route is
   subsequently suspended; no replay creates a second event or outbox record.
7. Finalized Solana evidence remains authoritative for token movement.

## Verification

- UI tests prove that the request body contains only `route_id` for the
  merchant-side topology and that route loading is required.
- Storage/API compilation and focused vault-release tests pass.
- The Docker integration harness suspends a verified route after claim
  creation, proves an exact retry returns HTTP 200 with the original claim,
  and proves lifecycle-event and outbox counts do not increase.
- The full API, web, accessibility, Docker, and browser gates remain required
  before release.

## Consequences

The route registry becomes the single operational entry point for release
topology. Route suspension blocks new proposals and requires operators to
select another active route. Existing claim snapshots remain auditable and
immutable, and an exact retry remains recoverable during suspension; this
change does not authorize movement or replace chain evidence.
