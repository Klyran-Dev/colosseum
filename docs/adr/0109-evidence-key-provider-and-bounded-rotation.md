# ADR-0109: Evidence key provider port and bounded key rotation

- Status: Accepted
- Date: 2026-09-27
- Owners: Security, Tax Platform, and SRE
- Scope: Application encryption of normalized tax replay evidence

## Context

ADR-0108 requires application encryption before PostgreSQL persistence. A
single process-local key is sufficient for a local Docker fixture but is not a
production key lifecycle: production needs KMS/HSM-backed material, active and
previous key resolution during rotation, bounded memory, and fail-closed
behavior when a referenced key is unavailable.

The quote path must not synchronously call a remote KMS or HSM. A remote call
would introduce an unbounded external dependency into a latency-sensitive
transaction. Key material or wrapped data-encryption keys must therefore be
loaded into a process-local provider under an explicit startup/rotation
operation and resolved synchronously from a bounded ring.

## Decision

1. `EvidenceKeyProvider` is the storage port. It exposes only `current_key`
   and `resolve_key`; it never exposes a database connection, HTTP client, or
   caller-controlled key selection.
2. `EvidenceCipher` depends on an injected provider. New ciphertext always
   uses the provider's current key ID; replay resolves the persisted key ID
   through the provider and keeps the AEAD associated data unchanged.
3. A key ring contains one current key and at most seven previous keys. Local
   Docker may use the bounded JSON ring adapter or the single synthetic key
   compatibility variables. Production adapters must preload wrapped keys or
   data-encryption keys from an approved KMS/HSM and must not persist raw key
   material in PostgreSQL, logs, HTTP requests, or repository fixtures.
4. Rotation is additive: load the new key, make it current, retain old keys
   through the bounded evidence re-encryption window, then revoke/remove old
   keys only after an audited coverage check proves no active ciphertext
   references remain. Facts digests, calculation IDs, rule digests, and audit
   records never change during rotation.
5. Unknown, revoked, malformed, or over-capacity key references fail closed.
   The service does not fall back to another key or decrypt with a caller-
   supplied key.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Call KMS/HSM for every quote/replay | Rejected | Adds remote latency, outage coupling, and retry ambiguity to the hot path. |
| Keep one immutable API environment key forever | Rejected | No bounded rotation, no previous-key replay window, and excessive blast radius. |
| Store all historical keys in PostgreSQL | Rejected | Makes the evidence database a key-custody boundary and weakens least privilege. |
| Let callers select a decryption key | Rejected | Turns key selection into an untrusted input and creates downgrade/oracle risk. |
| **Injected provider with bounded warm key ring** | **Selected** | Preserves low latency, supports KMS/HSM adapters, bounds memory, and makes rotation auditable. |

## Verification and remaining operational gate

- Unit tests cover active-key encryption, previous-key replay, unknown-key
  rejection, duplicate IDs, missing current keys, and key-ring size limits.
- Docker continues to use synthetic key material only; it is not production
  key custody evidence.
- Production remains gated on an approved KMS/HSM adapter, rotation/revocation
  runbook, key-recovery drill, coverage query, and audit-log evidence.
