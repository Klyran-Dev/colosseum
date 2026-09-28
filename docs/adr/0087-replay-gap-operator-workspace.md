# ADR-0087: Replay-gap operator workspace

- Status: Accepted
- Date: 2026-09-26
- Owners: Chain Data / SRE / Web
- Scope: React operator surface for authenticated replay-gap recovery

## Context

ADR-0086 provides a system-level, authenticated replay-gap API. An operator
should not be required to use SQL or an undocumented HTTP client for a
dead-letter review. The web surface must remain an evidence view: it may show
server responses and request progress, but it cannot infer repair, checkpoint
advancement, chain finality, or authorization from local state.

## Decision

Add a lazy-loaded `Replay operations` workspace to the React/Vite SPA. The
workspace:

1. requires an in-memory bearer token, stream key, and explicit bounded status
   filter before requesting data;
2. parses every list/action response against a bounded TypeScript contract,
   including UUIDv7 IDs, slot/attempt bounds, status, timestamps, and optional
   acknowledgement evidence;
3. renders dead-lettered rows with acknowledgement and requeue actions only
   when the server has returned the corresponding state;
4. requires a bounded operator note for each mutation, disables duplicate
   submission while a request is active, and reloads the server projection
   after a successful action;
5. presents `409` state conflicts, authentication failures, timeouts, stale
   data, and service failures with the API request correlation reference when
   available; and
6. explicitly explains that acknowledgement is review evidence, requeue is
   projection repair work, and neither operation proves finalized settlement.

The browser does not decode JWTs, decide role membership, cache operator
secrets, or mark a gap repaired. The API remains the authorization and state
authority. Slots are normalized as decimal strings in the UI boundary so a
future API representation cannot silently lose precision.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| SQL-only operations runbook | Rejected | No browser audit context, weak operator ergonomics, and bypasses API authorization. |
| Generic JSON/admin console | Rejected | Allows accidental unbounded input and renders unvalidated state. |
| Always-visible workspace with cached gap state | Rejected | Can imply stale repair status and exposes an operational control without explicit scope context. |
| **Lazy-loaded, typed, authenticated operator workspace** | **Selected** | Preserves API authority, bounded input, accessible failure states, and evidence-first UX. |

## Invariants and budgets

- The UI never displays a locally invented gap or success state.
- Acknowledge is enabled only for a server-returned `dead_lettered` row;
  requeue is enabled only when the row is acknowledged by the server.
- Notes are 1–1024 UTF-8 code units in the browser and are bounded again by
  the API/storage boundary.
- List requests use a limit of at most 100 and a five-second deadline.
- One mutation per row is allowed at a time; no client retry may duplicate an
  action without a fresh server response.
- The lazy chunk must remain below 30 KiB gzip and introduce no new runtime
  dependency. UI interaction work is budgeted at p95 < 100 ms after response
  parsing on the reference development device.

## Evidence

- `apps/web/src/replayOps.ts` and its unit tests validate the response and
  request boundary.
- `apps/web/src/ReplayOpsPanel.tsx` renders the authenticated workflow.
- The Docker integration harness and API tests remain the authoritative
  server-side authorization/state evidence; browser tests cover the parser and
  action contract.
