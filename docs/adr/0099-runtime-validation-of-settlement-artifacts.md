# ADR-0099: Runtime validation of browser settlement artifacts

- Status: Accepted
- Date: 2026-09-27

## Context

The browser receives settlement routes and compiled instruction envelopes from
the authenticated API. TypeScript interfaces disappear at runtime, and a
server, proxy, stale tab, or compromised response can return malformed account
keys, unsupported token programs, duplicate routes, invalid digests, or an
instruction envelope that does not satisfy the Solana settlement contract.

The signing boundary already performs defense-in-depth account and instruction
validation. Accepting unvalidated response objects before that boundary still
creates misleading review UI and makes route selection depend on unchecked
data.

## Decision

Add bounded runtime parsers in the browser settlement module:

- `parseSettlementRoutes` validates UUIDv7 route identities, bounded route
  metadata, canonical Solana public keys, supported token programs, and unique
  route IDs.
- `parseCompiledSettlementArtifact` validates the durable intent and digest
  envelope, exact response variant, canonical base64, account metadata, bounded
  collection sizes, and the same instruction invariants used immediately
  before wallet signing.

`CheckoutPanel` renders and stores only parser output. The API remains the
authority; browser parsers are not a substitute for server-side message
verification. A malformed response fails closed with an explicit retry/error
state and never enables signing.

## Alternatives considered

| Alternative | Decision |
| --- | --- |
| Trust TypeScript interfaces and validate only at signing | Rejected: malformed responses can still corrupt review UI and route selection. |
| Use a schema-generation runtime dependency | Rejected for this bounded surface: it adds dependency and bundle cost where explicit parsers provide stricter Solana-specific invariants. |
| Duplicate validation only in the React component | Rejected: it would be hard to unit-test and would couple domain serialization to rendering. |
| Share the pure parser with Rust through generated WebAssembly | Rejected for the current hot path: it adds startup and supply-chain complexity; cross-language canonical vectors remain the contract. |

## Invariants and evidence

- Route lists are bounded, use supported token programs, and contain no
  duplicate UUIDv7 identities.
- Compiled artifacts contain exactly one supported settlement variant, a
  non-null UUIDv7 intent, 32-byte request/instruction digests, canonical
  account metadata, and the expected instruction discriminator/length.
- Parser rejection leaves the wallet signing control disabled.
- The server repeats all authority-sensitive checks on the signed message.
- Unit tests cover malformed route fields, duplicate routes, invalid compiled
  artifacts, and valid shared-vector artifacts.

Evidence: `apps/web/src/checkout.ts`, `apps/web/src/CheckoutPanel.tsx`, and
`apps/web/src/checkout.test.ts`.

