# ADR-0384: Authoritative atomic scale in the quote contract

- Status: Accepted
- Date: 2026-10-05
- Owners: Tax Engine, Settlement, Product UI

## Context

ADR-0383 removed implicit formatter precision, but left quote and document
surfaces on a named two-decimal convention. That is safe only for accounting
currencies whose scale is known to be two. A Solana route can use zero, six, or
nine mint decimals, and a quote must be displayable and settleable without
guessing. A client-selected display value also cannot be allowed to become the
authority for a settlement asset.

## Decision

Add required `atomic_decimals` metadata to `TaxFacts` and
`TaxDocumentFacts`, bounded to `0..=9`. Include it in the canonical facts
digest and carry it into `CalculationResult` and `TaxDocumentResult`. The
server-owned finalized settlement route remains authoritative: every
quote-to-settlement compiler rejects a scale mismatch before producing an
instruction or payment intent.

The browser sends an explicit scale for simulations and quotes and renders
the scale returned by the server-owned result. The field is metadata about the
atomic unit; it does not alter arithmetic, Solana instruction bytes, or tax
authority. Existing persisted rows without the field are legacy evidence and
must fail closed when decoded for a new settlement or display operation; no
implicit two-decimal migration is permitted.

## Alternatives considered

| Alternative | Precision safety | Backward compatibility | Settlement authority | Decision |
| --- | --- | --- | --- | --- |
| Keep the named two-decimal convention | Fail for non-2-decimal mints | High | Ambiguous | Rejected |
| Infer from ISO currency or locale | Partial and browser-dependent | Medium | Not bound to route | Rejected |
| Read mint metadata during rendering | Partial; adds RPC latency and races | Medium | Late and non-atomic | Rejected |
| **Require bounded scale in facts/results and compare with the finalized route** | **Pass** | **Explicit fail-closed migration** | **Pass** | **Selected** |

## Invariants and failure policy

- `atomic_decimals` is an integer in `0..=9` at every trusted boundary.
- A facts digest changes when the scale changes.
- Calculation replay preserves the exact scale.
- A quote/document result cannot compile against a route with another mint
  precision.
- Missing, malformed, or legacy scale metadata never falls back to two.
- Formatting uses the returned result scale and never converts through a
  JavaScript `Number`.

## Verification

- Rust domain tests cover validation, canonical binding, serialization,
  calculation results, and replay.
- Browser boundary tests reject missing/out-of-range scale and verify display
  calls use the result scale.
- API tests cover route mismatch fail-closed behavior.
- Full workspace, browser, accessibility, and public documentation gates are
  required before release.

## Consequences

Quote payloads become slightly more explicit and old incomplete evidence is
not silently reinterpreted. The UI can accurately present arbitrary supported
Solana mint precision, while settlement retains a single server-owned route
authority. A future currency metadata registry may prefill the field, but it
cannot bypass route comparison.
