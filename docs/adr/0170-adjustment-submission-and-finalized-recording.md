# ADR-0170: Wallet-bound adjustment submission and finalized recording evidence

- Status: Accepted — implementation in progress
- Date: 2026-09-28
- Owners: Settlement, Indexing, Tax Platform

## Context

ADR-0166 and ADR-0167 create a server-owned immutable adjustment-evidence
intent and verify the finalized on-chain adjustment-authority registry. ADR-
0153 defines `record_adjustment` as the bounded Solana evidence instruction.
The remaining boundary must connect the approver wallet signature to that
intent and must not report an adjustment as recorded merely because an RPC
accepted a transaction.

## Decision

Keep `settlement_adjustment_evidence_intents` as an immutable compiler and
evidence snapshot. Add a separate tenant-scoped
`settlement_adjustment_chain_intents` record for the retryable wallet
submission lifecycle: `prepared_unsubmitted`, `signed`, `submitting`,
`submitted`, `processed`, `confirmed`, `finalized`, `rejected`, or `unknown`.
Identity and evidence columns are immutable; only an explicit bounded
transition may change lifecycle fields.

The submission endpoint accepts a wallet-signed transaction, but the server
proves the canonical bytes contain exactly the server-compiled
`record_adjustment` instruction, expected account order and signer/writable
flags, expected instruction data, a bounded payer signature, and the
immutable intent digest. A client-supplied digest or status is never authority
evidence.

An adjustment becomes `finalized` only when the signature is observed at
finalized commitment and a typed `SettlementAdjustmentRecorded` event plus
its raw Anchor bytes exactly match the immutable intent: adjustment PDA,
original receipt, merchant, mint, nonce, approver, signed deltas, adjustment
digest, and evidence digest. Missing, stale, duplicate, conflicting, or
provider-disagreeing evidence fails closed as `unknown` or `rejected`.

No submission acknowledgement implies recording, payment, filing, or legal
compliance. Wallet rejection, timeout, RPC ambiguity, process crash, indexer
lag, and signature reuse remain explicit supportable states.

## Alternatives considered

| Option | Decision | Reason |
| --- | --- | --- |
| Mutate the immutable evidence-intent row with signature/status | Rejected | Couples compiler evidence to a retryable transport lifecycle and weakens replay semantics. |
| Reuse generic settlement intents | Rejected | Its verifier and identity model are settlement-specific and cannot prove adjustment account topology. |
| Trust client signature, digest, or status callback | Rejected | Caller-controlled values cannot establish message binding or finalized chain evidence. |
| Treat RPC acceptance or confirmed status as recorded | Rejected | Acceptance and non-final commitment do not prove the expected event. |
| **Separate chain-intent lifecycle plus typed finalized event projection** | **Selected** | Preserves immutable evidence, enables bounded retries, proves message binding, and makes finality independently auditable. |

## Invariants

- One merchant-scoped adjustment evidence intent has one canonical instruction
  digest and at most one accepted chain signature.
- Every lifecycle transition is authenticated, tenant-scoped, idempotent,
  monotonic, and bounded; identity columns cannot be rewritten.
- Finalized status requires the exact finalized event and raw evidence, not
  only a provider status response.
- Signed deltas retain the domain conservation and magnitude constraints.
- Projection conflicts stop reconciliation and raise an operational signal;
  providers are never silently preferred.

## Consequences and verification

This adds a migration, typed indexer projection, transaction verifier,
submission/reconciliation API contracts, and UI states for signing, unknown,
indexing lag, finalized evidence, and operator intervention. Failing-first
tests must cover canonical message binding, account substitution, signature
reuse, lifecycle transitions, malformed/duplicate/conflicting events,
provider disagreement, indexer lag, crash recovery, and pinned-validator
conformance. Until those gates pass, adjustment recording remains disabled for
production.
