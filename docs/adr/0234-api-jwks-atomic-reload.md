# ADR-0234: Atomic API JWKS reload for bounded identity rotation

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Platform Security, SRE
- Bounded contexts: Identity, API Runtime, Deployment Operations

## Context

The API already verifies OIDC-compatible EdDSA JWTs locally with explicit
issuer, audience, lifetime, scope, role, and merchant binding checks. Its
configured JWKS is immutable after startup, which makes a production key
rotation depend on a process restart. Restart-only rotation expands the
failure window and makes an emergency issuer-key response compete with
deployment scheduling.

The request path must remain free of filesystem, network, or identity-provider
round trips. Rotation must never replace a valid key snapshot with malformed
or incomplete material, and an invalid replacement must not make the API
accept unauthenticated requests or discard the last known-good snapshot.

## Decision

Add an optional `API_AUTH_JWKS_FILE` deployment input. When configured, the API
loads a bounded JWKS document from that file at startup and listens for Unix
`SIGHUP`. A deployment controller must atomically replace the file, then send
one `SIGHUP` to the intended process. The reload path:

1. reads and bounds the complete file in a blocking filesystem task outside
   the async request executor;
2. parses and validates every key using the existing JWKS authority parser;
3. atomically swaps the immutable `Arc<HashMap<kid, Ed25519 key>>` only after
   complete validation; and
4. retains the previous snapshot on every read, parse, size, or key-validation
   failure.

The request path obtains the current immutable snapshot with a lock-free
atomic load. It does not read the file, perform a network request, or await a
reload. A successful rotation must publish an overlapping key set during the
configured token lifetime plus clock skew; the identity provider remains
responsible for issuing and revoking tokens. Removing a key stops acceptance
of tokens signed by it after the atomic swap, subject to already accepted
requests and the provider's token policy.

`API_AUTH_JWKS_JSON` and the bounded compatibility key pair remain supported
for startup-only/local compatibility. A file and an inline representation
cannot be configured together. Invalid reloads are observable through
structured logs and a fixed-cardinality counter, but log output never contains
JWKS contents, token material, subjects, merchants, or public-key values.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Restart-only rotation | Rejected | Increases rotation and emergency-response downtime; does not provide atomic last-known-good behavior. |
| Fetch JWKS from the identity provider on requests | Rejected | Adds network latency and an availability dependency to the authorization hot path. |
| Periodic background JWKS polling | Rejected for the first rotation contract | Requires stale-cache, backoff, provider-authentication, and split-brain policy before it can be safe; it also creates hidden external load. |
| `RwLock` around a mutable key map | Rejected | Adds request-path lock contention and makes the hot-path cost dependent on reload behavior. |
| **Atomic immutable snapshot plus operator-triggered file reload** | **Selected** | Zero request-path I/O, bounded validation, deterministic failure behavior, explicit deployment ownership, and measurable rotation evidence. |

## Invariants and failure behavior

- The API never serves a partially parsed or partially validated key set.
- Every accepted token is checked against one immutable key snapshot.
- An invalid, missing, oversized, or unreadable replacement retains the last
  known-good snapshot and never enables an unauthenticated fallback.
- A reload never logs key bytes, bearer tokens, JWT claims, or tenant identity.
- At most one reload is processed per delivered signal; filesystem work is
  bounded by the document-size limit and runs outside the async executor.
- Startup fails closed when a configured JWKS file cannot be read or parsed.
- File-based and inline JWKS configuration are mutually exclusive.
- Key overlap, revocation timing, secret-manager delivery, permissions,
  process targeting, and operator approval remain deployment responsibilities.

## Verification and release impact

- Unit tests prove key-snapshot replacement, valid rotation, invalid reload
  retention, duplicate-key rejection, and token acceptance/rejection across a
  rotation.
- A runtime contract will prove a real API process accepts the original key,
  atomically loads a replacement, rejects the removed key, and retains the
  original snapshot after an invalid replacement.
- Production still requires provider-managed key rotation, secret-manager
  atomic delivery, file permissions, revocation/overlap timing, alerting,
  regional reload evidence, and an approved identity-provider ceremony.
