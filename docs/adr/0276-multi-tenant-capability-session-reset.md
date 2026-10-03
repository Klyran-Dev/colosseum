# ADR-0276: Multi-tenant capability session reset evidence

- Status: Accepted
- Date: 2026-10-03
- Owners: Product, Security, Frontend

## Context

The React shell receives an opaque session credential and replaces its
server-owned capability snapshot whenever that credential changes. The browser
must never decode claims or retain the previous tenant's account reference,
subject, or capability decisions while the replacement request is pending.

Unit tests cover parsing and the request header, but a production UI claim also
needs real-browser evidence across the observable transition from one tenant to
another. This is a privacy and authorization boundary: stale tenant context can
mislead an operator even when every protected API operation still rejects the
wrong tenant.

## Decision

Add a real-browser journey that intercepts only the capability endpoint with
bounded synthetic server responses for two different opaque credentials. The
journey proves that:

- tenant A's server-confirmed account reference and capability matrix render;
- replacing the credential immediately returns the shell to an unconfirmed
  state while the new request is pending;
- tenant B's server-confirmed account reference and decisions replace tenant A's
  state; and
- tenant A's account reference is no longer rendered after the replacement.

The test does not treat a browser credential as proof, does not call a real
identity provider, and does not authorize an API operation. API tenant
derivation and re-authorization remain server responsibilities.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Parser unit test only | Fast and deterministic | Cannot prove React state replacement or rendered stale-context removal | Rejected |
| Browser test that keeps one tenant | Proves initial rendering | Misses session replacement and cross-tenant leakage | Rejected |
| Decode JWT claims in the browser to switch context | Easy fixture setup | Violates the server-owned authorization boundary | Rejected |
| **Real-browser two-tenant capability-response journey** | Exercises network, state reset, rendering, and stale-context removal | Uses synthetic capability responses and is not identity-provider evidence | **Selected** |

## Invariants and acceptance criteria

- Capability responses are bounded and validated using the production parser.
- The browser shows no tenant A account reference after tenant B is confirmed.
- The test verifies the bearer header changes with the session credential and
  does not assert authorization from token contents.
- The journey remains separate from live identity-provider, multi-tenant API,
  screen-reader, and staging evidence gates.

