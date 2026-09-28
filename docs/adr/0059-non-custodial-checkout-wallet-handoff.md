# ADR-0059: Non-custodial checkout wallet handoff

- Status: Accepted for implementation
- Date: 2026-09-26
- Deciders: Principal Low-Latency Blockchain Architecture
- Scope: React checkout surface, wallet boundary, settlement API handoff

## Context

The current React surface can calculate or persist a quote, but it stops before
the user can review and authorize the exact settlement instruction. A
production checkout must make the financial and policy consequences visible,
keep keys inside the wallet, and preserve a recoverable distinction between
wallet rejection, signed-but-not-submitted, submitted, and chain commitment.

The settlement API already accepts a deterministic compiled instruction and
cryptographically verifies the returned signed message. The client therefore
needs to construct exactly that artifact into a transaction, obtain a recent
blockhash, request a wallet signature, and return the unchanged signed bytes.

## Decision

1. Add a dedicated Checkout/Settlement Handoff workspace to the React SPA. It
   is an explicit product surface, not a simulated success screen.
2. Use the latest stable `@solana/web3.js` 1.x maintenance release for this
   slice because browser wallets commonly expose the `Transaction` and
   `signTransaction` compatibility boundary. The dependency is exact-pinned
   and reviewed as a wallet-interoperability choice; migration to the newer
   Kit API requires a separate compatibility test against every supported
   wallet provider.
3. Define a narrow browser wallet port with `connect` and `signTransaction`.
   The SPA never receives, derives, stores, or logs private keys. Wallet
   discovery is capability-based and unsupported wallets produce a recoverable
   state.
4. Construct a legacy Solana transaction containing the exact server-owned
   compute-budget prefix from ADR-0162, the compiled settlement instruction,
   the connected payer as fee payer, and a confirmed recent blockhash. No ATA
   creation, memo, swap, or implicit account substitution is permitted.
5. Render a pre-sign review containing gross, tax, net, asset mint, policy
   sequence/digest, program ID, account count, quote/intent identifiers, and
   the network/blockhash policy. The signing control is disabled until all
   required fields are valid and the review is visible.
6. After signing, serialize once to canonical base64 and submit the exact
   bytes plus the unchanged compiled artifact. The browser never reports
   success based on signing alone; it displays the server's durable status and
   signature evidence.
7. Every asynchronous boundary has explicit loading, cancellation/retry,
   wallet rejection, stale blockhash, authorization, RPC timeout, unknown,
   and reconciliation-required states. Tokens remain in memory only.

## Alternatives considered

### `@solana/kit` 8.x as the first wallet boundary

Kit is the current Solana application SDK and is the strategic direction for
new clients. It was not selected for this first handoff because the repository
must interoperate with existing injected browser wallets whose stable public
surface is still the web3.js `Transaction` object. A compatibility adapter can
be introduced after wallet-standard support is tested across the supported
provider matrix.

### Solana Wallet Adapter packages

Rejected for the first slice because they add a provider registry and UI
abstraction that is not needed for the narrow non-custodial port and would
increase bundle and lifecycle surface. The port can later be backed by an
adapter without changing the checkout state machine.

### Server-side transaction construction or signing

Rejected. The API must not custody payer keys or change a transaction after
the user reviews it. Server verification remains authoritative.

### Direct `sendTransaction` from the browser

Rejected as the sole path. It hides durable intent state and makes provider
ambiguity difficult to reconcile. The browser returns signed bytes to the
bounded server coordinator, which preserves evidence and failover semantics.

## Consequences

Positive:

- The user sees the exact money movement before wallet authorization.
- The web layer remains non-custodial and never fabricates chain state.
- The server's ADR-0058 message-binding verifier remains the final trust gate.
- Wallet/provider failures become supportable state transitions with intent IDs.

Trade-offs and release gates:

- The first implementation supports only wallets exposing the selected
  compatibility port and legacy single-instruction transactions.
- Merchant Solana-account mapping must be resolved from authenticated tenant
  configuration before non-test enablement; free-form account entry is not an
  authority boundary.
- Wallet matrix, accessibility, quote-expiry, localnet, and staging tests are
  required before production submission scope is enabled.
