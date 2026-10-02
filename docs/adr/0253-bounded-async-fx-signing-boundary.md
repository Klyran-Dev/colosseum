# ADR-0253: Bounded async FX signing boundary

- Status: Accepted for implementation
- Date: 2026-10-02
- Scope: HSM/KMS-compatible signing for server-owned FX evidence

## Context

The FX issuer currently exposes a synchronous signer port. That is useful for
deterministic local tests, but a production HSM/KMS signer is an external
boundary with network latency, throttling, transient errors, key rotation, and
operator-controlled availability. Calling it synchronously from an async API
would risk blocking the Tokio executor and would leave the signing deadline
implicit.

The signer must never receive user private keys from the browser, and the
issuer must not accept a signature merely because a signer returned bytes. The
canonical payload, key identity, public key, signature shape, and full evidence
envelope must still be verified before issuance.

## Decision

Add an object-safe `AsyncFxEvidenceSigner` port and a bounded `issue_async` /
`issue_candidates_async` path to `fx-provider`.

1. The async signer supplies only a server-owned key ID, public key, and async
   signing operation. Private key material remains inside the HSM/KMS adapter.
2. `FxSigningConfig` requires a positive timeout and caps it at five seconds;
   the default is 500 ms. Timeout is applied around the signer future and maps
   to `SigningDeadlineExceeded`.
3. The issuer prepares one canonical `UnsignedFxEvidence` payload, calls the
   signer exactly once, rejects malformed signatures, and verifies the final
   envelope with the supplied public key before returning evidence.
4. Policy, merchant/facts binding, legal acceptance, provider identity,
   freshness, deviation, and agreement validation occur before signing. A
   timeout or signer error produces no issuance result.
5. The synchronous signer path remains for local deterministic fixtures and
   adapters that are proven non-blocking. Production adapters must use the
   async path or an explicitly reviewed blocking isolation boundary.
6. Retry, circuit breaking, key rotation, audit publication, and provider
   authentication remain adapter/control-plane responsibilities. The issuer
   does not retry a signing operation because duplicate remote signing attempts
   can create ambiguous custody and audit behavior.

## Invariants and failure policy

- The signer cannot change the canonical payload or financial fields.
- A zero key ID/public key, malformed signature, invalid public key, failed
  envelope verification, timeout, or signer error fails closed.
- No external signer call occurs when policy, request, or candidate validation
  fails.
- No raw private key, secret, provider response, or credential is logged or
  persisted by the issuer.
- The issuer remains independent of a specific HSM/KMS vendor and does not
  claim that a local test signer is custody evidence.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Keep synchronous signer for all adapters | Minimal API change | Blocks async executors or requires hidden blocking pools; no explicit deadline | Rejected |
| Put HSM/KMS HTTP calls in the tax engine | Fewer boundaries | Makes pure calculation nondeterministic and couples tax to custody | Rejected |
| Accept signer bytes without local verification | Lower latency | Allows identity, payload, and signature substitution | Rejected |
| Queue signing through a broker | Durable retry | Adds expiry/idempotency complexity and is too slow for quote evidence | Rejected |
| **Async signer port with explicit deadline and local envelope verification** | Non-blocking, vendor-neutral, bounded, auditable, and fail-closed | Requires adapter-specific custody and rotation implementation | **Selected** |

## Verification

Tests must prove successful async issuance, policy rejection before signer use,
signing timeout, signer error, malformed signature, public-key mismatch, exact
payload binding, and no duplicate retry. Production HSM/KMS ceremony, key
rotation/revocation, regional latency, recovery, audit delivery, and custody
approval remain release gates.
