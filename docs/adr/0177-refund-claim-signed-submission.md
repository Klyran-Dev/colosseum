# ADR-0177: Exact signed refund-claim submission

## Status

Accepted — signed submission and durable recovery boundary implemented; proof
reconciliation remains a release gate.

## Context

ADR-0176 persists a server-owned refund claim artifact before a wallet signs.
The next boundary must prevent a browser from replacing the compiled
instruction, account metas, signer topology, or artifact digest at submission
time. A provider acknowledgement is also not proof of finality, and a timeout
cannot safely be treated as failure because the transaction may have landed.

## Decision

Add tenant-scoped claim recovery and submission routes:

- `GET /v1/settlement/refund-claims/{intent_id}` returns the persisted intent
  and artifact for wallet recovery;
- `POST /v1/settlement/refund-claims/submit` accepts only `intent_id` and
  canonical base64 signed transaction bytes; and
- the API loads the immutable artifact from PostgreSQL, verifies its stored
  instruction digest, validates the exact legacy-SPL refund instruction with
  `SignedTransaction::verify_refund_instruction`, validates the recent
  blockhash, and only then calls the bounded RPC coordinator.

Submission is fenced through the durable claim lifecycle:

`prepared_unsubmitted -> signed -> submitting -> submitted|processed|confirmed`.

An RPC error after the submission fence advances the intent to `unknown` with
an error code. The API never retries with changed bytes and never reports a
refund from a wallet signature or provider acknowledgement. Recovery must
reconcile the exact signature against finalized chain evidence before a later
proof-gated transition.

The endpoint requires the authenticated tenant's claim-proposer role and
requires the persisted artifact's proposer subject to match that principal.
The refund-funder signature remains a required on-chain signer and is checked
by the exact transaction verifier; no private key is held by the API.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Accept the compiled instruction from the browser | Simple client contract | Allows artifact and account substitution | Rejected |
| Recompile from browser fields at submit time | Flexible | Reopens stale topology and amount races | Rejected |
| Server signs/submits without wallet bytes | Fewer round trips | Custody expansion and no explicit consent | Rejected |
| Treat RPC acknowledgement as success | Low latency response | Does not prove inclusion or finality; unsafe on timeout | Rejected |
| **Load immutable artifact, verify exact signed bytes, then fence and submit** | Non-custodial, replay-safe, account-substitution resistant, and recoverable | Requires durable lifecycle and reconciliation | **Selected** |

## Invariants and failure policy

- all reads and transitions are tenant-scoped;
- the caller cannot submit a replacement artifact or instruction;
- signed bytes are bound to one intent and one signature;
- a different signed-byte digest is a conflict;
- only the fixed Solana program, one exact refund instruction, no lookup table,
  no extra instruction, and the expected account/signer policy are admitted;
- submission is attempted once per fenced signature; ambiguous outcomes become
  `unknown`; and
- only later finalized event and balance proof can establish movement success.

## Verification plan

- tests for artifact substitution, account reorder, signer removal, extra
  instructions, lookup tables, stale blockhash, signature mismatch, duplicate
  submission, and unknown provider outcome;
- PostgreSQL compare-and-set contention tests proving one submission fence;
- API recovery tests proving tenant isolation and persisted-artifact use; and
- Docker RPC-fixture integration tests for submitted, rejected, and ambiguous
  claim paths, including concurrent synthetic-wallet duplicate submissions
  proving one provider send and one durable `submitted` transition.

## Consequences

The claim endpoint becomes a real non-custodial wallet transport boundary with
durable recovery. It deliberately stops short of claiming a refund: finalized
typed event evidence, execute authorization, and transaction-bound balance
proof remain separate bounded-context work.
