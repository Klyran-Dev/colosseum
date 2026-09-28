# ADR-0094: Governed Split Plan checkout review

- Status: Accepted
- Date: 2026-09-27
- Owners: Checkout / Product Security
- Scope: Discovering and reviewing the authenticated active Split Plan before wallet signing

## Context

The API can compile a Split Plan by an authenticated ID, but the checkout UI
currently exposes only the legacy single-destination flow. A caller or UI
that guesses a plan ID cannot provide a supportable review, and showing
recipient destinations without server resolution would create a second source
of authority. Production checkout must make the selected governed plan
explicit before a wallet signing prompt.

## Decision

Add a tenant-scoped, bounded read endpoint:

`GET /v1/settlement/split-plans`

The endpoint:

- requires `tax:settlement:compile` and derives tenant scope from the
  authenticated principal;
- returns only active, currently effective plans, ordered by plan key,
  descending version, and UUIDv7 ID, with a maximum of 64 rows;
- recomputes and validates each persisted canonical digest before serialization;
- returns recipient account, basis points, and plan digest as review evidence;
- never accepts a merchant ID, recipient list, amount, or digest from the
  browser.

Checkout adds an explicit settlement mode: `Single destination` or `Split Plan`.
Split mode requires a loaded active plan and displays the complete bounded
recipient allocation, plan version, digest, quote gross/tax/net, and route
compatibility before enabling compilation. The selected plan ID is sent to the
existing quote-bound split compiler; recipients and amounts remain server-owned.
The browser validates the untrusted list response again: maximum 64 plans,
valid Solana public keys, exactly 32 digest bytes, one-to-eight unique
recipients, and a 10,000 basis-point conservation invariant. Invalid response
shape fails closed and cannot enable compilation.

Every load and compile operation has loading, empty, stale, error, retry, and
disabled states. Changing route, quote, or plan invalidates the compiled
artifact and requires a fresh review. A failed or expired plan load never
falls back to a caller-supplied recipient list or legacy success state.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Accept recipient JSON from the browser | Rejected | Allows destination substitution and bypasses governance. |
| Put plan fields on settlement routes | Rejected | Couples independent lifecycle/versioning and complicates effective selection. |
| Fetch a plan by opaque ID only | Rejected | Provides no discoverable review surface and encourages guessed identifiers. |
| Redis-only plan read model | Rejected | Cache is disposable and cannot validate canonical evidence. |
| **Authenticated bounded PostgreSQL list plus existing ID-bound compiler** | **Selected** | Keeps tenant scoping, canonical validation, and chain-bound compilation in the server authority while giving checkout complete review evidence. |

## Invariants and failure behavior

1. The browser cannot authorize a plan that was not returned by the authenticated
   list or accepted by the quote-bound compiler.
2. The selected plan must match the selected route's merchant and mint before
   compilation.
3. The exact plan ID/version/digest and server-owned quote amounts are bound to
   the durable intent and compiled instruction.
4. Route, quote, plan, policy, or wallet changes invalidate the prior artifact.
5. Empty, stale, inaccessible, or integrity-invalid plans fail closed and are
   rendered as actionable product states.

## Budgets and evidence

The list is capped at 64 plans, uses one bounded PostgreSQL query, and has the
same API body/authentication and request correlation controls as route listing.
Evidence is a storage/API tenant-isolation test, TypeScript state/validation
tests, API contract documentation, and a Docker integration assertion for the
active-plan list.
