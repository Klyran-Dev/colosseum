# ADR-0098: Fail-closed browser recovery for durable settlement intents

- Status: Accepted
- Date: 2026-09-27

## Context

The non-custodial checkout creates a durable UUIDv7 settlement intent before a
wallet signs. A provider timeout after the signed bytes may have reached an RPC
provider is intentionally represented by the API as `unknown`; the user must
reconcile that intent rather than sign or submit a second transaction.

The browser previously handled a failed submit request as a generic error. It
could lose the intent status from the screen and return to a state that allowed
another signing attempt. Direct TypeScript casts of status responses also left
the UI without a runtime contract for server-owned lifecycle data.

## Decision

The checkout client will:

1. validate every settlement-intent response against a bounded status,
   UUIDv7 intent ID, optional signature, and optional non-negative provider
   index before rendering it;
2. treat a submit request as an irreversible observation boundary once the
   signed bytes have been handed to the API;
3. fetch the durable intent after an unsuccessful submit response and display
   the server-owned state;
4. if the follow-up status request also fails, retain the intent ID locally in
   the active screen, display `unknown`, disable signing, and instruct the user
   not to retry authorization; and
5. permit another attempt only when the authoritative intent remains in
   `prepared_unsubmitted` or `signed` and the API has explicitly returned that
   state.

The client never infers finality from a wallet callback, HTTP success alone, or
the presence of signed bytes. The API and finalized chain evidence remain the
authorities for settlement state.

## Alternatives considered

| Alternative | Decision |
| --- | --- |
| Reset the form after any submit error | Rejected: it permits duplicate signing after an unknown provider outcome. |
| Keep the old response via an unchecked TypeScript cast | Rejected: malformed or broadened lifecycle data could create unsafe UI actions. |
| Poll indefinitely in the browser | Rejected: unbounded work and hidden network load violate the latency and cancellation mandates. |
| Store signed transaction bytes in browser local storage | Rejected: it increases sensitive-data exposure and is not needed to preserve the durable intent boundary. |
| Recover the intent from the server and fail closed when recovery is unavailable | Selected: preserves the intent identity, avoids duplicate authorization, and keeps chain truth server-owned. |

## Invariants and evidence

- No wallet signing control is enabled after an uncertain submission unless the
  server explicitly reports a retryable prepared/signed state.
- A status response with an invalid UUIDv7, lifecycle status, signature, or
  provider index is rejected before rendering.
- A submit timeout never clears the intent ID from the active checkout.
- The browser does not claim `confirmed` or `finalized` without the API's
  corresponding persisted evidence.
- Unit tests cover response-shape rejection and all recovery branches.

Evidence: `apps/web/src/checkout.ts`, `apps/web/src/CheckoutPanel.tsx`, and
`apps/web/src/checkout.test.ts`.

