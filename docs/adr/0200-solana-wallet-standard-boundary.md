# ADR-0200: Solana Wallet Standard signing boundary

## Status

Accepted — the compatibility slice is implemented while preserving the
existing server-owned web3.js transaction and signed-byte contract. Live
provider and production enablement evidence remain open.

## Context

The browser currently discovers a wallet through the legacy `window.solana`
global. That is useful compatibility evidence but is not a sufficient
production wallet boundary: it cannot enumerate multiple providers, does not
advertise chain or signing capabilities, and can silently select an unintended
wallet when several providers are installed. The product must remain
non-custodial and preserve the existing server authority: the browser reviews
and signs an immutable server-compiled transaction, then submits unchanged
signed bytes for server verification and durable chain-state reconciliation.

## Decision

Add an application-owned wallet port backed first by Wallet Standard and
second by the legacy injected compatibility adapter.

1. Discover standard wallets with `@wallet-standard/app` `getWallets()` and
   filter for a Solana chain plus the `solana:signTransaction` feature and
   `standard:connect` capability. Use exact-pinned packages:
   `@wallet-standard/app` 1.1.1, `@wallet-standard/base` 1.1.1,
   `@wallet-standard/features` 1.1.1, and
   `@solana/wallet-standard-features` 1.5.0.
2. Expose wallet descriptors containing a stable session-local selection key,
   display name/icon, supported chains, and only the capability needed by the
   product. If more than one compatible provider is present, the UI requires
   explicit user selection; the adapter never chooses the first implicitly.
3. Call `standard:connect` to obtain authorized accounts. If the wallet
   exposes more than one Solana signing account, surface a required account
   selector in the shared wallet component; never choose the first account.
   After connection, render the selected public key in the same wallet
   workspace so the operator can compare the active signer with the
   server-owned artifact before each role-signing action.
   Require the selected account's public key to equal the server-compiled
   signer, and call `solana:signTransaction` with serialized legacy
   transaction bytes and an explicit Solana chain identifier. Convert returned
   bytes back to the existing web3.js boundary and verify the expected signer
   before returning canonical base64.
4. Retain `window.solana` only as a compatibility fallback when no standard
   provider is registered. A discovered but incompatible standard provider is
   reported as unsupported rather than silently bypassed.
5. Keep the existing server-side message-binding verifier, immutable signed
   bytes, durable intent, and finality/proof state machine unchanged. Wallet
   discovery is never settlement authority.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Continue using `window.solana` only | Smallest bundle and compatible with existing tests | Provider ambiguity, no capability discovery, legacy-global coupling | Rejected |
| **Wallet Standard with legacy fallback** | Explicit multi-provider selection, chain/features checks, broad interoperability, preserves current transaction authority | Adds small dependencies and selection state | **Selected** |
| Solana Wallet Adapter registry | Mature provider adapters and UI ecosystem | Larger lifecycle/bundle surface and a second state abstraction for this narrow port | Rejected for this slice |
| `@solana/kit` wallet boundary | Strategic modern SDK direction | Still requires Wallet Standard interoperability and a broader compiler migration | Deferred behind compatibility evidence |
| Direct browser `sendTransaction` | Fewer server round trips | Hides durable intent and provider ambiguity; weakens exact-byte reconciliation | Rejected |

## Invariants and failure behavior

- No private key, wallet secret, or signed transaction is persisted or logged.
- No wallet is selected implicitly when multiple compatible providers exist;
  no account is selected implicitly when a provider exposes multiple signing
  accounts.
- Only a wallet advertising `solana:signTransaction` can sign this path;
  `signAndSendTransaction` is intentionally not used.
- The selected account, transaction bytes, program/account policy, and server
  intent remain bound before any submit request.
- Empty discovery, unsupported chain, missing feature, user rejection,
  account mismatch, malformed returned bytes, and provider disagreement are
  explicit recoverable UI errors.
- The adapter is bounded: one selected wallet, one account, one transaction,
  and one signing invocation per user action.

## Verification

- Unit tests cover discovery, capability filtering, explicit provider/account selection,
  connection/account binding, returned-byte parsing, mismatch, malformed
  output, rejection, and legacy fallback (`7` tests passing).
- Existing transaction serialization and server signed-message tests remain
  mandatory; no browser test may assert success from a wallet signature alone.
- The React test suite covers the accessible multi-provider selector and the
  pinned NGINX browser suite remains green (`13/13` journeys), including
  Wallet Standard provider registration, mandatory multi-account selection,
  reload behavior, wallet rejection, and signer-bound transaction workflows.
- `pnpm --dir apps/web test`, typecheck, and production build pass with
  `95` tests passing and `6` skipped; targeted wallet/checkout/refund tests
  pass as part of that suite.
- The browser provider is synthetic test infrastructure; a real injected-wallet
  provider/account matrix and Linux validator/provider evidence remain separate
  release gates.

## Consequences

The React product can support multiple modern Solana wallets without changing
the low-latency server or on-chain authority boundaries. The browser gains a
small selection and lifecycle surface, and the legacy fallback remains only
for environments that have no Wallet Standard registration. Production wallet
support still requires a documented provider matrix, browser accessibility
evidence, and live-wallet staging rehearsal.
