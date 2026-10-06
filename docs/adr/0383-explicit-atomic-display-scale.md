# ADR-0383: Explicit atomic display scale at the browser boundary

- Status: Accepted
- Date: 2026-10-05
- Owners: Product UI, Tax Engine, Settlement

## Context

The protocol carries financial values as canonical atomic-unit strings. The
browser formatter previously supplied a default of two decimal places when a
caller omitted the asset scale. That default was convenient, but it made a
missing display contract indistinguishable from a deliberate two-decimal
currency policy. The error is especially dangerous for Solana assets whose
mint precision may be zero, six, or nine decimals.

The settlement checkout already receives finalized server-owned
`mint_decimals`. Tax and document quote workspaces needed an authoritative
scale rather than an accounting-currency display convention; ADR-0384 carries
that scale through the versioned facts and result contract.

## Decision

Make `formatAtomic` require an explicit `atomicDecimals` argument and reject
values outside `0..=9`. Route-bound checkout formatting uses finalized
`mint_decimals`; quote and document formatting uses the authoritative result
scale defined by ADR-0384.

The formatter remains presentation-only: it parses atomic strings with
`BigInt`, never converts through a JavaScript `Number`, and never changes the
wire value, tax calculation, attestation, or settlement authority. Missing
scale is a programmer/configuration error and must be caught by tests and
TypeScript rather than silently rendered.

## Alternatives considered

| Alternative | Missing-scale safety | Solana precision | Migration cost | Decision |
| --- | --- | --- | --- | --- |
| Keep an implicit two-decimal default | Fail | Fail | Low | Rejected |
| Infer precision from the currency symbol or locale | Partial and browser-dependent | Fail for arbitrary mints | Medium | Rejected |
| Fetch mint metadata from RPC during rendering | Partial | Better, but adds latency and an authority race | High | Rejected |
| **Require explicit scale; use server-owned route precision** | **Pass** | **Pass for bound assets** | **Low and bounded** | **Selected** |

## Invariants and failure policy

- Every formatter call supplies an explicit integer scale in `0..=9`.
- Route-bound settlement values use only finalized server-owned mint precision.
- Tax/document quote presentation uses the server-returned result scale and
  does not imply asset admission or settlement authority.
- Invalid atomic strings, currency labels, or scales fail closed.
- Formatting never changes canonical evidence, signed bytes, or identifiers.

## Verification

- `apps/web/src/formatters.test.ts` proves explicit scale behavior, large
  `BigInt` values, and invalid-scale rejection.
- TypeScript compilation ensures production call sites cannot omit the scale.
- `make web-browser-check`, `make web-color-contract`, and the full `make
  check` remain required.

## Consequences

The UI cannot silently display a value with an accidental precision. The
quote/document surfaces are bound to explicit result metadata, while the
settlement surface remains bound to finalized chain-route evidence.
