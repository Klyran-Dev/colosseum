# ADR-0274: Aggregate tax workspaces require an explicit currency

- Status: Accepted
- Date: 2026-10-03
- Owners: Product, Tax, Payments

## Context

The Tier-1 Tax Engine and API support a currency-bound calculation and an
optional cross-currency evidence path. The aggregate quote and scenario UI,
however, sent `USD` without asking the operator. That creates a hidden
financial assumption and makes a global tax product appear broader than its
actual browser contract.

## Decision

The aggregate quote and scenario workspaces show a required `Currency` field.
The value is trimmed, upper-cased at the boundary, length-bounded, and sent as
the calculation currency. The browser never chooses a currency from a hidden
default. The API remains authoritative for supported currencies, precision,
FX evidence, and settlement-route compatibility.

Document quote and other workflows that already expose currency retain their
existing explicit field. Display formatting continues to use the currency
returned by the server, never the submitted browser value.

## Alternatives considered

| Alternative | Result |
| --- | --- |
| Keep `USD` as a hidden default | Rejected: silently changes the financial meaning of the request. |
| Infer currency from browser locale | Rejected: locale is a display preference, not a settlement or tax fact. |
| Accept currency only from a client-side route | Rejected for the quote boundary: route selection is not yet available and client state cannot be an authority. |
| **Require explicit currency and let the server validate support** | Selected: honest input, global scope, and one authoritative compatibility boundary. |

## Invariants and acceptance criteria

- No aggregate tax request is sent with a browser-invented or hidden currency.
- Currency is normalized to a bounded uppercase code before submission.
- Empty or oversized currency input is rejected by browser validation/API
  contract before calculation.
- The result is formatted using the server-returned currency.
- Tests and browser journeys cover explicit currency entry and empty initial
  state.

