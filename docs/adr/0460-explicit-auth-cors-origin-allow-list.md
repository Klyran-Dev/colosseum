# ADR-0460: Use an explicit auth-service CORS origin allow-list

- Status: Accepted
- Date: 2026-10-10
- Owners: Identity, platform, and web engineering

## Context

The local SPA can be opened at either `http://localhost:8088` or
`http://127.0.0.1:8088`. The auth service accepted the OTP request but emitted
only one `Access-Control-Allow-Origin` value. When the browser was opened with
the other valid local hostname, the response was blocked by the browser and the
UI reported only that the code could not be sent. The sign-in entry point was
therefore visibly present but unusable in a supported Docker workflow.

## Decision

Configure the auth service with a comma-separated, exact-origin allow-list.
Local Compose defaults include both documented loopback origins. Each origin
is trimmed, validated as an HTTP `Origin` header value, and installed into the
CORS layer as an explicit list. Empty entries, wildcard origins, and invalid
values fail closed at startup. Non-local deployments must provide their own
approved exact origins; this does not become an origin reflection mechanism.

## Alternatives considered

1. **Keep a single hostname.** Rejected because both loopback forms are valid
   local entry points and the failure is silent from the operator's perspective.
2. **Allow `*`.** Rejected because authentication responses must not be
   exposed to arbitrary origins.
3. **Reflect the request `Origin`.** Rejected because it turns deployment
   configuration into an ambient trust decision.
4. **Use an explicit comma-separated allow-list.** Selected because it is
   deterministic, backward-compatible with one origin, and supports multiple
   approved browser origins without weakening the trust boundary.

## Acceptance criteria

- Both documented local loopback URLs can request and receive an OTP challenge.
- Unknown origins receive no CORS permission.
- Invalid, empty, or wildcard configuration fails before the listener binds.
- The browser sign-in journey reaches the code-entry state in Docker.
- Origin parsing has unit coverage and the service remains fail-closed outside
  local development.
