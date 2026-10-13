# ADR-0233: Real-container Tax Engine mTLS contract

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Tax Engine, Security, SRE
- Scope: Tax Engine deployed-image transport boundary
- Bounded contexts: Tax Engine, Service Runtime, Workload Identity

## Context

ADR-0227 and ADR-0228 define direct Rustls mutual TLS and bounded Unix
`SIGHUP` reload behavior for staging and production. Unit tests prove trusted
and untrusted Rustls handshakes and atomic reload semantics, while ADR-0230
proves the local plaintext container contract. The deployed image boundary
still needs an executable proof that non-local configuration cannot silently
fall back to plaintext, that the actual listener requires a client
certificate, and that the authenticated calculation path works over mTLS.

## Decision

Add `make tax-engine-mtls-runtime-contract`, an isolated Docker contract that:

- generates short-lived synthetic CA, server, trusted-client, and
  untrusted-client certificates outside the image;
- starts the real pinned Tax Engine image with `TAX_ENGINE_ENV=staging`, a
  non-local synthetic service token, server certificate/key, and client-CA
  paths mounted read-only;
- proves unauthenticated TLS clients cannot reach `/healthz`, while a trusted
  client reaches it successfully;
- proves trusted-client `/readyz` still requires the bearer token and that an
  authenticated calculation returns exact aggregate tax/net values;
- proves a client certificate signed by an untrusted CA is rejected;
- proves a trusted certificate with an unlisted URI-SAN workload identity is
  rejected before HTTP authentication;
- negotiates the configured `h2` ALPN protocol with a trusted client; and
- replaces the mounted CA file atomically, sends `SIGHUP`, and proves both an
  invalid replacement (last known-good configuration retained) and a valid
  replacement (reload remains usable) do not interrupt trusted health checks.

The contract uses only synthetic ephemeral keys, binds to loopback, owns an
isolated Compose project, retains no credentials, and removes only its own
container/network/volume state. It is transport and image evidence; it does
not prove production secret-manager delivery, certificate issuance, CA
revocation, regional reload latency, or consumer identity governance.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Rustls unit tests only | Fast and deterministic | Does not prove image paths, process startup, listener, or signal delivery | Rejected as the sole gate |
| Plaintext runtime contract only | Simple and portable | Cannot detect accidental non-local plaintext fallback or missing client authentication | Rejected |
| Sidecar/proxy-only mTLS test | Matches one deployment topology | Leaves direct Rustls configuration and image behavior unverified | Rejected as the service contract |
| **Ephemeral certificate authority plus real pinned-container mTLS probe** | Exercises deployed binary, certificate files, client verification, ALPN, and reload with public-safe cleanup | Requires Docker and host OpenSSL | **Selected** |

## Invariants and failure behavior

- Non-local Tax Engine startup requires all three TLS files and never binds a
  plaintext listener.
- A missing or untrusted client certificate cannot reach the HTTP application.
- Trusted transport does not bypass bearer authorization for readiness or
  calculation endpoints.
- Invalid replacement material cannot displace the last known-good Rustls
  configuration.
- No synthetic key, token, certificate body, or raw tax fact is retained in
  evidence output.
- Any health, handshake, ALPN, calculation, reload, or cleanup failure exits
  non-zero.

## Verification and release impact

- The shell contract is syntax-checked, public-repository checked, and wired
  into `make check` and a dedicated CI job.
- The existing focused Rustls tests remain necessary; this contract adds image
  and process evidence rather than replacing them.
- Production secret-manager, certificate lifecycle, revocation/CA overlap,
  identity issuance/rotation, regional SLO, and on-call evidence remain
  release gates.
