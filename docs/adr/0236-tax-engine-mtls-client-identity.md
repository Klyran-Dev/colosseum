# ADR-0236: Tax Engine mTLS client identity binding

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Security, SRE
- Scope: Tax Engine service-to-service authorization
- Bounded contexts: Tax Engine, Service Runtime, Workload Identity

## Context

ADR-0227 requires direct Rustls mutual TLS and ADR-0233 proves that a client
certificate chains to the configured CA. Chain validation alone identifies a
certificate authority, not the authorized workload. A certificate issued by a
trusted CA but intended for another internal service must not reach the Tax
Engine, even when it also presents the bearer token.

The identity check must not add a request-path network call, database lookup,
or unbounded parsing. It must also avoid trusting a client-supplied HTTP
identity header and must not place raw certificate identities in telemetry.

## Decision

Require a bounded exact URI-SAN allow-list for every non-local Tax Engine
deployment. The allow-list is supplied as a JSON array in
`TAX_ENGINE_MTLS_ALLOWED_CLIENT_URIS`, for example:

```text
["spiffe://solana-tax-protocol/api"]
```

The service uses a Rustls `ClientCertVerifier` wrapper. Rustls first validates
the certificate chain, validity, and client-auth usage through the standard
`WebPkiClientVerifier`; the wrapper then parses only the end-entity
certificate and requires exactly one URI SAN whose value is an exact byte-for-
byte member of the immutable configured allow-list. Common Name, DNS SAN,
email SAN, headers, and bearer-token claims are not identity substitutes.

The configuration is validated before the listener binds:

- one to 32 unique identities;
- each identity is 1 to 256 ASCII bytes, has no control or whitespace bytes,
  and uses the `spiffe://` scheme; and
- the encoded environment value is bounded to 8 KiB.

The identity policy is part of the same immutable startup/reload policy as
the CA bundle. Certificate rotation keeps the same URI identity; changing the
authorized workload set requires a reviewed deployment restart. A trusted
certificate with a missing, multiple, malformed, or unlisted URI SAN is
rejected during the TLS handshake, before HTTP authentication and calculation.
The existing bearer token remains defense in depth and is still required by
protected application routes.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Bearer token only | Rejected | Shared-secret replay and workload confusion remain, especially after a trusted-network misroute. |
| CA-chain validation only | Rejected | Any certificate issued by the trusted CA is accepted regardless of workload. |
| Subject/CN allow-list | Rejected | Subject naming is less interoperable and CN is not a reliable modern workload identity field. |
| **Exact URI-SAN allow-list at the Rustls handshake** | **Selected** | Standards-based workload identity, fail-closed authorization, no request-path I/O, and bounded constant-size policy. |
| External identity introspection per request | Rejected | Adds latency and availability dependencies to the calculation path and creates a new authority boundary. |

## Invariants and failure behavior

- Non-local startup fails if the allow-list is absent, malformed, duplicated,
  empty, oversized, or contains an unsupported identity format.
- The identity decision is made from the authenticated end-entity certificate
  presented in the TLS handshake, never from an HTTP header or client digest.
- Certificate-chain validation remains mandatory; a matching URI SAN cannot
  bypass trust, expiry, key usage, or signature checks.
- A rejected client certificate never reaches bearer authorization or tax
  calculation handlers.
- Reload preserves the last known-good complete TLS and identity policy on
  every validation error.
- Logs and metrics use fixed-cardinality reason codes only; URI values and
  certificate subjects are never emitted.

## Verification and release impact

- Unit tests cover valid identity, unlisted identity, missing/multiple URI
  SANs, malformed identities, duplicate policy entries, and bounded limits.
- Rustls handshake tests cover trusted-but-wrong workload rejection before
  HTTP authentication and trusted matching workload acceptance.
- The real-container mTLS contract covers matching and wrong-identity clients
  with the pinned image and retains existing bearer, ALPN, and reload checks.
- Production certificate issuance, secret-manager delivery, identity rotation,
  CA overlap/revocation, regional latency, and alert/on-call evidence remain
  deployment release gates.
