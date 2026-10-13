# ADR-0532 — Contain stale admin lazy-chunk failures

## Status

Accepted

## Context

The SPA uses hashed lazy chunks for route-specific admin workspaces. A browser
tab that remains open across an edge image deployment can retain the previous
entry bundle while the old chunk has been removed from the new container. The
browser then reports a raw “Failed to fetch dynamically imported module” error.

## Decision

All admin lazy route surfaces and the public checkout lazy surface will be
wrapped in the existing `WorkspaceErrorBoundary` around `Suspense`. Stale
chunk failures therefore use the reviewed recovery surface and its bounded
single-reload protection rather than exposing raw module URLs. The deployment
continues to use content-addressed immutable assets and a no-cache HTML entry;
the boundary is defense in depth for already-open tabs.

## Consequences

- Open tabs recover once after a deployment when the new entry bundle is
  available.
- Repeated chunk failures do not create an infinite reload loop.
- Users see an accessible, redacted recovery message if reload cannot repair
  the deployment mismatch.
- Asset retention or blue/green static releases remain future operational
  improvements; client recovery is not treated as a substitute for them.
