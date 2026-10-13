# ADR-0516: Full React admin application rewrite with Tailwind, TanStack, and ECharts

- Status: Accepted
- Date: 2026-10-11
- Owners: Web platform and security engineering

## Context

The previous UI change replaced the outer shell but retained the existing
workflow-oriented presentation tree. That does not satisfy the requirement to
discard the existing UI and build a complete admin application from scratch
against the supplied analytics-dashboard references.

The new application must keep the browser outside the settlement authority,
while providing a coherent product surface for overview, tax operations,
policy/rule governance, treasury, access administration, and evidence. It must
use React 19, Tailwind CSS, TanStack Router/Query/Table, and Apache ECharts.

## Decision

Create a new `src/admin` application and make it the only production entrypoint
mounted by `main.tsx`:

1. React 19 owns composition and stateful interaction.
2. Tailwind owns admin layout, responsive composition, and semantic utility
   classes. The repository's semantic color tokens remain the only color
   authority; arbitrary palette literals are prohibited.
3. TanStack Router owns typed admin/auth routes and safe return paths.
4. TanStack Query owns bounded server reads, retry/deadline behavior, stale
   state, and invalidation. It never becomes an authority for money movement.
5. TanStack Table owns accessible sorting, filtering, pagination, and row
   models for administrative records.
6. ECharts renders only verified API series. Missing or stale series render an
   explicit empty/reconciliation state; the client never invents chart values.
7. Admin data adapters call the API's authorization and evidence projections,
   and call the configured auth-service origin for OTP and tenant-directory
   operations. A missing projection remains an explicit unavailable state.
8. Existing Rust/API/auth/authorization modules remain integration contracts,
   but legacy React shell and panel components are not imported by the new
   production entrypoint.

## Admin information architecture

```text
/auth/signin
/auth/signup
/admin                     overview
/admin/tax/quotes          quote operations
/admin/tax/rules           rule lifecycle
/admin/settlements         settlement review
/admin/treasury            vault/release controls
/admin/access              RBAC / ABAC / PBAC and memberships
/admin/evidence            audit and replay evidence
```

Every protected route receives a server-owned capability snapshot before
mutating controls are enabled. Route navigation is not authorization.

## Data and state contracts

- Query keys are session-bound and tenant-scoped by the bearer-authenticated API;
  the returned authorization snapshot ID and expiry are displayed and enforced
  as freshness metadata, while protected controls are disabled when the server
  snapshot expires.
- Browser state contains display preferences and short-lived session state only;
  it does not persist bearer tokens, roles, wallet keys, or evidence payloads.
- TanStack Query errors distinguish loading, stale, unavailable, timeout, and
  reconciliation-required states.
- ECharts receives typed, immutable series from API adapters. A chart with no
  verified series is an empty state, not a demo chart.
- Mutations use server-issued idempotency keys and invalidate affected query
  scopes after a confirmed response.

## Rejected alternatives

- Continue extending `ProductShell` and the existing panel tree: rejected
  because it preserves the UI being discarded and makes the visual system
  depend on legacy selectors.
- Use a chart wrapper with hidden defaults: rejected because chart state and
  empty/error behavior must be explicit and auditable.
- Use client-side arrays as dashboard fixtures: rejected because fabricated
  operational data violates the repository's no-demo-path mandate.
- Use a global state store for server records: rejected because TanStack Query
  provides bounded cache semantics, invalidation, and stale-state handling.

## Acceptance criteria

- `main.tsx` mounts only the new admin application.
- `package.json` includes React 19, Tailwind, TanStack Router/Query/Table, and
  ECharts with a committed lockfile.
- The admin routes render a responsive reference-aligned dashboard and typed
  navigation without importing the legacy shell or panels.
- At least one accessible TanStack Table and one ECharts surface are present.
- Protected controls fail closed when the authorization snapshot is absent,
  stale, denied, or unavailable.
- Sign-in and sign-up remain server-configured and never offer client role
  selection.
- Unit, accessibility, typecheck, and production build gates pass.
