# ADR-0508: Keep the Metronic v9.5.0 composition source-bound and auditable

Status: Accepted  
Date: 2026-10-11

## Context

The licensed Metronic v9.5.0 package is available at
`/Users/rahul/development/metronic/metronic-v9.5.0`. Its React Demo 3
composition defines the visual structure requested for the product:

- a 58px header;
- a 58px compact rail that expands to a labeled navigation sidebar;
- a 56px secondary navbar with an underline for the active item;
- a fluid rounded content frame; and
- a two-column branded authentication layout with a focused card and a
  non-interactive security panel.

The repository is a React 19/Vite SPA with a repository-owned Material 3
semantic color contract and server-owned authorization. Copying the whole
Metronic application or importing its authentication provider would violate
those boundaries and would add unrelated demo dependencies.

## Decision

Use the following licensed source files as the composition reference:

- `metronic-tailwind-react-demos/typescript/vite/src/layouts/demo3/layout.tsx`
- `metronic-tailwind-react-demos/typescript/vite/src/layouts/demo3/components/header.tsx`
- `metronic-tailwind-react-demos/typescript/vite/src/layouts/demo3/components/sidebar.tsx`
- `metronic-tailwind-react-demos/typescript/vite/src/layouts/demo3/components/navbar.tsx`
- `metronic-tailwind-react-demos/typescript/vite/src/layouts/demo3/components/toolbar.tsx`
- `metronic-tailwind-react-demos/typescript/vite/src/auth/layouts/branded.tsx`

Adapt the composition into the repository-owned `ProductShell` and `AuthPage`
components. The DOM exposes `data-layout="demo3"`,
`data-container="fluid"`, `data-layout="auth-branded"`, and
`data-panel="brand"` so browser tests can prove which composition is active.

Keep the following out of the product bundle unless separately reviewed:

- Metronic demo routes, fake data, Supabase/Auth0 adapters, and demo-only
  dependencies;
- a second color-token system or raw decorative colors; and
- client-side role/capability decisions.

The application keeps its own OTP authentication service, localized semantic
tokens, accessibility behavior, and server-derived authorization snapshot.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Copy the complete Metronic React demo | Rejected | Imports unrelated demo behavior, fake data, and a second application architecture. |
| Use only a visual screenshot as inspiration | Rejected | Does not provide a reproducible source boundary or testable layout contract. |
| Import Metronic auth adapters and providers | Rejected | Conflicts with the repository's separate Rust OTP/OIDC service and would create a second identity authority. |
| **Adapt Demo 3 composition and preserve repository contracts** | **Selected** | Reproduces the requested layout while retaining the production auth, authorization, accessibility, and token boundaries. |

## Verification

- `AuthPage.test.tsx` proves the branded authentication layout contract.
- `productShell-ui.test.tsx` proves the Demo 3 shell, navbar, and content-frame
  contract.
- The existing browser, accessibility, Docker auth, and stylesheet-budget
  gates remain authoritative for the integrated surface.
