# ADR-0458: Bind server-owned payment display metadata to the settlement artifact

- Status: Accepted
- Date: 2026-10-10
- Owners: Product engineering, payments, and accessibility

## Context

The document-payment review artifact contains exact atomic amounts, but the
browser does not currently receive the authoritative currency, quote scale,
route identity, expiry, or rule version that explains those amounts. The UI
therefore has to label values as atomic units and cannot present a compact,
human-readable payment summary without risking a guessed conversion. The
checkout also needs to distinguish a priority-fee policy from an actual
network-fee estimate.

## Decision

Extend the versioned compiled document-settlement response with a required
server-owned display metadata object with an explicit schema identifier. It is assembled from the tenant-scoped
stored document quote, verified attestation, and finalized settlement route
already used to compile the instruction. It includes the quote currency and
atomic scale, rule identity/version, quote expiry, route key/version, mint,
mint decimals, token program, and compute-budget policy.

The browser validates the metadata before displaying human-unit amounts. It
may format exact amounts with integer arithmetic, but it must not infer a
currency, alter settlement values, or treat compute-budget policy as an exact
network-fee estimate. Until a provider-backed fee estimator is part of the
contract, the UI states that network fee is not estimated.

## Alternatives considered

1. Keep bare atomic amounts. Rejected because operators cannot quickly confirm
   the payment context and the UI remains unnecessarily dense.
2. Infer currency/scale from the selected route or browser state. Rejected
   because client state is not the authority for a quote-bound settlement.
3. Add a separate display-metadata request. Rejected because it creates a
   race and another consistency boundary between the reviewed artifact and
   what the operator sees.
4. **Bind metadata into the compiled artifact.** Selected because the display
   context is generated beside the exact instruction and can be replayed with
   the same quote, route, and attestation evidence.

## Acceptance criteria

- The API response contains bounded, server-owned display metadata for every
  document settlement artifact.
- The frontend rejects missing, malformed, or inconsistent metadata.
- Human-unit amounts are exact and locale-aware; no binary floating point or
  guessed currency is used.
- Quote expiry, rule version, route identity, and technical fee policy are
  available through progressive disclosure.
- Network-fee estimates are not claimed unless a provider-backed estimate is
  explicitly added to the contract.
- API, frontend, and Docker browser tests cover the contract and remain green.
