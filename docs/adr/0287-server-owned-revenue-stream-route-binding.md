# ADR-0287: Server-owned revenue-stream settlement-route binding

- Status: Accepted
- Date: 2026-10-04
- Owners: Settlement, Treasury, and API Architecture

## Context

The Partner payouts workflow can create an off-chain plan before a chain
instruction is prepared. A plan that stores only an asset and recipient is not
enough to compile a safe Solana instruction: the merchant account, tax vault,
token program, verified mint precision, and route version are authority-
sensitive server configuration. Accepting those accounts from the browser
would allow route substitution and would make a database plan ambiguous.

## Decision

When a plan is proposed, the API resolves exactly one active, effective,
finalized settlement route for the authenticated tenant and requested asset.
The route UUIDv7 is persisted in `revenue_stream_plans` and is protected by a
tenant-scoped foreign key. The plan request digest includes the resulting
route-bound plan specification. The browser never submits route IDs, merchant
accounts, vaults, token programs, or program IDs for this workflow.

Missing, suspended, expired, ambiguous, or unverified routes fail closed with
an unavailable/conflict response before plan persistence. Route activation and
on-chain topology verification remain separate gates; the route table is not
itself proof of current Solana account state.

## Alternatives considered

| Alternative | Strengths | Risks | Decision |
| --- | --- | --- | --- |
| Browser supplies route and accounts | Simple form/API shape | Tenant authority substitution and account redirection | Rejected |
| Plan stores only asset and resolves a route at claim time | Fewer plan fields | Route drift changes the meaning of an immutable plan | Rejected |
| Choose the newest matching active route | Convenient failover | Ambiguous authority and non-replayable behavior | Rejected |
| **Resolve one verified route at proposal and persist its UUIDv7** | Immutable replay, tenant binding, deterministic compiler input, no browser authority | Route rotation requires a new plan version | **Selected** |

## Invariants and verification

- `settlement_route_id` is UUIDv7 and references `(route_id, merchant_id)`.
- The route mint equals the requested plan asset.
- Exactly one active/effective route may match the tenant and asset.
- The route's merchant account, tax vault, token program, decimals, and
  verification evidence are server-owned and are not editable plan inputs.
- Reusing an idempotency key with a different route-bound request digest fails
  closed.
- Migration bootstrap, storage tests, API tests, and route ambiguity tests are
  required before a chain compiler endpoint can be enabled.

This ADR does not claim that a route row proves live chain state. The bounded
route-verifier workflow and Linux validator/live-RPC evidence remain required.
