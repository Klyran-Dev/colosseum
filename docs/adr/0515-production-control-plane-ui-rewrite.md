# ADR-0515: Production control-plane UI rewrite with server-owned authorization

- Status: Accepted
- Date: 2026-10-11
- Owners: Web platform and security engineering

## Context

The React surface had accumulated a large shell, several overlapping access
summaries, and authentication affordances that made the operator's current
authorization state difficult to understand. The UI must support sign-in,
sign-up, RBAC, ABAC, and PBAC without becoming an authority boundary or
implying that a hidden control is an authorization decision.

The requested change is a presentation rewrite, not a change to the Solana
settlement authority, the Rust authorization service, or the token contract.
Existing typed domain panels remain workflow adapters and are rendered inside a
new application shell.

## Decision

Replace the React presentation shell and authentication page with a
repository-owned, responsive control-plane UI. Keep the existing typed
authentication and authorization modules as integration boundaries:

1. OIDC, provider-backed email OTP, and local-only credential entry remain the
   configured authentication modes. Tokens remain memory-only in the browser.
2. The browser requests a short-lived, complete capability snapshot from the
   API. The snapshot is the only source used to label RBAC/PBAC/ABAC decisions
   in the UI. The browser never decodes JWT roles, invents role-to-scope maps,
   or accepts a role selector during sign-up.
3. Navigation presents every known workspace with a truthful access state:
   available, sign-in required, checking, denied, or unavailable. Protected
   children are not mounted until the server decision is fresh and the service
   is online.
4. Sign-in and sign-up share the same identity verification journey. Sign-up
   starts with standard operator access; invitations may display a
   server-confirmed responsibility but never make it client-selectable.
5. The shell exposes one global session control, one account context menu, and
   one protection panel. Authorization evidence, tenant context, roles, expiry,
   and risk level are visible without exposing bearer tokens or raw sensitive
   evidence.
6. The rewrite is a static React/Vite SPA. It preserves stable workspace paths,
   keyboard focus management, locale/time-zone preferences, RTL direction, and
   the repository-owned semantic color tokens.
7. The visual composition follows the supplied analytics-dashboard references:
   a persistent light rail, compact utility header, rounded metric cards,
   responsive chart surfaces, and lower activity cards. These surfaces are
   deliberately bound to authorization and service evidence; when verified
   business data is absent they render an explicit empty state rather than
   fabricated totals, trends, or events.

## Invariants

- No client-side UI state grants a capability.
- No protected action renders before a fresh server capability decision.
- No client-supplied merchant, tenant, role, wallet, or authority address is
  treated as authorization input.
- Authentication errors, expired access snapshots, service outages, and
  reconciliation-required states remain visible and recoverable.
- Sign-out clears memory-held credentials and authorization state.
- Wallet keys remain in wallets; the UI never asks for seed phrases or private
  keys.
- Amounts, tax values, chain counters, and evidence digests remain owned by
  the existing typed workflow panels and API contracts.

## Sequence and state model

```text
anonymous -> sign-in/sign-up route -> identity challenge -> signed-in
signed-in -> capability request -> server-allowed | denied | indeterminate
server-allowed + fresh snapshot -> workspace controls
server-allowed + expiry/outage -> indeterminate -> refresh or fail closed
any state -> sign-out -> anonymous
```

Sign-up carries only an invitation reference. The auth service binds email,
invitation status, tenant, role, and expiry; the browser only presents the
server result.

## Threat model

The UI treats URLs, local storage, form values, query strings, JWT contents,
navigation clicks, and invitation role labels as untrusted. It defends against
role spoofing, stale capability snapshots, confused-deputy tenant selection,
bearer-token disclosure, duplicate sign-in actions, unsafe return paths,
clickjacking-style ambiguity, and accessibility-hidden security state. The API
and Solana program remain responsible for final authorization, conservation,
replay, and settlement invariants.

## Budgets and acceptance criteria

- Shell first contentful render: p95 <= 150 ms after static assets are
  available on a warm browser; no network request is required to render the
  public simulator frame.
- Capability refresh: one bounded request per session refresh, with the
  existing API timeout and a maximum snapshot lifetime of 60 seconds.
- No unbounded DOM collections or client-side policy graph evaluation.
- 320 px reflow, keyboard navigation, visible focus, semantic landmarks, and
  WCAG 2.2 AA contrast remain release gates.
- Existing auth, authorization, panel, accessibility, and browser journeys
  must either pass unchanged or be updated with an explicit contract test for
  the new surface.
- Reference-style dashboard surfaces must expose accessible headings and
  server-derived counts for policy decisions, while empty audit activity must
  state that no verified activity is available.

## Rejected alternatives

- Decode roles and scopes from JWTs in the browser: rejected because claims do
  not prove tenant/resource/policy authorization and would drift from the
  server.
- Hide navigation items the user cannot currently use: rejected because it
  hides policy state, makes support harder, and creates a false sense that the
  product has no governed capability.
- Add a client role selector to sign-up: rejected because role assignment is a
  privileged server operation and must be invitation- or administrator-bound.
- Replace the API-owned auth flow with a hosted UI-only identity SDK: rejected
  because it would bypass the repository's OTP/OIDC, audit, and policy
  contracts.
- Rewrite all domain panels at once: rejected because it expands the change
  into money-moving code and obscures whether the presentation rewrite changed
  settlement behavior.

## Consequences

The shell becomes easier to audit and gives operators a consistent mental
model for identity, responsibility, resource attributes, policy decisions,
and action risk. The UI carries more state explanation and therefore requires
explicit copy and accessibility tests. The visual rewrite does not, by itself,
prove production authorization or legal tax compliance; those remain server,
program, and release gates.
