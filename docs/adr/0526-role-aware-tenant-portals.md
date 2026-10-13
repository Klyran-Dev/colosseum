# ADR-0526: Role-aware tenant portals without client-selected authority

## Status

Accepted for implementation.

## Problem

The authenticated application currently presents the same control-plane shell
to a tenant administrator and a merchant operator. The authorization service
does distinguish the accounts: the administrator receives the non-assignable
`administrator` responsibility and additional scopes, while a normal signup
receives `merchant_operator`. The browser nevertheless renders the same
navigation, product title, and account description, so users reasonably infer
that the accounts are equivalent.

The product also has a third actor, the guest customer, who is not a merchant
operator and must use a short-lived public checkout session. A platform
administrator console does not exist in this slice and must not be implied by
the UI.

## Users and scope

In scope:

- derive a presentation-only portal mode from the server-owned capability
  snapshot and roles;
- label tenant administrators as tenant administrators;
- label merchant operators as merchant operations users;
- show a role-appropriate navigation surface using server decisions;
- keep authorization enforcement on every server endpoint unchanged;
- identify public customer checkout as a separate guest surface;
- provide loading, unavailable, stale, and denied states without optimistic
  client authorization.

Out of scope:

- creating a platform-admin role or platform-admin console;
- allowing the browser to select or request a role during login;
- changing tenant membership, role assignment, settlement authority, or tax
  calculations;
- merging users into a new merchant/workspace entity model.

## Invariants and trust boundaries

1. OTP authentication identifies a subject; it never selects authority.
2. The server-owned authorization snapshot is the only input to portal-mode
   presentation after authentication.
3. Navigation visibility is not a security boundary. Protected APIs repeat
   tenant, role, scope, resource, and policy checks.
4. A denied capability is never presented as available. An indeterminate or
   unavailable snapshot does not grant access.
5. The UI must not call a tenant administrator a platform administrator.
6. Guest checkout remains token-bound and wallet-bound; it does not inherit a
   merchant session.

## Sequence and state model

```text
OTP verified -> access token -> capability snapshot
                                      |
                         +------------+------------+
                         |                         |
                  administrator              merchant_operator
                         |                         |
                Tenant admin portal       Merchant operations portal
                         \                         /
                          +-- server-protected APIs

Guest checkout link -> public checkout session -> wallet authorization
```

Portal presentation states are `loading`, `server-confirmed`, `stale`,
`denied`, and `unavailable`. Only `server-confirmed` decisions may expose a
protected navigation entry as available.

## API and contract impact

No new authority-bearing API is introduced. The existing
`authorization-capabilities/v1` response remains authoritative and already
contains `roles`, `tenant_id`, expiry, and per-capability decisions. The web
client adds a pure presentation mapping from those fields to a portal mode and
filters navigation using the returned capability decisions. This is backward
compatible with existing snapshots.

## Alternatives

See the corresponding entry in `docs/architecture/alternative-analysis.md`.

## Latency and resource budget

- No additional network request: the layout reuses the existing TanStack Query
  snapshot request and cache key.
- Portal derivation and navigation filtering are O(n) over the bounded catalog
  (currently fewer than 16 navigation entries).
- No database, Solana RPC, signing, or settlement-path changes.
- Snapshot expiry and query failure remain fail-closed and visible.

## Threat model

- A malicious browser may hide or reveal navigation locally; this cannot grant
  server access because endpoint authorization is unchanged.
- A stale snapshot may mislead the user; expiry removes protected affordances
  and protected API requests continue to enforce current policy.
- A role string not present in the catalog is treated as a generic tenant
  member, never as administrator.
- A future `platform_admin` role is not silently mapped to tenant authority;
  it requires an explicit contract and separate portal decision.

## Acceptance criteria

- Administrator and merchant-operator snapshots produce different visible
  portal labels and navigation where their server decisions differ.
- No login form contains a role selector or client-controlled role parameter.
- Guest checkout remains visually and functionally separate from the tenant
  portal.
- Unit tests cover unknown roles, mixed roles, denied capabilities, stale
  snapshots, and unavailable snapshots.
- Existing Rust, web, Docker, contract, accessibility, and deployment gates
  remain green.
