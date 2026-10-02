# ADR-0258: FX evidence replay integrity and shared signer ownership

- Status: Implemented as bounded primitives; durable quote binding is defined by ADR-0259
- Date: 2026-10-02
- Owners: Principal Architect, Security/Governance, Tax/Legal
- Scope: `fx-evidence` verification and `fx-provider` async signer ownership

## Context

FX evidence is issued under a bounded freshness window, but a persisted tax
quote may be replayed after that window has elapsed. Replay must prove that
the historical signed envelope, merchant binding, facts binding, canonical
payload digest, and Ed25519 signature are intact without treating an expired
quote as if it were currently issuable. The issuer also needs to share one
HSM/KMS-backed signer across concurrent requests without copying private-key
material or requiring an adapter to implement a synthetic `Clone` contract.

## Decision

1. Add a replay-oriented `verify_integrity` operation to `FxEvidenceEnvelope`.
   It validates the immutable envelope structure, merchant/facts binding,
   canonical payload digest, and signature against the active provider key
   supplied by the caller. It
   deliberately does not evaluate `observed_at_unix`, `expires_at_unix`, or
   current time. The existing `verify_with_key_at` operation remains the
   issuance/acceptance operation and continues to enforce future-skew and
   expiry.
2. Implement `AsyncFxEvidenceSigner` for `Arc<S>` when `S` implements the
   signer port. A coordinator may therefore hold an `Arc<dyn
   AsyncFxEvidenceSigner>` and share a signer adapter safely; the port does not
   require cloning or exporting private key state.
3. Keep durable storage of the complete envelope, evidence access auditing,
   and quote replay integration as a separate application/storage change. A
   conversion digest alone is not sufficient proof for that future boundary.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Reuse `verify_with_key_at` for replay | Rejected | Historical quotes would fail replay solely because their validity window elapsed. |
| Trust the stored conversion fields and digest | Rejected | It cannot prove the canonical payload or signature was not altered. |
| Require every signer adapter to implement `Clone` | Rejected | It couples the port to key ownership and is incompatible with non-exportable HSM/KMS sessions. |
| Verify integrity without time and share signers through `Arc` | Selected | Separates historical evidence integrity from current issuance policy and preserves one shared custody boundary. |

## Invariants

- Replay integrity never authorizes a new quote, settlement, withholding, or
  legal conclusion.
- Issuance still requires the configured provider-key/policy checks and a
  bounded current-time validity window.
- The integrity path binds the envelope to the authenticated merchant digest
  and the exact pre-FX facts digest.
- The shared signer path performs no key export, no retry, and no unbounded
  task creation.

## Verification

- Failing-first vectors cover valid historical replay, payload tampering,
  signature tampering, merchant/facts substitution, and expired issuance.
- Coordinator tests prove that an `Arc`-owned signer can issue evidence and
  that the signer is not required to be cloned.
- `cargo test --locked -p fx-evidence -p fx-provider` and the repository gate
  must pass before this ADR is considered implemented.
