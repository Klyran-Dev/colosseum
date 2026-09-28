# ADR-0175: Durable refund claim and execute intents

## Status

Accepted for implementation.

## Context

The settlement program has two distinct refund instructions: a proposer creates
an on-chain claim and a different approver executes it. A browser must not be
trusted to provide the receipt, token accounts, amounts, claim nonce, funding
configuration, or signer topology. A wallet signature or RPC acknowledgement
also does not prove that the intended instruction was submitted or finalized.

The existing funding-configuration and finalized-topology records establish a
provisioning boundary, but they do not yet provide a durable movement intent,
an idempotent nonce allocator, an exact server-bound wallet artifact, or a
proof-gated lifecycle. Reusing the generic adjustment chain intent would blur
the claim and execute authority boundaries and could allow an execute retry to
be mistaken for a claim creation.

## Decision

Create a Refund Movement bounded context with separate tenant-scoped,
UUIDv7, durable `refund_claim_intents` and `refund_execute_intents` aggregates.
Each aggregate has an idempotency key, canonical request and instruction
digests, immutable server-owned financial/topology inputs, a bounded unsigned
artifact, signed-transaction evidence, and an append-only lifecycle event
stream. Creation of the intent and its first outbox event occur in one
serializable PostgreSQL transaction.

The server-owned artifact builder performs these steps:

1. load an approved adjustment, a finalized original receipt binding, an
   active verified funding configuration, and the required wallet bindings;
2. require a distinct proposer and refund-funder wallet for claim creation,
   and a distinct approver and refund-funder wallet for execution;
3. derive the refund destination from the finalized receipt payer, mint, and
   explicitly supported token program rather than accepting it from the
   client;
4. allocate the claim nonce transactionally per merchant and adjustment;
5. compile the fixed settlement instruction and canonical account order;
6. bind the unsigned artifact to the tenant, intent ID, configuration digest,
   adjustment digest, receipt signature/slot, claim PDA, authority public keys,
   and expiry; and
7. return an inspectable wallet artifact whose exact bytes are persisted before
   any wallet signature is requested.

The wallet may only sign the returned server artifact. Submission accepts a
canonical base64 signed transaction and verifies, before any RPC call, the
exact message bytes, payer/authority signatures, fixed settlement program,
instruction discriminator, account order, instruction data, recent blockhash
policy, intent digest, and required signer set. A client-supplied digest is
never evidence of message binding.

Claim creation and execution have separate state machines:

`prepared_unsubmitted -> signed -> submitting -> submitted -> processed ->
confirmed -> finalized`, with `rejected`, `unknown`, and `cancelled` terminal
or operator-recovery states as appropriate. A movement is not reported as
paid, refunded, or complete until a finalized, transaction-bound typed event
and balance/conservation proof match the exact intent. Timeouts become
`unknown`, never implicit failure or success, and retries are lease-fenced and
signature-idempotent.

Funding configurations must also record the subject authorized to control the
configured refund-funder wallet. The public key is bound through the existing
single-use wallet challenge mechanism and is required to equal the configured
funder key; a caller cannot choose a replacement funder during compilation or
submission.

The initial movement contract remains legacy SPL Token only. Token-2022,
multi-instruction transactions, address lookup tables, and arbitrary extra
instructions require separate compatibility and validator-conformance ADRs.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Reuse generic settlement chain intents | Less schema and code | Conflates adjustment evidence with two-step refund authority and proof | Rejected |
| Accept all claim/execute fields from the browser | Simple wallet UX | Caller-controlled amounts, accounts, nonce, and signer topology | Rejected |
| Compile and immediately submit from a server signer | Fewer wallet round trips | Custody expansion, weaker merchant consent, and larger key compromise blast radius | Rejected |
| Create claim and execute in one instruction | Fewer lifecycle states | Removes four-eyes separation and makes delayed review impossible | Rejected |
| **Separate server-owned claim/execute intents with exact signed-message verification** | Durable retries, non-custodial consent, explicit authority, replayable evidence | More persistence and reconciliation work | **Selected** |

## Invariants and failure policy

- all intent, adjustment, merchant, and funding-configuration identities are
  tenant-scoped UUIDv7 values;
- claim and execute amounts come only from the finalized receipt and approved
  adjustment, and conservation is checked before compilation and after finality;
- claim nonce allocation is unique per merchant/adjustment and cannot be
  supplied or changed by a caller;
- the refund destination, claim PDA, registry PDA, and vault-authority PDA are
  derived from server-owned inputs and are included in the persisted digest;
- proposer, approver, and refund-funder wallet bindings are active, distinct,
  and role-appropriate;
- a funding configuration must be active with unexpired finalized topology
  evidence at intent creation and submission;
- signed bytes, signature, slot, and finalized event evidence are immutable
  once recorded; a correction creates a new evidence record;
- every retry is bounded, deadline-aware, idempotent, and lease-fenced; and
- any malformed artifact, signer mismatch, provider disagreement, stale
  configuration, unsupported token program, transaction ambiguity, or missing
  finalized proof fails closed and remains visible as an actionable state.

## Verification plan

- red/green domain tests for nonce allocation, digest binding, authority
  separation, destination derivation, conservation, and lifecycle transitions;
- property and adversarial signed-transaction tests for account substitution,
  instruction mutation, signer removal, extra instructions, stale blockhash,
  and digest mismatch;
- PostgreSQL contention tests proving one nonce and one intent per idempotency
  key under concurrent requests, with no duplicate outbox creation;
- API contract and accessibility tests for wallet review, rejection, timeout,
  unknown, reconciliation-required, and finalized proof states;
- Docker validator conformance for claim creation, execute authorization,
  replay, cancel, account rotation, wrong source account, and exact finalized
  event/balance proof; and
- load/soak evidence for p50/p95/p99 intent preparation, signed-message
  verification, submission, and reconciliation under bounded pool and RPC
  concurrency.

## Consequences

Refund movement becomes a real auditable workflow rather than a compiler-only
feature. The database stores more intent and evidence state, and the API must
coordinate wallet interaction and reconciliation. In return, no browser,
cache, RPC provider, or wallet signature becomes a hidden settlement authority,
and every movement can be replayed and explained from server-owned inputs.
