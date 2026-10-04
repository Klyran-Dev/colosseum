# ADR-0314: Executable non-custodial wallet source contract

## Status

Accepted — the browser wallet port and API request boundary are covered by a
source contract; live wallet-provider and independent security review remain
release gates.

## Context

The product is non-custodial: user wallets own private keys and the browser
requests connection and signing only. Existing wallet tests and ADR-0059/
ADR-0200 describe that boundary, but a production repository also needs a
repeatable check that future request fields or browser helpers cannot quietly
introduce seed phrases, mnemonics, private keys, or client-side keypair
construction.

The check must not reject legitimate server-owned issuer/HSM key identifiers,
public keys, transaction signatures, or test-only keypairs used to construct
synthetic signed fixtures. Its scope is the browser production source and
authenticated API request definitions.

## Decision

Add `scripts/verify_non_custodial_wallet_contract.sh` to the standard `make
check` gate. The contract:

1. scans non-test `apps/web/src` and `crates/api/src` request-boundary source
   for forbidden user-key material terms and client-side secret/keypair
   construction patterns;
2. requires the browser wallet port to expose connection/public-key/signing
   behavior and the documented non-custodial marker;
3. requires the API source to contain the explicit non-custodial boundary
   marker and forbids request fields named for seed phrases, mnemonics, private
   keys, or user secret keys;
4. leaves server-owned signing infrastructure and test fixtures outside this
   browser/API contract, where their separate HSM/KMS and fixture contracts
   apply.

The contract is static defense in depth. Wallet-provider behavior, signed-byte
verification, authorization, and Solana finality remain enforced by the
runtime boundaries and their tests; the source contract never grants trust to
the browser.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Rely only on wallet-provider policy and code review | Low maintenance | Regression can enter request types or browser helpers unnoticed | Rejected |
| Ban every occurrence of `key`, `secret`, or `sign` | Simple pattern | Rejects public keys, server-owned signer IDs, and legitimate wallet APIs | Rejected |
| Scan only generated bundles | Tests shipped output | Source-level ownership and API request intent are harder to diagnose | Rejected |
| **Scoped source contract plus runtime wallet tests** | Fast, reviewable, low false-positive scope, defense in depth | Requires explicit exclusions for test fixtures and server-owned key IDs | **Selected** |

## Invariants and failure policy

- The browser never accepts, stores, derives, or sends a user's private key,
  seed phrase, mnemonic, or secret key.
- API request DTOs never accept user private-key material or seed material.
- Public wallet addresses, transaction bytes, transaction signatures, and
  server-owned signer identifiers remain allowed.
- A contract match fails the build; it is never downgraded to a warning.
- Test-only synthetic keypairs remain isolated from production source paths.

## Verification

- The script is run directly and by `make check`.
- Existing Wallet Standard tests continue to prove explicit provider/account
  selection and wallet-owned signing.
- Existing API and signed-message tests prove server-owned artifacts and exact
  signed-byte binding.
- `make public-docs-check` requires this ADR, script, and Make target to remain
  indexed in the public repository.

## References

- [ADR-0059: Non-custodial checkout wallet handoff](0059-non-custodial-checkout-wallet-handoff.md)
- [ADR-0200: Solana Wallet Standard signing boundary](0200-solana-wallet-standard-boundary.md)
- [ADR-0064: Browser settlement-envelope validation](0064-browser-settlement-envelope-validation.md)
