# ADR-0473: Self-heal stale SPA chunk failures after deployment

## Status

Accepted

## Context

The Vite SPA emits immutable, content-hashed JavaScript chunks. A browser can
keep an older entrypoint in memory while an edge container is replaced with a
new image whose old chunks are no longer present. A later navigation then
fails to import a workspace module and the operator sees an unavailable
workspace even though the deployment is healthy.

## Decision

When the workspace error boundary receives a known dynamic-import/chunk-load
failure, perform one guarded page reload using a session-scoped recovery
timestamp. The current HTML is already served with `no-cache`, so the reload
obtains the current entrypoint. A short cooldown prevents an infinite reload
loop while allowing a later deployment in the same browser session to recover
again. If storage or reload is unavailable, retain the redacted manual recovery
surface.

## Alternatives considered

- Keep every historical hashed chunk in the image: rejected because container
  replacement has no durable asset store and retaining arbitrary history grows
  the image without a bounded retention policy.
- Disable immutable caching for all assets: rejected because it increases
  transfer cost and removes the cache correctness benefit of content hashes.
- Reload on every workspace error: rejected because application defects and API
  failures need an actionable recovery surface and must not trigger loops.
- Add a service worker: rejected because it introduces lifecycle, cache, and
  rollback complexity beyond this SPA's current deployment contract.

## Consequences

- A normal rolling image replacement can recover from a stale lazy chunk without
  operator intervention.
- The recovery is bounded by a short session cooldown and does not expose
  exception details.
- Genuine failures still render the existing localized, accessible recovery
  surface.

## Verification

- Boundary test for a known stale dynamic-import failure and one-time reload
  marker.
- Full frontend suite, typecheck, `git diff --check`, and Docker browser gate.
