# ADR-0150: Immutable settlement adjustments bounded context

## Status

Accepted. Domain library and durable proposal/approval projection are
implemented; on-chain posting, UI review, and finalized reconciliation remain
subsequent gates.

## Context

Real tax and payment operations need refunds, reversals, corrections,
chargebacks, and remittance adjustments. Treating one of these as an update to
the original receipt would destroy the evidence needed to replay the original
calculation, reconcile the tax vault, and explain a customer-visible change.
Treating it as an untyped negative payment would also allow tax/net amounts to
drift, over-refunds, duplicate posting, or an operator to bypass the governed
settlement path.

The adjustment is therefore a separate bounded context. It records a new,
immutable, versioned financial event that references an original settlement
and an evidence digest. It proposes a signed atomic delta; it does not itself
move tokens, mutate a receipt, decide tax law, or authorize a Solana
transaction.

## Decision

Create a pure Rust `settlement-adjustments` domain crate with these rules:

1. Every adjustment and original settlement reference must be UUIDv7. The
   domain validates the version but does not mint identifiers or read a clock.
2. The signed `gross`, `tax`, and `net` deltas are exact `i128` values whose
   absolute values fit `u64` atomic units and must satisfy
   `gross_delta = tax_delta + net_delta`.
3. Refund, reversal, and chargeback deltas must be non-positive and non-zero.
   Correction and remittance-adjustment deltas may be positive or negative but
   must be non-zero and remain conservation-preserving.
4. An adjustment binds the merchant, mint, original settlement, reason,
   source calculation, replacement calculation when applicable, and evidence
   digest into a canonical SHA-256 content digest. The server-generated record
   UUID is deliberately excluded from that content digest, so a retry can
   reconstruct a proposal before the idempotency fence returns the original
   UUIDv7 record. The request digest additionally binds tenant, actor, and
   idempotency key. Neither digest is an authorization to post funds.
5. A bounded in-memory ledger model proves that posted negative deltas cannot
   exceed the original gross/tax/net amounts and that a duplicate ID is
   idempotent only when its canonical digest is identical. A different payload
   under the same ID is rejected.
6. Lifecycle transitions are monotonic: proposed → approved → posted, with
   rejected/voided terminal branches. Approval requires a distinct subject
   from the proposer in the application/storage layer; the pure domain keeps
   the subject identities in the event contract.
7. Durable PostgreSQL persistence will use a new append-only adjustment table,
   UUIDv7 constraints, tenant-scoped composite foreign keys, an idempotency
   key, and an outbox record in one transaction. That migration and API are
   not smuggled into this domain-only slice.
8. On-chain posting must compile a new governed settlement or vault-release
   action. No adjustment endpoint may call a token program, alter an original
   receipt, or infer legal filing/remittance completion.

## Invariants

- Original receipt evidence is immutable and remains queryable.
- All deltas conserve the adjustment's own gross/tax/net amount.
- A posted adjustment is applied at most once to the domain ledger.
- Negative cumulative deltas cannot take any original component below zero.
- Same ID plus different digest is a conflict, never a replay success.
- No adjustment can authorize funds without a separate attestation, route,
  wallet/signature, and Solana finality boundary.

## Alternatives considered

See the dedicated section in `docs/architecture/alternative-analysis.md`.
The selected pure domain ledger is intentionally smaller than a direct API or
on-chain implementation so arithmetic and replay invariants are executable
before introducing persistence, authorization, or token movement.

## Consequences

Positive: corrections are explainable, replayable, idempotent, and compatible
with immutable financial evidence. The model supports negative refund flows
without allowing negative balances or floating-point arithmetic.

Cost: an adjustment requires a separate approval/posting workflow and more
read-model work. The first slice does not claim refunds are operationally
available; the readiness gate remains open until database, API, UI, chain, and
reconciliation evidence exist.

## Verification

- Domain examples cover every adjustment kind and lifecycle branch.
- Property tests cover conservation, signed bounds, duplicate identity, and
  cumulative non-negative balances.
- Canonical digest vectors will be added before any cross-language client.
- Follow-on ADRs must cover durable storage/API authorization, UI review,
  on-chain posting, reconciliation, and chargeback evidence retention.
