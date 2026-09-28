# ADR-0058: Cryptographic binding of wallet-signed settlement messages

- Status: Accepted
- Date: 2026-09-26
- Deciders: Principal Low-Latency Blockchain Architecture
- Scope: settlement coordinator, settlement API, wallet handoff

## Context

An intent digest and a signed-transaction digest prove that the server saw the
same bytes more than once; they do not prove that those bytes authorize the
compiled tax settlement. A malicious or buggy client could bind an unrelated
valid transaction to an intent unless the server parses the signed Solana
message and compares its executable instruction and account domain with the
compiler output.

The settlement endpoint is a non-custodial boundary. It must never request a
private key, reconstruct a transaction from partially trusted fields, or rely
on a client-provided digest as proof of authorization. Solana versioned
transactions also permit address-table lookups, which cannot be verified
against the compiler's static account policy without a governed lookup
resolver and immutable lookup evidence.

## Decision

Before changing a durable intent from `prepared_unsubmitted` to `signed`, and
before any RPC call, the coordinator will:

1. Decode exactly one canonical Solana wire transaction using the pinned
   `solana-transaction` wincode implementation and reject trailing bytes.
2. Sanitize the transaction and verify every required signature, including the
   payer signature, using Solana's native verification implementation.
3. Admit v0 address-table lookups only through the separately governed
   server-owned resolver in ADR-0161; unknown versions remain rejected.
4. Require exactly three instructions in the settlement profile: the exact
   ADR-0162 compute-budget pair followed by the settlement instruction.
   Auxiliary instructions remain rejected.
5. Compare the compiled artifact and signed instruction exactly: program ID,
   instruction data, account count, account order, account public keys,
   signer flags, and writable flags.
6. Hash the canonical serialized compiled instruction artifact—not merely its
   instruction data—and compare that digest with the durable intent.
7. Validate the signed message's recent blockhash with `isBlockhashValid` at
   confirmed commitment through the bounded provider pool. A stale response is
   a client error; provider uncertainty fails closed and does not advance the
   intent.
8. Preserve the original signed bytes for bounded provider retries. The
   coordinator never rebuilds, mutates, or re-signs the transaction.

The API submit request therefore carries both the intent ID and the compiled
instruction artifact returned by compile, alongside canonical base64 wallet-
signed bytes. This is a reviewable binding envelope; it does not give the API
custody of a private key.

## Alternatives considered

### Trust the client-provided instruction digest

Rejected. A digest is only meaningful when the server independently derives or
verifies the bytes it represents. Trusting it would permit intent confusion.

### Reconstruct and sign the transaction at the API

Rejected. This violates the non-custodial boundary, creates key-management
risk, and makes the API responsible for wallet and blockhash semantics.

### Accept any valid transaction signed by the payer

Rejected. A valid payer signature does not authorize the expected tax amount,
program, policy-attestation accounts, token accounts, or instruction data.

### Resolve address-table lookups immediately

Rejected for this release. Lookup resolution adds a mutable external dependency
and a new evidence model. It will be designed as a separate bounded context
with explicit slot/provider consistency tests before enablement.

### Permit arbitrary compute-budget and memo instructions by default

Rejected. Extra instructions can change fees, execution behavior, or wallet
review semantics. ADR-0162 supersedes this decision only for the exact,
allowlisted two-instruction compute-budget prefix; memo and other auxiliary
instructions remain rejected.

## Consequences

Positive:

- Intent-to-message authorization is cryptographically checked at the trust
  boundary.
- The API cannot submit a transaction with substituted accounts, program, or
  tax payload even when the transaction is otherwise valid on Solana.
- Wallet signing remains fully non-custodial and provider failover remains
  byte-preserving.
- Invalid signatures, malformed wire data, unsupported lookups, and policy
  mismatches fail closed before durable lifecycle advancement.
- Stale blockhashes are rejected before durable lifecycle advancement, while
  provider uncertainty remains distinguishable from client invalidity.

Trade-offs:

- The current submit envelope is stricter than many generic Solana RPC
  clients; clients must retain and return the compiler artifact.
- Versioned transactions with address-table lookups and multi-instruction
  messages are intentionally unsupported until separately governed.
- Full production enablement still requires blockhash freshness/validity
  policy, localnet evidence, staging RPC evidence, and independent audit.

## Verification

- Unit tests construct and sign a real Solana transaction using the pinned
  transaction libraries.
- Tests reject altered data, program ID, account order, trailing bytes, and
  invalid signatures.
- API integration tests and golden vectors must cover the same binding envelope
  in Rust and TypeScript before production scope is enabled.
