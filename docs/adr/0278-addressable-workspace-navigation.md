# ADR-0278: Addressable workspace navigation for the static SPA

## Status

Accepted — implemented and locally verified; staging and production evidence remain open.

## Context

The React shell currently changes workspaces in in-memory state. That is
adequate for a tab strip but not for a production operations product: an
operator must be able to bookmark a review surface, refresh it without losing
the selected workspace, share a non-sensitive support link, and use browser
back/forward navigation. The static Vite SPA is already served by NGINX with
an `index.html` fallback, so the missing contract is the browser route state,
not server-side rendering.

The route must not encode bearer tokens, financial facts, wallet keys,
authorization decisions, or calculation evidence. It must remain compatible
with the existing lazy workspace boundary and focus-management contract.

## Decision

Add a small typed route adapter with one stable path per workspace. It owns:

1. the allow-listed `ProductWorkspace` to pathname mapping;
2. safe fallback of `/` and unknown paths to the quote workspace;
3. `history.pushState` for user-selected navigation;
4. `history.replaceState` only for the initial canonicalization of `/` or an
   unknown path, while preserving OIDC callback parameters until the
   authentication callback handler consumes them; and
5. a bounded `popstate` listener that updates the selected workspace and lets
   the existing focus boundary move focus after React commits.

The URL is a navigation affordance only. Session state remains in memory,
financial input remains in workspace state, and every API operation continues
to re-authorize and bind its own tenant/resource/evidence context. The NGINX
`try_files ... /index.html` fallback is part of the deployment contract for
direct deep links.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Keep in-memory workspace state | Smallest implementation | No bookmark, refresh, support-link, or back/forward contract | Rejected |
| Hash fragments | Works without server fallback | Produces less useful URLs and complicates canonical support links and CSP review | Rejected |
| Add a full client router | Mature nested route/data APIs | Adds dependency and lifecycle surface beyond the bounded workspace requirement; can duplicate authority and focus transitions | Rejected |
| Server-rendered or Next.js routing | Rich navigation and SEO | Violates the static React/Vite/NGINX product boundary and adds a server runtime | Rejected |
| **Typed History API route adapter over the static SPA** | Stable shareable paths, no new runtime, bounded state, compatible with lazy loading and focus management | Requires an explicit allow-list and NGINX fallback contract | **Selected** |

## Invariants and failure behavior

- Only known workspace identifiers map to paths; unknown paths never select a
  hidden or arbitrary component.
- Route paths contain no bearer token, private key, amount, evidence, tenant
  identifier, authorization decision, or wallet account.
- User navigation creates one browser history entry; `popstate` never creates
  another entry.
- Route changes do not bypass authentication, capability checks, API
  re-authorization, expiry, or finalized chain evidence.
- Direct deep links load the static shell through NGINX and preserve the
  selected workspace.
- Existing skip-link, workspace-focus, lazy-import error, loading, and failure
  states remain the authority for the rendered content transition.

## Verification

- Route unit tests cover every workspace mapping, canonical root fallback,
  unknown-path fallback, and route non-disclosure of sensitive values.
- React tests cover push navigation and `popstate` workspace replacement.
- The Docker browser gate opens a direct deep link, verifies the workspace,
  uses back/forward navigation, and retains the accessibility audit.
- NGINX configuration tests continue to prove the SPA fallback and asset
  cache rules.

## Remaining gates

This closes the local navigation contract only. Staging support-link review,
localization review, screen-reader review, and production release-manifest
evidence remain open.
