# ADR-0530: Resolve deployment context and present unnamed tenants truthfully

## Status

Accepted — 2026-10-13

## Context

The authorization catalog marks quote preparation as requiring the deployment
`environment` attribute. The API capability evaluator treated every required
attribute as unresolved, even though the API cannot start without a validated
`API_ENV` of `local`, `staging`, or `production`. As a result, a tenant
administrator with the `tax:quote` scope received
`indeterminate/resource_context_required` and the quote form was disabled.

The tenant lifecycle also intentionally persists `Untitled workspace` until a
tenant administrator names the workspace. That is a truthful readiness marker,
but exposing the internal marker in the persistent shell is poor product
language and makes the tenant appear incorrectly provisioned.

## Decision

1. Resolve `environment` as a server-known capability attribute. Keep selected
   resource attributes, including `jurisdiction`, indeterminate until the
   server has the required resource context.
2. Keep capability decisions server-owned and preserve handler-level scope,
   role, tenant, policy, and evidence checks. An administrator role never
   bypasses those checks.
3. Preserve the durable incomplete-profile marker for readiness accounting, but
   render it in the SPA as “Name your workspace” and use the existing audited
   settings mutation for naming. Do not infer a name from an email or write a
   hard-coded production tenant name.

## Invariants

- `tax.quote.create` is allowed in the snapshot only when `tax:quote` is
  present and the catalog's roles/scopes are satisfied; the quote handler still
  validates rules, evidence, tenant scope, and all financial inputs.
- A capability requiring an unresolved resource attribute remains
  `indeterminate`; the browser cannot convert it to `allow`.
- The database remains the authority for tenant identity and readiness.
- The visible workspace label never claims that an unnamed profile is complete.
- No browser token, tenant identifier, or role is persisted or trusted from UI
  state.

## Alternatives considered

See the capability-environment and unnamed-workspace comparison in
`docs/architecture/alternative-analysis.md`. Administrator bypass and
email-derived names were rejected because they weaken authority boundaries and
create unverifiable tenant identity.

## Verification

- Rust capability tests prove deployment environment resolves while
  jurisdiction remains resource-dependent.
- React tenant-model tests prove explicit names are preserved and the durable
  unnamed marker maps to the user-facing setup label.
- API, SPA typecheck, and targeted UI tests pass before release.
- The deployment health check and authenticated browser review must confirm the
  administrator can reach quote preparation while an unnamed workspace still
  shows an actionable profile setup state.
