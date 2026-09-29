# ADR-0176: Server-owned refund claim preparation API

## Status

Accepted for implementation.

## Context

ADR-0175 defines durable refund claim and execute intents, but a browser-facing
preparation boundary is still required. A claim preparation request must not
accept receipt accounts, token accounts, amount fields, claim nonces, funding
configuration fields, or signer topology from the browser. It must also be
safe under retries and concurrent clicks.

## Decision

Add a tenant-scoped `POST /v1/settlement/refund-claims` preparation endpoint.
The request contains only the tenant-scoped approved adjustment ID, an
idempotency key, and explicit confirmation. The adjustment ID selects the
aggregate; it is not an authority boundary and cannot supply financial or
chain-account values. The
authenticated API service performs the following bounded sequence:

1. load the approved `Refund` adjustment;
2. require an active proposer wallet binding for the recorded proposer subject;
3. load the finalized original settlement receipt binding;
4. load the active, finalized-verified refund funding configuration;
5. require the configured refund-funder subject's active wallet binding and
   compare its public key with the configured funder key;
6. allocate the claim nonce using the merchant/adjustment counter;
7. derive the payer destination and all instruction accounts from the loaded
   chain identities;
8. compile the fixed legacy-SPL claim instruction and persist the unsigned
   artifact, request digest, and first lifecycle event transactionally; and
9. return the persisted artifact and intent identity for wallet review.

The endpoint never submits a transaction and never reports a refund. Claim
submission, exact signed-message verification, reconciliation, execute intent
creation, and finalized balance proof remain separate operations and release
gates. A retry with the same merchant/idempotency key returns the original
intent only when the canonical request digest matches; a different request
fails with a conflict.

The response must expose the quote-like review data needed by the wallet UI:
gross, tax, net, mint, token program, destination, receipt signature/slot,
funding configuration digest, claim PDA, instruction digest, artifact digest,
expiry, and lifecycle status. It must not expose private evidence or imply
legal compliance, filing, payment, or finality.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Browser supplies claim fields | Small request | Allows account, amount, nonce, and authority substitution | Rejected |
| Reuse adjustment compiler endpoint | Existing route | Wrong authority model and no funding/topology binding | Rejected |
| Server signs or submits the claim | Fewer wallet steps | Custody expansion and removes explicit wallet consent | Rejected |
| **Server-owned preparation endpoint with durable idempotent intent** | Non-custodial, replayable, tenant-bound, and reviewable | Requires bounded storage and wallet UX | **Selected** |

## Invariants and failure policy

- all identifiers are tenant-scoped UUIDv7 values;
- only an approved `Refund` adjustment with a negative, conserving delta can
  create a claim intent;
- the finalized receipt, active funding configuration, and wallet bindings are
  loaded by the server and are never caller-selected;
- the proposer and configured refund-funder keys are distinct and role-bound;
- only the canonical legacy SPL token program is admitted in this first slice;
- nonce allocation is unique per merchant and adjustment and may safely leave
  a gap after a process crash;
- preparation is idempotent and creates at most one first outbox event; and
- any missing, stale, ambiguous, unsupported, or conflicting evidence fails
  closed without a partial claim intent.

## Verification plan

- failing-first API tests for missing confirmation, wrong role, unapproved
  adjustment, stale funding, missing wallet binding, and idempotent replay;
- storage tests for concurrent nonce allocation and outbox atomicity;
- artifact tests for account substitution, digest mutation, and unsupported
  Token-2022 input;
- PostgreSQL migration and integration drills on a clean Docker stack; and
- browser accessibility tests for review, rejection, timeout, and recovery
  states, plus wallet-workspace interaction tests for server-owned preparation
  and role-separated signing before submission.

## Consequences

The API has a small, auditable preparation surface and the browser receives a
complete review artifact without becoming a financial authority. The first
implementation deliberately stops before submission; later ADRs must add the
signed-message verifier and proof-gated movement lifecycle rather than
silently extending this endpoint.
