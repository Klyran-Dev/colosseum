# ADR-0256: Bounded HTTP FX signer adapter

- Status: Implemented as a vendor-neutral transport boundary; concrete custody pending
- Date: 2026-10-02
- Decision owners: FX Provider bounded context, Platform Security, Governance

## Context

ADR-0253 defines the async signer port and requires local verification of the
returned evidence envelope, but no production transport adapter exists. The
transport must preserve the exact payload, keep private key material outside
the application, and bound response size, timeout, redirect behavior, and
credential exposure. It must not select an HSM/KMS vendor or turn a remote
signing timeout into an ambiguous retry.

## Decision

Create a separate vendor-neutral `fx-signer-http` Rust crate implementing
`fx_provider::AsyncFxEvidenceSigner`. The adapter sends one configured key
identity and one exact canonical payload to a configured signer endpoint over a
bounded HTTP request. The issuer remains the authority that verifies the
signature and complete evidence envelope.

The adapter will:

- require HTTPS for staging and production; local HTTP is allowed only for an
  explicit loopback fixture with an explicit port;
- reject endpoint credentials, query strings, fragments, missing hosts,
  unbounded endpoint/token configuration, and redirects;
- keep the bearer credential private and exclude it from errors, logs, traces,
  metrics, and `Debug` output;
- encode key ID and payload as canonical lowercase hexadecimal/base64 fields;
  the payload bytes are not normalized or reconstructed by the adapter;
- accept only a strict JSON response containing one base64 signature that
  decodes to exactly 64 bytes; unknown fields and non-success responses fail
  closed;
- apply one bounded timeout and never retry. The issuer's signing deadline
  remains the outer bound, and duplicate remote sign attempts are not created;
- expose configured key ID and public key as server-owned metadata only. The
  endpoint cannot substitute either identity.

The adapter does not generate, export, persist, rotate, revoke, or recover
private keys. HSM/KMS vendor selection, non-exportability, mTLS or equivalent
service identity, ceremony, dual control, audit delivery, regional latency,
and recovery remain release decisions in the Tier-1 decision register.

## Bounds and invariants

| Resource or invariant | Bound |
| --- | --- |
| Endpoint text | 2 KiB |
| Bearer token | 4 KiB |
| Per-request timeout | positive, at most 5 seconds; default 500 ms |
| Request body | at most 16 KiB |
| Response body | at most 4 KiB |
| Signature | exactly 64 decoded bytes |
| Retry behavior | none inside the adapter |
| Signing identity | configured server-side key ID/public key only |

## Alternatives considered

- Put HTTP in `fx-provider`: rejected because transport/custody concerns would
  leak into policy and issuance orchestration.
- Use a vendor SDK directly: rejected because vendor, region, and custody
  decisions are not frozen and would make the domain boundary non-portable.
- Queue signing through a broker: rejected because it adds expiry,
  idempotency, and duplicate-sign ambiguity to the quote path.
- Accept arbitrary signature bytes: rejected because the issuer needs a strict
  signature shape and identity-bound final verification.
- **Separate bounded vendor-neutral HTTP signer:** selected as the smallest
  executable transport boundary while preserving custody release gates.

## Verification

Fixture tests must prove exact payload transmission, key identity binding,
strict response decoding, signature length, endpoint policy, body limits,
credential redaction, redirect rejection, timeout, cancellation, and no retry.
The production gate additionally requires HSM/KMS non-exportability, key
ceremony, rotation/revocation overlap, recovery, audit, mTLS/service identity,
and regional latency evidence.
