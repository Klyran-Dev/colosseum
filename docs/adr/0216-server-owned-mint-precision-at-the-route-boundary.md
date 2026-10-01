# ADR-0216: Server-owned mint precision at the settlement-route boundary

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Solana Protocol, Web Platform
- Bounded contexts: Settlement Route Registry, Checkout UI, Finalized Chain Verification

## Context

The finalized route verifier already reads the mint's decimals from the
verified Solana mint account and binds that value into the route verification
digest. The storage/API route projection discarded it, while the browser's
atomic formatter assumed exactly two decimal places. That can display a
zero-decimal asset or a six/nine-decimal token with a materially wrong amount
before wallet authorization.

Atomic values are authority-sensitive. A browser, currency symbol, or caller
must not choose the display scale for a wallet review. The scale must be
server-owned, derived from finalized route evidence, bounded, immutable for an
active route version, and available without an RPC call in the checkout hot
path.

## Decision

Persist `mint_decimals` on `settlement_routes` when a pending or suspended
route is activated from a valid finalized (or explicitly local-synthetic)
verification. The value is the verifier's mint-account value and is bounded to
`0..=9` by the route contract. Active, suspended, and retired route versions
cannot mutate it; a corrected precision requires a new route version and fresh
chain verification.

Expose `mint_decimals` in the authenticated active-route response. The
browser validates the bounded integer and uses it only for presentation; it
never supplies or changes the precision used by compilation, attestation, or
on-chain movement. The API and Solana program continue to derive financial
authority from the persisted quote, route, attestation, and exact atomic
values.

The formatter accepts an explicit bounded decimal count and uses `BigInt`
scaling without converting atomic amounts through a JavaScript `number`.
Legacy tax-only and document-only views retain their explicit two-decimal
display default until those facts carry a server-owned asset scale; checkout
uses the selected verified route scale and labels the asset mint/precision in
the authorization review.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Assume two decimals everywhere | Rejected | Wrong for Solana assets and can misstate the amount shown before signing. |
| Let the browser or caller submit a decimal count | Rejected | Creates a presentation/authority substitution boundary and is not evidence-bound. |
| Fetch mint metadata from RPC during checkout | Rejected | Adds a hot-path dependency, creates stale/provider disagreement risk, and duplicates route verification. |
| Parse untyped verification JSON on every route read | Rejected | Makes a critical financial field dependent on mutable JSON shape and adds repeated parsing work. |
| **Persist verifier-derived precision on the immutable route version** | **Selected** | Finalized evidence-bound, O(1) read, no checkout RPC, explicit migration, and compatible with bounded UI formatting. |

## Invariants and failure policy

- A route can become active only with a persisted `mint_decimals` in `0..=9`.
- The value must match the verifier output that is included in the route
  verification evidence and digest.
- Active, suspended, and retired route precision is immutable.
- Missing, malformed, or out-of-range precision fails route activation/read
  validation; the browser must not guess a scale.
- Formatting is presentation only and cannot alter the atomic strings sent to
  compilation or signing.
- `BigInt` is the only arithmetic representation used for browser amount
  scaling; JavaScript safe-integer conversion is prohibited.
- The route precision is not a secret, bearer credential, tax rule, or legal
  compliance determination.

## Verification and release impact

- The migration contract proves backfill from existing verifier evidence,
  database evidence binding, active-route constraints, and immutability guards.
- Rust storage/API compilation covers activation persistence, bounds, and route
  response serialization; the SQL verification contract proves database-side
  evidence binding and immutability. TypeScript boundary tests reject missing,
  fractional, negative, and out-of-range precision.
- Formatter tests cover zero, two, six, and nine decimals, large `u64` values,
  locale grouping, and RTL output without `Number` conversion.
- The browser checkout review renders the selected route's mint precision and
  formats gross/tax/net with that server-owned value.
- The authenticated Docker integration contract now asserts that the active
  route API preserves the server-owned precision before quote-bound settlement
  compilation proceeds.
- Validator, live-wallet, and production SLO evidence remain independent
  release gates.
