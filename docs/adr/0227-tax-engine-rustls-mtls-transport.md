# ADR-0227: Rustls mTLS transport for the Tax Engine service

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Security, SRE
- Scope: Tax Engine service-to-service transport
- Bounded contexts: Tier-1 Tax Engine, Service Runtime, Platform Security

## Context

The warm Tax Engine adapter is an internal service boundary. A bearer token
alone does not provide transport confidentiality or workload identity, and a
plaintext non-local listener would violate the service's production security
boundary. NGINX terminates browser-facing TLS, but it is not the authority for
the internal Tax Engine listener and should not be inserted into the quote or
settlement hot path.

## Decision

Use the pinned Rustls/axum-server stack for the Tax Engine listener:

- local development uses the existing plain HTTP listener inside the isolated
  Compose network;
- staging and production require `TAX_ENGINE_TLS_CERT_FILE`,
  `TAX_ENGINE_TLS_KEY_FILE`, `TAX_ENGINE_MTLS_CA_FILE`, and
  `TAX_ENGINE_MTLS_ALLOWED_CLIENT_URIS` together;
- the server presents its configured certificate and requires every client to
  present a certificate chaining to the configured CA;
- ALPN is explicitly restricted to `h2` and `http/1.1` for the supported HTTP
  server paths; and
- the process explicitly installs Rustls' AWS-LC provider before constructing
  client or server configurations, because the workspace also contains a
  separate `ring` feature path for telemetry and automatic provider selection
  would otherwise be ambiguous; and
- incomplete files, malformed PEM, empty CA stores, or missing non-local TLS
  configuration fail before binding.

The bearer token remains an application-layer defense in depth and is not
treated as a replacement for mTLS identity. ADR-0236 binds the client
certificate to an exact URI-SAN allow-list during the handshake. Certificate provisioning,
rotation/reload ceremony, workload authorization policy, CA compromise
response, and production secret storage remain deployment-owned controls.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Plain HTTP plus private network policy | Rejected | Does not provide cryptographic confidentiality or workload identity at the service boundary. |
| NGINX/sidecar-only mTLS termination | Rejected for the default listener | Adds a hop and an independently configured proxy authority to a low-latency internal path; may remain a platform deployment option only when it preserves the same contract. |
| Bearer token only over HTTPS terminated elsewhere | Rejected as the service default | Token replay and misrouted workload risk remain if the internal hop is plaintext or the proxy is bypassed. |
| **Direct Rustls mTLS in the Tax Engine process** | **Selected** | No plaintext non-local fallback, explicit CA/client verification, bounded dependency footprint, and direct ownership of the service security boundary. |

For the Rustls cryptographic provider, automatic feature-based selection is
rejected because the workspace enables both AWS-LC and ring transitively. The
explicit AWS-LC provider is selected for a deterministic process-wide choice;
switching providers requires a separate compatibility and compliance review.

## Invariants and failure behavior

- A non-local process cannot bind a plaintext listener.
- Server certificates and private keys are loaded only at startup and are
  never logged or included in calculation evidence.
- A client without a valid certificate chain is rejected during TLS handshake,
  before HTTP authentication or tax calculation.
- Bearer authentication remains required for protected calculation routes.
- TLS configuration errors fail closed and never downgrade to local mode.
- TLS handshake and certificate-rotation metrics must be added only with fixed
  cardinality; peer certificate subjects and raw identities are not labels.

## Verification and release impact

- Configuration tests cover local/plain and staging/production/mTLS modes,
  partial-file rejection, and local-token rejection.
- Rustls configuration tests cover malformed certificate/key/CA material,
  explicit ALPN/client-verifier construction, explicit AWS-LC provider
  initialization when provider features are ambiguous, a trusted client
  completing the mTLS handshake, and an untrusted client being rejected before
  HTTP authentication.
- CI builds the locked service image and runs the service contract.
- Production certificate provisioning, rotation/reload, CA rotation,
  compromise response, regional mTLS load, and live SLO evidence remain
  release gates.
