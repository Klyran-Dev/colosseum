# Revenue Streaming bounded context

## Purpose

Revenue Streaming turns finalized, attested settlement allocations into
governed release claims for a partner or recipient. It does not calculate tax,
replace the settlement receipt, custody private keys, or make a database row an
authority to move tokens.

```text
finalized settlement evidence
        -> immutable stream accrual
        -> bounded claim preparation
        -> wallet/HSM or governed signer
        -> finalized release evidence
```

An accrual is accepted only when its settlement receipt, policy/rule digest,
asset, recipient, stream-plan digest, amount conservation, and finalized slot
are bound together. A stream claim is an unsigned release artifact until the
chain verifies the exact instruction and finalized evidence is recorded.

## Bounded model

### Stream plan

A stream plan is immutable after activation and contains tenant and merchant
identity, the UUIDv7 of the server-resolved active settlement route, asset and
recipient identity, named payout-preparer subject, source split-plan digest,
tax-policy/rule digest, start and optional end slot, maximum lifetime and
claim amounts, cadence/window bounds, and four-eyes governance evidence. The
browser supplies the asset identity and tenant-scoped payout-preparer subject
only; it cannot choose the merchant account, tax vault, token program, route
version, or wallet public key. The preparer's wallet is attached later through
the shared one-time, role-qualified wallet-proof flow.

One plan has one recipient and one asset. A multi-recipient product creates
separate plans from the same immutable Split Plan; it does not create an
unbounded recipient list in one stream instruction.

### Accrual

An accrual is an immutable projection of one finalized settlement allocation.
It records the source transaction signature, finalized slot, receipt digest,
gross/tax/net amounts, recipient allocation, and stream-plan digest. The
recipient allocation cannot exceed the server-attested net amount, and gross
must conserve with tax plus net.

The ledger is idempotent by source signature and plan. An identical duplicate
is harmless; a duplicate with different evidence or amounts is a conflict and
fails closed.

### Release claim

A release claim is a bounded artifact over unclaimed accruals. Preparation does
not mark funds as released. The claim becomes released only after exact signed
bytes, expected plan digest, recipient, asset, amount, nonce, and finalized
chain evidence are verified. Claims are single-use; unknown provider outcomes
remain pending and require reconciliation.

## State machine

```text
Draft -> Proposed -> Approved -> Active -> Paused -> Active
                              |          |
                              v          v
                         Cancelled   Completed
```

The proposer and approver must be distinct authenticated principals. Pause and
cancellation stop new claims but preserve historical accrual and release
evidence. Completion is allowed only when the end window is reached and the
unreleased balance is zero or separately governed for closure.

## Invariants

1. Tax is never recalculated by this context.
2. Only finalized and attested settlement evidence can create an accrual.
3. `released + pending_claims <= accrued - finalized_decreases` at every state.
4. A claim cannot change its plan, asset, recipient, amount, nonce, or source
   digest after preparation.
5. Source signatures, claim IDs, and finalized release signatures are
   idempotent identities; conflicting reuse fails closed.
6. A stream cannot bypass settlement pause, vault governance, tenant scope,
   token-interface restrictions, or append-only evidence.
7. Amount arithmetic is checked integer arithmetic with no floating point.
8. All bounded collections reject overflow at their explicit maximum.

## Boundary

The finalized-accrual path is asynchronous and idempotent. Claim preparation
is a bounded validation/read/write operation and must not call an external
provider. Wallet signing, HSM/KMS approval, RPC submission, and finality
reconciliation remain outside the domain library behind ports with deadlines,
cancellation, and bounded retries.

No navigation item is added until the API, authorization, evidence, and browser
journeys are complete. The eventual operator language will be “Partner payout
plans”, “Review accrued balance”, and “Prepare payout release”; raw IDs,
digests, slots, and account keys will be progressive technical details.

## Durable and API contract

PostgreSQL is the durable projection and evidence index. Plan creation is
tenant-scoped and idempotent by merchant plus caller idempotency key. Claim
preparation is idempotent by merchant, plan, and idempotency key; a retry with
different amount fails closed. Claim IDs and plan IDs are server-generated
UUIDv7 values. Browser requests never provide merchant identity, approver
identity, recipient account authority, or a release signature. A plan proposal
does provide the named payout-preparer subject, which is tenant-scoped and
distinct from the proposer; it is not a wallet key and cannot authorize a
transfer.

The authenticated API surface is:

- `GET/POST /v1/revenue-streaming/plans` for scoped plan list and proposal;
- `GET /v1/revenue-streaming/plans/{plan_id}` for tenant-scoped detail;
- `POST /v1/revenue-streaming/plans/{plan_id}/transition` for four-eyes
  governance and finalized completion evidence; and
- `POST /v1/revenue-streaming/plans/{plan_id}/claims` for bounded, idempotent
  claim reservation.
- `POST /v1/revenue-streaming/authority/challenges` and
  `POST /v1/revenue-streaming/authority/bindings` for the separate, one-time
  wallet-proof workflow. These endpoints accept a context-qualified role and
  proof; they do not make a browser-supplied wallet key authoritative.

Plan creation resolves exactly one active, effective, finalized settlement
route for the tenant and asset and persists that route identity atomically
with the plan. Zero or multiple active routes are unavailable states; no
fallback route or caller-supplied account topology is accepted.

Every endpoint derives tenant and actor identity from the verified OIDC
principal, evaluates scope and role policy, and returns explicit conflict or
unavailable states. The API does not submit a token transfer; a later chain
adapter must verify the exact claim digest and finalized Solana evidence before
calling the release-finalization port.
