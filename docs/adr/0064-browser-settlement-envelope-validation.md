# ADR-0064: Validate the settlement envelope before wallet signing

- Status: Accepted
- Date: 2026-09-26
- Deciders: Principal Low-Latency Blockchain Architecture
- Scope: React wallet handoff and client-side transaction construction

## Context

The API is the settlement artifact authority and cryptographically verifies the
wallet-signed message before RPC submission. The browser nevertheless sits
directly in front of a user authorization prompt. A malformed API response,
stale frontend bundle, or compromised intermediary must not cause the wallet to
show an account-substituted or unrelated instruction without an immediate local
failure.

The browser cannot prove chain state and must not become a settlement
authority. It can, however, validate the bounded protocol envelope before
requesting a signature.

## Decision

The React wallet boundary validates every compiled artifact before requesting a
recent blockhash or opening the wallet prompt. It requires:

- the `prepared_unsubmitted` lifecycle state and fixed settlement program ID;
- exactly twelve account metas in the canonical Anchor order;
- exact signer and writable flags for every position;
- payer equality with the first signer account;
- equality between the account positions and the four returned PDA fields;
- a supported legacy SPL Token or Token-2022 program and the System Program;
- distinct source, tax-vault, and net-destination token accounts;
- canonical base64, an 80-byte instruction payload, and the `settle`
  discriminator.

The API repeats and strengthens these checks against the actual signed Solana
message. The browser check is defense in depth and never authorizes, submits,
or reports settlement success.

## Alternatives considered

### Rely only on API verification

Rejected as the sole user-safety measure. The API gate remains authoritative,
but local validation provides immediate feedback before a wallet prompt and
reduces accidental signing of a visibly invalid envelope.

### Recompute all PDAs and tax values in the browser

Rejected. It duplicates authority, risks numeric/rule drift, and would turn the
browser into an accidental financial-calculation authority. The browser checks
the returned artifact's internal consistency; the server owns quote amounts and
PDA derivation.

### Add RPC account discovery before signing

Rejected. It adds latency and provider dependence to the wallet hot path and
does not replace finalized route activation or server-side signed-message
verification.

### Share the Rust compiler through WASM

Deferred. It may reduce duplicate derivation code but requires measured bundle,
startup, memory, and wallet compatibility evidence before changing the SPA
boundary.

## Invariants

1. No blockhash lookup or wallet-signing request occurs for an invalid envelope.
2. The browser never accepts merchant-side account addresses from a checkout
   form.
3. API signed-message verification remains mandatory before submission.
4. Browser validation cannot claim chain state, tax compliance, confirmation, or
   finality.
5. Any program, account, byte, flag, PDA, or lifecycle substitution fails
   closed.

## Consequences

The user receives a deterministic local rejection for structural artifact
errors, and the wallet sees only the reviewed bounded instruction shape. The
frontend carries a small amount of protocol-envelope knowledge that must be
updated with the canonical vector and API contract. This does not replace
localnet execution, live RPC testing, or independent security review.
