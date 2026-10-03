# ADR-0262: Production SaaS authorization-aware UI shell

## Status

Accepted for the first production-grade UI surface. The server capability
snapshot contract is implemented by ADR-0264; real-browser evidence and live
identity-provider integration remain release gates.

## Context

The SPA already contains real quote, tax-rule, checkout, vault, adjustment,
refund, reconciliation, and replay workflows, but the presentation exposed
them as one long tab strip. That made the product look like a simplified
calculator, obscured the tenant/environment/security context, and did not give
operators a coherent way to understand RBAC, PBAC, and ABAC boundaries.

The browser is an untrusted client. Decoding JWT claims locally, accepting a
caller-supplied tenant, or treating a hidden button as authorization would be a
security defect. The UI must therefore be a high-quality affordance and
explanation layer while the Rust API remains the policy decision and
enforcement point.

## Decision

Build a single React 19.3 + TypeScript + Vite SPA shell with:

- a responsive sidebar grouped into Daily work, Rules & access, Money
  movement, and Records & issues;
- a server-bound workspace/environment/network context on every page;
- an explicit authorization posture strip and identity state;
- a versioned capability catalog describing action, scope, role, risk, and
  workspace for reviewability;
- a dedicated access-control workspace that explains RBAC (roles), PBAC
  (named capabilities/scopes), and ABAC (tenant, resource, environment,
  jurisdiction, and lifecycle attributes);
- preserved lazy loading, focus management, error containment, and real
  domain panels behind the new shell;
- visible loading, unavailable, indeterminate, unauthorized, and
  reconciliation-required states; no invented dashboard data or optimistic
  financial success;
- keyboard navigation, semantic landmarks, screen-reader labels, responsive
  narrow layouts, RTL-safe logical properties, and reduced-motion behavior.

The frontend authorization module has three strict rules:

1. no JWT decoding or locally trusted claims;
2. no caller-selected tenant or resource authority;
3. a token in memory is only `server-evaluation-required`, never `allow`.

The API now exposes a bounded, tenant-scoped capability snapshot. The pure
client evaluator may render an affordance from that server response, but it
must treat expired, malformed, failed, and resource-indeterminate results as
closed. Every protected API handler continues to enforce authentication, exact
scope, role/separation-of-duties, tenant ownership, resource state, policy
version, and step-up requirements. UI state never authorizes a Solana
transaction or vault release.

## Capability contract

`GET /v1/authorization/capabilities`, authenticated with the existing OIDC
bearer token, returns a bounded server-owned snapshot containing subject,
tenant, expiry, and resource/action decisions. It performs no storage, RPC,
broker, filesystem, or identity-provider call. The response is advisory for
rendering and is rechecked at the mutating API boundary. Unknown, expired,
mismatched, or unavailable snapshots render indeterminate/denied states and
never grant access. Resource-specific conditions remain indeterminate until a
protected workflow supplies the relevant context; see ADR-0264.

## Alternatives considered

- Keep the tab strip: rejected because it scales poorly, hides operating
  context, and makes risk boundaries difficult to understand.
- Decode JWT claims in React: rejected because an untrusted browser cannot
  prove issuer, audience, signature freshness, tenant binding, or resource
  authorization.
- Put authorization in a new frontend BFF: rejected because it adds a second
  authority and a latency/failure hop to wallet and tax flows.
- Split the app into micro-frontends: rejected because runtime dependency and
  policy drift would damage consistency and accessibility at this stage.
- Static mock dashboard metrics: rejected because it would imply live system
  state without evidence.

## Consequences

Positive:

- Operators can find the complete product surface and understand risk before
  acting.
- Policy intent is visible and testable without pretending the browser is a
  security boundary.
- Existing domain workflows remain independently deployable and code-split.
- Navigation adds no network round trip and does not enter the settlement hot
  path.

Costs and gates:

- The live identity provider, server-side PBAC/ABAC policy store, and
  resource-context integration must be evidenced before production claims
  about personalized navigation or authorization can be made.
- Real-browser responsive, screen-reader, visual regression, CSP, and
  multi-tenant authorization evidence remain required release gates.
- The catalog must be contract-tested against API scope/role constants to
  prevent UI drift.

## Verification

- `apps/web/src/authorization.test.ts` proves anonymous deny, token
  indeterminate behavior, trusted-snapshot RBAC/scope/tenant checks, and group
  organization.
- `apps/web/src/accessControl-ui.test.tsx` proves the RBAC/PBAC/ABAC and
  fail-closed copy is rendered.
- `apps/web/src/authorization-shell-fetch.test.tsx` proves the shell sends the
  bearer token to the capability endpoint and renders only the server result.
- `crates/api/src/lib.rs` tests prove authentication, exact scope/role
  decisions, resource-context indeterminacy, UUIDv7 identity, and bounded
  expiry.
- Existing axe-core, focus-management, lazy-workspace, and domain workflow
  tests remain green.
- `pnpm --dir apps/web build` is required for the production bundle.
