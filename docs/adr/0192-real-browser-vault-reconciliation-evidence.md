# ADR-0192: Real-browser vault-reconciliation evidence contract

- Status: Accepted
- Date: 2026-09-29
- Deciders: Principal Low-Latency Architecture, Finance/Tax, Security, and UX
- Scope: browser evidence for the tenant-scoped read model in ADR-0128

## Context

ADR-0128 defines a read-only, tenant-scoped reconciliation workspace. Unit
tests and the Docker indexer drill prove parsing, authorization, persistence,
and scheduled observations, but they do not prove that the shipped static SPA
can load the workspace, submit a memory-only access token, render server-owned
evidence, or stop visibly on a mismatch in a real browser.

This distinction matters operationally: a browser test must not turn synthetic
fixtures into production-chain evidence, and a visual status must never be
derived from a caller-controlled amount or a stale client cache.

## Decision

Extend the existing pinned Playwright browser journey with a bounded,
deterministic contract test for the production SPA:

1. Mock only the authenticated read API at the browser network boundary; keep
   the real built SPA, lazy import path, focus management, and accessibility
   checks active.
2. Return a server-shaped target with exact decimal-string balances and a
   latest mismatch run, then assert the UI renders the operational stop
   signal and immutable target identity.
3. Return bounded append-only run history and assert it is requested only
   after the operator selects a target.
4. Return a 403 response with a support-safe error and assert the workspace
   fails closed without retaining or inventing target state.
5. Keep live wallet, live provider, Linux validator, production identity, and
   production alert evidence as separate release gates. This test proves the
   browser contract, not chain finality or legal compliance.

The test must use synthetic UUIDv7 identifiers, no credentials, no raw tax
evidence, and no browser mutation endpoint. It runs through make
web-browser-check and is retained with the existing browser evidence.

## Invariants and budgets

- The browser cannot create or edit reconciliation targets, opening balances,
  runs, releases, or funds.
- Exact atomic values remain decimal strings at the API boundary.
- A mismatch is rendered as a stop condition and can never be displayed as
  matched by the client.
- A 401/403 response clears the target projection and exposes an actionable,
  non-secret error.
- The journey has the existing 30-second test timeout and one browser worker;
  it must not add polling or unbounded retries.
- Mocked API responses are explicitly labelled as browser-contract evidence;
  they do not close live-provider or production-readiness gates.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Unit/component tests only | Fast and deterministic | Does not exercise the built SPA, lazy loading, browser focus, or network boundary | Rejected as the sole gate |
| Full live-provider browser test in CI | Closest to production | Requires wallet/provider credentials, unstable external state, and would mix chain evidence with UI regression evidence | Rejected for the deterministic browser gate |
| Browser test against a fake success page | Simple | Proves neither API contract nor fail-closed behavior and can hide unsafe UX | Rejected |
| **Real built SPA with bounded network fixtures and explicit failure assertions** | Exercises shipped browser behavior while keeping evidence reproducible, safe, and fast | Does not replace live-provider or validator evidence | **Selected** |

## Verification and rollout

- Add the journey to apps/web/e2e/ui-journeys.spec.ts.
- Run pnpm --dir apps/web typecheck:e2e and make web-browser-check.
- Retain live-provider, wallet, Linux validator, production identity, and
  reconciliation SLO evidence as separate release gates in the readiness plan.
