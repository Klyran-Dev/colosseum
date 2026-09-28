# ADR-0147: Lazy workspace error containment and recovery

- Status: Accepted
- Date: 2026-09-27
- Owners: Principal Low-Latency Architecture, Web Platform, SRE
- Bounded contexts: Operator UI, Checkout UI, Governance UI, Evidence UI

## Context

The SPA deliberately lazy-loads checkout, governance, replay, and vault
workspaces to keep the initial quote surface small and responsive. `Suspense`
handles a pending dynamic import, but it does not handle a rejected import or a
render-time exception. Without an error boundary, a chunk-load failure or
unexpected component exception can replace the whole application with an
uncaught browser error, leaving operators without an actionable recovery state.

## Decision

Wrap the selected workspace region in a bounded React error boundary. The
boundary:

- catches rejected lazy imports and render-time workspace exceptions;
- keeps the global navigation, service status, and display settings usable;
- exposes a semantic `role="alert"` recovery surface with a stable workspace
  label and an explicit `Reload application` action;
- reveals no exception text, stack, token, address, evidence, or other
  potentially sensitive runtime data to the user;
- resets its error state when navigation selects a different workspace; and
- treats reload as an application recovery action, never as evidence that an
  API request, wallet signature, settlement, or chain operation succeeded.

The boundary is intentionally UI-only. API timeouts, stale state, wallet
rejection, RPC ambiguity, and reconciliation-required outcomes remain owned by
their typed service/workspace state machines and are not converted into a
generic reload.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Rely on `Suspense` alone | Rejected | `Suspense` does not catch rejected imports or render exceptions. |
| Catch every error at the application root | Rejected | It hides navigation and recovery controls and makes a workspace defect a full-app outage. |
| Render raw exception messages and stacks | Rejected | Risks leaking implementation details, identifiers, or sensitive evidence. |
| Automatically retry dynamic imports in an unbounded loop | Rejected | Can create retry storms and mask a persistent broken deployment. |
| **One bounded boundary around the selected lazy workspace with explicit reload** | **Selected** | Contains failures, preserves operator context, avoids secret disclosure, and gives a deterministic recovery action. |

## Invariants and failure behavior

- A workspace render/import failure cannot claim calculation, quote, wallet,
  submission, confirmation, finality, attestation, or reconciliation success.
- The error state is keyboard reachable, screen-reader announced, and does not
  rely on color alone.
- No retry loop or background task is created by the boundary.
- Navigating to another workspace clears the previous boundary failure.
- The global application shell remains available while the workspace is failed.

## Verification and follow-up

Rendered tests force a workspace child to throw, assert the accessible recovery
surface and absence of raw exception disclosure, and verify reset behavior on a
new workspace key. Full real-browser chunk-failure, CSP, service-worker/cache,
responsive, and assistive-technology evidence remain release gates.
