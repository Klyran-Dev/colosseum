# ADR-0336: Bounded route-verification operator evidence view

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal Low-Latency Blockchain Architecture, SRE, Operations
- Scope: operator recovery workspace and read-only route-verification evidence

## Context

The route verifier persists bounded run records for scheduled checks and
approval-bound backfills. Those records are operational evidence, not a source
of settlement authority. The current runbook provides a SQL query, but a
production operator should not need database access to answer whether a
verification pass completed, whether routes remain unavailable, or which
approved backfill reference produced the evidence.

The view must not expose system-wide route evidence to merchant users, must not
allow route activation or mutation, and must not make a database row appear to
prove finalized Solana state. It must remain outside quote, compile,
wallet-signing, and settlement-submit paths.

## Decision

Add a bounded read-only route-verification evidence section to the existing
system-operator recovery workspace. The API exposes the newest 1–50 run rows
through an authenticated endpoint requiring the existing indexer operator
authority. The browser loads the rows only after an explicit operator action;
it does not poll or add a navigation item.

The response exposes the run identity, kind, lifecycle status, bounded counts,
failure code, timestamps, and approval reference when present. The storage
parser validates UUIDv7 identities, allowed lifecycle values, bounded counts,
terminal count conservation, required completion timestamps, and the
scheduled/backfill approval invariant before values enter the API response.

The UI labels this as operational evidence and keeps the exact identifiers and
protocol details behind the existing evidence-oriented surface. It explicitly
states that `completed` describes the verifier process only; route activation
still requires finalized account evidence and the settlement program remains
authoritative for funds.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Direct SQL/runbook only | No application work | Requires database access, weakens least privilege, and is not a supportable SaaS workflow | Rejected |
| New route-management workspace | Discoverable and extensible | Adds navigation clutter and risks mixing provisioning with merchant workflows | Rejected |
| Merchant-scoped route list | Familiar UI pattern | Run records are system-wide and could leak cross-tenant operational data | Rejected |
| **Bounded disclosure in the existing operator workspace** | Reuses operator authority, no new nav item, explicit read-only boundary, low latency | Adds one endpoint, parser, and evidence section | **Selected** |
| Live polling dashboard | Fresh display | Adds unnecessary RPC/database load and can imply live finality; operators can explicitly reload | Rejected |

## Invariants and failure behavior

- Only the existing system operator authorization boundary may read the view.
- The query is bounded to at most 50 rows and uses stable `started_at, id`
  ordering.
- The API never accepts a merchant identifier or route account from the
  browser.
- Empty, unavailable, unauthorized, malformed, and timed-out responses remain
  distinct; no cached or fabricated evidence is rendered.
- A successful read never activates a route, authorizes a transaction, or
  claims finalized chain state.
- A `running` row remains operationally incomplete. A
  `completed_with_failures` or `failed` row requires remediation evidence
  before paid-state enablement.
- The browser keeps UUIDv7 IDs and protocol counters as strings.

## Latency and resource budget

- One bounded PostgreSQL query, no Solana RPC, and no external provider call.
- Request deadline remains the shared browser/API default.
- At most 50 rows and bounded evidence fields are returned per request.
- The endpoint is not on any financial hot path and is not polled.

## Verification

- Write parser and invariant tests before the storage/API implementation.
- Add authenticated API tests for exact operator role, missing credentials,
  bounded limits, and malformed/terminal evidence.
- Add React tests for explicit loading, empty, unavailable, unauthorized, and
  read-only evidence states.
- Add a real-browser journey proving the disclosure does not load until asked,
  rejects unauthorized access, and never presents a route as activated merely
  because a run row was read.
- Add an authenticated Docker integration assertion that seeds one synthetic
  terminal run in PostgreSQL, reads it through the API, verifies the bounded
  response and failure classification, and rejects an out-of-range limit.
- Update the runbook, requirements traceability, and public review guide.

## References

- [ADR-0328: Durable route-verification run evidence](0328-route-verification-run-evidence.md)
- [ADR-0329: Approval-bound route-verification backfill mode](0329-approved-route-verification-backfill-mode.md)
- [Settlement-route verification runbook](../runbooks/settlement-route-verification.md)
- [Architecture review guide](../review/architecture-review-guide.md)
