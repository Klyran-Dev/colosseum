# ADR-0288: Isolated browser-test Compose project

- Status: Accepted
- Date: 2026-10-04
- Owners: Platform and Web Experience

## Context

The real-browser gate exercises the built React SPA through the NGINX edge,
API, migration runner, and PostgreSQL. Running it in the default Compose
project reuses the developer's durable PostgreSQL volume and host ports. That
creates two unsafe test properties: an already-applied local migration can
make the gate fail because its checksum has changed, and an unrelated local
stack can prevent the browser gate from starting. The migration runner must
continue to fail closed on checksum drift; the browser test harness must not
work around that invariant by mutating migration history.

## Decision

`make web-browser-check` uses a dedicated Compose project and
`compose.browser.yaml`. The browser overlay removes host-port publication for
PostgreSQL, the API, and NGINX because the Playwright container reaches the
edge over the Compose network. The target creates an ephemeral project-scoped
PostgreSQL volume, runs the real migration/API/edge/browser topology, and
removes only that test project's containers, network, and volume on exit.

The browser gate therefore proves a clean-checkout flow and remains
independent of the developer's long-lived local database, while production and
normal local development retain their durable volumes and explicit ports.

## Alternatives considered

1. Reuse the default project and volume. Rejected because stale data and
   migration checksum drift make the gate non-reproducible.
2. Delete the default project's volume before every browser run. Rejected
   because it can destroy unrelated local development evidence and hides the
   boundary between application data and test data.
3. Start only a mock API or static edge. Rejected because it would not test
   migrations, authentication boundaries, NGINX behavior, or the real API.
4. Use an isolated project with an ephemeral volume and no host ports.
   Selected because it preserves real service boundaries while minimizing
   external state and port contention.

## Invariants and acceptance criteria

- The default Compose project and its volumes are never removed by the browser
  target.
- Browser tests run through the built NGINX edge and real API/migrator.
- A fresh database applies every checked-in migration and seed fixture.
- A failed test still executes bounded cleanup for only the dedicated project.
- Migration checksum drift remains a hard failure in every environment.
- The target is safe when the normal local stack already owns its published
  ports.

## Consequences

Browser tests no longer retain database state between runs; stateful database
testing belongs in the dedicated integration and contention drills. The
target takes responsibility for cleanup, and CI must retain the Playwright
artifact directory before cleanup if failure evidence is required.
