# ADR-0179: Durable, proof-gated refund-execute intent

## Status

Accepted for implementation; execute preparation, wallet transport, and
finalized-proof reconciliation remain release gates.

## Context

Claim preparation and exact signed claim submission are now durable, but the
second on-chain movement instruction still has no server-owned intent API. A
browser or operator must not choose the claim nonce, refund accounts,
approver, funding configuration, or source evidence. Reusing the claim intent
would collapse the four-eyes boundary and make claim creation look like a
completed refund.

The execute instruction is valid only after a finalized claim exists and the
active funding topology still matches the original receipt and adjustment.
The API must prepare an immutable execute artifact, accept exact signed bytes
from the approver and refund-funder wallets, and leave the movement
proof-pending until finalized event and transaction-bound balance evidence
match the intent.

## Decision

Implement execute as a separate tenant-scoped `refund_execute_intents`
aggregate with:

- UUIDv7 identity and merchant-scoped idempotency;
- a request digest bound to claim intent, finalized claim signature/slot,
  adjustment, funding-configuration digest, signer subjects, and claim nonce;
- an immutable server-owned unsigned artifact using the fixed fourteen-account
  legacy-SPL `execute_adjustment_refund` instruction;
- role-separated approver and refund-funder wallet handoff, with no server
  key custody and no browser-supplied amounts or account topology;
- exact signed-message verification of program, discriminator, account
  order/flags, instruction data, both signatures, intent binding, and bounded
  recent-blockhash policy before RPC submission;
- compare-and-swap lifecycle transitions plus append-only event/outbox
  records; and
- a proof-gated final state: only finalized typed refund event and
  transaction-bound conservation proof can advance the intent to `finalized`.

The preparation endpoint accepts only an approved claim intent ID,
idempotency key, and explicit confirmation. It requires finalized claim
evidence, loads the active verified funding configuration, loads distinct
approver and refund-funder authority bindings, recompiles the execute
instruction, validates persisted claim topology, and stores the artifact
before requesting a wallet signature.

The endpoint family is:

- `POST /v1/settlement/refund-executions` — prepare or idempotently replay;
- `GET /v1/settlement/refund-executions/{intent_id}` — recover durable state;
- `POST /v1/settlement/refund-executions/submit` — verify and submit unchanged
  signed bytes; and
- reconciliation-worker transitions — record only finalized event/balance
  evidence, never caller-supplied claims of success.

The initial contract remains legacy SPL Token only. Token-2022, address lookup
tables, extra instructions, and batched execution require separate ADRs and
validator evidence.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Reuse the claim intent | Less schema and UI work | Blurs claim and execute authority and proof lifecycle | Rejected |
| Accept a claim ID and compile from mutable current configuration | Simple recovery | A configuration rotation can change accounts after approval | Rejected |
| Trust the submitted signature or event as completion | Low latency | Does not prove finalized transaction-bound conservation | Rejected |
| Server-sign with the refund-funder key | Fewer wallet steps | Violates non-custody and role separation | Rejected |
| **Immutable execute intent with exact signed verification and proof-gated reconciliation** | Four-eyes authority, replayable evidence, safe retries, and explicit uncertainty | More durable state and integration work | **Selected** |

## Invariants and failure policy

- the execute intent is tenant-scoped and bound to exactly one finalized claim;
- claim nonce, claim PDA, receipt, adjustment, mint, token program, vault,
  source, destination, and signer keys are server-derived;
- approver and refund-funder are distinct active role bindings and match the
  compiled instruction;
- active funding digest and finalized topology match at preparation and
  submission;
- a signed transaction digest is immutable once recorded; a different digest
  is a conflict, not a replacement;
- stale blockhash, provider timeout, signer mismatch, malformed instruction,
  provider disagreement, missing proof, or stale configuration fails closed;
- provider acknowledgement and an indexed event without transaction-bound
  proof never become `finalized`; and
- retries are bounded, idempotent, CAS/lease fenced, and observable.

## Verification plan

- failing-first tests for execute digest binding, role separation, claim
  topology, and exact fourteen-account compilation;
- adversarial signed-message tests for account substitution, instruction
  mutation, missing signer, extra instruction, stale blockhash, and digest
  mismatch;
- PostgreSQL contention tests for idempotent creation and submission fencing;
- API contract and accessibility tests for preparation, wallet recovery,
  unknown provider outcomes, and proof-pending state;
- Docker RPC-fixture integration covers concurrent synthetic-wallet claim and
  execute submissions, one-send fencing, and durable `submitted` state; this
  is transport evidence only and does not establish finalized movement;
- local-validator conformance for claim-before-execute, replay, wrong source,
  wrong funder, stale configuration, and finalized proof binding; and
- load/soak evidence for bounded preparation, verification, submission, and
  reconciliation concurrency.

## Consequences

Refund execution becomes an explicit, auditable bounded context and cannot be
mistaken for claim preparation or a transport acknowledgement. This adds
storage, API, UI, and reconciliation work, but preserves non-custody,
four-eyes authorization, deterministic replay, and fail-closed financial
semantics.
