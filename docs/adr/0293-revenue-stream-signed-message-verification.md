# ADR-0293: Revenue-stream signed-message verification boundary

- Status: Accepted
- Date: 2026-10-04
- Owners: Settlement and platform engineering

## Context

Revenue-stream plan, accrual, claim, execution, and cancellation instructions
are compiled by the server, but the browser wallet returns a signed Solana
transaction. A caller-controlled instruction digest, plan address, or account
list is not evidence that the wallet signed the intended message. The
settlement coordinator must therefore verify the actual signed message before
submission in every non-test environment.

The existing settlement, adjustment, and refund flows already enforce this
boundary. Revenue-stream instructions must have the same protection without
introducing a second wire contract or permitting address lookup tables and
additional instructions on a bounded revenue operation.

## Decision

Add `SignedTransaction::verify_revenue_stream_instruction` to the settlement
coordinator. It accepts only the server-owned
`CompiledRevenueStreamInstruction` and performs these checks:

1. The compiler artifact is in `prepared_unsubmitted` state.
2. The signed packet is within the conservative Solana packet budget and is a
   cryptographically valid, canonical versioned transaction.
3. The message contains exactly one instruction and no address lookup tables.
4. The instruction program ID and canonical instruction bytes exactly equal the
   compiler artifact.
5. The instruction account indexes resolve to the exact ordered account list,
   with the same signer and writable policy and no duplicate instruction
   accounts.
6. Static accounts contain only the expected instruction accounts, the program
   account, and the fee payer when it is not already an expected account. The
   program account occurs exactly once.

This verifier proves message binding only. It does not prove wallet ownership
of merchant authorities, chain finality, route activation, or the correctness
of a tax calculation; those remain separate server, indexer, and on-chain
invariants.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`.

- Trust the browser-supplied digest: rejected because it does not bind the
  digest to the signed message.
- Rebuild the transaction from request fields: rejected because request fields
  are not proof of what the wallet signed and rebuilding can drift from the
  actual bytes.
- Permit generic transaction inspection and rely on the program: rejected
  because extra instructions, lookup-table substitution, and account topology
  would remain outside the intended bounded contract.
- **Verify the actual signed message against the typed server artifact:**
  selected because it is deterministic, auditable, bounded, and consistent
  with the existing settlement/refund authority boundary.

## Consequences

Positive:

- Revenue-stream submission has the same message-binding guarantee as the
  existing settlement and refund paths.
- The browser cannot add accounts, instructions, lookup tables, or substitute a
  revenue program while retaining a valid server artifact.
- Pure unit tests can exercise the security boundary without RPC or a wallet
  provider.

Trade-offs:

- Wallets must sign the exact bounded message emitted by the server compiler.
- Future revenue instructions that intentionally use address lookup tables or
  multiple instructions require a new typed artifact and ADR rather than a
  permissive verifier change.

## Verification

- Unit tests cover exact acceptance, data substitution, account substitution,
  extra instructions, and non-submission status.
- Coordinator clippy and workspace tests are required before merge.
- Live staging evidence must additionally prove actual wallet signing,
  finalized chain submission, route activation, and provider outage behavior.
