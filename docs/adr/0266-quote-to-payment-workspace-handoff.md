# ADR-0266: Quote-to-payment workspace handoff

## Status

Accepted. This decision governs the browser handoff between a prepared quote
and the payment-review workspace.

## Context

A quote is a durable, server-owned record, while the browser owns only a
short-lived view of the prepared response. The operator must be able to move
from Tax quotes to Payment review without re-entering the quote or accidentally
losing the server response. Payment review must then re-check expiry and all
server-controlled settlement prerequisites before any wallet action.

Clearing every result whenever the operator leaves the quote workspace breaks
that workflow and can turn a valid quote into an unexplained empty checkout.
Retaining arbitrary workspace state, however, risks stale or unrelated data
being presented as settlement authority.

## Decision

The application shell retains the parsed server response only across the
bounded quote/payment handoff cluster:

- Tax quotes → Payment review preserves the prepared quote.
- Payment review → Tax quotes preserves the quote so the operator can correct
  facts or restart deliberately.
- Leaving that cluster clears the aggregate quote result from the browser view.
- Document quote → Document payment uses its separate calculation-ID handoff
  and never reuses aggregate quote state.
- Checkout still loads server-owned routes, validates the quote expiry, and
  compiles a fresh durable intent. Browser state never authorizes token movement.

The server remains authoritative for quote persistence, expiry, authorization,
route activation, signed-message admission, and finalized chain evidence.

## Alternatives considered

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Clear all state on every navigation | Simple lifecycle and low memory | Breaks the primary quote-to-payment journey and loses operator context | Rejected |
| Keep every workspace result globally | Smooth navigation | Stale cross-context facts can appear authoritative and memory grows with the shell | Rejected |
| Encode the full quote in the URL | Reloadable and shareable | Leaks financial context into history/telemetry and creates a second client transport contract | Rejected |
| **Retain only the bounded aggregate quote/payment handoff** | Preserves the real operator journey, limits stale state, and keeps authority server-side | Requires explicit transition tests | **Selected** |

## Acceptance criteria

1. A prepared quote remains visible when Payment review opens.
2. An expired quote is visibly marked expired and compilation/signing remains
   unavailable.
3. Leaving the aggregate quote/payment cluster clears aggregate quote state.
4. Document quote/payment state remains isolated from aggregate quote state.
5. Reload, replay, authorization, and finalized-chain behavior remain server
   owned.

## Verification

- The real-browser expired-quote journey proves the handoff and disabled
  compilation state.
- Existing checkout, document-payment, expiry, and wallet-boundary tests remain
  green.

