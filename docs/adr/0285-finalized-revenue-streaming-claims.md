# ADR-0285: Finalized revenue-streaming accruals and governed release claims

- Status: Accepted
- Date: 2026-10-04
- Owners: Settlement, Tax, and Treasury Architecture

## Context

The evaluation material includes streaming net payouts to partners. The current
settlement implementation provides atomic per-transaction splits, but no
bounded context turns finalized allocations into a partner payout schedule.
Reusing the settlement endpoint with a stream cursor would create a second,
ambiguous money-moving authority and could release funds before finality or
after a tax/rule correction.

## Decision

Introduce a pure Rust `revenue-streaming` domain library. It will:

1. accept only server-verified, finalized settlement allocations;
2. record immutable, idempotent accruals bound to the source receipt, asset,
   recipient, stream-plan digest, policy/rule digest, and finalized slot;
3. prepare bounded, single-use release claims without marking them released;
4. record release only after exact instruction binding and finalized chain
   evidence are supplied by an application adapter; and
5. fail closed on conflicts, stale windows, conservation errors, duplicate
   identities, paused/cancelled plans, and bounded-cap violations.

The library has no HTTP, database, RPC, wallet, clock, filesystem, or signing
dependency. The PostgreSQL projection and authenticated API adapter are
separate boundaries with their own contracts and release evidence. The bounded
Solana plan/accrual/claim instruction and deterministic compiler are defined
in [ADR-0286](0286-on-chain-revenue-stream-claim-boundary.md). The wallet/HSM
flow, finalized proof adapter, and live validator evidence remain separate
release gates. The operator UI is a review and preparation surface; it cannot
claim a transfer.

## Alternatives considered

| Alternative | Strengths | Risks | Decision |
| --- | --- | --- | --- |
| Reuse atomic settlement for every payout | Small authority surface | No accrual schedule or partner payout semantics | Rejected |
| Off-chain balance plus direct periodic transfer | Flexible schedule | Database becomes an unsafe movement authority | Rejected |
| Continuous mutable on-chain stream first | Familiar UX | New token authority, epoch/finality and tax-correction complexity | Rejected for first implementation slice |
| **Finalized accrual ledger plus governed single-use release claims** | Finality-bound, replayable, bounded, adapter-friendly | Requires a new context and later chain/UI adapters | **Selected** |

## Security and failure behavior

- No claim is prepared from a merely submitted, processed, or confirmed
  transaction.
- Unknown submission outcomes leave the claim pending; callers reconcile the
  exact signed bytes and claim identity.
- Duplicate identical evidence is idempotent. Conflicting evidence is an
  error and is never silently overwritten.
- A negative correction cannot reduce the releasable balance below released or
  pending amounts.
- Tenant, recipient, asset, plan digest, and source receipt are checked at
  every authority-sensitive boundary.

## Consequences

The domain slice can be reviewed and fuzzed independently of a signer or
validator. Tenant-scoped persistence and API authorization are now covered by
separate adapters with durable idempotency and append-only claim evidence.
The wallet/HSM flow, finalized proof adapter, browser signing, Linux validator
conformance, audit, and live provider evidence remain explicit follow-up gates.

## Verification

Tests were written before implementation for lifecycle, conservation, finality,
replay, duplicate/conflicting evidence, correction, cap, and bounded repeated
claim preparation. The initial domain slice has nine passing tests. Storage
transition, conservation, window-binding, and completion tests are added, and
the API adapter passes Clippy. The public implementation status remains
pre-production until the chain, signer, browser, and external-evidence gates
are green.
