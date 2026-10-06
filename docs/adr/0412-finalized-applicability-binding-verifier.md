# ADR-0412: Finalized applicability-binding account verifier

- Status: Accepted
- Date: 2026-10-06
- Owners: Blockchain/indexer platform
- Scope: finalized-chain evidence for the applicability-bound settlement path

## Context

ADR-0411 adds a receipt-scoped `ApplicabilitySettlementBinding` PDA to the
versioned Solana settlement instructions. The emitted event is retained as raw
finalized evidence, but an event alone does not prove that the binding account
currently exists, is owned by the settlement program, has the expected Anchor
layout, or contains the same receipt, merchant, mint, context, and attestation
digest. Treating the event as sufficient would allow a projection to claim an
applicability binding without account-state evidence.

## Decision

Add a bounded, server-owned finalized-account verifier in the indexer domain.
The verifier:

1. derives the expected binding PDA from `b"applicability-binding"` and the
   receipt using the fixed settlement program ID;
2. requests exactly that account with `getMultipleAccounts` at `finalized`
   commitment and base64 encoding;
3. enforces one response account, bounded response/data size, non-executable
   settlement-program ownership, the exact Anchor discriminator and account
   length, and non-zero identity/digest fields;
4. compares every decoded field with the server-owned settlement event/quote
   target; and
5. returns a typed verification record containing the provider slot and exact
   binding identity. Any ambiguity, null account, provider error, weak
   commitment, malformed layout, PDA mismatch, or field substitution fails
   closed.

This verifier is a separate reconciliation/projection dependency. It is not
called from quote, transaction compilation, wallet signing, or settlement
submission. A future persistence change may mark the payment applicability
bound only after this typed verification record and the raw event are committed
atomically; until then the raw event remains evidence but is not a binding
approval.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Trust the emitted event | Rejected | Does not prove finalized account state or PDA/layout integrity. |
| Fetch the binding account in the settlement hot path | Rejected | Adds RPC latency and an external failure dependency to wallet/signing preparation. |
| Read account state with `getAccountInfo` from an unbounded generic adapter | Rejected | Lacks the one-account, response-size, exact-layout, and commitment contract. |
| Include account data in every block projection | Rejected | Inflates normal indexer hot-path work; not every settlement needs applicability verification. |
| Dedicated bounded verifier port | Selected | Preserves low-latency settlement paths and makes finalized evidence independently testable and replayable. |

## Invariants

- The binding PDA must be derived from the event receipt and the fixed program
  ID; caller-provided binding addresses are never authoritative.
- The account owner must equal the settlement program and the account must not
  be executable.
- The account data must be exactly `8 + 32 + 32 + 32 + 16 + 32 + 1` bytes and
  use the `ApplicabilitySettlementBinding` Anchor discriminator.
- Receipt, merchant, mint, context ID, attestation digest, and bump must match
  the server-owned target; zero identities/digests are invalid.
- The returned slot must come from the finalized RPC context and be retained
  with the verification record.

## Consequences

The indexer now has a production-grade account-verification port and executable
wire/layout tests. Projection persistence and production scheduling still need
to be completed before the UI or API may present an applicability binding as
finalized. This explicit boundary prevents a partial verifier from becoming a
false compliance or settlement claim.

