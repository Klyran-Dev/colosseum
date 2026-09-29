# ADR-0196: Client quote-expiry boundary

## Status

Accepted — client-side prevention is implemented; server-side expiry remains
the authority and browser/live-provider evidence remains a separate gate.

## Context

Settlement compilation and submission are bound to a persisted, server-owned
quote. The API already rejects expired quotes, but the checkout UI previously
left its compile/sign path available until an API round trip failed. That
creates an avoidable wallet-prompt and retry hazard at the exact point where a
quote becomes stale. The browser clock is untrusted and may be skewed, so it
cannot authorize a quote or replace the server check.

## Decision

Add a small pure UI boundary that treats a quote as expired when its explicit
Unix expiry is less than or equal to the observed current Unix second. Refresh
that observation once per second while an expiry is present. The checkout
must:

- reject compile attempts locally once the quote is expired;
- disable compile and sign controls while the quote is expired;
- retain an explicit accessible expiry message that tells the operator to
  create a new quote; and
- continue sending the quote identity to the API, which remains the
  authoritative expiry and lifecycle boundary.

The client does not extend expiry, infer validity from a successful render, or
persist financial state locally. A skewed client clock can cause an early
wallet-action refusal, but cannot make an expired quote settle; a fast clock
is therefore safer than prompting a signature for a stale quote.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Rely only on API rejection | One authority and no client clock logic | Allows stale compile/sign prompts and poor recovery UX | Rejected |
| Client-only expiry | Immediate UX and fewer requests | Unsafe; cannot authorize financial validity | Rejected |
| **Client guard plus server authority** | Prevents stale wallet prompts while preserving authoritative validation | Small timer and clock-skew behavior must be documented/tested | **Selected** |
| Disable only after a failed request | Minimal UI change | Race between expiry and wallet prompt; ambiguous recovery state | Rejected |

## Invariants and verification

- Missing expiry does not become an invented expiry.
- The exact expiry second is expired (`expiry <= observed_now`).
- No wallet signing call is made through the expired UI path.
- API/server expiry validation remains mandatory and unchanged.
- Pure boundary tests cover missing, future, exact, and past expiry values.
- The pinned NGINX Playwright suite covers the rendered expired-quote alert,
  disabled compile control, wallet rejection, and WCAG A/AA audit; the
  route/provider fixtures in those browser tests are synthetic.
- Browser accessibility and live-provider evidence remain required before
  production enablement.
