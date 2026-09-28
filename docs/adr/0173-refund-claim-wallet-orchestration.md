# ADR-0173: Server-owned refund-claim wallet orchestration

## Status

Accepted — design complete; implementation not yet enabled.

## Context

The repository now has the bounded on-chain refund-claim/execute contract from
ADR-0154, finalized refund-event projection and transaction-bound proof
contracts from ADR-0155 through ADR-0158, and a production wallet workflow for
record-only adjustment evidence from ADR-0170 and ADR-0171. It does not yet
have an API or browser workflow for the two refund movement transactions.

Adding a refund button directly to the adjustment workspace would be unsafe:
the refund contract needs an explicit merchant funder, a payer-owned token
destination, a tax-vault binding, claim windows, and distinct proposer,
approver, and funder authorities. None of those values may be reconstructed
from a payer, supplied as arbitrary browser topology, or inferred from an
approval status.

## Decision

Implement refund movement as a separate `refund-claim` bounded context with
server-owned topology, durable intent fencing, and the same wallet boundary as
record-only adjustment evidence.

### Funding configuration and authorities

Create a tenant-scoped, four-eyes funding configuration containing:

- merchant and mint;
- explicit refund-funder public key;
- explicit merchant-owned net-source token account;
- tax-vault token account and vault-authority PDA evidence;
- supported token program and extension policy;
- activation/revocation lifecycle and immutable evidence digest.

Configuration is provisioned by an authenticated governance workflow, never by
the refund request. The API verifies the finalized account topology and owner,
mint, token-program, authority, and extension constraints before it can be
used. A configuration change is a new version and cannot mutate an existing
claim intent.

### Claim and execution workflow

1. The authenticated approver requests a claim compile for an approved,
   negative `Refund` adjustment. The server loads the canonical finalized
   receipt, adjustment evidence, active funding configuration, and active
   proposer/approver/funder bindings.
2. The server reserves one UUIDv7 claim intent and one positive `u64` claim
   nonce under a serializable tenant-scoped idempotency fence. Its canonical
   request digest includes every financial field, authority, account, window,
   token program, reference digest, and funding-configuration digest.
3. The API compiles the exact `create_adjustment_refund_claim` instruction.
   The browser receives only the server-owned artifact and exact decimal
   strings for all `u64` values. It may not alter accounts, amounts, nonce,
   windows, instruction bytes, or authority fields.
4. The connected proposer wallet must match the compiled proposer account.
   The browser submits unchanged signed bytes through a durable claim-intent
   lifecycle. Wallet rejection, timeout, provider ambiguity, and signature
   reconciliation are explicit states.
5. Only a finalized typed `AdjustmentRefundClaimCreated` event matching the
   exact intent may advance the claim to `created`. Approval is a separate
   authenticated action by the recorded approver and cannot be performed by
   the proposer or refund-funder subject.
6. The approver requests the server-owned execute compile. The server reloads
   the immutable claim topology, verifies the finalized claim account, and
   compiles `execute_adjustment_refund`. The connected approver and funder
   wallets must match the exact signers. No browser-supplied topology is
   accepted.
7. Finalized execution evidence is initially `executed_evidence_pending`.
   The proof worker from ADR-0158 must bind the exact signature, claim PDA,
   event, slot, and three transaction-bound token balance deltas before the
   UI may render `chain_executed`. The product must not render `paid`,
   `refunded`, `compliant`, or `tax filed` as an automatic consequence.

### State machine

```text
prepared_unsubmitted -> submitting -> unknown
prepared_unsubmitted -> submitting -> submitted
submitted -> created | rejected | unknown
created -> approval_pending -> approved | cancelled
approved -> execute_prepared -> execute_submitting -> execute_unknown
execute_submitting -> execute_submitted | execute_unknown
execute_submitted -> executed_evidence_pending | execute_unknown
executed_evidence_pending -> chain_executed | reconciliation_required
```

Every transition is a compare-and-set guarded append-only event. `unknown` is
terminal for automatic resubmission until a bounded reconciliation action
finds the exact signature or an operator cancels the intent under the recovery
runbook. A timeout never enables a second wallet signing flow automatically.

### Latency and concurrency budgets

- Warm claim/execute compilation: p50 <= 2 ms and p99 <= 10 ms inside the
  service, excluding bounded finalized RPC verification.
- Finalized topology verification: p95 <= 250 ms per provider attempt and at
  most two complete provider attempts.
- One idempotency key produces at most one claim intent and one chain send
  under 100 concurrent duplicate requests across independent API replicas.
- One worker invocation processes one claim candidate and uses bounded memory;
  no user-controlled fan-out or unbounded retry is permitted.
- Wallet signing is outside the API latency SLO. Submission, confirmation,
  finalization, and proof latency are measured separately by provider and
  region.

## Invariants and fail-closed rules

- Only negative, conserving `Refund` adjustments can enter this workflow.
- All claim financial fields and topology are immutable after reservation.
- Proposer, approver, and refund-funder role bindings are distinct according
  to the approved authority matrix.
- The destination owner is exactly the finalized original receipt payer.
- Tax vault, net source, and destination are pairwise distinct and bind to one
  mint and supported token program.
- The claim nonce is positive, canonical, retry-stable, and serialized as a
  decimal string at browser boundaries.
- Signed bytes are verified against the immutable server artifact before any
  provider call. Client-supplied digests are not authority evidence.
- A provider acknowledgement, wallet signature, `submitted` status, or
  indexed event alone never means token movement succeeded.
- Conflicting provider, event, claim-account, or balance evidence blocks the
  state transition and creates reconciliation evidence.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Browser-supplied refund topology | Rejected | Creates account-substitution, beneficiary, and authority confusion. |
| Immediate claim-and-execute flow | Rejected | Removes four-eyes approval and makes ambiguous submission unsafe. |
| Generic settlement-intent reuse | Rejected | Does not bind claim PDA, two refund signers, or movement proof. |
| Treat the finalized refund event as payment proof | Rejected | An event does not prove the exact transaction-bound three-leg balance deltas. |
| Hosted custody/signing service | Rejected for the default path | Violates the non-custodial wallet boundary and adds a high-value signer dependency. |
| **Separate server-owned claim and execute intents with shared wallet adapter, finalized typed evidence, and transaction-bound proof** | **Selected** | Preserves authority separation, replay fencing, exact topology, non-custody, and supportable failure recovery. |

## Design-before-code deliverables

Before implementation is enabled, add:

1. An additive PostgreSQL migration for funding configurations, claim intents,
   execute intents, append-only transitions, idempotency keys, and evidence
   digests with native `uuid`/UUIDv7 and numeric `u64` bounds.
2. Versioned API schemas and cross-language claim/execute artifacts using
   canonical decimal `u64` strings.
3. Failing-first domain, storage-concurrency, signed-message, browser wallet,
   accessibility, and failure-path tests.
4. A topology verifier using complete provider attempts and the existing
   finalized account/claim readers; no browser input may satisfy this boundary.
5. Real-browser tests for proposer/approver mismatch, wallet rejection,
   unknown/reconcile, stale claim windows, and proof-pending states.
6. Linux validator conformance proving claim creation, cancellation, execution,
   duplicate rejection, wrong topology atomic failure, and exact balances.

## Consequences

Refund movement becomes a real, bounded product capability without promoting
approval or submission to a financial success claim. It adds durable tables,
wallet UX, governance setup, and operational proof work. Other adjustment
kinds remain unavailable until their own funding and legal semantics are
modeled.
