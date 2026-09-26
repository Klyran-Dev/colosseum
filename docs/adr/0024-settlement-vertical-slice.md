# ADR-0024: Implement the first settlement vertical slice with legacy SPL Token

- Status: Accepted
- Date: 2026-09-25

ADR-0031 extends this slice with a cached on-chain `PolicyAttestation` PDA;
the original legacy-token and conservation decisions remain in force.

## Context

The repository needs an executable Solana boundary that proves the most important settlement invariant before policy attestations, Token-2022 compatibility, multi-recipient splits, and governance are added. A partial transfer path that does not prove conservation would create false confidence.

## Decision

Implement the first settlement slice as an Anchor 1.2.0-compatible Rust program using the legacy SPL Token program only. The instruction:

- requires a payer signature and explicit merchant, mint, source, tax-vault, destination, and receipt accounts;
- requires `tax + net = gross`, a non-zero policy digest, matching mints, payer-owned source, PDA-controlled tax vault, and sufficient source balance;
- performs bounded `TransferChecked` CPIs for tax and net amounts;
- creates a deterministic receipt PDA keyed by payer, merchant, and client nonce;
- emits a structured settlement event.

Token-2022 is not guessed at or silently accepted. It remains a separately registered compatibility path after extension-by-extension tests prove conservation and authority behavior. Multi-recipient fan-out, policy signature verification, pause/revocation, and governance are subsequent slices.

## Alternatives considered

- **Implement the entire settlement program at once:** rejected because it would combine too many authority and token-extension decisions before invariant evidence exists.
- **Create an instruction that only records a receipt:** rejected because it would not exercise actual token movement or amount conservation at the ledger boundary.
- **Accept both legacy SPL Token and Token-2022 immediately:** rejected because transfer fees, hooks, delegates, and other extensions change amount semantics and require explicit compatibility contracts.

## Consequences

- The initial on-chain contract is intentionally narrow and auditable.
- Legacy SPL Token is the only supported settlement asset path in this slice.
- The program ID in local configuration is a deterministic development identifier and must be replaced through a key ceremony before any deployed environment.
- The next implementation slice must add signed policy-attestation verification and a governed asset registry before production settlement claims are possible.
