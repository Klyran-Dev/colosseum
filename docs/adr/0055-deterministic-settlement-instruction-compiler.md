# ADR-0055: Deterministic, non-custodial settlement instruction compiler

- Status: Accepted
- Date: 2026-09-26
- Decision owners: Principal low-latency architecture and settlement domain
- Scope: `crates/settlement-client` and the API compile boundary

## Context

The protocol's authoritative settlement action is an atomic Solana
`settle` instruction. The quote and attestation slices establish the amount
and policy evidence, but the repository did not yet produce the exact bounded
instruction that a wallet can review and sign. Returning only a quote is not
an end-to-end settlement path; accepting raw account metas from a caller would
also permit account substitution.

## Decision

Create a pure Rust settlement-client bounded context that compiles exactly one
`settle` instruction for the deployed Anchor program. It:

- accepts decimal-string amounts and canonical base58 public keys;
- accepts only the configured settlement program ID; callers cannot substitute
  another program domain;
- validates positive nonce and policy sequence, non-zero policy digest, amount
  conservation, supported token program, and distinct source/vault/destination
  accounts before serialization;
- derives the singleton protocol-config PDA, merchant/mint vault-authority
  PDA, policy-attestation PDA, and payer/merchant/nonce receipt PDA from the
  program's fixed seeds;
- emits a deterministic account-meta order matching the Anchor instruction;
- serializes the Anchor discriminator and little-endian instruction arguments
  exactly once; and
- returns `prepared_unsubmitted`, never a signature, confirmation, or wallet
  success claim.

The compiler is deliberately free of RPC, database, wallet, clock, and private
key dependencies. A later transaction coordinator may add a bounded recent
blockhash, compute-budget policy, v0/ALT selection, wallet handoff, and
submission state machine only after those choices have their own measured
budgets and ADRs.

## Alternatives considered

1. **Let the browser construct arbitrary account metas:** rejected because the
   API/program boundary would not provide a single canonical account layout.
2. **Have the API sign or custody a payer transaction:** rejected because the
   protocol is non-custodial and user approval must remain in the wallet.
3. **Compile a full transaction with a live RPC call in the domain crate:**
   rejected because it couples deterministic serialization to network latency,
   blockhash freshness, and provider availability.
4. **Return only a quote and defer compilation:** rejected because it leaves a
   critical production boundary unimplemented and prevents exact wallet review.

## Invariants and release evidence

- Same valid request and program ID produce byte-identical output.
- Derived PDAs and account ordering are covered by golden tests.
- Invalid conservation, digest, sequence, token-program, aliasing, and key
  inputs fail before instruction bytes are returned.
- The compiler never accepts or accesses a private key.
- API responses distinguish `prepared_unsubmitted` from every on-chain state.
- Live localnet execution, wallet integration, transaction submission, status
  tracking, and finality remain explicit follow-on release gates.
