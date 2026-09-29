# ADR-0184: Typed finalized adjustment proof envelope

- Status: Accepted — implemented and locally verified; production adoption pending
- Date: 2026-09-29
- Deciders: Principal Low-Latency Architecture, Settlement, Indexing, and UI
- Scope: Wallet-bound settlement-adjustment evidence read and finality UX

## Context

The adjustment chain-intent reconciler already queries finalized raw evidence,
requires the exact `SettlementAdjustmentRecorded` discriminator, and compares
the complete indexed event against the immutable adjustment/evidence intent
before transitioning the durable intent to `finalized`. The public response,
however, currently returns only `status` and `signature`. A browser can
therefore receive a finalized status without receiving a typed proof envelope
that it can display or validate. That weakens recovery UX and makes it harder
to prove that a finalized state is not merely transport confirmation.

## Decision

Persist a compact finalized-proof binding on the adjustment chain intent:

- finalized event index;
- finalized event evidence digest;
- the existing server-bound signature, finalized slot, and commitment; and
- the immutable intent/evidence fields already checked by the storage query.

Expose a `finalized_evidence` object in the adjustment-chain-intent API
response only when the stored status is `finalized`. The object contains the
typed event discriminator/version, signature, finalized slot, event index,
adjustment PDA, original receipt, merchant, mint, nonce, adjustment digest,
adjustment evidence digest, event evidence digest, and approver. The API
constructs it only after the storage boundary has matched the complete typed
event; it is evidence, not an authority to move funds.

The React parser rejects a `finalized` response that lacks a valid proof
envelope or whose proof commitment/discriminator is invalid. All non-finalized
states retain the existing bounded transport response. The UI labels the
state “finalized typed event evidence” and continues to state that it is not
payment, filing, or legal compliance.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Return only `status=finalized` | Small response and no migration | Browser cannot distinguish typed proof from a status projection | Rejected |
| Return an unsigned client-supplied proof | Flexible recovery | Caller-controlled evidence and substitution risk | Rejected |
| Re-query indexed evidence on every browser render | Avoids new columns | Adds latency, creates an availability dependency, and still lacks a stable response contract | Rejected |
| **Persist event index/digest and return a server-owned typed proof envelope** | Refresh-safe, replayable, bounded, and explicit about the exact finality evidence | Requires an additive migration and cross-language parsing vectors | **Selected** |

## Invariants

- A chain intent cannot be `finalized` without signature, finalized slot,
  `finalized` commitment, event index, and a non-zero event evidence digest.
- Finalized proof fields are written only by the exact event-match path.
- The proof signature and slot equal the durable chain intent signature and
  slot; the event index and digest are immutable after finalization.
- A `finalized` API response always contains a valid typed proof envelope.
- A non-finalized API response never implies payment or legal compliance.
- Tenant scope remains enforced by the API and storage query.
- The proof envelope cannot authorize token movement or replace Solana-native
  finality evidence.

## Verification and release impact

- Rust storage tests cover finalization proof-field validation and immutable
  binding.
- API tests cover proof emission only after exact finalized-event matching.
- TypeScript parser/UI tests reject a status-only finalized response and render
  the proof-backed state.
- Migration `0048` and `db/verification/adjustment_finalized_proof.sql` verify
  the additive PostgreSQL contract and partial finalized-proof index.
- `make integration-check` passes on a clean PostgreSQL 18 Docker stack,
  including the adjustment lifecycle, migration contract, two-replica
  settlement fencing, ambiguous/crash recovery, and rollback checks.
- Validator, live RPC/wallet, production SLO, and full browser accessibility
  evidence remain separate release gates.
