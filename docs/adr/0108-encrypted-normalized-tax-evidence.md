# ADR-0108: Encrypted normalized tax evidence and jurisdiction-data minimization

- Status: Accepted
- Date: 2026-09-27
- Owners: Tax Platform, Security, Privacy, and SRE
- Scope: Persisted quote replay facts and server-side jurisdiction evidence handling

## Context

The Tax Engine engine must replay a quote from normalized facts, but facts can
contain customer, address, exemption, and jurisdiction-sensitive information.
The provider-signed jurisdiction envelope is an input to resolution, not a
durable tax record: retaining the raw envelope would duplicate potentially
sensitive evidence and increase breach and retention scope. The current
database replay column is JSONB and therefore readable by any database reader
with table access.

The system needs deterministic replay without making PostgreSQL or application
logs a source of plaintext sensitive evidence. Encryption must happen before
the database write, bind ciphertext to the merchant and facts digest, support
key rotation, and fail closed when a production key is unavailable.

## Decision

1. Persist only the normalized `TaxFacts` required for replay. Raw provider
   jurisdiction envelopes are verified in memory, reduced to the selected
   jurisdiction, policy digest, source, bounded decision trace, and evidence
   digests, and then discarded. Request bodies and signed evidence are never
   logged.
2. Encrypt normalized replay facts in the storage adapter using an AEAD
   envelope with a versioned key identifier, random nonce, and associated data
   containing the schema version, merchant UUID, and facts digest. Store
   ciphertext, nonce, and key ID in separate bounded columns. New rows do not
   populate plaintext `facts_document`.
3. Keep `facts_document` as a nullable expand/contract compatibility column for
   legacy rows only. Replay of a legacy plaintext row is allowed only during
   the documented migration window; new production quote writes fail closed if
   an evidence cipher is not configured.
4. Inject a cipher/key-provider port at process startup. Local Docker uses a
   synthetic key supplied through an explicitly local-only environment variable.
   Production uses a KMS/HSM-backed data-encryption-key provider; raw keys are
   never persisted in PostgreSQL, emitted in logs, or accepted from HTTP.
5. Audit access to replay evidence using the authenticated merchant, actor
   subject, correlation/request ID, calculation ID, purpose, and allow/deny
   outcome. The audit record contains digests and identifiers, never facts or
   raw jurisdiction evidence.

## Invariants and failure behavior

- A ciphertext cannot be replayed under another merchant or facts digest.
- Missing, unknown, revoked, or expired keys fail replay closed.
- Authentication, tenant scope, and purpose authorization occur before
  decryption; unauthorized callers cannot use decryption as an oracle.
- AEAD authentication failure is an evidence-integrity failure, not a retryable
  database or provider error.
- Key rotation creates a new key ID and re-encrypts through a bounded,
  auditable migration job; it never mutates the facts digest or rule evidence.
- Encryption adds no RPC call and no synchronous external provider call to the
  settlement compile path; quote/replay storage uses a warm in-process key
  handle with bounded CPU and memory.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Store raw jurisdiction envelopes in PostgreSQL | Rejected | Excess retention, unnecessary PII duplication, and wider access scope. |
| PostgreSQL `pgcrypto` with a database-held secret | Rejected | Database compromise and application/database privilege boundaries collapse; key rotation and KMS audit are weak. |
| Transparent disk/database encryption only | Rejected as sole control | Protects media, but not plaintext visible to database roles, backups, queries, or application projections. |
| Plaintext normalized JSONB forever | Rejected | Violates least-privilege evidence handling and makes accidental export materially worse. |
| **Application AEAD with injected KMS/HSM-backed key provider** | **Selected** | Binds evidence cryptographically before persistence, supports rotation and audit, keeps replay deterministic, and preserves a local synthetic path. |

## Budgets and verification

- Add at most one bounded AEAD operation to quote persistence and one to replay.
- Bound encrypted evidence payloads to the existing normalized-facts limit;
  reject oversized facts before encryption.
- Unit tests cover round-trip, merchant/digest associated-data substitution,
  tampering, missing key, key ID mismatch, and legacy-row fail-closed policy.
- Docker integration proves new quote rows contain ciphertext and no plaintext
  facts, replay still verifies, and a direct plaintext mutation cannot produce
  a valid replay. The replay endpoint appends both allowed and denied access
  outcomes to `evidence_access_audit_events`; the audit row is digest-only.
- Backup/restore retains ciphertext and key IDs but never includes the key
  material; key-recovery drills remain a production release gate.

## Consequences

Replay requires a configured evidence-key capability, and operational key
rotation becomes a first-class runbook. Database support tooling can inspect
digests and status but cannot inspect customer facts without the audited API
path. Existing legacy rows need a bounded migration or explicit retirement
before the plaintext compatibility column can be removed.
