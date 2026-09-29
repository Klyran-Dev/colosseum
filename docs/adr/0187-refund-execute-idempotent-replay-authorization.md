# ADR-0187: Refund-execute idempotent replay authorization

## Status

Accepted — required by the authenticated API contention gate.

## Context

Refund-execute preparation is intentionally idempotent. Concurrent callers
using the same merchant-scoped key must receive the same durable artifact after
the first transaction commits. The artifact's authority_subject field
contains the canonical approver public key used in Solana account metas; it is
not the OIDC subject string. Comparing those different identity domains caused
valid idempotent replays to fail with 409 under contention.

The replay path is also an authorization boundary. A role-bearing principal
from the same merchant must not replay another approver's intent merely because
the idempotency key is known.

## Decision

On every refund-execute replay, load the immutable stored adjustment and require
both bindings:

1. authority_subject equals the durable intent's compiled approver public key;
2. the adjustment's durable approver_subject equals the authenticated JWT
   subject for the current merchant.

Apply the same helper to prepare replay, intent reads, and submit reads. Keep
Solana public keys and OIDC subjects as distinct typed concepts at the boundary;
do not rename or reinterpret an existing artifact field without a versioned
artifact-schema migration.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Compare the compiled public key to the JWT subject | No extra query | Compares different identity domains and rejects valid replays | Rejected |
| Trust the merchant role alone | Lowest replay latency | Any merchant approver can replay another approver's durable intent | Rejected |
| Add an approver-subject column to execute intents immediately | Explicit durable binding | Expand/contract migration and artifact/storage version coordination for a defect that can be closed by existing immutable adjustment evidence | Deferred; revisit if replay authorization needs independent intent ownership |
| **Validate compiled approver key plus immutable adjustment approver subject** | Correct identity domains, no schema migration, preserves authorization and artifact integrity | One bounded PostgreSQL lookup on replay/read/submit paths | **Selected** |

## Invariants and budgets

- A successful replay returns the same intent ID, request digest, artifact
  digest, and unsigned artifact as the committed creation.
- A caller cannot replay an intent for another merchant or another approved
  adjustment approver subject.
- A tampered artifact approver key is rejected as an idempotency conflict.
- The replay authorization query uses the existing tenant-scoped primary-key
  lookup and remains inside the API database deadline; it never calls Solana or
  an external provider.
- The execute-intent write, first lifecycle event, and first outbox event
  remain one serializable transaction.

## Verification and rollout

- cargo test -p api --lib covers the API guard suite.
- make refund-execute-api-contention-drill drives authenticated claim and
  finalized execute preparation through 16 concurrent same-key requests and
  verifies 16 HTTP 200 responses, one intent, one lifecycle event, one outbox
  row, and one response digest.
- The drill uses synthetic server-owned finalized evidence and does not close
  live wallet, provider, validator, retry-telemetry, or production SLO gates.
