# ADR-0259: Durable FX envelope quote binding

- Status: Implemented for storage and replay; authenticated API composition remains gated
- Date: 2026-10-02
- Owners: Principal Architect, Security/Privacy, Tax/Legal
- Scope: PostgreSQL quote evidence and authenticated replay

## Context

The FX provider bounded context produces a signed `FxEvidenceEnvelope`, while
the tax engine consumes a smaller exact `FxConversionEvidence` projection. A
conversion rate and digest alone are not enough for a Tier-1 audit: the
complete signed envelope, its policy/authority metadata, and its binding to the
original facts must remain available for replay. The quote transaction already
persists immutable `evidence_refs`, encrypted normalized facts, and its first
outbox record atomically.

## Decision

Persist a bounded, typed FX evidence object inside the quote row's immutable
`evidence_refs` JSON document:

- `pre_resolution_facts_digest` binds the envelope to the authenticated input
  before the server adds jurisdiction-resolution evidence or FX conversion.
- `envelope` retains the complete signed FX envelope, including schema,
  provider/signer identity, canonical payload digest, signature, exact rate,
  validity window, and evidence digest.
- The storage adapter validates identity, shape, canonical envelope integrity,
  conversion-field equality, and pre-resolution digest equality before the
  calculation row is inserted.
- The same checks run during replay. Historical replay uses integrity
  verification without treating quote expiry as a current issuance failure.
- Existing no-FX rows and legacy rows remain compatible. A row containing FX
  conversion without the complete FX evidence object fails closed during
  replay.

This keeps quote, encrypted facts, FX envelope metadata, and outbox creation in
one bounded serializable transaction. It does not make PostgreSQL authoritative
for Solana funds, and it does not allow a browser to supply FX evidence.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Persist only the projected conversion and digest | Rejected | Signature, policy identity, and canonical payload cannot be independently replayed. |
| Put the envelope in encrypted customer facts | Rejected | Couples tax-domain facts to provider wire evidence and makes evidence access/audit semantics ambiguous. |
| Add a second polymorphic FX table and join every replay | Deferred/rejected for this boundary | Strong relational shape but adds another hot-path write/join and complicates aggregate/document scope without a measured need. |
| **Use a typed bounded object in immutable `evidence_refs`** | Selected | Atomic with the quote/outbox, no hot-path join, backwards-compatible, and validated by the storage adapter. |

## Invariants

- The persisted envelope's signer/provider identity, merchant digest, facts
  digest, canonical payload digest, and signature are mutually consistent.
- The projected conversion in normalized facts is semantically equal to the
  signed envelope projection.
- The envelope is never accepted from the browser or treated as a settlement
  authority.
- Replay is tenant-scoped, does not decrypt facts before authorization, and
  does not issue a new FX quote or settlement transaction.

## Verification

- Storage unit vectors cover complete envelope persistence shape, projection
  mismatch, pre-FX digest mismatch, tampering, missing envelope, and legacy
  no-FX compatibility.
- Quote insertion includes FX evidence and its outbox in one transaction;
  replay revalidates the stored envelope and normalized facts.
- The API remains forbidden from accepting caller-owned FX evidence until the
  provider/legal/custody decision register is frozen and the server-owned
  coordinator is configured.

